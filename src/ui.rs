use cushy::figures::units::UPx;
use cushy::figures::Size;
use cushy::value::Dynamic;
use cushy::widget::MakeWidget;
use cushy::Run;
use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use pyo3::types::PyFunction;

use crate::conversion::Input;
use crate::ui_inputs::input_widget;

pub fn run_ui(sliders: &[Input], callback: &Bound<'_, PyFunction>) -> PyResult<()> {
    let py = callback.py();
    let callback = callback.clone().unbind();

    py.allow_threads(|| {
        // For controlling initial window size see: https://github.com/khonsulabs/cushy/discussions/159
        let inner_size = Dynamic::new(Size::new(UPx::new(1600), UPx::new(1000)));

        let window = Python::with_gil(|py| {
            input_widget(py, sliders, callback)
                .into_window()
                .inner_size(inner_size)
                .titled("pico app")
        });
        let result = window.run();
        result.map_err(|e| PyRuntimeError::new_err(format!("Failed to run widget: {}", e)))
    })
}
