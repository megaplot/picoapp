# type: ignore

import numpy as np

import picoapp as pa

inputs = [
    (slider_a := pa.Slider("a", -10.0, 0.5, 10.0)),
    (slider_b := pa.Slider("b", -10.0, 0.5, 10.0)),
    (slider_c := pa.Slider("c", -10.0, 0.5, 10.0)),
]


def callback() -> pa.Outputs:
    a = slider_a.value
    b = slider_b.value
    c = slider_c.value

    xs = np.linspace(-10.0, 10.0, 100)
    ys = a * xs**2 + b * xs + c

    return pa.Outputs(
        pa.Plot(xs, ys, x_limits=(-10, +10), y_limits=(-10, +10)),
    )


pa.run(inputs, callback)
