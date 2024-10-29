from . import _picoapp
from ._types_inputs import Inputs
from ._types_reactive import Callback


def run(inputs: Inputs, callback: Callback) -> None:
    _picoapp.run(inputs.inputs, callback)
