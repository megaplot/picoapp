"""Drive an X display (default :99) through XTEST: pointer moves, clicks, drags, keys.

usage:
  input.py click X Y
  input.py drag X1 Y1 X2 Y2 [STEPS]
  input.py move X Y
  input.py key KEYSYM            (e.g. Return, space, a)
Set DISPLAY_NAME to override the display.
"""

import os
import sys
import time

from Xlib import XK, X, display  # type: ignore[import-untyped]
from Xlib.ext import xtest  # type: ignore[import-untyped]

D = display.Display(os.environ.get("DISPLAY_NAME", ":99"))


def move(x: int, y: int) -> None:
    xtest.fake_input(D, X.MotionNotify, x=x, y=y)
    D.sync()


def button(down: bool) -> None:
    xtest.fake_input(D, X.ButtonPress if down else X.ButtonRelease, 1)
    D.sync()


def click(x: int, y: int) -> None:
    move(x, y)
    time.sleep(0.05)
    button(True)
    time.sleep(0.05)
    button(False)


def drag(x1: int, y1: int, x2: int, y2: int, steps: int = 12) -> None:
    move(x1, y1)
    time.sleep(0.05)
    button(True)
    for i in range(1, steps + 1):
        move(x1 + (x2 - x1) * i // steps, y1 + (y2 - y1) * i // steps)
        time.sleep(0.03)
    button(False)


def key(name: str) -> None:
    code = D.keysym_to_keycode(XK.string_to_keysym(name))
    xtest.fake_input(D, X.KeyPress, code)
    xtest.fake_input(D, X.KeyRelease, code)
    D.sync()


if __name__ == "__main__":
    cmd, args = sys.argv[1], sys.argv[2:]
    if cmd == "click":
        click(int(args[0]), int(args[1]))
    elif cmd == "move":
        move(int(args[0]), int(args[1]))
    elif cmd == "drag":
        drag(*(int(a) for a in args[:4]), *(int(a) for a in args[4:5]))
    elif cmd == "key":
        key(args[0])
