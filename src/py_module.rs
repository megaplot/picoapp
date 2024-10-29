use pyo3::prelude::*;
use pyo3::types::{PyFunction, PyList};

use super::inputs::{Input, Inputs};
use super::main_run_ui::run_ui;

#[pyfunction]
fn run(inputs: &Bound<'_, PyList>, callback: &Bound<'_, PyFunction>) -> PyResult<()> {
    let inputs: Inputs = inputs.extract()?;
    run_ui(&inputs, callback)?;
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
