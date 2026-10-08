import numpy as np

import picoapp as pa

# All coefficient sliders exist up front; `view` only shows the first ones. A
# coefficient keeps its value while hidden, so reducing and increasing the
# order again restores it.
max_order = 10
order_slider = pa.IntSlider("Polynomial order", 0, 4, max_order)
coefficient_sliders = [
    pa.Slider(f"coefficient of x^{i}", -10.0, 0.5, 10.0) for i in range(max_order + 1)
]


@pa.memoize
def plot() -> pa.Plot:
    order = order_slider.value
    print(f"Polynomial order: {order}")
    xs = np.linspace(-10.0, 10.0, 100)
    ys = np.zeros_like(xs)
    for k in range(order + 1):
        ys += coefficient_sliders[k].value * xs**k
    return pa.Plot(xs, ys, y_limits=(-10, +10))


def view() -> pa.Element:
    return pa.Row(
        pa.Column(order_slider, *coefficient_sliders[: order_slider.value + 1]),
        plot,
    )


def main() -> None:
    pa.run(view)


if __name__ == "__main__":
    main()
