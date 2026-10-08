import numpy as np

import picoapp as pa

_SAMPLE_RATE = 22050

slider_freq = pa.Slider("Frequency", 20.0, 440.0, 10_000.0, log=True, decimal_places=2)


def create_sine(n: int, freq: float) -> np.ndarray:
    return np.sin(2.0 * np.pi * freq * np.arange(n) / _SAMPLE_RATE)


@pa.memoize
def outputs() -> pa.Column:
    sine = create_sine(n=_SAMPLE_RATE, freq=slider_freq.value)
    return pa.Column(
        pa.Plot(xs=np.arange(len(sine)), ys=sine),
        pa.Audio(sine, sr=_SAMPLE_RATE),
    )


def view() -> pa.Element:
    return pa.Row(slider_freq, outputs)


pa.run(view)
