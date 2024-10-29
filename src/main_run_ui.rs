use cushy::figures::units::UPx;
use cushy::figures::Size;
use cushy::value::Dynamic;
use cushy::widget::MakeWidget;
use cushy::window::ThemeMode;
use cushy::Run;
use log::info;
use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;

use crate::inputs::Input;
use crate::logging_setup::setup_logging;
use crate::utils::Callback;
use crate::widgets::reactive_input_output_widget;

pub fn run_ui(py: Python<'_>, sliders: &[Input], callback: Callback) -> PyResult<()> {
    setup_logging();

    py.allow_threads(|| {
        // For controlling initial window size see: https://github.com/khonsulabs/cushy/discussions/159
        let inner_size = Dynamic::new(Size::new(UPx::new(1600), UPx::new(1000)));

        info!("Initialing app...");

        let window = Python::with_gil(|py| {
            reactive_input_output_widget(py, sliders, &callback)
                .into_window()
                .inner_size(inner_size)
                .titled("pico app")
                .themed_mode(ThemeMode::Dark)
        });
        let result = window.run();
        result.map_err(|e| PyRuntimeError::new_err(format!("Failed to run widget: {}", e)))
    })
}
