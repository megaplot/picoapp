from __future__ import annotations

from collections.abc import Callable
from types import TracebackType
from typing import Generic, TypeVar, cast

from . import _tracking

T = TypeVar("T")
T_co = TypeVar("T_co", covariant=True)
U = TypeVar("U")


class CycleError(Exception):
    """Raised when a memoized node (transitively) calls itself."""


class Memoized(Generic[T_co]):
    """A cached computation whose dependencies are tracked automatically.

    Create via `pa.memoize`. Calling the node returns its value, re-evaluating
    it only if an input or node it read during its last evaluation changed.
    """

    def __init__(self, fn: Callable[[], T_co]) -> None:
        self._id = _tracking.next_id()
        self._fn = fn
        self._name: str = getattr(fn, "__qualname__", repr(fn))
        self._version = 0
        self._evaluated = False
        self._value: T_co | None = None
        self._error: Exception | None = None
        self._error_tb: TracebackType | None = None
        self._deps: list[tuple[_tracking.Source, int]] = []
        self._checked_epoch = -1

    def __call__(self) -> T_co:
        self._ensure_fresh()
        _tracking.record_read(self)
        return self._result()

    def map(self, fn: Callable[[T_co], U]) -> Memoized[U]:
        """A node computing `fn(self())`, e.g. to project one part of a result."""
        node = Memoized(lambda: fn(self()))
        node._name = f"{self._name}.map({getattr(fn, '__qualname__', repr(fn))})"
        return node

    def __repr__(self) -> str:
        return f"Memoized({self._name})"

    def _result(self) -> T_co:
        if self._error is not None:
            # Restoring the original traceback keeps it from growing by the
            # re-raising frames on every call.
            raise self._error.with_traceback(self._error_tb)
        return cast(T_co, self._value)

    def _ensure_fresh(self) -> None:
        """Brings the node up to date (pull with ordered short-circuit)."""
        # Checked here, not only in `__call__`: a dependency walk can reach a
        # node that is still evaluating, which must not be re-entered.
        if any(frame.node is self for frame in _tracking.stack.get()):
            raise CycleError(f"memoized node `{self._name}` calls itself")
        epoch = _tracking.current_epoch()
        if self._checked_epoch == epoch:
            return
        if not self._evaluated or self._any_dep_changed():
            self._evaluate()
        self._checked_epoch = epoch

    def _any_dep_changed(self) -> bool:
        # Walk in read order and stop at the first change: a dependency that
        # only an old branch read is never evaluated.
        for source, seen in self._deps:
            if isinstance(source, Memoized):
                source._ensure_fresh()
            if source._version != seen:
                return True
        return False

    def _maybe_stale(self) -> bool:
        """Whether `_ensure_fresh` might re-evaluate, without evaluating anything."""
        if not self._evaluated:
            return True
        if self._checked_epoch == _tracking.current_epoch():
            return False
        for source, seen in self._deps:
            if source._version != seen:
                return True
            if isinstance(source, Memoized) and source._maybe_stale():
                return True
        return False

    def _evaluate(self) -> None:
        frame = _tracking.Frame(self)
        token = _tracking.stack.set((*_tracking.stack.get(), frame))
        try:
            self._value = self._fn()
            self._error = None
            self._error_tb = None
        except Exception as e:  # noqa: BLE001 (user code may raise anything)
            self._value = None
            self._error = e
            self._error_tb = e.__traceback__
        finally:
            _tracking.stack.reset(token)
        self._deps = frame.deps
        self._evaluated = True
        self._version += 1


def memoize(fn: Callable[[], T]) -> Memoized[T]:
    """Turns a zero-argument function into a memoized node, usable as decorator."""
    return Memoized(fn)
