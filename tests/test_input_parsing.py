from dataclasses import dataclass

import picoapp as pa
from picoapp._picoapp import _parse_input


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
