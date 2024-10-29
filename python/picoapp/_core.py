from . import _picoapp
from ._types_reactive import ReactiveBase


def run(reactive: ReactiveBase) -> None:
    _picoapp.run(reactive.inputs.inputs, reactive.__call__)
