# Issues with linking and shipping picoapp (Linux)

Findings from investigating "Audio plays on the HDMI monitor speakers instead of
the system default device, and prints ALSA/xkbcommon errors" (2026-10-18 to
2026-10-29, picoapp 0.4.0). Earlier, build-time oriented notes (pkg-config,
cross-compiling, alsa-sys) are in `notes.md`. The raw chat answer this file
grew out of is `ai/notes/issues_with_linking__raw_anwer.md`.

## TL;DR

- The PyPI Linux wheels bundled (via maturin's auditwheel `repair`)
  `libasound`, `libxkbcommon`, `libxkbcommon-x11`, `libxcb`, `libxcb-xkb`,
  `libXau`. The bundled `libasound` and `libxkbcommon` were built on
  AlmaLinux 8 (manylinux_2_28) and don't match the plugins/config/data files of
  the system they run on. That caused the wrong audio device and all of the
  stderr noise. Not a rodio/cpal bug.
- Bundling is the manylinux standard, but it is the wrong tool for libraries
  that are *clients of system services* and load system plugins or data files
  at runtime (ALSA, xkbcommon, GL/Vulkan drivers, fontconfig).
- The clean approach (what fastplotlib's stack does) is to link nothing but
  glibc and load all desktop libraries at runtime with `dlopen`. picoapp can do
  that for `libxcb` today (x11rb's `dl-libxcb` feature), but not for
  `libasound` (`alsa-sys` links it) or `libxkbcommon(-x11)` (gpui uses the
  linking `xkbcommon` crate).
- Decision for 0.4.1: stop bundling (`--auditwheel warn`, keeping the
  `manylinux_2_28` tag and its glibc check), load `libxcb` at runtime, and import
  the extension lazily so `import picoapp` works without those libraries. The
  wheels now require `libasound.so.2`, `libxkbcommon.so.0` and
  `libxkbcommon-x11.so.0` from the system.

## Symptoms (PyPI wheel 0.4.0, Ubuntu with PipeWire)

- Audio didn't go to the default sink. PipeWire never saw a stream from the
  app (`wpctl status`, "Streams"). The user heard it on the HDMI monitor.
- On stderr, on every run:
  - `ALSA lib dlmisc.c:341:(snd_dlobj_cache_get0) Cannot open shared library
    libasound_module_pcm_pipewire.so (/usr/lib64/alsa-lib/libasound_module_pcm_pipewire.so: ...)`,
    the same for the `jack`, `oss`, `pulse`, `speex`, `upmix`, `vdownmix`
    plugins.
  - `xkbcommon: ERROR: /usr/share/X11/locale/en_US.UTF-8/Compose:1661:1:
    unrecognized keysym "dead_hamza" on left-hand side` (many lines).
- A `maturin develop` build, and a local wheel built without bundling, show
  neither: the stream goes to the default sink and nothing is printed.

## Root cause

### libasound

- The bundled `picoapp.libs/libasound-0ceed182.so.2` is alsa-lib 1.2.10 from
  AlmaLinux 8. Its plugin directory is compiled in: `/usr/lib64/alsa-lib`.
  Ubuntu's plugins are in `/usr/lib/x86_64-linux-gnu/alsa-lib` (Debian
  multiarch). The system had alsa-lib 1.2.15.
- The bundled lib still reads the *system* configuration (`/usr/share/alsa`,
  `/etc/alsa/conf.d/99-pipewire-default.conf`), which defines the `default`
  PCM as the PipeWire plugin. Loading the plugin fails (wrong directory), so
  opening `default` fails.
- rodio's `DeviceSinkBuilder::open_default_sink` then falls back to
  enumerating all cpal output devices and takes the first one whose driver
  isn't `null` (`rodio-0.22.2/src/stream.rs`). Here that's a raw `hw:` device
  of the NVidia HDMI output. (Mechanism inferred from the rodio source plus the
  observed behavior; the exact device picked wasn't logged.) The device
  enumeration is also what prints the bulk of the `ALSA lib` lines.

### libxkbcommon

- The bundled `libxkbcommon` is older than the system's X11 locale data and
  doesn't know the `dead_hamza` keysym used in Ubuntu's Compose file. The
  errors are harmless for typing in picoapp but are real errors, not warnings
  to be silenced.

### The general pattern

Both libraries ship code/config/data *outside* the `.so`: plugins, config
files, keymaps/Compose tables. A bundled copy carries the build machine's
paths and version expectations, and the files it finds belong to a different
version. Self-contained libraries (compression, codecs, math) bundle fine.
This is also why the manylinux policy allow-lists `libGL` instead of bundling
it.

## History: it was always like this

| wheel | tag | bundled libs |
|---|---|---|
| 0.1.0 | manylinux2014 | none (no audio yet) |
| 0.2.0 | manylinux2014 | `libasound` |
| 0.2.1 | manylinux2014 | `libasound`, `libdbus-1`, `libsystemd` and their deps (lz4, lzma, elf, gcrypt, selinux, pcre, ...) |
| 0.3.0, 0.4.0 | manylinux_2_28 | `libasound`, `libxkbcommon`, `libxkbcommon-x11`, `libxcb`, `libxcb-xkb`, `libXau` |

The cushy-era (0.2.x) `libasound` was built on CentOS 7 and has the same
compiled-in `/usr/lib64/alsa-lib`. The real 0.2.1 wheel, with its own
`examples/example_freq.py`, reproduces the bug on the same machine today (no
PipeWire stream, same `ALSA lib` errors). The wheels were manylinux-compliant,
and that compliance is exactly what caused the bug. It most likely went
unnoticed because development used `maturin develop`, which links the system
libraries.

## What the extension needs from the system

`readelf -d` of `_picoapp.*.so` (built without bundling), 0.4.0:

| library | how | pulled in by | needed when |
|---|---|---|---|
| `libasound.so.2` | linked (`NEEDED`) | `alsa-sys` ← `alsa` ← `cpal` ← `rodio` | always (load time) |
| `libxkbcommon.so.0` | linked | `xkbcommon` 0.8 ← `gpui-pre-linux` | always |
| `libxkbcommon-x11.so.0` | linked | same (`x11` feature) | always |
| `libxcb.so.1` | linked | `x11rb` `allow-unsafe-code` ← `gpui-pre-linux` | always |
| `libwayland-client.so.0`, `libwayland-egl.so.1` | `dlopen` | `wayland-sys` with its `dlopen` feature | only on the Wayland backend |
| `libvulkan.so.1`, `libEGL.so.1` | `dlopen` | wgpu | when rendering starts |
| fontconfig | `dlopen` | gpui's font stack (see `ai/wheel_size.md`) | when fonts load |

- gpui picks the backend at runtime (`gpui::guess_compositor()`): Wayland if
  `WAYLAND_DISPLAY` is set, else X11 if `DISPLAY` is set, else headless. Pure
  X11 systems never touch the Wayland libraries.
- The asymmetry goes the other way: the X11 libraries are linked, so a pure
  Wayland system needs `libxkbcommon-x11` (and `libxcb`, before 0.4.1) even
  though it never uses X11.
- A *linked* library that is missing makes loading the extension fail:
  `ImportError: libasound.so.2: cannot open shared object file` (simulated by
  renaming the `NEEDED` entry with `patchelf --replace-needed`). Up to 0.4.0
  `_core.py` imported `_picoapp` at module level, so this broke
  `import picoapp` as a whole, even for apps without audio. Making only audio
  optional would need `libasound` loaded with `dlopen` (see below).
- auditwheel only sees `NEEDED` entries. "Bundled and compliant" never meant
  "self-contained": Vulkan/EGL/Wayland/fontconfig always came from the system.

## How likely are the libraries present?

- Desktop Linux: practically always. On the Ubuntu machine used here,
  `libasound2t64` has 29 installed reverse dependencies (PipeWire, SDL, ...),
  `libxcb1` 42 (Chrome, Mesa GLX, mutter, libpulse, Qt). `libxkbcommon-x11`
  itself links `libxcb` and `libxcb-xkb`; `libX11` links `libxcb`.
- Realistic gaps: headless environments, i.e. Docker slim images (believed to
  have none of them, not verified), minimal servers, some CI images, probably
  minimal WSL installs. Also possible: a pure-Wayland desktop without
  `libxkbcommon-x11` (e.g. a minimal Arch + sway setup).
- Headless environments can't open a picoapp window anyway (no display, no
  Vulkan/EGL). What matters there is that `import picoapp` keeps working, e.g.
  for tests of user code. Hence the lazy import.

## Options evaluated

### 1. Don't bundle (chosen)

`--auditwheel warn` in `deploy.yml`'s Linux job. maturin-action already
passes `--manylinux 2_28` (it never adds `--compatibility`/`--auditwheel`
itself; `PyO3/maturin-action` `src/index.ts`).

- Uses the system's libasound/xkbcommon, so its plugins/config/data match.
- Keeps the `manylinux_2_28` tag, and keeps the glibc symbol-version check:
  `--auditwheel warn --compatibility manylinux_2_28` on a host with a newer
  glibc fails with "not manylinux_2_28 compliant because of the presence of
  too-recent versioned symbols". It only stops bundling and failing on
  external libraries.
- Strictly speaking the wheel violates the manylinux policy (it depends on
  libraries outside the allow list). PyPI only validates the tag, not the
  dependencies.
- Smaller wheel.

maturin mechanics, for reference:

| flags | result |
|---|---|
| default (`repair`) | bundles non-allow-listed libs into `picoapp.libs/` |
| `--auditwheel skip` | no checks, tag `linux_x86_64` unless `--compatibility` is given; PyPI rejects `linux_*` |
| `--auditwheel warn` | warns, no bundling; without an explicit compatibility it falls back to `linux_x86_64` ("No compatible platform tag found") |
| `--auditwheel warn --compatibility manylinux_2_28` | `manylinux_2_28` tag, glibc check enforced, no bundling |

maturin 1.15 has no per-library exclude.

### 2. Bundle everything except libasound/xkbcommon

`--auditwheel skip`, then run `auditwheel repair --exclude ...` ourselves.

- Same policy violation as option 1 (compliance is all-or-nothing per
  `NEEDED` entry), more CI code.
- Keeping `libxcb` bundled gains nothing: the system `libxkbcommon-x11` links
  the system `libxcb`, so two `libxcb` copies would be loaded.

### 3. Keep bundling, point the bundled lib at the system files

`ALSA_PLUGIN_DIR=/usr/lib/x86_64-linux-gnu/alsa-lib` with the 0.4.0 wheel:
routing works and the ALSA errors disappear, but:

- The process then has *two* alsa-libs loaded: bundled 1.2.10 and the
  system's 1.2.15, because `libasound_module_pcm_pipewire.so` itself links
  `libasound.so.2` (verified via `/proc/<pid>/maps`). A PCM opened by one
  version is driven by a plugin running against the other. Works only as long
  as the plugin ABI stays compatible.
- The system config is written for the system's alsa-lib version. Newer
  syntax could break the old bundled parser.
- Discovery of the plugin dir is doable (locate the system `libasound.so.2`,
  take the `alsa-lib` dir next to it), but layouts differ: Debian multiarch,
  Fedora `/usr/lib64`, Arch `/usr/lib`, NixOS store paths. The bundled lib
  would be configured by Python code it doesn't know about.
- Doesn't help xkbcommon, which needs its own workaround (or giving up Compose
  support).

### 4. Link nothing but glibc, load everything at runtime (the clean one)

What fastplotlib's stack does (wheels inspected 2026-10-29):

| wheel | native lib | `NEEDED` | loaded at runtime |
|---|---|---|---|
| fastplotlib 0.7.0, pygfx 0.17.0, rendercanvas 2.7.2 | none (pure Python) | - | - |
| wgpu 0.32.0 (`manylinux_2_28`) | `libwgpu_native-release.so` | glibc only | Vulkan etc. |
| glfw 2.11.0 (`manylinux2014`) | `x11/libglfw.so`, `wayland/libglfw.so` | glibc only | X11/Xcursor/Xrandr/..., or wayland-client/xkbcommon, plus EGL/GL/Vulkan |

glfw ships one build per window system, and its Python side picks one via
`ctypes.CDLL`. Fully manylinux-compliant *and* using the system libraries;
`import` works headless; a missing library only fails when that backend
starts. fastplotlib has no audio.

For picoapp:

| library | runtime loading available? |
|---|---|
| `libxcb` | yes: x11rb's `dl-libxcb` feature, enabled from our `Cargo.toml` via feature unification. Verified: `libxcb` disappears from `NEEDED`. Adopted in 0.4.1. |
| `libxkbcommon(-x11)` | no: `xkbcommon` 0.8 has no such feature. The runtime-loading equivalent `xkbcommon-dl` exists (used by winit/smithay-client-toolkit), but gpui would have to switch to it (upstream, or a fork via `[patch]`). |
| `libasound` | no: `alsa-sys` 0.4.0's only feature is `use-bindgen`. Would need an upstream feature in `alsa-sys`/`cpal`, or a different audio output path. |

Once both land, picoapp could go back to strict compliance without bundling.

### 5. Static linking / vendoring alsa-lib (alsa-sys#10)

[diwic/alsa-sys#10](https://github.com/diwic/alsa-sys/issues/10) "Feature
request: allow static linking to libasound" (open since 2021, `help wanted`
since 2026-06). The maintainer would accept an opt-in feature via PR, and
suggested building alsa-lib from source like dbus-rs's `vendored` feature
([diwic/dbus-rs#408](https://github.com/diwic/dbus-rs/pull/408)). Nobody has
done it. `dlopen` isn't mentioned in the thread.

It would help the *build-time* problems `notes.md` describes (cross-compiling
without the target's `libasound2-dev`, pkg-config setup), and it would make the
wheel "compliant" again since `libasound` would leave `NEEDED`. It would not
fix this runtime bug, and would arguably make it worse:

- A statically linked alsa-lib has the same compiled-in plugin/config paths
  as a bundled one, so it fails to find the system's plugins the same way.
- System plugins (PipeWire, PulseAudio) link the system `libasound.so.2`, so
  loading one would again put two alsa-lib versions in the process (option 3).
- alsa-lib is LGPL-2.1 (general knowledge, not checked here). Static linking
  comes with relinking obligations that dynamic linking doesn't.

## Verified for 0.4.1 (local wheel from `scripts/build_wheel`)

- No `picoapp.libs/`. `NEEDED`: `libasound.so.2`, `libxkbcommon.so.0`,
  `libxkbcommon-x11.so.0` plus glibc/libgcc (`libxcb` gone).
- Wayland and X11 (`WAYLAND_DISPLAY` unset) backends both start. On X11 the
  system's `libxcb*` show up in `/proc/<pid>/maps`, loaded at runtime.
- The stream links to the current default sink (checked via `pw-dump` link
  objects; `wpctl`'s nicks like "USB Audio #1" are ambiguous). Nothing on
  stdout/stderr.
- `import picoapp` doesn't load the extension (`tests/test_run.py`).
- Wheel size 17,987,139 → 17,245,744 bytes (`ai/wheel_size_history.md`).

## Reproducing

```sh
# Released wheel vs. a wheel without bundling, same example
uv venv -p 3.12 v && uv pip install -p v/bin/python picoapp==0.4.0
v/bin/python examples/example_freq.py   # stderr: ALSA lib / xkbcommon errors
wpctl status                            # "Streams": no entry for python

maturin build --release --auditwheel skip --interpreter python3.12 --out dist_skip
# install that one instead: stream shows up on the default sink, no output

# What the extension links vs. loads at runtime
readelf -d picoapp/_picoapp*.so | grep NEEDED
strings picoapp/_picoapp*.so | grep -E '^lib.*\.so'
unzip -l picoapp-*.whl | grep picoapp.libs   # bundled libs

# Which alsa-libs a running process has loaded
grep libasound /proc/<pid>/maps
```
