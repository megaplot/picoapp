from __future__ import annotations

from typing import Callable

from ._types_inputs import Inputs
from ._types_outputs import Outputs


class Reactive:
    def __init__(self, inputs: Inputs, callback: Callback):
        self.inputs = inputs
        self.callback = callback

    def __call__(self) -> Outputs | Reactive:
        return self.callback()


Callback = Callable[[], Outputs | Reactive]
