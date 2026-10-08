import numpy as np

import picoapp as pa


class PolynomialApp:
    """The same app as `example_dynamic_inputs.py`, with its state in a class."""

    def __init__(self, max_order: int = 10) -> None:
        self.order_slider = pa.IntSlider("Polynomial order", 0, 4, max_order)
        self.coefficient_sliders = [
            pa.Slider(f"coefficient of x^{i}", -10.0, 0.5, 10.0)
            for i in range(max_order + 1)
        ]
        # A node per instance: `pa.memoize` on the bound method.
        self.plot = pa.memoize(self._plot)

    def _plot(self) -> pa.Plot:
        order = self.order_slider.value
        print(f"Polynomial order: {order}")
        xs = np.linspace(-10.0, 10.0, 100)
        ys = np.zeros_like(xs)
        for k in range(order + 1):
            ys += self.coefficient_sliders[k].value * xs**k
        return pa.Plot(xs, ys, y_limits=(-10, +10))

    def __call__(self) -> pa.Element:
        return pa.Row(
            pa.Column(
                self.order_slider,
                *self.coefficient_sliders[: self.order_slider.value + 1],
            ),
            self.plot,
        )


def main() -> None:
    pa.run(PolynomialApp())


if __name__ == "__main__":
    main()
