import numpy as np

import picoapp as pa


class Outer(pa.ReactiveBase):
    def __init__(self) -> None:
        self.master_slider = pa.IntSlider("Number of sliders", 1, 5, 10)

    @property
    def inputs(self) -> pa.Inputs:
        return pa.Inputs(self.master_slider)

    def __call__(self) -> pa.ReactiveBase:
        order = self.master_slider.value
        print(f"Polynomial order: {order}")
        return Inner(order=order)


class Inner(pa.ReactiveBase):
    def __init__(self, order: int):
        self.order = order
        self.sliders = [
            pa.Slider(f"coefficient of x^{i}", -10.0, 0.5, 10.0)
            for i in range(order + 1)
        ]

    @property
    def inputs(self) -> pa.Inputs:
        return pa.Inputs(*self.sliders)

    def __call__(self) -> pa.Outputs:
        xs = np.linspace(-10.0, 10.0, 100)
        ys = np.zeros_like(xs)
        for k in range(self.order + 1):
            ys += self.sliders[k].value * xs**k
        return pa.Outputs(
            pa.Plot(xs, ys, y_limits=(-10, +10)),
        )


def main() -> None:
    pa.run(Outer())


if __name__ == "__main__":
    main()
