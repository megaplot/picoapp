import numpy as np

import picoapp as pa

inputs = pa.Inputs(
    (slider_a := pa.Slider("a", -10.0, 0.5, 10.0)),
    (slider_b := pa.Slider("b", -10.0, 0.5, 10.0)),
    (slider_c := pa.Slider("c", -10.0, 0.5, 10.0)),
    (negate := pa.Checkbox("Negate")),
)


def callback() -> pa.Outputs:
    print(f"{slider_a.value=} {slider_b.value=} {slider_c.value=} {negate.value=}")
    a = slider_a.value
    b = slider_b.value
    c = slider_c.value

    xs = np.linspace(-10.0, 10.0, 100)
    ys = a * xs**2 + b * xs + c

    if negate:
        ys *= -1

    return pa.Outputs(
        pa.Plot(xs, ys, x_limits=(-10, +10), y_limits=(-10, +10)),
    )


pa.run(pa.Reactive(inputs, callback))
