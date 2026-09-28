use std::marker::PhantomData;

use pyo3::prelude::*;

/// Wrapper newtype for the underlying PyObject instance.
#[derive(Debug)]
pub struct PySlider<T>(PyObject, PhantomData<T>)
where
    T: IntoPy<Py<PyAny>>;

impl<T> PySlider<T>
where
    T: IntoPy<Py<PyAny>>,
{
    pub fn new(obj: PyObject) -> Self {
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
    T: IntoPy<Py<PyAny>>,
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
    T: IntoPy<Py<PyAny>>,
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
impl<'py, T> FromPyObject<'py> for Slider<T>
where
    T: for<'a> FromPyObject<'a> + IntoPy<Py<PyAny>>,
{
    fn extract_bound(obj: &Bound<'py, PyAny>) -> PyResult<Self> {
        let name: String = obj.getattr("_name")?.extract()?;
        let min: T = obj.getattr("_min")?.extract()?;
        let init: T = obj.getattr("_init")?.extract()?;
        let max: T = obj.getattr("_max")?.extract()?;
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
            py_slider: PySlider::new(obj.clone().unbind()),
        })
    }
}
