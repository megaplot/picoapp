"""Many inputs and plots, to check how layouts behave when they overflow.

The "Layout" selector arranges the same elements in different ways.
"""

import numpy as np

import picoapp as pa

layout = pa.Radio(
    "Layout", ["separate columns", "single column", "single column (nested)"]
)
sliders = [pa.Slider(f"frequency {i}", 0.1, 1.0 + i / 2, 20.0) for i in range(24)]


def plots() -> list[pa.Plot]:
    xs = np.linspace(0.0, 2.0 * np.pi, 200)
    return [pa.Plot(xs, np.sin(slider.value * xs)) for slider in sliders[:12]]


def view() -> pa.Element:
    if layout.value == "separate columns":
        return pa.Row(pa.Column(layout, *sliders), pa.Column(*plots()))
    if layout.value == "single column":
        return pa.Column(layout, *sliders, *plots())
    # Looks the same as "single column": a column inside a column takes its
    # content's height, and the outer column scrolls.
    return pa.Column(pa.Column(layout, *sliders), pa.Column(*plots()))


pa.run(view)
