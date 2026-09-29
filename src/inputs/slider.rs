use std::marker::PhantomData;

use pyo3::prelude::*;

/// Wrapper newtype for the underlying PyObject instance.
#[derive(Debug)]
pub struct PySlider<T>(Py<PyAny>, PhantomData<T>)
where
    T: for<'py> IntoPyObject<'py>;

impl<T> PySlider<T>
where
    T: for<'py> IntoPyObject<'py>,
{
    pub fn new(obj: Py<PyAny>) -> Self {
        PySlider(obj, PhantomData)
    }
    pub fn set_value(&self, py: Python<'_>, value: T) -> PyResult<()> {
        self.0.setattr(py, "_value", value)
    }
}

/// Pure-Rust half of a `Slider<T>`, sent to the UI thread; see `into_parts`.
#[derive(Debug, Clone)]
pub struct SliderSpec<T> {
    pub name: String,
    pub min: T,
    pub init: T,
    pub max: T,
    // Leaky abstraction: So far the following is only supported (or makes only sense)
    // for float sliders.
    pub log: bool,
    pub decimal_places: Option<usize>,
}

#[derive(Debug)]
pub struct Slider<T>
where
    T: for<'py> IntoPyObject<'py>,
{
    pub name: String,
    pub min: T,
    pub init: T,
    pub max: T,
    // Leaky abstraction: So far the following is only supported (or makes only sense)
    // for float sliders.
    pub log: bool,
    pub decimal_places: Option<usize>,
    pub py_slider: PySlider<T>,
}

impl<T> Slider<T>
where
    T: for<'py> IntoPyObject<'py>,
{
    /// Splits into the plain-data half sent to the UI and the `PyObject`
    /// handle that stays on the worker thread.
    pub fn into_parts(self) -> (SliderSpec<T>, PySlider<T>) {
        let Slider {
            name,
            min,
            init,
            max,
            log,
            decimal_places,
            py_slider,
        } = self;
        (
            SliderSpec {
                name,
                min,
                init,
                max,
                log,
                decimal_places,
            },
            py_slider,
        )
    }
}

// https://github.com/PyO3/pyo3/discussions/3058
impl<'a, 'py, T> FromPyObject<'a, 'py> for Slider<T>
where
    T: FromPyObjectOwned<'py> + for<'p> IntoPyObject<'p>,
{
    type Error = PyErr;

    fn extract(obj: Borrowed<'a, 'py, PyAny>) -> Result<Self, Self::Error> {
        let name: String = obj.getattr("_name")?.extract()?;
        // `?` needs `PyErr: From<T::Error>`, but `T::Error` is only known to
        // satisfy the (equivalent) `Into<PyErr>` at this generic call site —
        // https://pyo3.rs, "FromPyObject reworked" migration note.
        let min: T = obj.getattr("_min")?.extract().map_err(Into::into)?;
        let init: T = obj.getattr("_init")?.extract().map_err(Into::into)?;
        let max: T = obj.getattr("_max")?.extract().map_err(Into::into)?;
        let log: bool = obj.hasattr("_log")? && obj.getattr("_log")?.extract()?;
        let decimal_places: Option<usize> = if obj.hasattr("_decimal_places")? {
            obj.getattr("_decimal_places")?.extract()?
        } else {
            None
        };

        Ok(Slider {
            name,
            min,
            init,
            max,
            log,
            decimal_places,
            py_slider: PySlider::new(obj.to_owned().unbind()),
        })
    }
}
