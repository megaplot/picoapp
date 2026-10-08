from __future__ import annotations

import weakref
from collections.abc import Sequence
from typing import Generic, TypeVar

from . import _tracking
from ._types_element import Element

T = TypeVar("T")
T_co = TypeVar("T_co", covariant=True)

# All live inputs by id, so the engine can write values the UI sends back.
_inputs_by_id: weakref.WeakValueDictionary[int, InputBase] = (
    weakref.WeakValueDictionary()
)


class InputBase(Element):
    """Non-generic base of all inputs, e.g. for `list[pa.InputBase]`."""

    def __init__(self) -> None:
        self._id = _tracking.next_id()
        self._version = 0
        _inputs_by_id[self._id] = self

    def _write_ui_value(self, raw: object) -> None:
        """Writes a value sent by the UI; bumps the version if it differs."""
        raise NotImplementedError()


class Input(InputBase, Generic[T_co]):
    def __init__(self, value: T_co) -> None:
        super().__init__()
        self._value = value

    @property
    def value(self) -> T_co:
        _tracking.record_read(self)
        return self._value

    def _write_ui_value(self, raw: object) -> None:
        value = self._decode(raw)
        if value != self._value:
            self._value = value
            self._version += 1
            _tracking.bump_epoch()

    def _decode(self, raw: object) -> T_co:
        """Converts the UI's raw value (float, int, bool or index) to `T_co`."""
        raise NotImplementedError()


class Slider(Input[float]):
    def __init__(
        self,
        name: str,
        min: float,
        init: float,
        max: float,
        log: bool = False,
        decimal_places: int | None = None,
    ) -> None:
        if not (min <= init <= max):
            raise ValueError(f"Slider {min=}/{init=}/{max=} must be monotonous.")
        if log and not (min > 0 and init > 0 and max > 0):
            raise ValueError(
                f"For a logarithmic slider, {min=}/{init=}/{max=} must be positive."
            )
        super().__init__(init)
        self._name = name
        self._min = min
        self._max = max
        self._log = log
        self._decimal_places = decimal_places

    @property
    def name(self) -> str:
        return self._name

    @property
    def min(self) -> float:
        return self._min

    @property
    def max(self) -> float:
        return self._max

    def _decode(self, raw: object) -> float:
        if not isinstance(raw, (int, float)):
            raise TypeError(f"Slider value must be a number, got {raw!r}")
        return float(raw)


class IntSlider(Input[int]):
    def __init__(self, name: str, min: int, init: int, max: int) -> None:
        if not (min <= init <= max):
            raise ValueError(f"Slider {min=}/{init=}/{max=} must be monotonous.")
        super().__init__(init)
        self._name = name
        self._min = min
        self._max = max

    @property
    def name(self) -> str:
        return self._name

    @property
    def min(self) -> int:
        return self._min

    @property
    def max(self) -> int:
        return self._max

    def _decode(self, raw: object) -> int:
        if not isinstance(raw, int):
            raise TypeError(f"IntSlider value must be an int, got {raw!r}")
        return raw


class Checkbox(Input[bool]):
    def __init__(self, name: str, init: bool = False) -> None:
        super().__init__(init)
        self._name = name

    def __bool__(self) -> bool:
        return self.value

    def _decode(self, raw: object) -> bool:
        if not isinstance(raw, bool):
            raise TypeError(f"Checkbox value must be a bool, got {raw!r}")
        return raw


class Radio(Input[T]):
    def __init__(self, name: str, values: Sequence[T], init: T | None = None) -> None:
        if len(values) == 0:
            raise ValueError("Radio values must not be empty")
        index = 0 if init is None else values.index(init)
        super().__init__(values[index])
        self._name = name
        self._values = values
        self._index = index

    def _write_ui_value(self, raw: object) -> None:
        super()._write_ui_value(raw)
        self._index = self._values.index(self._value)

    def _decode(self, raw: object) -> T:
        if not isinstance(raw, int):
            raise TypeError(f"Radio value must be an index, got {raw!r}")
        return self._values[raw]
