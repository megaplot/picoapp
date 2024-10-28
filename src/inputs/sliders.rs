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
    pub fn clone_ref(&self, py: Python<'_>) -> PySlider<T> {
        PySlider::new(self.0.clone_ref(py))
    }
    pub fn set_value(&self, py: Python<'_>, value: T) -> PyResult<()> {
        self.0.setattr(py, "_value", value)
    }
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
    pub py_slider: PySlider<T>,
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

        Ok(Slider {
            name,
            min,
            init,
            max,
            py_slider: PySlider::new(obj.clone().unbind()),
        })
    }
}
