use pyo3::prelude::*;

/// Pure-Rust description of a `Slider`/`IntSlider`, sent to the UI thread.
#[derive(Debug, Clone)]
pub struct SliderSpec<T> {
    pub name: String,
    pub min: T,
    /// The slider's current value.
    pub value: T,
    pub max: T,
    // Leaky abstraction: So far the following is only supported (or makes only sense)
    // for float sliders.
    pub log: bool,
    pub decimal_places: Option<usize>,
}

// https://github.com/PyO3/pyo3/discussions/3058
impl<'a, 'py, T> FromPyObject<'a, 'py> for SliderSpec<T>
where
    T: FromPyObjectOwned<'py>,
{
    type Error = PyErr;

    fn extract(obj: Borrowed<'a, 'py, PyAny>) -> Result<Self, Self::Error> {
        let name: String = obj.getattr("_name")?.extract()?;
        // `?` needs `PyErr: From<T::Error>`, but `T::Error` is only known to
        // satisfy the (equivalent) `Into<PyErr>` at this generic call site —
        // https://pyo3.rs, "FromPyObject reworked" migration note.
        let min: T = obj.getattr("_min")?.extract().map_err(Into::into)?;
        let value: T = obj.getattr("_value")?.extract().map_err(Into::into)?;
        let max: T = obj.getattr("_max")?.extract().map_err(Into::into)?;
        let log: bool = obj.hasattr("_log")? && obj.getattr("_log")?.extract()?;
        let decimal_places: Option<usize> = if obj.hasattr("_decimal_places")? {
            obj.getattr("_decimal_places")?.extract()?
        } else {
            None
        };

        Ok(SliderSpec {
            name,
            min,
            value,
            max,
            log,
            decimal_places,
        })
    }
}
