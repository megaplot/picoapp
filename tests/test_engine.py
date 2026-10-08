from __future__ import annotations

import gc

import picoapp as pa
from picoapp._engine import Engine, SlotResult


def drain(engine: Engine) -> list[SlotResult]:
    results = []
    while (result := engine.step()) is not None:
        results.append(result)
    return results


def make_engine(view: pa.Memoized[pa.Element]) -> Engine:
    engine = Engine(view)
    drain(engine)
    return engine


class Counter:
    """Counts evaluations per name."""

    def __init__(self) -> None:
        self.counts: dict[str, int] = {}

    def hit(self, name: str) -> None:
        self.counts[name] = self.counts.get(name, 0) + 1


# --- engine: slots, priority, visibility -------------------------------------


def test_first_step_sends_the_root() -> None:
    x = pa.Slider("x", 0.0, 1.0, 2.0)
    root = pa.memoize(lambda: pa.Row(x))
    engine = Engine(root)
    assert engine.stale_visible_slots() == [root._id]
    result = engine.step()
    assert result is not None
    assert result.node_id == root._id == engine.root_id
    assert isinstance(result.content, pa.Row)
    assert engine.step() is None
    assert engine.stale_visible_slots() == []


def test_root_returning_a_non_layout_is_wrapped_in_a_column() -> None:
    x = pa.Slider("x", 0.0, 1.0, 2.0)
    engine = Engine(pa.memoize(lambda: x))
    result = engine.step()
    assert result is not None
    assert isinstance(result.content, pa.Column)
    assert result.content._children == (x,)


def test_step_priority_is_root_then_fragments_then_leaves() -> None:
    x = pa.IntSlider("x", 0, 0, 10)
    leaf_a = pa.memoize(lambda: pa.Plot([0.0], [float(x.value)]))
    leaf_b = pa.memoize(lambda: pa.Plot([0.0], [float(x.value)]))
    fragment = pa.memoize(lambda: (x.value, pa.Column(leaf_b))[1])
    root = pa.memoize(lambda: (x.value, pa.Row(x, leaf_a, fragment))[1])
    engine = make_engine(root)

    engine.set_values([(x._id, 1)])
    order = [result.node_id for result in drain(engine)]
    assert order == [root._id, fragment._id, leaf_a._id, leaf_b._id]


def test_failed_fragment_keeps_its_priority() -> None:
    x = pa.IntSlider("x", 0, 0, 10)

    def fragment_fn() -> pa.Column:
        if x.value == 1:
            raise ValueError("failing")
        return pa.Column(pa.Plot([0.0], [float(x.value)]))

    leaf = pa.memoize(lambda: pa.Plot([0.0], [float(x.value)]))
    fragment = pa.memoize(fragment_fn)
    engine = make_engine(pa.memoize(lambda: pa.Row(x, leaf, fragment)))

    engine.set_values([(x._id, 1)])
    drain(engine)
    engine.set_values([(x._id, 2)])
    order = [result.node_id for result in drain(engine)]
    assert order == [fragment._id, leaf._id]


def test_unchanged_slots_are_not_resent() -> None:
    x = pa.Slider("x", 0.0, 1.0, 2.0)
    y = pa.Slider("y", 0.0, 1.0, 2.0)
    plot_x = pa.memoize(lambda: pa.Plot([0.0], [x.value]))
    plot_y = pa.memoize(lambda: pa.Plot([0.0], [y.value]))
    engine = make_engine(pa.memoize(lambda: pa.Row(pa.Column(x, y), plot_x, plot_y)))

    engine.set_values([(x._id, 2.0)])
    assert engine.stale_visible_slots() == [plot_x._id]
    assert [result.node_id for result in drain(engine)] == [plot_x._id]


def test_hidden_nodes_are_not_evaluated_and_keep_their_result() -> None:
    counter = Counter()
    show = pa.Checkbox("show", init=True)
    x = pa.Slider("x", 0.0, 1.0, 2.0)

    @pa.memoize
    def plot() -> pa.Plot:
        counter.hit("plot")
        return pa.Plot([0.0], [x.value])

    engine = make_engine(
        pa.memoize(lambda: pa.Row(show, plot) if show.value else pa.Row(show))
    )
    assert counter.counts == {"plot": 1}

    engine.set_values([(show._id, False)])
    drain(engine)
    engine.set_values([(x._id, 2.0)])
    assert drain(engine) == []
    assert counter.counts == {"plot": 1}

    # Re-shown and stale: re-evaluated.
    engine.set_values([(show._id, True)])
    drain(engine)
    assert counter.counts == {"plot": 2}

    # Hidden and re-shown without a change: re-sent from the cache.
    engine.set_values([(show._id, False)])
    drain(engine)
    engine.set_values([(show._id, True)])
    sent = [result.node_id for result in drain(engine)]
    assert plot._id in sent
    assert counter.counts == {"plot": 2}


def test_set_values_mid_round_replans() -> None:
    counter = Counter()
    x = pa.Slider("x", 0.0, 1.0, 2.0)
    y = pa.Slider("y", 0.0, 1.0, 2.0)

    @pa.memoize
    def plot_x() -> pa.Plot:
        counter.hit("x")
        return pa.Plot([0.0], [x.value])

    @pa.memoize
    def plot_y() -> pa.Plot:
        counter.hit("y")
        return pa.Plot([0.0], [y.value])

    engine = make_engine(pa.memoize(lambda: pa.Row(pa.Column(x, y), plot_x, plot_y)))
    engine.set_values([(x._id, 1.5), (y._id, 1.5)])
    first = engine.step()
    assert first is not None and first.node_id == plot_x._id
    # A new value for `x` arrives before `plot_y` ran: `plot_x` is stale again
    # and comes first in tree order.
    engine.set_values([(x._id, 2.0)])
    assert [result.node_id for result in drain(engine)] == [plot_x._id, plot_y._id]
    assert counter.counts == {"x": 3, "y": 2}


