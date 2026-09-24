"""Screenshot an X display (default :99) to a PNG using only python-xlib + pillow.

usage: shot.py OUT.png [DISPLAY]
"""

import sys

from PIL import Image
from Xlib import X, display  # type: ignore[import-untyped]


def main() -> None:
    out = sys.argv[1]
    d = display.Display(sys.argv[2] if len(sys.argv) > 2 else ":99")
    root = d.screen().root
    geo = root.get_geometry()
    raw = root.get_image(0, 0, geo.width, geo.height, X.ZPixmap, 0xFFFFFFFF)
    Image.frombytes("RGB", (geo.width, geo.height), raw.data, "raw", "BGRX").save(out)


if __name__ == "__main__":
    main()
