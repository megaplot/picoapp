from __future__ import annotations

from dataclasses import dataclass
from typing import Callable, Sequence

import numpy as np


class Slider:
    def __init__(
        self, name: str, min: float, init: float, max: float, log: bool = False
    ) -> None:
        if not (min <= init <= max):
            raise ValueError(f"Slider {min=}/{init=}/{max=} must be monotonous.")
        if log and not (min > 0 and init > 0 and max > 0):
            raise ValueError(
                f"For a logarithmic slider, {min=}/{init=}/{max=} must be positive."
            )
        self._name = name
        self._min = min
        self._init = init
        self._max = max
        self._log = log

        self._value = init

    @property
    def name(self) -> str:
        return self._name

    @property
    def min(self) -> float:
        return self._min

    @property
    def value(self) -> float:
        return self._value

    @property
    def max(self) -> float:
        return self._max


class IntSlider:
    def __init__(self, name: str, min: int, init: int, max: int) -> None:
        if not (min <= init <= max):
            raise ValueError(f"Slider {min=}/{init=}/{max=} must be monotonous.")
        self._name = name
        self._min = min
        self._init = init
        self._max = max

        self._value = init

    @property
    def name(self) -> str:
        return self._name

    @property
    def min(self) -> int:
        return self._min

    @property
    def value(self) -> int:
        return self._value

    @property
    def max(self) -> int:
        return self._max


Input = Slider | IntSlider


class Inputs:
    def __init__(self, *inputs: Input, callback: Callback):
        self.inputs = inputs
        self.callback = callback


Data = np.ndarray | Sequence[float]


class Plot:
    def __init__(
        self,
        xs: Data,
        ys: Data,
        *,
        x_limits: tuple[float, float] | None = None,
        y_limits: tuple[float, float] | None = None,
    ):
        self.xs = xs
        self.ys = ys
        self.x_limits = _use_or_infer(x_limits, xs)
        self.y_limits = _use_or_infer(y_limits, ys)


def _use_or_infer(
    limits: tuple[float, float] | None, data: Data
) -> tuple[float, float]:
    if limits is None:
        return float(np.min(data)), float(np.max(data))
    else:
        return limits


@dataclass
class Audio:
    data: np.ndarray
    sr: int


# Union type of all supported outputs (it remains to be seen if we rather want
# to introduce a base type, and some sort of interface, but since each type
# basically needs an explicit implementation on the Rust side, a union type
# seems more appropriate on first glance).
Output = Plot | Audio


class Outputs:
    def __init__(self, *outputs: Output):
        self.outputs = outputs


Callback = Callable[[], Outputs | Inputs]
