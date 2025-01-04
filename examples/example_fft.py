import numpy as np

import picoapp as pa

_SAMPLE_RATE = 22050


inputs = pa.Inputs(
    (slider_freq := pa.Slider("Frequency", 20.0, 440.0, 10_000.0, log=True)),
    (slider_kernel_size := pa.IntSlider("Kernel size", 16, 64, 1024)),
    (radio_window := pa.Radio("Window", ["Box", "Hann", "Hamming"])),
    (
        radio_complex_mode := pa.Radio(
            "Complex Mode",
            ["complex", "real: cosine", "real: sine", "real: chained/convolved"],
        )
    ),
)


def create_sine(n: int, freq: float) -> np.ndarray:
    return np.sin(2.0 * np.pi * freq * np.arange(n) / _SAMPLE_RATE)


def callback() -> pa.Outputs:
    print(f"{slider_freq.value=} {slider_kernel_size.value=} {radio_window.value=}")

    freq = slider_freq.value
    n_kernel = slider_kernel_size.value

    phases = 2 * np.pi * freq * np.arange(n_kernel) / _SAMPLE_RATE
    kernel = np.cos(phases) + 1j * np.sin(phases)

    if radio_complex_mode.value == "real: cosine":
        kernel = np.real(kernel)
    elif radio_complex_mode.value == "real: sine":
        kernel = np.imag(kernel)
    elif radio_complex_mode.value == "real: chained/convolved":
        kernel = np.convolve(np.real(kernel), np.imag(kernel), mode="full")
        n_kernel = len(kernel)

    window = None
    if radio_window.value == "Hann":
        window = np.hanning(n_kernel)
    elif radio_window.value == "Hamming":
        window = np.hamming(n_kernel)

    if window is not None:
        kernel *= window

    # When using `np.fft.fft` with an implicit length that is larger then the signal
    # itself, it gets zero padded, leading to an increased spectral resolution.
    n_block = _SAMPLE_RATE

    return pa.Outputs(
        pa.Plot(
            xs=np.arange(n_kernel),
            ys=np.real(kernel),
        ),
        pa.Plot(
            xs=np.arange(n_kernel),
            ys=np.imag(kernel),
        ),
        pa.Plot(
            xs=np.arange(n_block) / n_block * 2 * _SAMPLE_RATE,
            ys=np.abs(np.fft.fft(kernel, n=n_block)),
        ),
    )


pa.run(pa.Reactive(inputs, callback))
