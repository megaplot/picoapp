from __future__ import annotations

from ._memoize import Memoized
from ._types_element import Element

# What a layout accepts: an element, or a node producing one. `view` itself
# returns a plain `Element`.
ElementLike = Element | Memoized[Element]


class Layout(Element):
    def __init__(self, *children: ElementLike) -> None:
        for child in children:
            if not isinstance(child, (Element, Memoized)):
                raise TypeError(
                    f"{type(self).__name__} children must be elements or memoized "
                    f"nodes, got {type(child).__name__}"
                )
        self._children = children


class Row(Layout):
    """Arranges its children horizontally."""


class Column(Layout):
    """Arranges its children vertically."""
