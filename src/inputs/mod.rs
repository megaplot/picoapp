mod checkbox;
mod radio;
mod slider;

pub use checkbox::{Checkbox, CheckboxSpec, PyCheckbox};
pub use radio::{PyRadio, Radio, RadioSpec};
pub use slider::{parse_slider, PySlider, SliderBinding, SliderSpec};

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

/// Pure-Rust input description sent to the UI thread.
#[derive(Debug, Clone)]
pub enum InputSpec {
    Slider(SliderSpec<f64>),
    IntSlider(SliderSpec<i64>),
    Checkbox(checkbox::CheckboxSpec),
    Radio(radio::RadioSpec),
}

/// Python handle kept on the worker thread, one per input, same order as
/// its `InputSpec` sibling.
pub enum InputBinding {
    Slider(SliderBinding<f64>),
    IntSlider(SliderBinding<i64>),
    Checkbox(checkbox::CheckboxBinding),
    Radio(radio::RadioBinding),
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

pub fn parse_input(obj: &Bound<'_, PyAny>) -> PyResult<(InputSpec, InputBinding)> {
    let type_name = obj.get_type().name()?;
    if type_name == "Slider" {
        let (spec, binding) = parse_slider::<f64>(obj)?;
        Ok((InputSpec::Slider(spec), InputBinding::Slider(binding)))
    } else if type_name == "IntSlider" {
        let (spec, binding) = parse_slider::<i64>(obj)?;
        Ok((InputSpec::IntSlider(spec), InputBinding::IntSlider(binding)))
    } else if type_name == "Checkbox" {
        let (spec, binding) = checkbox::parse_checkbox(obj)?;
        Ok((InputSpec::Checkbox(spec), InputBinding::Checkbox(binding)))
    } else if type_name == "Radio" {
        let (spec, binding) = radio::parse_radio(obj)?;
        Ok((InputSpec::Radio(spec), InputBinding::Radio(binding)))
    } else {
        Err(PyValueError::new_err(format!(
            "Invalid input type: {:?}",
            type_name
        )))
    }
}

pub fn parse_inputs(objs: &[Bound<'_, PyAny>]) -> PyResult<(Vec<InputSpec>, Vec<InputBinding>)> {
    let mut specs = Vec::with_capacity(objs.len());
    let mut bindings = Vec::with_capacity(objs.len());
    for obj in objs {
        let (spec, binding) = parse_input(obj)?;
        specs.push(spec);
        bindings.push(binding);
    }
    Ok((specs, bindings))
}

/// Kept for the existing `_parse_input` Python test hook (`py_module.rs`),
/// which only needs to know parsing doesn't error, not the split result.
pub type Input = InputSpec;

impl<'py> FromPyObject<'py> for Input {
    fn extract_bound(obj: &Bound<'py, PyAny>) -> PyResult<Self> {
        Ok(parse_input(obj)?.0)
    }
}

pub type Inputs = Vec<Input>;
