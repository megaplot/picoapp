use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

mod radio;
mod slider;
pub use radio::Radio;
pub use slider::Slider;

pub enum Input {
    Slider(Slider<f64>),
    IntSlider(Slider<i64>),
    Radio(Radio),
}

impl<'py> FromPyObject<'py> for Input {
    fn extract_bound(obj: &Bound<'py, PyAny>) -> PyResult<Self> {
        if obj.get_type().name()? == "Slider" {
            Ok(Input::Slider(obj.extract()?))
        } else if obj.get_type().name()? == "IntSlider" {
            Ok(Input::IntSlider(obj.extract()?))
        } else if obj.get_type().name()? == "Radio" {
            Ok(Input::Radio(obj.extract()?))
        } else {
            return Err(PyValueError::new_err(format!(
                "Invalid callback return type: {:?}",
                obj.get_type().name()?
            )));
        }
    }
}

pub type Inputs = Vec<Input>;
