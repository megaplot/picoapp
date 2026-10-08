import numpy as np

import picoapp as pa

slider_a = pa.Slider("a", -3.0, 1.0, 3.0)
slider_b = pa.Slider("b", -3.0, 2.0, 3.0)


@pa.memoize
def matrix_plot() -> pa.MatrixPlot:
    print(f"{slider_a.value=} {slider_b.value=}")
    a = slider_a.value
    b = slider_b.value

    xs = a ** np.arange(200)
    ys = b ** np.arange(100)

    matrix = np.outer(xs, ys)

    return pa.MatrixPlot(matrix=matrix)


def view() -> pa.Element:
    return pa.Row(pa.Column(slider_a, slider_b), matrix_plot)


pa.run(view)
