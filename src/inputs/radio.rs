use pyo3::prelude::*;
use pyo3::types::PySequence;

#[derive(Debug)]
pub struct PyRadio(PyObject);

impl PyRadio {
    pub fn new(obj: PyObject) -> Self {
        PyRadio(obj)
    }
    pub fn set_to_index(&self, py: Python<'_>, index: usize) -> PyResult<()> {
        let py_radio = self.0.bind(py);
        let values = py_radio.getattr("_values")?.downcast_into::<PySequence>()?;
        py_radio.setattr("_value", values.get_item(index)?)
    }
}

#[derive(Debug, Clone)]
pub struct RadioSpec {
    pub name: String,
    pub init_index: usize,
    pub value_names: Vec<String>,
}

#[derive(Debug)]
pub struct RadioBinding {
    py_radio: PyRadio,
}

impl RadioBinding {
    pub fn set_to_index(&self, py: Python<'_>, index: usize) -> PyResult<()> {
        self.py_radio.set_to_index(py, index)
    }
}

pub fn parse_radio(obj: &Bound<'_, PyAny>) -> PyResult<(RadioSpec, RadioBinding)> {
    let name: String = obj.getattr("_name")?.extract()?;
    let init_index: usize = obj.getattr("_init_index")?.extract()?;

    let mut value_names = Vec::<String>::new();
    let raw_values = obj.getattr("_values")?;
    let raw_values = raw_values.downcast::<PySequence>()?;
    for raw_value in raw_values.iter()? {
        let raw_value = raw_value?;
        let value_name: String = raw_value.call_method("__str__", (), None)?.extract()?;
        value_names.push(value_name);
    }

    Ok((
        RadioSpec {
            name,
            init_index,
            value_names,
        },
        RadioBinding {
            py_radio: PyRadio::new(obj.clone().unbind()),
        },
    ))
}

pub type Radio = RadioSpec;
