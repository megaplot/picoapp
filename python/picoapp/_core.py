from typing import Sequence

from . import _picoapp
from ._types_inputs import Input
from ._types_reactive import Callback


def run(inputs: Sequence[Input], callback: Callback) -> None:
    _picoapp.run(inputs, callback)
