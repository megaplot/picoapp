use pyo3::prelude::*;

/// Pure-Rust description of a `Checkbox`, sent to the UI thread.
#[derive(Debug, Clone)]
pub struct CheckboxSpec {
    pub name: String,
    /// The checkbox's current value.
    pub value: bool,
}

impl<'a, 'py> FromPyObject<'a, 'py> for CheckboxSpec {
    type Error = PyErr;

    fn extract(obj: Borrowed<'a, 'py, PyAny>) -> Result<Self, Self::Error> {
        let name: String = obj.getattr("_name")?.extract()?;
        let value: bool = obj.getattr("_value")?.extract()?;

        Ok(CheckboxSpec { name, value })
    }
}
