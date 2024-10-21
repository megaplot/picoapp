from __future__ import annotations

from dataclasses import dataclass
from typing import Callable

import numpy as np


@dataclass
class Slider:
    name: str
    min: float
    value: float
    max: float

    def __post_init__(self) -> None:
        assert self.min <= self.value <= self.max


@dataclass
class IntSlider:
    name: str
    min: int
    value: int
    max: int

    def __post_init__(self) -> None:
        assert self.min <= self.value <= self.max


Input = Slider | IntSlider


class Inputs:
    def __init__(self, *inputs: Input, callback: Callback):
        self.inputs = inputs
        self.callback = callback


Data = np.ndarray | list[float]


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
