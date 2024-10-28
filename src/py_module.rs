use pyo3::prelude::*;
use pyo3::types::{PyFunction, PyList};

use super::inputs::Inputs;
use super::main_run_ui::run_ui;

#[pyfunction]
fn run(inputs: &Bound<'_, PyList>, callback: &Bound<'_, PyFunction>) -> PyResult<()> {
    let inputs: Inputs = inputs.extract()?;
    run_ui(&inputs, callback)?;
    Ok(())
}

#[pymodule]
fn _picoapp(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(run, m)?)?;
    Ok(())
}
