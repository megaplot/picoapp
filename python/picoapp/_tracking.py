"""Dependency tracking shared by inputs and memoized nodes.

While a node evaluates, a `Frame` for it is on top of a `contextvars` stack.
Every tracked read (`Input.value`, `Memoized.__call__`) records the source and
the version it saw in the top frame, in read order.
"""

from __future__ import annotations

import itertools
from contextvars import ContextVar
from typing import Protocol


class Source(Protocol):
    """Something a node can depend on: an input or another node."""

    @property
    def _version(self) -> int: ...


class Frame:
    """The reads recorded while `node` evaluates."""

    def __init__(self, node: object) -> None:
        self.node = node
        self.deps: list[tuple[Source, int]] = []


stack: ContextVar[tuple[Frame, ...]] = ContextVar("picoapp_stack", default=())

# Process-unique ids for inputs and nodes. Used across the FFI instead of
# `id()`, which CPython may reuse after an object is freed.
_ids = itertools.count(1)

# Bumped on every input write. A node that checked its dependencies in the
# current epoch is known to be up to date without walking them again.
_epoch = 0


def next_id() -> int:
    return next(_ids)


def current_epoch() -> int:
    return _epoch


def bump_epoch() -> None:
    global _epoch
    _epoch += 1


def record_read(source: Source) -> None:
    frames = stack.get()
    if frames:
        frames[-1].deps.append((source, source._version))
