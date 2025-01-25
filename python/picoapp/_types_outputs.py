from __future__ import annotations

from dataclasses import dataclass
from typing import Sequence

import numpy as np

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

        if isinstance(xs, np.ndarray):
            if xs.ndim != 1:
                raise ValueError(
                    f"Plot data must be 1-dimensional, but xs has {xs.ndim} dimensions"
                )
            else:
                len_xs = xs.shape[0]
        else:
            len_xs = len(xs)

        if isinstance(ys, np.ndarray):
            if ys.ndim != 1:
                raise ValueError(
                    f"Plot data must be 1-dimensional, but ys has {ys.ndim} dimensions"
                )
            else:
                len_ys = ys.shape[0]
        else:
            len_ys = len(xs)

        if len_xs != len_ys:
            raise ValueError(
                f"Plot data has inconsistent length: xs has length {len_xs}, ys has length {len_ys}"
            )


class MatrixPlot:
    def __init__(self, matrix: np.ndarray):
        if matrix.ndim != 2:
            raise ValueError(
                f"MatrixPlot requires 2-dimension matrix, but matrix has {matrix.ndim} dimensions."
            )
        self.matrix = matrix
        self.num_rows = matrix.shape[0]
        self.num_cols = matrix.shape[1]
        self.min_value = np.min(matrix)
        self.max_value = np.max(matrix)


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


@dataclass
class Image:
    data: np.ndarray
    width: int
    height: int

    @staticmethod
    def from_3d_array(data: np.ndarray) -> Image:
        if data.ndim != 3:
            raise ValueError(
                f"Image data must be 3-dimensional, but has {data.ndim} dimensions."
            )
        if data.shape[-1] != 4:
            raise ValueError(
                f"Last dimension of image data have a size of 4, representing rgba data, "
                f"but has size of {data.shape[-1]}."
            )
        return Image(
            data=data.flatten(),
            width=data.shape[1],
            height=data.shape[0],
        )

    def __post_init__(self) -> None:
        if self.data.ndim != 1:
            raise ValueError(
                f"Image data must be flattened to 1-dim, but has {self.data.ndim} dimensions."
            )
        if self.data.dtype != np.uint8:
            raise ValueError(
                f"Image data must be of type np.uint8, but is {self.data.dtype}."
            )


# Union type of all supported outputs (it remains to be seen if we rather want
# to introduce a base type, and some sort of interface, but since each type
# basically needs an explicit implementation on the Rust side, a union type
# seems more appropriate on first glance).
Output = Plot | MatrixPlot | Audio | Image


class Outputs:
    def __init__(self, *outputs: Output):
        self.outputs = outputs
