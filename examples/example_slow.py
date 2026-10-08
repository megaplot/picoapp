import time

import numpy as np

import picoapp as pa

delay = pa.Slider("Callback delay (s)", 0.0, 1.0, 3.0)
raise_above = pa.Slider("Raise above", 0.0, 10.0, 11.0)


# A failing run shows an error card above the plot's last good version, and
# the sliders stay usable: `view` itself never calls this node.
@pa.memoize
def plot() -> pa.Plot:
    time.sleep(delay.value)
    if delay.value > raise_above.value:
        raise ValueError(
            f"delay.value={delay.value} exceeded raise_above.value={raise_above.value}"
        )

    xs = np.linspace(-10.0, 10.0, 100)
    ys = np.sin(xs + delay.value)
    return pa.Plot(xs, ys, y_limits=(-1.5, 1.5))


def view() -> pa.Element:
    return pa.Row(pa.Column(delay, raise_above), plot)


pa.run(view)
