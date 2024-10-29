from __future__ import annotations

from typing import Callable

from ._types_inputs import Input
from ._types_outputs import Outputs


class Inputs:
    def __init__(self, *inputs: Input, callback: Callback):
        self.inputs = inputs
        self.callback = callback


Callback = Callable[[], Outputs | Inputs]
