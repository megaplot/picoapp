# UI QA without a desktop session

A run that merely "opens without crashing" cannot catch rendering or
interaction bugs (it missed a shutdown deadlock, non-updating checkboxes,
an unclickable int slider, ...). These tools let an agent — or you — see and
drive the real UI headlessly. Two backends exist; **pick by what you are
testing**:

| I want to check ...                                              | Use                | Can click/drag? |
|------------------------------------------------------------------|--------------------|-----------------|
| layout, styling, plots, interactions, callback rates, audio, ... | **X11** (Xvfb)     | yes             |
| window decorations / header bar, Wayland-specific behavior        | **Wayland** (private GNOME Shell) | no (screenshot only) |
| comparison with another install (e.g. the old cushy version)      | either (see below) | X11: yes        |

Default to X11: it is the only one that can send input. Use the Wayland
path only for anything the X11 path cannot show, mainly decorations: under
X11 a window manager draws the frame (Xvfb has none, so there is no frame),
while on GNOME/Wayland the compositor refuses to draw decorations and picoapp's
own `HeaderBar` appears; only the Wayland path shows that fallback.

## X11 path (Xvfb + XTEST)

gpui uses its X11 backend when `WAYLAND_DISPLAY` is unset.

```sh
scripts/qa/xvfb_start                         # Xvfb on :99
uv pip install python-xlib pillow             # into ./venv (screenshot + input helpers)
scripts/qa/run_example.sh python examples/example_1.py /tmp/shot.png 6     # start, screenshot, kill
scripts/qa/session.sh example_1 /tmp/e1 "shot:0;click:30,335;wait:1;shot:checked;drag:167,100,260,100;shot:dragged"
python scripts/qa/input.py click 30 335       # ad-hoc: click | drag | move | key
python scripts/qa/shot.py /tmp/shot.png       # ad-hoc screenshot of :99
```

Getting Xvfb without root (Ubuntu): `apt-get download xvfb && dpkg -x xvfb_*.deb ~/.local/xvfb`
(`xvfb_start` looks there). Nothing else needs installing: screenshots and input
use pure-Python `python-xlib` (no ImageMagick/xdotool needed).

Tips and pitfalls:
- **One app at a time.** There is no window manager, so all windows share the
  screen and parallel runs overlap in the screenshot.
- Coordinates are screen pixels; take a screenshot first and read positions off
  it. The app window starts at (0,0).
- Count callback invocations from the log: `session.sh` runs Python unbuffered,
  and the examples print in their callbacks (e.g. `example_nested_func`).
- Mesa prints an EGL/DRI3 warning on stderr under Xvfb. That is an Xvfb
  artifact (picoapp itself must print nothing).

## Wayland path (private headless GNOME Shell)

For decorations. `gnome_shot.sh` starts a private GNOME Shell on its own D-Bus
session with a virtual monitor (`gnome-shell` must be installed) and screenshots
through the shell's own Screenshot API (allowed because of `--unsafe-mode`):

```sh
scripts/qa/gnome_shot.sh python examples/example_1.py /tmp/shot.png 8
scripts/qa/gnome_shot.sh /path/to/other/venv/bin/python examples/example_1.py /tmp/other.png 9
```

Why not the obvious things (all tried on GNOME 49 / Wayland, none work):
`grim` (needs wlr-screencopy, which mutter lacks), the Shell screenshot D-Bus
API and the XDG screenshot portal *of the real session* (access denied / needs
a user prompt), and gpui's `Window::render_to_image` (test builds only, not
implemented for the Linux backend).

Limitation: no input injection here (no ydotool/wtype installed; the Mutter
RemoteDesktop D-Bus API would be the way, not attempted). Drive interactions
through the X11 path instead.

## Comparing with another picoapp install

Pass that install's python as the first argument of `run_example.sh` /
`gnome_shot.sh`. The old cushy version uses wgpu and needs software
presentation under Xvfb: `export VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json`.

## What is not covered

macOS and Windows (native title bars, Metal/DirectX rendering) cannot be
tested from here.
