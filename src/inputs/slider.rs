use std::marker::PhantomData;

use pyo3::prelude::*;

/// Wrapper newtype for the underlying PyObject instance. Stays on the worker
/// thread; never sent to the UI.
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

/// The pure-Rust description of a slider, safe to send to the UI thread.
#[derive(Debug, Clone)]
pub struct SliderSpec<T> {
    pub name: String,
    pub min: T,
    pub init: T,
    pub max: T,
    // Leaky abstraction: so far only meaningful for float sliders.
    pub log: bool,
    pub decimal_places: Option<usize>,
}

/// The Python handle used to write a slider's `_value`. Stays on the worker.
#[derive(Debug)]
pub struct SliderBinding<T>
where
    T: IntoPy<Py<PyAny>>,
{
    py_slider: PySlider<T>,
}

impl<T> SliderBinding<T>
where
    T: IntoPy<Py<PyAny>>,
{
    pub fn set_value(&self, py: Python<'_>, value: T) -> PyResult<()> {
        self.py_slider.set_value(py, value)
    }
}

pub fn parse_slider<'py, T>(
    obj: &Bound<'py, PyAny>,
) -> PyResult<(SliderSpec<T>, SliderBinding<T>)>
where
    T: FromPyObject<'py> + IntoPy<Py<PyAny>>,
{
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

    Ok((
        SliderSpec {
            name,
            min,
            init,
            max,
            log,
            decimal_places,
        },
        SliderBinding {
            py_slider: PySlider::new(obj.clone().unbind()),
        },
    ))
}
