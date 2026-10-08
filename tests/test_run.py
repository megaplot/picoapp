import pytest

import picoapp as pa
from picoapp import _core
from picoapp._engine import Engine


def test_run_passes_an_engine_with_the_view_as_root(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    engines: list[Engine] = []
    monkeypatch.setattr(_core._picoapp, "run", engines.append)

    def view() -> pa.Element:
        return pa.Row()

    pa.run(view)
    [engine] = engines
    result = engine.step()
    assert result is not None
    assert result.node_id == engine.root_id
    assert isinstance(result.content, pa.Row)
