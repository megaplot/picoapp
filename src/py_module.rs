use pyo3::prelude::*;

use crate::ui::run_ui;
use crate::view_tree::parse_slot_content;

#[pyfunction]
fn run(engine: &Bound<'_, PyAny>) -> PyResult<()> {
    run_ui(engine.py(), engine.clone().unbind())
}

/// Test-only hook: parses a slot content (an element or an error string) the
/// way the worker does, and returns `SlotContent::summary`. Raises the parse
/// error, if any.
///
/// Exposed because it's otherwise very hard to unit test the Python -> Rust
/// parsing from Python.
#[pyfunction]
#[pyo3(name = "_parse_slot_content")]
fn parse_slot_content_summary(content: &Bound<'_, PyAny>) -> PyResult<String> {
    Ok(parse_slot_content(content)?.summary())
}

#[pymodule]
fn _picoapp(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(run, m)?)?;
    m.add_function(wrap_pyfunction!(parse_slot_content_summary, m)?)?;
    Ok(())
}
