from collections.abc import Callable

from . import _picoapp
from ._engine import Engine
from ._memoize import memoize
from ._types_element import Element


def run(view: Callable[[], Element]) -> None:
    """Opens the app window; `view` arranges inputs, outputs and nodes."""
    _picoapp.run(Engine(memoize(view)))
