from __future__ import annotations

import pytest

import picoapp as pa


class Counter:
    """Counts evaluations per name."""

    def __init__(self) -> None:
        self.counts: dict[str, int] = {}

    def hit(self, name: str) -> None:
        self.counts[name] = self.counts.get(name, 0) + 1


# --- tracking --------------------------------------------------------------


def test_reads_are_recorded_in_order() -> None:
    a = pa.Slider("a", 0.0, 1.0, 2.0)
    b = pa.Checkbox("b", init=True)

    @pa.memoize
    def node() -> float:
        return a.value if b.value else 0.0

    node()
    assert [source for source, _ in node._deps] == [b, a]


def test_dependencies_follow_the_taken_branch() -> None:
    flag = pa.Checkbox("flag")
    x = pa.Slider("x", 0.0, 1.0, 2.0)
    y = pa.Slider("y", 0.0, 1.0, 2.0)

    @pa.memoize
    def node() -> float:
        return x.value if flag.value else y.value

    node()
    assert [source for source, _ in node._deps] == [flag, y]
    flag._write_ui_value(True)
    node()
    assert [source for source, _ in node._deps] == [flag, x]


def test_reads_outside_any_node_are_untracked() -> None:
    x = pa.Slider("x", 0.0, 1.0, 2.0)
    assert x.value == 1.0  # no error, nothing recorded


def test_cached_value_is_returned_without_reevaluation() -> None:
    counter = Counter()
    x = pa.Slider("x", 0.0, 1.0, 2.0)

    @pa.memoize
    def node() -> float:
        counter.hit("node")
        return x.value * 2

    assert node() == 2.0
    assert node() == 2.0
    assert counter.counts == {"node": 1}


def test_only_nodes_reading_a_changed_input_rerun() -> None:
    counter = Counter()
    x = pa.Slider("x", 0.0, 1.0, 2.0)
    y = pa.Slider("y", 0.0, 1.0, 2.0)

    @pa.memoize
    def uses_x() -> float:
        counter.hit("x")
        return x.value

    @pa.memoize
    def uses_y() -> float:
        counter.hit("y")
        return y.value

    uses_x(), uses_y()
    x._write_ui_value(1.5)
    assert (uses_x(), uses_y()) == (1.5, 1.0)
    assert counter.counts == {"x": 2, "y": 1}


def test_writing_an_equal_value_does_not_invalidate() -> None:
    counter = Counter()
    x = pa.IntSlider("x", 0, 1, 2)

    @pa.memoize
    def node() -> int:
        counter.hit("node")
        return x.value

    node()
    x._write_ui_value(1)
    node()
    assert counter.counts == {"node": 1}


def test_diamond_evaluates_each_node_once() -> None:
    counter = Counter()
    x = pa.Slider("x", 0.0, 1.0, 2.0)

    @pa.memoize
    def left() -> float:
        counter.hit("left")
        return x.value + 1

    @pa.memoize
    def right() -> float:
        counter.hit("right")
        return x.value + 2

    @pa.memoize
    def sink() -> float:
        counter.hit("sink")
        return left() + right()

    sink()
    x._write_ui_value(2.0)
    assert sink() == 7.0
    assert counter.counts == {"left": 2, "right": 2, "sink": 2}


def test_short_circuit_skips_dependencies_of_the_old_branch() -> None:
    counter = Counter()
    flag = pa.Checkbox("flag", init=True)
    x = pa.Slider("x", 0.0, 1.0, 2.0)

    @pa.memoize
    def expensive() -> float:
        counter.hit("expensive")
        return x.value

    @pa.memoize
    def node() -> float:
        return expensive() if flag.value else 0.0

    node()
    # `x` changes too, but `flag` is read first and switches the branch away
    # from `expensive`, so `expensive` must not re-run.
    x._write_ui_value(2.0)
    flag._write_ui_value(False)
    assert node() == 0.0
    assert counter.counts == {"expensive": 1}


def test_map_projects_a_result() -> None:
    x = pa.IntSlider("x", 0, 1, 5)

    @pa.memoize
    def pair() -> tuple[int, int]:
        return x.value, -x.value

    first = pair.map(lambda p: p[0])
    assert first() == 1
    x._write_ui_value(3)
    assert first() == 3


# --- errors and cycles -----------------------------------------------------


def test_errors_are_cached_and_propagate() -> None:
    counter = Counter()
    x = pa.Slider("x", 0.0, 1.0, 2.0)

    @pa.memoize
    def failing() -> float:
        counter.hit("failing")
        if x.value > 0.5:
            raise ValueError("too large")
        return x.value

    @pa.memoize
    def dependent() -> float:
        return failing() * 2

    with pytest.raises(ValueError, match="too large"):
        dependent()
    with pytest.raises(ValueError, match="too large"):
        dependent()
    assert counter.counts == {"failing": 1}

    x._write_ui_value(0.25)
    assert dependent() == 0.5


def test_reraised_error_keeps_its_original_traceback() -> None:
    @pa.memoize
    def failing() -> int:
        raise ValueError("boom")

    @pa.memoize
    def dependent() -> int:
        return failing()

    lengths = []
    for _ in range(3):
        with pytest.raises(ValueError) as info:
            failing()
        lengths.append(len(info.traceback))
    assert lengths[0] == lengths[1] == lengths[2]


def test_cycle_raises_cycle_error() -> None:
    @pa.memoize
    def a() -> int:
        return b()

    @pa.memoize
    def b() -> int:
        return a()

    with pytest.raises(pa.CycleError):
        a()
