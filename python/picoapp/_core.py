from collections.abc import Callable

from ._engine import Engine
from ._memoize import memoize
from ._types_element import Element


def run(view: Callable[[], Element]) -> None:
    """Opens the app window; `view` arranges inputs, outputs and nodes."""
    # Imported here, not at module level: the extension links system libraries
    # (libasound, libxkbcommon), and `import picoapp` should keep working on a
    # system without them, e.g. to test an app's nodes headlessly. See
    # `ai/notes/issues_with_linking_and_shipping.md`.
    from . import _picoapp

    _picoapp.run(Engine(memoize(view)))
