use pyo3::prelude::*;

/// Wrapper newtype for the underlying PyObject instance.
#[derive(Debug)]
pub struct PyCheckbox(Py<PyAny>);

impl PyCheckbox {
    pub fn new(obj: Py<PyAny>) -> Self {
        PyCheckbox(obj)
    }
    pub fn set_value(&self, py: Python<'_>, value: bool) -> PyResult<()> {
        self.0.setattr(py, "_value", value)
    }
}

/// Pure-Rust half of a `Checkbox`, sent to the UI thread; see `into_parts`.
#[derive(Debug, Clone)]
pub struct CheckboxSpec {
    pub name: String,
    pub init: bool,
}

#[derive(Debug)]
pub struct Checkbox {
    pub name: String,
    pub init: bool,
    pub py_checkbox: PyCheckbox,
}

impl Checkbox {
    /// Splits into the plain-data half sent to the UI and the `PyObject`
    /// handle that stays on the worker thread.
    pub fn into_parts(self) -> (CheckboxSpec, PyCheckbox) {
        let Checkbox {
            name,
            init,
            py_checkbox,
        } = self;
        (CheckboxSpec { name, init }, py_checkbox)
    }
}

impl<'a, 'py> FromPyObject<'a, 'py> for Checkbox {
    type Error = PyErr;

    fn extract(obj: Borrowed<'a, 'py, PyAny>) -> Result<Self, Self::Error> {
        let name: String = obj.getattr("_name")?.extract()?;
        let init: bool = obj.getattr("_init")?.extract()?;

        Ok(Checkbox {
            name,
            init,
            py_checkbox: PyCheckbox::new(obj.to_owned().unbind()),
        })
    }
}
