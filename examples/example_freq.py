import numpy as np

import picoapp as pa

_SAMPLE_RATE = 22050

inputs = pa.Inputs(
    slider_freq := pa.Slider(
        "Frequency", 20.0, 440.0, 10_000.0, log=True, decimal_places=2
    )
)


def create_sine(n: int, freq: float) -> np.ndarray:
    return np.sin(2.0 * np.pi * freq * np.arange(n) / _SAMPLE_RATE)


def callback() -> pa.Outputs:
    sine = create_sine(n=_SAMPLE_RATE, freq=slider_freq.value)
    return pa.Outputs(
        pa.Plot(xs=np.arange(len(sine)), ys=sine),
        pa.Audio(sine, sr=_SAMPLE_RATE),
    )


pa.run(pa.Reactive(inputs, callback))