def test_node_error_becomes_an_error_result() -> None:
    @pa.memoize
    def failing() -> pa.Plot:
        raise ValueError("boom")

    engine = Engine(pa.memoize(lambda: pa.Row(failing)))
    results = drain(engine)
    error = results[-1]
    assert error.node_id == failing._id
    assert isinstance(error.content, str)
    assert error.content.startswith("ValueError: boom")
    assert "_evaluate" not in error.content


def test_non_element_result_is_a_slot_error() -> None:
    @pa.memoize
    def number() -> int:
        return 42

    engine = Engine(pa.memoize(lambda: pa.Row(number)))  # type: ignore[arg-type]
    results = drain(engine)
    assert (
        results[-1].content
        == "node `test_non_element_result_is_a_slot_error.<locals>.number` returned int, expected an Element"
    )


def test_error_keeps_previous_content_visible() -> None:
    fail = pa.Checkbox("fail")
    inner = pa.memoize(lambda: pa.Plot([0.0], [0.0]))

    @pa.memoize
    def fragment() -> pa.Column:
        if fail.value:
            raise ValueError("boom")
        return pa.Column(inner)

    engine = make_engine(pa.memoize(lambda: pa.Row(fail, fragment)))
    engine.set_values([(fail._id, True)])
    drain(engine)
    # `inner` stays visible below the error, so it is still a stale-able slot.
    assert inner._id in [node._id for node in engine._visible().slots]


def test_duplicate_input_is_a_slot_error() -> None:
    x = pa.Slider("x", 0.0, 1.0, 2.0)
    fragment = pa.memoize(lambda: pa.Column(x))
    engine = Engine(pa.memoize(lambda: pa.Row(x, fragment)))
    results = drain(engine)
    assert results[-1].node_id == fragment._id
    assert isinstance(results[-1].content, str)
    assert "appears more than once" in results[-1].content


def test_node_placing_itself_is_a_slot_error() -> None:
    @pa.memoize
    def fragment() -> pa.Column:
        return pa.Column(fragment)

    engine = Engine(pa.memoize(lambda: pa.Row(fragment)))
    results = drain(engine)
    assert isinstance(results[-1].content, str)


def test_reject_restores_the_previous_content() -> None:
    show = pa.Checkbox("show", init=True)
    inner = pa.memoize(lambda: pa.Plot([0.0], [0.0]))
    fragment = pa.memoize(lambda: pa.Column(inner) if show.value else pa.Column())
    engine = make_engine(pa.memoize(lambda: pa.Row(show, fragment)))

    engine.set_values([(show._id, False)])
    result = engine.step()
    assert result is not None and result.node_id == fragment._id
    engine.reject(fragment._id)
    assert inner._id in [node._id for node in engine._visible().slots]
    # The UI never dropped `inner`, so nothing must be re-sent.
    assert engine.stale_visible_slots() == []


def test_input_moving_to_an_earlier_slot_is_not_a_lasting_duplicate() -> None:
    flag = pa.Checkbox("flag")
    s = pa.Slider("s", 0.0, 0.5, 1.0)
    first = pa.memoize(lambda: pa.Column(s) if flag.value else pa.Column())
    second = pa.memoize(lambda: pa.Column() if flag.value else pa.Column(s))
    engine = make_engine(pa.memoize(lambda: pa.Row(flag, first, second)))

    engine.set_values([(flag._id, True)])
    results = drain(engine)
    # `first` may briefly see `s` still in `second`, but must end up showing it.
    assert results[-1].node_id == first._id
    assert isinstance(results[-1].content, pa.Column)
    assert engine.stale_visible_slots() == []


def test_set_values_ignores_collected_inputs() -> None:
    x = pa.Slider("x", 0.0, 1.0, 2.0)
    engine = Engine(pa.memoize(lambda: pa.Row()))
    stale_id = x._id
    del x
    gc.collect()
    engine.set_values([(stale_id, 1.5)])


# --- review focus ----------------------------------------------------------


def test_view_creating_new_inputs_and_nodes_each_run_converges() -> None:
    order = pa.IntSlider("order", 0, 2, 5)

    def view() -> pa.Element:
        # Anti-pattern (fresh objects every run), but it must not loop.
        sliders = [pa.Slider(f"c{i}", 0.0, 0.5, 1.0) for i in range(order.value + 1)]
        plot = pa.memoize(lambda: pa.Plot([0.0], [sum(s.value for s in sliders)]))
        return pa.Row(pa.Column(order, *sliders), plot)

    engine = Engine(pa.memoize(view))
    assert len(drain(engine)) == 2
    engine.set_values([(order._id, 4)])
    assert len(drain(engine)) == 2
    assert engine.step() is None


def test_root_failing_on_the_first_run_sends_an_error_and_stops() -> None:
    def view() -> pa.Element:
        raise ValueError("broken view")

    engine = Engine(pa.memoize(view))
    results = drain(engine)
    assert len(results) == 1
    assert results[0].node_id == engine.root_id
    assert isinstance(results[0].content, str)
    assert "broken view" in results[0].content
    assert engine.stale_visible_slots() == []


def test_leaf_slot_can_be_an_input() -> None:
    x = pa.Slider("x", 0.0, 1.0, 2.0)
    shown_x = pa.memoize(lambda: x)
    engine = Engine(pa.memoize(lambda: pa.Row(shown_x)))
    results = drain(engine)
    assert results[-1].node_id == shown_x._id
    assert results[-1].content is x
