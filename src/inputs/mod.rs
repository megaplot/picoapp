use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

mod sliders;
pub use sliders::Slider;

pub enum Input {
    Slider(Slider<f64>),
    IntSlider(Slider<i64>),
}

impl<'py> FromPyObject<'py> for Input {
    fn extract_bound(obj: &Bound<'py, PyAny>) -> PyResult<Self> {
        if obj.get_type().name()? == "Slider" {
            Ok(Input::Slider(obj.extract()?))
        } else if obj.get_type().name()? == "IntSlider" {
            Ok(Input::IntSlider(obj.extract()?))
        } else {
            return Err(PyValueError::new_err("Invalid callback return type."));
        }
    }
}

pub type Inputs = Vec<Input>;
