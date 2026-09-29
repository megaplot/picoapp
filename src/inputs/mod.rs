use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

mod checkbox;
mod radio;
mod slider;
pub use checkbox::{Checkbox, CheckboxSpec, PyCheckbox};
pub use radio::{PyRadio, Radio, RadioSpec};
pub use slider::{PySlider, Slider, SliderSpec};

pub enum Input {
    Slider(Slider<f64>),
    IntSlider(Slider<i64>),
    Checkbox(Checkbox),
    Radio(Radio),
}

impl<'a, 'py> FromPyObject<'a, 'py> for Input {
    type Error = PyErr;

    fn extract(obj: Borrowed<'a, 'py, PyAny>) -> Result<Self, Self::Error> {
        if obj.get_type().name()? == "Slider" {
            Ok(Input::Slider(obj.extract()?))
        } else if obj.get_type().name()? == "IntSlider" {
            Ok(Input::IntSlider(obj.extract()?))
        } else if obj.get_type().name()? == "Checkbox" {
            Ok(Input::Checkbox(obj.extract()?))
        } else if obj.get_type().name()? == "Radio" {
            Ok(Input::Radio(obj.extract()?))
        } else {
            return Err(PyValueError::new_err(format!(
                "Invalid input type: {:?}",
                obj.get_type().name()?
            )));
        }
    }
}

pub type Inputs = Vec<Input>;

/// Pure-Rust input description sent to the UI thread; the other half of an
/// `Input` split by `Input::into_parts`/`split_inputs`.
#[derive(Debug, Clone)]
pub enum InputSpec {
    Slider(SliderSpec<f64>),
    IntSlider(SliderSpec<i64>),
    Checkbox(CheckboxSpec),
    Radio(RadioSpec),
}

/// Python handle kept on the worker thread, one per input, same order as
/// its `InputSpec` sibling.
pub enum InputBinding {
    Slider(PySlider<f64>),
    IntSlider(PySlider<i64>),
    Checkbox(PyCheckbox),
    Radio(PyRadio),
}

impl InputBinding {
    pub fn set_value(&self, py: Python<'_>, value: &InputValue) -> PyResult<()> {
        match (self, value) {
            (InputBinding::Slider(b), InputValue::F64(v)) => b.set_value(py, *v),
            (InputBinding::IntSlider(b), InputValue::I64(v)) => b.set_value(py, *v),
            (InputBinding::Checkbox(b), InputValue::Bool(v)) => b.set_value(py, *v),
            (InputBinding::Radio(b), InputValue::Index(v)) => b.set_to_index(py, *v),
            _ => Err(PyValueError::new_err(
                "InputValue does not match InputBinding variant",
            )),
        }
    }
}

/// A value coming back from the UI, sent to the worker to write into `_value`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InputValue {
    F64(f64),
    I64(i64),
    Bool(bool),
    Index(usize),
}

impl Input {
    /// Splits into the `InputSpec` sent to the UI and the `InputBinding`
    /// that stays on the worker thread.
    pub fn into_parts(self) -> (InputSpec, InputBinding) {
        match self {
            Input::Slider(s) => {
                let (spec, binding) = s.into_parts();
                (InputSpec::Slider(spec), InputBinding::Slider(binding))
            }
            Input::IntSlider(s) => {
                let (spec, binding) = s.into_parts();
                (InputSpec::IntSlider(spec), InputBinding::IntSlider(binding))
            }
            Input::Checkbox(c) => {
                let (spec, binding) = c.into_parts();
                (InputSpec::Checkbox(spec), InputBinding::Checkbox(binding))
            }
            Input::Radio(r) => {
                let (spec, binding) = r.into_parts();
                (InputSpec::Radio(spec), InputBinding::Radio(binding))
            }
        }
    }
}

pub fn split_inputs(inputs: Inputs) -> (Vec<InputSpec>, Vec<InputBinding>) {
    inputs.into_iter().map(Input::into_parts).unzip()
}
