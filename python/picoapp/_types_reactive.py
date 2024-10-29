from __future__ import annotations

import abc
from typing import Callable

from ._types_inputs import Inputs
from ._types_outputs import Outputs


class ReactiveBase(abc.ABC):
    @property
    @abc.abstractmethod
    def inputs(self) -> Inputs:
        raise NotImplementedError()

    @abc.abstractmethod
    def __call__(self) -> Outputs | ReactiveBase:
        raise NotImplementedError()


class Reactive(ReactiveBase):
    def __init__(self, inputs: Inputs, callback: Callback):
        # Note that I don't see a good way to avoid the `_inputs` vs `input` property
        # boilerplate, see e.g.:
        # https://discuss.python.org/t/abc-add-abstract-attributes-via-abstract-type-hint/26164/15
        self._inputs = inputs
        self._callback = callback

    @property
    def inputs(self) -> Inputs:
        return self._inputs

    def __call__(self) -> Outputs | ReactiveBase:
        return self._callback()


Callback = Callable[[], Outputs | ReactiveBase]
