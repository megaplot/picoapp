import numpy as np

import picoapp as pa


def main() -> None:

    inputs = pa.Inputs(
        (master_slider := pa.IntSlider("Number of sliders", 1, 5, 10)),
    )

    def callback_1() -> pa.Reactive:

        order = master_slider.value
        print(f"Polynomial order: {order}")

        sliders = [
            pa.Slider(f"coefficient of x^{i}", -10.0, 0.5, 10.0)
            for i in range(order + 1)
        ]

        def callback_2() -> pa.Outputs:
            xs = np.linspace(-10.0, 10.0, 100)
            ys = np.zeros_like(xs)
            for k in range(order + 1):
                ys += sliders[k].value * xs**k
            return pa.Outputs(
                pa.Plot(xs, ys, y_limits=(-10, +10)),
            )

        return pa.Reactive(pa.Inputs(*sliders), callback=callback_2)

    pa.run(pa.Reactive(inputs, callback_1))


if __name__ == "__main__":
    main()
