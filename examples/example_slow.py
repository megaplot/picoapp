import time

import numpy as np

import picoapp as pa

inputs = pa.Inputs(
    (delay := pa.Slider("Callback delay (s)", 0.0, 1.0, 3.0)),
    (raise_above := pa.Slider("Raise above", 0.0, 10.0, 11.0)),
)


def callback() -> pa.Outputs:
    time.sleep(delay.value)
    if delay.value > raise_above.value:
        raise ValueError(f"delay.value={delay.value} exceeded raise_above.value={raise_above.value}")

    xs = np.linspace(-10.0, 10.0, 100)
    ys = np.sin(xs + delay.value)
    return pa.Outputs(pa.Plot(xs, ys, y_limits=(-1.5, 1.5)))


pa.run(pa.Reactive(inputs, callback))
