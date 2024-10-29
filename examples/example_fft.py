import numpy as np

import picoapp as pa

_SAMPLE_RATE = 22050


inputs = pa.Inputs(
    (slider_freq := pa.Slider("Frequency", 20.0, 440.0, 10_000.0, log=True)),
    (slider_kernel_size := pa.IntSlider("Kernel size", 8, 16, 32)),
    (radio_window := pa.Radio("Window", ["Box", "Hann", "Hamming"])),
)


def create_sine(n: int, freq: float) -> np.ndarray:
    return np.sin(2.0 * np.pi * freq * np.arange(n) / _SAMPLE_RATE)


def callback() -> pa.Outputs:
    audio = create_sine(n=_SAMPLE_RATE, freq=slider_freq.value)

    n = slider_kernel_size.value

    phases = 2 * np.pi * 3 * np.arange(n) / n
    kernel = np.cos(phases) + 1j * np.sin(phases)

    print(radio_window.value)

    return pa.Outputs(
        pa.Plot(
            xs=np.arange(len(audio)),
            ys=np.abs(np.fft.fft(kernel, n=len(audio))),
        ),
        pa.Audio(audio, sr=_SAMPLE_RATE),
    )


pa.run(pa.Reactive(inputs, callback))
