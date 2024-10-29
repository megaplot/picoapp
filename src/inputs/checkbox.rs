use pyo3::prelude::*;

/// Wrapper newtype for the underlying PyObject instance.
#[derive(Debug)]
pub struct PyCheckbox(PyObject);

impl PyCheckbox {
    pub fn new(obj: PyObject) -> Self {
        PyCheckbox(obj)
    }
    pub fn clone_ref(&self, py: Python<'_>) -> PyCheckbox {
        PyCheckbox::new(self.0.clone_ref(py))
    }
    pub fn set_value(&self, py: Python<'_>, value: bool) -> PyResult<()> {
        self.0.setattr(py, "_value", value)
    }
}

#[derive(Debug)]
pub struct Checkbox {
    pub name: String,
    pub init: bool,
    pub py_checkbox: PyCheckbox,
}

impl<'py> FromPyObject<'py> for Checkbox {
    fn extract_bound(obj: &Bound<'py, PyAny>) -> PyResult<Self> {
        let name: String = obj.getattr("_name")?.extract()?;
        let init: bool = obj.getattr("_init")?.extract()?;

        Ok(Checkbox {
            name,
            init,
            py_checkbox: PyCheckbox::new(obj.clone().unbind()),
        })
    }
}
