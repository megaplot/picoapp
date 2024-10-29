use std::ops::Range;

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

use crate::inputs::Inputs;
use crate::utils::Callback;

#[derive(Debug, Clone, PartialEq)]
pub struct Plot {
    pub xs: Vec<f64>,
    pub ys: Vec<f64>,
    pub x_limits: Range<f32>,
    pub y_limits: Range<f32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Audio {
    pub data: Vec<f32>,
    pub sr: u32,
}

impl Audio {
    pub fn length_in_sec(&self) -> f32 {
        self.data.len() as f32 / self.sr as f32
    }
}

pub enum Output {
    Plot(Plot),
    Audio(Audio),
}

pub enum CallbackReturn {
    Outputs(Vec<Output>),
    Inputs(Inputs, Callback),
}

impl PartialEq for CallbackReturn {
    fn eq(&self, _other: &Self) -> bool {
        // PartialEq is need for setting a `Dynamic`. In this use case, we probably want
        // to assume that each invocation of the callback returns a different result (which
        // is probably true any if an `PyFunction` is involved). Even if the returned value
        // would be the same, there is no real harm in re-updating the UI. In terms of performance
        // the evaluation of the callback itself probably outweighs the UI update anyway.
        false
    }
}

pub fn parse_callback_return(py: Python<'_>, cb_return: PyObject) -> PyResult<CallbackReturn> {
    let cb_return = cb_return.bind(py);
    if cb_return.get_type().name()? == "Outputs" {
        return Ok(CallbackReturn::Outputs(parse_outputs(
            py,
            cb_return.getattr("outputs")?.into(),
        )?));
    } else {
        // Approximate interface of 'Reactive' (duck typing style). In principle it would be
        // nice to be able to use the equivalent of `instance(cb_retrun, ReactiveBase)`. The
        // challenge is how to obtain the reference to the `ReactiveBase` type. Options:
        // * Importing it from Python would add a weird reverse import direction.
        // * Passing the type itself in from Python may look a bit weird as well, but
        //   perhaps this is the way to go, especially since we could leverage that
        //   pattern in other places as well (where we want nominal typing).
        if cb_return.is_callable() && cb_return.hasattr("inputs")? {
            let inputs = cb_return.getattr("inputs")?.getattr("inputs")?.extract()?;
            let callback: Callback = cb_return.getattr("__call__")?.extract()?;
            return Ok(CallbackReturn::Inputs(inputs, callback));
        } else {
            return Err(PyValueError::new_err(format!(
                "Invalid callback return type: {:?}",
                cb_return.get_type().name()?
            )));
        }
    }
}

pub fn parse_outputs(py: Python<'_>, outputs: PyObject) -> PyResult<Vec<Output>> {
    let output = outputs.bind(py);
    let mut results = Vec::new();
    // TODO: This can be improved a lot. Most likely we could leverage the buffer
    // protocol (or https://github.com/PyO3/rust-numpy) to make this zero copy?
    for object in output.iter()? {
        let object = object?;
        if object.hasattr("xs")?
            && object.hasattr("ys")?
            && object.hasattr("x_limits")?
            && object.hasattr("y_limits")?
        {
            let xs: Vec<f64> = object.getattr("xs")?.extract()?;
            let ys: Vec<f64> = object.getattr("ys")?.extract()?;
            let x_limits: (f64, f64) = object.getattr("x_limits")?.extract()?;
            let y_limits: (f64, f64) = object.getattr("y_limits")?.extract()?;
            results.push(Output::Plot(Plot {
                xs,
                ys,
                x_limits: x_limits.0 as f32..x_limits.1 as f32,
                y_limits: y_limits.0 as f32..y_limits.1 as f32,
            }));
        }
        // TODO: Decide if this should use a nominal type system, or rather structural
        // duck typing.
        if object.get_type().name()? == "Audio" {
            let data: Vec<f32> = object.getattr("data")?.extract()?;
            let sr: u32 = object.getattr("sr")?.extract()?;
            results.push(Output::Audio(Audio { data, sr }));
        }
    }
    Ok(results)
}
