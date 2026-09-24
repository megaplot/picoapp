# Headless UI QA

picoapp's UI can be exercised without a desktop session (and without
Wayland screenshot permissions) by running it on a virtual X server. gpui
uses its X11 backend when `WAYLAND_DISPLAY` is unset.

```sh
scripts/qa/xvfb_start                       # Xvfb on :99 (see script for a no-root install)
uv pip install python-xlib pillow           # screenshot + input helpers
scripts/qa/run_example.sh python examples/example_1.py /tmp/shot.png 6
python scripts/qa/input.py click 30 335     # XTEST click; also: drag, move, key
python scripts/qa/shot.py /tmp/shot.png     # screenshot of :99
```

Notes:
- Run examples one at a time: without a window manager all windows share
  the same screen, so parallel runs overlap in the screenshot.
- Xvfb has no DRI3, so Mesa prints an EGL warning on stderr; that is an
  Xvfb artifact, not picoapp output.
- Client-side window decorations are a Wayland feature; under Xvfb only
  the in-app `TitleBar` is visible, so check the real window controls on
  an actual desktop.
- To compare with another picoapp install (e.g. the cushy version), pass its
  python; with NVIDIA drivers set `VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json`
  for wgpu-based apps that need software presentation under Xvfb.
