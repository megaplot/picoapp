"""Many inputs and plots, to check how layouts behave when they overflow.

The "Layout" selector arranges the same elements in different ways.
"""

import numpy as np

import picoapp as pa

layout = pa.Radio("Layout", ["sidebar", "inputs above plots", "mixed column"])
sliders = [pa.Slider(f"frequency {i}", 0.1, 1.0 + i / 2, 20.0) for i in range(24)]


def plots() -> list[pa.Plot]:
    xs = np.linspace(0.0, 2.0 * np.pi, 200)
    return [pa.Plot(xs, np.sin(slider.value * xs)) for slider in sliders[:12]]


def view() -> pa.Element:
    if layout.value == "sidebar":
        return pa.Row(pa.Column(layout, *sliders), pa.Column(*plots()))
    if layout.value == "inputs above plots":
        return pa.Column(pa.Column(layout, *sliders), pa.Column(*plots()))
    return pa.Column(layout, *sliders, *plots())


pa.run(view)
