import numpy as np

import picoapp
from picoapp import Audio, Outputs, Plot, Slider

_SAMPLE_RATE = 22050

elements = [
    (slider_freq := Slider("Frequency", 20.0, 440.0, 10_000.0)),
]


def create_sine(n: int, freq: float) -> np.ndarray:
    return np.sin(2.0 * np.pi * freq * np.arange(n) / _SAMPLE_RATE)


def callback() -> Outputs:
    sine = create_sine(n=_SAMPLE_RATE, freq=slider_freq.value)
    return Outputs(
        Plot(xs=np.arange(len(sine)), ys=sine),
        Audio(sine, sr=_SAMPLE_RATE),
    )


picoapp.run(elements, callback)
