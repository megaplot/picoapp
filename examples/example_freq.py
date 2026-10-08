import numpy as np

import picoapp as pa

_SAMPLE_RATE = 22050

slider_freq = pa.Slider("Frequency", 20.0, 440.0, 10_000.0, log=True, decimal_places=2)
slider_samples_shown = pa.IntSlider("Samples shown", 100, _SAMPLE_RATE, _SAMPLE_RATE)


def create_sine(n: int, freq: float) -> np.ndarray:
    return np.sin(2.0 * np.pi * freq * np.arange(n) / _SAMPLE_RATE)


@pa.memoize
def sine() -> np.ndarray:
    return create_sine(n=_SAMPLE_RATE, freq=slider_freq.value)


@pa.memoize
def plot() -> pa.Plot:
    n = slider_samples_shown.value
    return pa.Plot(xs=np.arange(n), ys=sine()[:n])


# Only depends on the frequency: moving "Samples shown" keeps the audio playing.
@pa.memoize
def audio() -> pa.Audio:
    return pa.Audio(sine(), sr=_SAMPLE_RATE)


def view() -> pa.Element:
    return pa.Row(pa.Column(slider_freq, slider_samples_shown), pa.Column(plot, audio))


pa.run(view)
