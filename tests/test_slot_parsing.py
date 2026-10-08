from dataclasses import dataclass

import numpy as np
import pytest

import picoapp as pa
from picoapp._picoapp import _parse_slot_content


def test_parse_slider() -> None:
    slider = pa.Slider("a", min=0.1, init=0.5, max=1.0)
    assert _parse_slot_content(slider) == f"Slider#{slider._id}(a=0.5)"


def test_parse_int_slider() -> None:
    slider = pa.IntSlider("n", min=0, init=5, max=10)
    assert _parse_slot_content(slider) == f"IntSlider#{slider._id}(n=5)"


def test_parse_checkbox() -> None:
    checkbox = pa.Checkbox("c", init=True)
    assert _parse_slot_content(checkbox) == f"Checkbox#{checkbox._id}(c=true)"


def test_parse_radio() -> None:
    radio = pa.Radio("r", values=["foo", "bar", "baz"], init="bar")
    assert _parse_slot_content(radio) == f"Radio#{radio._id}(r=1)"


def test_parse_radio__custom_types() -> None:
    @dataclass
    class Custom:
        label: str

        def __str__(self) -> str:
            return f"<{self.label}>"

    radio = pa.Radio("r", values=[Custom("foo"), Custom("bar"), Custom("baz")])
    assert _parse_slot_content(radio) == f"Radio#{radio._id}(r=0)"


def test_parse_input_carries_the_current_value() -> None:
    slider = pa.IntSlider("n", min=0, init=5, max=10)
    radio = pa.Radio("r", values=["foo", "bar"])
    slider._write_ui_value(7)
    radio._write_ui_value(1)
    assert _parse_slot_content(slider) == f"IntSlider#{slider._id}(n=7)"
    assert _parse_slot_content(radio) == f"Radio#{radio._id}(r=1)"


def test_parse_outputs() -> None:
    assert _parse_slot_content(pa.Plot([0.0, 1.0], [1.0, 2.0])) == "Plot"
    assert _parse_slot_content(pa.MatrixPlot(np.zeros((2, 3)))) == "MatrixPlot"
    assert _parse_slot_content(pa.Audio(np.zeros(4), sr=4)) == "Audio"
    image = pa.Image(np.zeros(4, dtype=np.uint8), width=1, height=1)
    assert _parse_slot_content(image) == "Image"


def test_parse_layout_with_inputs_outputs_and_slots() -> None:
    slider = pa.Slider("a", min=0.0, init=0.5, max=1.0)
    node = pa.memoize(lambda: pa.Plot([0.0], [0.0]))
    tree = pa.Row(pa.Column(slider), pa.Plot([0.0], [0.0]), node)
    assert _parse_slot_content(tree) == (
        f"Row(Column(Slider#{slider._id}(a=0.5)), Plot, Slot#{node._id})"
    )


def test_parse_error_string() -> None:
    assert _parse_slot_content("boom") == "Error(boom)"


def test_parse_invalid_output_raises() -> None:
    image = pa.Image(np.zeros(4, dtype=np.uint8), width=2, height=2)
    with pytest.raises(ValueError, match="needs 16"):
        _parse_slot_content(pa.Column(image))


def test_parse_unknown_element_raises() -> None:
    class Custom(pa.Element):
        pass

    with pytest.raises(ValueError, match="Invalid output type"):
        _parse_slot_content(Custom())
