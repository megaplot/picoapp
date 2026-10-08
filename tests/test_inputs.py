import pytest

import picoapp as pa


def test_inputs_get_distinct_ids() -> None:
    a = pa.Slider("a", 0.0, 0.5, 1.0)
    b = pa.Checkbox("b")
    assert a._id != b._id


def test_write_bumps_the_version_only_on_a_change() -> None:
    slider = pa.Slider("a", 0.0, 0.5, 1.0)
    slider._write_ui_value(0.5)
    assert slider._version == 0
    slider._write_ui_value(0.75)
    assert (slider.value, slider._version) == (0.75, 1)


def test_slider_accepts_ints_from_the_ui() -> None:
    slider = pa.Slider("a", 0.0, 0.5, 1.0)
    slider._write_ui_value(1)
    assert slider.value == 1.0
    assert isinstance(slider.value, float)


def test_int_slider_and_checkbox_values() -> None:
    int_slider = pa.IntSlider("n", 0, 1, 10)
    checkbox = pa.Checkbox("c")
    int_slider._write_ui_value(7)
    checkbox._write_ui_value(True)
    assert int_slider.value == 7
    assert checkbox.value is True
    assert checkbox


def test_radio_value_is_written_by_index() -> None:
    radio = pa.Radio("r", ["a", "b", "c"])
    radio._write_ui_value(2)
    assert radio.value == "c"
    assert radio._index == 2


def test_radio_init_selects_the_index() -> None:
    radio = pa.Radio("r", ["a", "b", "c"], init="b")
    assert (radio.value, radio._index) == ("b", 1)


def test_write_rejects_a_mismatched_type() -> None:
    with pytest.raises(TypeError):
        pa.Checkbox("c")._write_ui_value(1.5)


def test_inputs_and_outputs_are_elements() -> None:
    assert isinstance(pa.Slider("a", 0.0, 0.5, 1.0), pa.Element)
    assert isinstance(pa.Plot([0.0], [0.0]), pa.Output)
    assert isinstance(pa.Plot([0.0], [0.0]), pa.Element)
