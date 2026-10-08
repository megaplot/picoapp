use pyo3::prelude::*;
use pyo3::types::PySequence;

/// Pure-Rust description of a `Radio`, sent to the UI thread.
#[derive(Debug, Clone)]
pub struct RadioSpec {
    pub name: String,
    // Note that a radio is not concerned with the underlying user (Python)
    // type, it only cares about the string representations of the values
    // and internally operates on indices.
    /// Index of the currently selected value.
    pub index: usize,
    pub value_names: Vec<String>,
}

impl<'a, 'py> FromPyObject<'a, 'py> for RadioSpec {
    type Error = PyErr;

    fn extract(obj: Borrowed<'a, 'py, PyAny>) -> Result<Self, Self::Error> {
        let name: String = obj.getattr("_name")?.extract()?;
        let index: usize = obj.getattr("_index")?.extract()?;

        // Note that a radio supports arbitrary underlying types, and we are using `__str__` calls to
        // infer the label strings.
        let mut value_names = Vec::<String>::new();
        let raw_values = obj.getattr("_values")?;
        let raw_values = raw_values.cast::<PySequence>()?;
        for raw_value in raw_values.try_iter()? {
            let raw_value = raw_value?;
            let value_name: String = raw_value.call_method("__str__", (), None)?.extract()?;
            value_names.push(value_name);
        }

        Ok(RadioSpec {
            name,
            index,
            value_names,
        })
    }
}
