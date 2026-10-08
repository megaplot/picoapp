from collections.abc import Sequence

from ._types_inputs import Input, T
from ._types_reactive import Callback

def run(inputs: Sequence[Input[T]], callback: Callback) -> None: ...
def _parse_input(input: Input[T]) -> None: ...
def _run_worker_job(
    inputs: Sequence[Input[T]], callback: Callback, values: Sequence[object]
) -> str: ...
