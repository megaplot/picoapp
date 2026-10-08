"""Static typing checks of the public API, enforced by `mypy .` in CI.

Positive cases must type-check. Each negative case carries a
`# type: ignore[<code>]`; `warn_unused_ignores` fails CI if the expected error
disappears. The test functions are never called by pytest; the module only
needs to import.
"""

from __future__ import annotations

import numpy as np

import picoapp as pa

slider = pa.Slider("s", 0.0, 0.5, 1.0)
int_slider = pa.IntSlider("i", 0, 1, 2)
checkbox = pa.Checkbox("c")
radio = pa.Radio("r", ["a", "b"])


@pa.memoize
def plot_node() -> pa.Plot:
    return pa.Plot([0.0], [slider.value])


@pa.memoize
def audio_node() -> pa.Audio:
    return pa.Audio(np.zeros(10), sr=10)


@pa.memoize
def dataset() -> list[float]:
    return [slider.value]


def view() -> pa.Element:
    return pa.Row(pa.Column(slider, checkbox), plot_node)


def check_values() -> None:
    radio_value: str = radio.value
    float_value: float = slider.value
    int_value: int = int_slider.value
    bool_value: bool = checkbox.value
    data: list[float] = dataset()
    first: pa.Memoized[float] = dataset.map(lambda d: d[0])
    print(radio_value, float_value, int_value, bool_value, data, first)


def check_heterogeneous_children() -> None:
    pa.Row(
        slider,
        radio,
        pa.Plot([0.0], [0.0]),
        plot_node,
        audio_node,
        pa.memoize(lambda: pa.Plot([0.0], [0.0])),
        pa.Column(checkbox, pa.memoize(lambda: pa.Row(plot_node))),
    )


def check_lists() -> None:
    nodes = [plot_node, audio_node]
    pa.Row(*nodes)

    inputs: list[pa.InputBase] = [slider, radio, checkbox]
    inputs.append(int_slider)
    pa.Column(*inputs)

    mixed: list[pa.ElementLike] = [slider, plot_node]
    mixed.append(pa.Plot([0.0], [0.0]))
    pa.Row(*mixed)

    node_dict = {"plot": plot_node, "audio": audio_node}
    pa.Row(*node_dict.values())
    pa.Row(plot_node if checkbox.value else audio_node)


def check_variance() -> None:
    as_output: pa.Memoized[pa.Output] = plot_node
    as_input: pa.Input[object] = slider
    print(as_output, as_input)


def check_run() -> None:
    pa.run(view)


def check_negative_cases() -> None:
    pa.Row(42)  # type: ignore[arg-type]
    pa.Row(lambda: pa.Plot([0.0], [0.0]))  # type: ignore[arg-type]
    pa.Row(dataset)  # type: ignore[arg-type]
    pa.Row(pa.memoize(lambda: "text"))  # type: ignore[arg-type, return-value]
    pa.memoize(lambda x: x)  # type: ignore[arg-type, misc]

    bad_dict = {"plot": plot_node, "data": dataset}
    pa.Row(*bad_dict.values())  # type: ignore[arg-type]
    pa.run(lambda: 42)  # type: ignore[arg-type, return-value]
