"""The evaluation engine, driven by the Rust worker thread.

The engine decides which visible slot to bring up to date next. A *slot* is a
`Memoized` placed in a layout (or the root). The engine mirrors what the UI
shows: `_shown` holds each visible slot's last good content (an error result
keeps the previous content on screen), and visibility is computed from the
root through `_shown`. The UI computes the same from the same results.
"""

from __future__ import annotations

import traceback
from collections.abc import Iterator, Sequence
from types import TracebackType
from typing import NamedTuple

from ._memoize import Memoized
from ._types_element import Element
from ._types_inputs import InputBase, _inputs_by_id
from ._types_layout import Column, Layout


class SlotResult(NamedTuple):
    node_id: int
    version: int
    # The slot's element, or an error message (message + traceback).
    content: Element | str


class _Visible(NamedTuple):
    # Visible slots in tree order (pre-order: a slot before the slots inside it).
    slots: list[Memoized[object]]
    # Ids of everything placed directly in each visible slot's shown content.
    placed: dict[int, list[int]]


class _Undo(NamedTuple):
    """The state before a step, so `reject` can restore it."""

    node_id: int
    shown: dict[int, Element]
    sent: dict[int, int]
    waiting: dict[int, tuple[int, Element | None]]


class Engine:
    def __init__(self, root: Memoized[Element]) -> None:
        self._root = root
        # Last good content per visible slot, as shown by the UI.
        self._shown: dict[int, Element] = {}
        # Node version of the last result sent per visible slot.
        self._sent: dict[int, int] = {}
        # Slots whose last result was a duplicate error caused by another
        # slot, with that slot's shown content at the time. The other slot
        # may just be moving the item away, so the check is repeated once its
        # content changes.
        self._waiting: dict[int, tuple[int, Element | None]] = {}
        self._last_step: _Undo | None = None

    @property
    def root_id(self) -> int:
        return self._root._id

    def set_values(self, changes: Sequence[tuple[int, object]]) -> None:
        """Writes values the UI sent, as `(input_id, raw_value)` pairs.

        Ids of inputs that no longer exist are ignored.
        """
        for input_id, raw in changes:
            input = _inputs_by_id.get(input_id)
            if input is not None:
                input._write_ui_value(raw)

    def stale_visible_slots(self) -> list[int]:
        """Ids of visible slots that the next `step`s will update."""
        return [node._id for node in self._visible().slots if self._needs_step(node)]

    def step(self) -> SlotResult | None:
        """Brings the highest-priority visible slot up to date and returns it.

        Returns `None` when every visible slot is up to date and sent.
        """
        node = self._next_slot()
        if node is None:
            self._last_step = None
            return None
        node._ensure_fresh()
        self._last_step = _Undo(
            node._id, dict(self._shown), dict(self._sent), dict(self._waiting)
        )
        self._waiting.pop(node._id, None)
        content = self._slot_content(node)
        self._sent[node._id] = node._version
        if isinstance(content, Element):
            self._shown[node._id] = content
        self._prune()
        return SlotResult(node._id, node._version, content)

    def reject(self, node_id: int) -> None:
        """Called when the UI could not display the last step's content.

        Restores the state before that step: the UI keeps showing the slot's
        previous content, and with it the slots inside. The slot itself counts
        as sent, so the unparsable content isn't sent again.
        """
        undo = self._last_step
        if undo is None or undo.node_id != node_id:
            return
        sent_version = self._sent[node_id]
        self._shown, self._sent, self._waiting = undo.shown, undo.sent, undo.waiting
        self._sent[node_id] = sent_version
        self._last_step = None

    def _needs_step(self, node: Memoized[object]) -> bool:
        if node._maybe_stale() or self._sent.get(node._id) != node._version:
            return True
        waiting = self._waiting.get(node._id)
        return waiting is not None and self._shown.get(waiting[0]) is not waiting[1]

    def _next_slot(self) -> Memoized[object] | None:
        """Priority: the root, then fragments, then leaves, each in tree order."""
        pending = [node for node in self._visible().slots if self._needs_step(node)]
        for node in pending:
            if node is self._root:
                return node
        for node in pending:
            if not node._evaluated or isinstance(node._value, Layout):
                return node
        return pending[0] if pending else None

    def _slot_content(self, node: Memoized[object]) -> Element | str:
        if node._error is not None:
            return _format_error(node._error, node._error_tb)
        value = node._value
        is_root = node is self._root
        if is_root and isinstance(value, Element) and not isinstance(value, Layout):
            value = Column(value)
        if not isinstance(value, Element):
            return (
                f"node `{node._name}` returned {type(value).__name__}, "
                "expected an Element"
            )
        duplicate = self._find_duplicate(node, value)
        if duplicate is not None:
            item, other_slot = duplicate
            if other_slot is not None:
                self._waiting[node._id] = (other_slot, self._shown.get(other_slot))
            return f"{item} appears more than once in the view"
        return value

    def _find_duplicate(
        self, node: Memoized[object], content: Element
    ) -> tuple[str, int | None] | None:
        """The first item placed twice, and the other slot placing it (if any)."""
        visible = self._visible()
        # Placed item id -> the slot placing it (`None`: not another slot).
        taken: dict[int, int | None] = {
            placed_id: slot_id
            for slot_id, ids in visible.placed.items()
            if slot_id != node._id
            for placed_id in ids
        }
        taken[self._root._id] = None
        taken[node._id] = None
        for item in _placed(content):
            if item._id in taken:
                return repr(item), taken[item._id]
            taken[item._id] = None
        return None

    def _visible(self) -> _Visible:
        slots: list[Memoized[object]] = []
        placed: dict[int, list[int]] = {}

        def visit(node: Memoized[object]) -> None:
            slots.append(node)
            content = self._shown.get(node._id)
            if content is None:
                return
            items = list(_placed(content))
            placed[node._id] = [item._id for item in items]
            for item in items:
                if isinstance(item, Memoized):
                    visit(item)

        visit(self._root)
        return _Visible(slots, placed)

    def _prune(self) -> None:
        """Forgets slots that are no longer visible (the UI drops them too)."""
        visible_ids = {node._id for node in self._visible().slots}
        for slot_map in (self._shown, self._sent, self._waiting):
            for slot_id in list(slot_map):
                if slot_id not in visible_ids:
                    del slot_map[slot_id]


def _placed(content: Element) -> Iterator[InputBase | Memoized[object]]:
    """Inputs and nodes placed in `content`, not descending into nodes."""
    if isinstance(content, Layout):
        for child in content._children:
            if isinstance(child, Memoized):
                yield child
            else:
                yield from _placed(child)
    elif isinstance(content, InputBase):
        yield content


def _format_error(error: Exception, tb: TracebackType | None) -> str:
    # The first frame is `Memoized._evaluate` calling the user's function.
    frames = "".join(traceback.format_tb(tb.tb_next if tb is not None else None))
    message = "".join(traceback.format_exception_only(type(error), error)).strip()
    return f"{message}\n\n{frames}" if frames else message
