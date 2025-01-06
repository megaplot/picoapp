import numpy as np

import picoapp as pa

inputs = pa.Inputs(
    (slider_wavelen_signal := pa.IntSlider("Wave Length Signal", 8, 16, 64)),
    (slider_repeat_signal := pa.IntSlider("Repeat Signal", 1, 8, 16)),
    (slider_wavelen_filter := pa.IntSlider("Wave Length Filter", 8, 16, 64)),
    (slider_repeat_filter := pa.IntSlider("Repeat Filter", 1, 2, 4)),
    (radio_window := pa.Radio("Window", ["Box", "Hann", "Hamming"])),
    (
        radio_complex_mode := pa.Radio(
            "Complex Mode",
            ["complex", "real: cosine", "real: sine", "real: chained/convolved"],
        )
    ),
)


def callback() -> pa.Outputs:

    wavelen_signal = slider_wavelen_signal.value
    wavelen_filter = slider_wavelen_filter.value
    repeat_signal = slider_repeat_signal.value
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

    signal = np.concatenate(
        [
            np.zeros(len(kernel)),
            np.sin(
                2 * np.pi * np.arange(wavelen_signal * repeat_signal) / wavelen_signal
            ),
            np.zeros(len(kernel)),
        ]
    )

    signal_convolved = np.convolve(signal, kernel, mode="same")

    n_max = max(len(signal), len(kernel))

    return pa.Outputs(
        pa.Plot(
            xs=np.arange(n_max),
            ys=np.pad(np.real(kernel), (0, n_max - len(kernel))),
        ),
        pa.Plot(
            xs=np.arange(n_max),
            ys=np.pad(np.imag(kernel), (0, n_max - len(kernel))),
        ),
        pa.Plot(
            xs=np.arange(len(signal)),
            ys=signal,
        ),
        pa.Plot(
            xs=np.arange(len(signal_convolved)),
            ys=np.real(signal_convolved),
        ),
        pa.Plot(
            xs=np.arange(len(signal_convolved)),
            ys=np.imag(signal_convolved),
        ),
        pa.Plot(
            xs=np.arange(len(signal_convolved)),
            ys=np.abs(signal_convolved),
        ),
    )


pa.run(pa.Reactive(inputs, callback))
