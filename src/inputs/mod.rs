use std::convert::Infallible;

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

mod checkbox;
mod radio;
mod slider;
pub use checkbox::CheckboxSpec;
pub use radio::RadioSpec;
pub use slider::SliderSpec;

/// Pure-Rust input description sent to the UI thread. Carries the input's
/// current value, since a re-shown input's widget is re-created from it.
#[derive(Debug, Clone)]
pub enum InputSpec {
    Slider(SliderSpec<f64>),
    IntSlider(SliderSpec<i64>),
    Checkbox(CheckboxSpec),
    Radio(RadioSpec),
}

impl<'a, 'py> FromPyObject<'a, 'py> for InputSpec {
    type Error = PyErr;

    fn extract(obj: Borrowed<'a, 'py, PyAny>) -> Result<Self, Self::Error> {
        if obj.get_type().name()? == "Slider" {
            Ok(InputSpec::Slider(obj.extract()?))
        } else if obj.get_type().name()? == "IntSlider" {
            Ok(InputSpec::IntSlider(obj.extract()?))
        } else if obj.get_type().name()? == "Checkbox" {
            Ok(InputSpec::Checkbox(obj.extract()?))
        } else if obj.get_type().name()? == "Radio" {
            Ok(InputSpec::Radio(obj.extract()?))
        } else {
            return Err(PyValueError::new_err(format!(
                "Invalid input type: {:?}",
                obj.get_type().name()?
            )));
        }
    }
}

impl InputSpec {
    /// The input's current value, as the UI sends it back.
    pub fn value(&self) -> InputValue {
        match self {
            InputSpec::Slider(s) => InputValue::F64(s.value),
            InputSpec::IntSlider(s) => InputValue::I64(s.value),
            InputSpec::Checkbox(s) => InputValue::Bool(s.value),
            InputSpec::Radio(s) => InputValue::Index(s.index),
        }
    }
}

/// A value coming back from the UI, sent to the worker to write into the
/// input (via `Engine.set_values`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InputValue {
    F64(f64),
    I64(i64),
    Bool(bool),
    Index(usize),
}

impl<'py> IntoPyObject<'py> for InputValue {
    type Target = PyAny;
    type Output = Bound<'py, PyAny>;
    type Error = Infallible;

    fn into_pyobject(self, py: Python<'py>) -> Result<Self::Output, Self::Error> {
        Ok(match self {
            InputValue::F64(v) => v.into_pyobject(py)?.into_any(),
            InputValue::I64(v) => v.into_pyobject(py)?.into_any(),
            InputValue::Bool(v) => v.into_pyobject(py)?.to_owned().into_any(),
            InputValue::Index(v) => v.into_pyobject(py)?.into_any(),
        })
    }
}
