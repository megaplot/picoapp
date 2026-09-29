use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PySequence;

use crate::inputs::{Input, InputValue, Inputs, split_inputs};
use crate::ui::run_ui;
use crate::utils::Callback;
use crate::worker::{Registry, UiLevelResult};

#[pyfunction]
fn run(inputs: &Bound<'_, PySequence>, callback: &Bound<'_, PyAny>) -> PyResult<()> {
    let py = inputs.py();
    let inputs: Inputs = inputs.extract()?;
    let callback: Callback = callback.extract()?;
    run_ui(py, inputs, callback)?;
    Ok(())
}

/// Exposing the input parsing is currently only needed for unit testing.
/// TODO: Figure out a way how to test the "value setting" part as well.
#[pyfunction]
#[pyo3(name = "_parse_input")]
fn parse_input(_input: &Bound<'_, PyAny>) -> PyResult<()> {
    let _input: Input = _input.extract()?;
    Ok(())
}

/// Test-only hook: runs exactly one job through the real worker logic
/// (value writes, callback call, output parsing) without any gpui or
/// threading involved. Returns a short human-readable summary so pytest
/// can assert on outcomes without needing bindings for every Rust type.
///
/// Exposed for the same reason as `_parse_input`: it's otherwise very
/// hard to unit test the "value setting + callback + parse" path from
/// Python.
#[pyfunction]
#[pyo3(name = "_run_worker_job")]
fn run_worker_job(
    inputs: &Bound<'_, PySequence>,
    callback: &Bound<'_, PyAny>,
    values: &Bound<'_, PySequence>,
) -> PyResult<String> {
    let py = inputs.py();

    let inputs: Inputs = inputs.extract()?;
    let (_, bindings) = split_inputs(inputs);
    let callback: Callback = callback.extract()?;

    if values.len()? != bindings.len() {
        return Err(PyValueError::new_err(format!(
            "values has {} entries, but there are {} inputs",
            values.len()?,
            bindings.len()
        )));
    }

    let mut input_values = Vec::new();
    for (binding, raw_value) in bindings.iter().zip(values.try_iter()?) {
        let raw_value = raw_value?;
        let value = match binding {
            crate::inputs::InputBinding::Slider(_) => InputValue::F64(raw_value.extract()?),
            crate::inputs::InputBinding::IntSlider(_) => InputValue::I64(raw_value.extract()?),
            crate::inputs::InputBinding::Checkbox(_) => InputValue::Bool(raw_value.extract()?),
            crate::inputs::InputBinding::Radio(_) => InputValue::Index(raw_value.extract()?),
        };
        input_values.push(value);
    }

    let mut registry = Registry::new();
    let level = registry.register(bindings, callback);
    let result = registry.run_job(py, level, &input_values);

    Ok(match result {
        UiLevelResult::Outputs(outputs) => format!("Outputs({})", outputs.len()),
        UiLevelResult::Nested { specs, .. } => format!("Nested({})", specs.len()),
        UiLevelResult::Error(msg) => format!("Error({msg})"),
        UiLevelResult::Discarded => "Discarded".to_string(),
    })
}

#[pymodule]
fn _picoapp(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(run, m)?)?;
    m.add_function(wrap_pyfunction!(parse_input, m)?)?;
    m.add_function(wrap_pyfunction!(run_worker_job, m)?)?;
    Ok(())
}
