use pyo3::prelude::*;

#[derive(Debug)]
pub struct PyCheckbox(PyObject);

impl PyCheckbox {
    pub fn new(obj: PyObject) -> Self {
        PyCheckbox(obj)
    }
    pub fn set_value(&self, py: Python<'_>, value: bool) -> PyResult<()> {
        self.0.setattr(py, "_value", value)
    }
}

#[derive(Debug, Clone)]
pub struct CheckboxSpec {
    pub name: String,
    pub init: bool,
}

#[derive(Debug)]
pub struct CheckboxBinding {
    py_checkbox: PyCheckbox,
}

impl CheckboxBinding {
    pub fn set_value(&self, py: Python<'_>, value: bool) -> PyResult<()> {
        self.py_checkbox.set_value(py, value)
    }
}

pub fn parse_checkbox(obj: &Bound<'_, PyAny>) -> PyResult<(CheckboxSpec, CheckboxBinding)> {
    let name: String = obj.getattr("_name")?.extract()?;
    let init: bool = obj.getattr("_init")?.extract()?;
    Ok((
        CheckboxSpec { name, init },
        CheckboxBinding {
            py_checkbox: PyCheckbox::new(obj.clone().unbind()),
        },
    ))
}

/// Kept for compatibility with the old name used by the Input enum's
/// `Debug`; not otherwise used.
pub type Checkbox = CheckboxSpec;
