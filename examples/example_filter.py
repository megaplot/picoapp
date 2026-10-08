from collections.abc import Callable

import numpy as np

import picoapp as pa

slider_wavelen_signal = pa.IntSlider("Wave Length Signal", 8, 16, 64)
slider_repeat_signal = pa.IntSlider("Repeat Signal", 1, 8, 16)
slider_wavelen_filter = pa.IntSlider("Wave Length Filter", 8, 16, 64)
slider_repeat_filter = pa.IntSlider("Repeat Filter", 1, 2, 4)
radio_window = pa.Radio("Window", ["Box", "Hann", "Hamming"])
radio_complex_mode = pa.Radio(
    "Complex Mode",
    ["complex", "real: cosine", "real: sine", "real: chained/convolved"],
)


# Each intermediate result is its own node: moving a signal slider re-runs
# `signal` and what depends on it, but not `kernel`.


@pa.memoize
def kernel() -> np.ndarray:
    wavelen_filter = slider_wavelen_filter.value
    repeat_filter = slider_repeat_filter.value

    phases = 2 * np.pi * np.arange(wavelen_filter * repeat_filter) / wavelen_filter
    kernel = np.cos(phases) + 1j * np.sin(phases)

    if radio_complex_mode.value == "real: cosine":
        kernel = np.real(kernel)
    elif radio_complex_mode.value == "real: sine":
        kernel = np.imag(kernel)
    elif radio_complex_mode.value == "real: chained/convolved":
        kernel = np.convolve(np.real(kernel), np.imag(kernel), mode="full")

    window = None
    if radio_window.value == "Hann":
        window = np.hanning(len(kernel) + 1)[:-1]
    elif radio_window.value == "Hamming":
        window = np.hamming(len(kernel) + 1)[:-1]

    if window is not None:
        kernel *= window

    return kernel


@pa.memoize
def signal() -> np.ndarray:
    wavelen_signal = slider_wavelen_signal.value
    repeat_signal = slider_repeat_signal.value
    return np.concatenate(
        [
            np.zeros(len(kernel())),
            np.sin(
                2 * np.pi * np.arange(wavelen_signal * repeat_signal) / wavelen_signal
            ),
            np.zeros(len(kernel())),
        ]
    )


@pa.memoize
def signal_convolved() -> np.ndarray:
    return np.convolve(signal(), kernel(), mode="same")


def kernel_plot(part: Callable[[np.ndarray], np.ndarray]) -> pa.Memoized[pa.Plot]:
    def plot() -> pa.Plot:
        n_max = max(len(signal()), len(kernel()))
        return pa.Plot(
            xs=np.arange(n_max),
            ys=np.pad(part(kernel()), (0, n_max - len(kernel()))),
        )

    return pa.memoize(plot)


def convolved_plot(
    part: Callable[[np.ndarray], np.ndarray],
) -> pa.Memoized[pa.Plot]:
    def plot() -> pa.Plot:
        return pa.Plot(
            xs=np.arange(len(signal_convolved())),
            ys=part(signal_convolved()),
        )

    return pa.memoize(plot)


plots = [
    kernel_plot(np.real),
    kernel_plot(np.imag),
    pa.memoize(lambda: pa.Plot(xs=np.arange(len(signal())), ys=signal())),
    convolved_plot(np.real),
    convolved_plot(np.imag),
    convolved_plot(np.abs),
]


def view() -> pa.Element:
    return pa.Row(
        pa.Column(
            slider_wavelen_signal,
            slider_repeat_signal,
            slider_wavelen_filter,
            slider_repeat_filter,
            radio_window,
            radio_complex_mode,
        ),
        pa.Column(*plots),
    )


pa.run(view)
