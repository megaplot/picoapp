use pyo3::prelude::*;
use pyo3::types::PyList;

/// Wrapper newtype for the underlying PyObject instance.
#[derive(Debug)]
pub struct PyRadio(PyObject);

impl PyRadio {
    pub fn new(obj: PyObject) -> Self {
        PyRadio(obj)
    }
    pub fn clone_ref(&self, py: Python<'_>) -> PyRadio {
        PyRadio::new(self.0.clone_ref(py))
    }
    pub fn set_to_index(&self, py: Python<'_>, index: usize) -> PyResult<()> {
        let py_radio = self.0.bind(py);
        let values = py_radio.getattr("_values")?.downcast_into::<PyList>()?;
        py_radio.setattr("_value", values.get_item(index)?)
    }
}

#[derive(Debug)]
pub struct Radio {
    pub name: String,
    // Note that a radio is not concerned with the underlying user (Python)
    // type, it only cares about the string representations of the values
    // and internally operates on indices.
    pub init_index: usize,
    pub value_names: Vec<String>,
    pub py_radio: PyRadio,
}

impl<'py> FromPyObject<'py> for Radio {
    fn extract_bound(obj: &Bound<'py, PyAny>) -> PyResult<Self> {
        let name: String = obj.getattr("_name")?.extract()?;
        let init_index: usize = obj.getattr("_init_index")?.extract()?;

        // TODO: We probably have to make explicit __str__ calls to the elements of
        // the list here.
        let value_names: Vec<String> = obj.getattr("_values")?.extract()?;

        Ok(Radio {
            name,
            init_index,
            value_names,
            py_radio: PyRadio::new(obj.clone().unbind()),
        })
    }
}
