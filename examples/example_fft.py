import numpy as np

import picoapp
from picoapp import Audio, IntSlider, Outputs, Plot, Radio, Slider

_SAMPLE_RATE = 22050


elements = [
    (slider_freq := Slider("Frequency", 20.0, 440.0, 10_000.0, log=True)),
    (slider_kernel_size := IntSlider("Kernel size", 8, 16, 32)),
    (radio_window := Radio("Window", ["Box", "Hann", "Hamming"])),
]


def create_sine(n: int, freq: float) -> np.ndarray:
    return np.sin(2.0 * np.pi * freq * np.arange(n) / _SAMPLE_RATE)


def callback() -> Outputs:
    audio = create_sine(n=_SAMPLE_RATE, freq=slider_freq.value)

    n = slider_kernel_size.value

    phases = 2 * np.pi * 3 * np.arange(n) / n
    kernel = np.cos(phases) + 1j * np.sin(phases)

    print(radio_window.value)

    return Outputs(
        Plot(
            xs=np.arange(len(audio)),
            ys=np.abs(np.fft.fft(kernel, n=len(audio))),
        ),
        Audio(audio, sr=_SAMPLE_RATE),
    )


picoapp.run(elements, callback)
