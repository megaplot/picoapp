from typing import Sequence

from ._types_inputs import Input
from ._types_reactive import Callback

def run(inputs: Sequence[Input], callback: Callback) -> None: ...
def _parse_input(input: Input) -> None: ...
def _run_worker_job(
    inputs: Sequence[Input], callback: Callback, values: Sequence[object]
) -> str: ...
