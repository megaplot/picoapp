use pyo3::prelude::*;
use pyo3::types::PySequence;

use crate::inputs::{Input, Inputs};
use crate::main_run_ui::run_ui;
use crate::utils::Callback;

#[pyfunction]
fn run(inputs: &Bound<'_, PySequence>, callback: &Bound<'_, PyAny>) -> PyResult<()> {
    let py = inputs.py();
    let inputs: Inputs = inputs.extract()?;
    let callback: Callback = callback.extract()?;
    run_ui(py, &inputs, callback)?;
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

#[pymodule]
fn _picoapp(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(run, m)?)?;
    m.add_function(wrap_pyfunction!(parse_input, m)?)?;
    Ok(())
}
