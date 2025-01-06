import numpy as np

import picoapp as pa

inputs = pa.Inputs(
    (slider_a := pa.Slider("a", -3.0, 1.0, 3.0)),
    (slider_b := pa.Slider("b", -3.0, 2.0, 3.0)),
)


def callback() -> pa.Outputs:
    print(f"{slider_a.value=} {slider_b.value=}")
    a = slider_a.value
    b = slider_b.value

    xs = a ** np.arange(200)
    ys = b ** np.arange(100)

    matrix = np.outer(xs, ys)

    return pa.Outputs(
        pa.MatrixPlot(matrix=matrix),
    )


pa.run(pa.Reactive(inputs, callback))
