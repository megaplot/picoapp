from typing import Sequence

from . import _picoapp
from .types import Callback, Input


def run(inputs: Sequence[Input], callback: Callback):
    _picoapp.run(inputs, callback)
