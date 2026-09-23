from dataclasses import dataclass

import picoapp as pa
from picoapp._picoapp import _parse_input, _run_worker_job


def test_parse_slider():
    input = pa.Slider("Name", min=0.1, init=0.5, max=1.0)
    _parse_input(input)


def test_parse_int_slider():
    input = pa.IntSlider("Name", min=0, init=5, max=10)
    _parse_input(input)


def test_parse_checkbox():
    input = pa.Checkbox("Name")
    _parse_input(input)


def test_parse_radio():
    input = pa.Radio("Name", values=["foo", "bar", "baz"])
    _parse_input(input)


def test_parse_radio__custom_types():

    @dataclass
    class Custom:
        label: str

        def __str__(self) -> str:
            return f"<{self.label}>"

    input = pa.Radio("Name", values=[Custom("foo"), Custom("bar"), Custom("baz")])
    _parse_input(input)


def test_worker_job_writes_value_and_returns_outputs():
    slider = pa.Slider("a", min=0.0, init=1.0, max=10.0)

    def callback():
        return pa.Outputs(pa.Plot([0.0, 1.0], [slider.value, slider.value]))

    result = _run_worker_job([slider], callback, [7.5])
    assert result == "Outputs(1)"
    assert slider.value == 7.5


def test_worker_job_returns_nested():
    outer = pa.IntSlider("n", min=1, init=2, max=5)

    def callback():
        inner = pa.Slider("x", min=0.0, init=0.0, max=1.0)
        return pa.Reactive(pa.Inputs(inner), lambda: pa.Outputs())

    result = _run_worker_job([outer], callback, [3])
    assert result == "Nested(1)"


def test_worker_job_returns_error_on_exception():
    checkbox = pa.Checkbox("c")

    def callback():
        raise ValueError("boom")

    result = _run_worker_job([checkbox], callback, [True])
    assert result.startswith("Error(")
    assert "boom" in result
