I haven't changed any code yet. Answers in order:

**1. How the cushy era handled this**

It didn't. The cushy-era wheels bundled `libasound` the same way:

| wheel         | bundled libs                                                |
| ------------- | ----------------------------------------------------------- |
| 0.1.0         | none (audio support came after 0.1.0)                       |
| 0.2.0         | `libasound`                                                 |
| 0.2.1         | `libasound` + dbus/systemd and their deps                   |
| 0.3.0 / 0.4.0 | `libasound`, `libxkbcommon(-x11)`, `libxcb(-xkb)`, `libXau` |

The 0.2.0/0.2.1 libasound was built on CentOS 7 and has the same `/usr/lib64/alsa-lib` plugin path. I ran the real 0.2.1 wheel with its own `example_freq.py` on your machine. It shows the same bug: no PipeWire stream and the same `ALSA lib ... Cannot open shared library libasound_module_pcm_pipewire.so` errors. The wheels were manylinux-compliant back then, which is exactly what caused this. I found no ALSA-related TODOs or issue links in the history (searched all 191 commits and the 0.2.1 tree). My guess is you were running a `maturin develop` build, which links the system libasound and works correctly.

**2. How likely the libs are present**

On a desktop, practically always. On your Ubuntu, `libasound2t64` has 29 installed reverse dependencies (PipeWire, SDL, …) and `libxcb1` has 42 (Chrome, Mesa GLX, mutter, libpulse, Qt). `libxkbcommon-x11` itself links `libxcb` and `libxcb-xkb`, and `libX11` links `libxcb`.

The realistic gaps are headless environments: Docker slim images, minimal servers, some CI images, probably minimal WSL. I believe `python:3.x-slim` has none of them, but that's from memory, not checked here. A desktop missing them is unlikely but possible, e.g. a pure-Wayland Arch setup without `libxkbcommon-x11`.

The bundling was never complete anyway. The extension already loads `libvulkan`, `libEGL` and `libwayland-client` from the system at runtime (via dlopen, so auditwheel never sees them). On a system without the desktop stack, `pa.run()` can't open a window today either.

**3. What happens if a lib is missing (option 1)**

`import picoapp` fails with `ImportError` for the whole package. I simulated it with patchelf: `ImportError: libasound_missing.so.2: cannot open shared object file`. The cause is that `_core.py` imports `_picoapp` at module level, and libasound is linked into that one extension. So it isn't limited to Audio: without libasound no app can start, even one with no audio output.

Two points put this in perspective:
- The only systems this newly breaks are ones that have a display, Vulkan/EGL and xkbcommon but no libasound. That's very rare.
- The one real regression is headless environments where `import picoapp` works today, for example tests of user code. Importing `_picoapp` lazily inside `run()` fixes that. Everything except `pa.run()` would then import anywhere, and `pa.run()` fails with a clear error.

Making audio truly optional would need libasound to be loaded with dlopen instead of linked. That isn't available: `alsa-sys` has no such feature (its only feature is `use-bindgen`), and neither does `xkbcommon` 0.8.

**4. The standard way to get "compliant and working"**

Bundling is the standard for self-contained libs: compression, codecs, math. It goes wrong for libs that load system plugins, config or data files, because then the bundled copy and the system's files don't match. Here that's two cases:
- `libasound`: plugins and `/usr/share/alsa` config.
- `libxkbcommon`: `/usr/share/X11/locale/.../Compose`. The `dead_hamza` errors are an old bundled xkbcommon failing to parse a newer Compose file.

The usual practice for this kind of lib is to load the system's copy at runtime. That's why `libGL` is on the manylinux allow list rather than bundled. It's also what gpui does for Vulkan/Wayland/EGL and fontconfig, and as far as I know SDL does the same with its audio backends. With these crates, that choice isn't available to us.

On option 3 (pointing the bundled lib at the system's files), I tried it with the 0.4.0 wheel and `ALSA_PLUGIN_DIR=/usr/lib/x86_64-linux-gnu/alsa-lib`. Routing works, but the process then has two alsa-libs loaded: the bundled 1.2.10 and the system's 1.2.15. That's because the PipeWire plugin itself links `libasound.so.2`. The problems:
- **Version mixing:** a PCM opened by 1.2.10 is driven by a plugin running against 1.2.15. It works today only because the plugin ABI happened to stay compatible.
- **Config written for a newer lib:** the system config is meant for the system's alsa-lib version. A distro with newer syntax could break the old bundled parser.
- **Discovery:** finding the plugin dir is doable (find the system `libasound.so.2` and take the `alsa-lib` dir next to it). But layouts differ: Debian multiarch, Fedora `/usr/lib64`, Arch `/usr/lib`, NixOS store paths. And the discovery would be done by Python code the bundled lib doesn't know about.
- **xkbcommon:** none of this fixes it. It would need its own env var workaround, or giving up Compose support.

**5. Where option 2 sits**

Compliance in auditwheel's sense is all-or-nothing: every linked lib must be either bundled or on the allow list. Option 2 leaves `libasound` and `libxkbcommon` unbundled, so it's exactly as non-compliant as option 1. Keeping `libxcb` bundled gains nothing either. The system `libxkbcommon-x11` links the system `libxcb`, so you'd load two `libxcb` copies, and `libxcb` is the one lib that is basically never missing. So option 2 is option 1 plus extra CI code plus duplicate libs.

**Recommendation:** option 1, plus the lazy `_picoapp` import so `import picoapp` works headless. It also makes the wheel smaller. The cost is that a wheel tagged manylinux depends on three common desktop libs, which is the same situation the dlopened Vulkan/EGL/Wayland libs are already in. I'd document that dependency in `notes.md` and the README.

Should I go ahead with option 1 and the lazy import, or do you want to look at anything else first?