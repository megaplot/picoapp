use std::ops::Range;

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

use crate::inputs::{parse_inputs, InputBinding, InputSpec};
use crate::utils::Callback;

#[derive(Debug, Clone, PartialEq)]
pub struct Plot {
    pub xs: Vec<f64>,
    pub ys: Vec<f64>,
    pub x_limits: Range<f32>,
    pub y_limits: Range<f32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MatrixPlot {
    pub matrix: Vec<Vec<f64>>,
    pub num_rows: u32,
    pub num_cols: u32,
    pub min_value: f64,
    pub max_value: f64,
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

#[derive(Debug, Clone, PartialEq)]
pub struct Image {
    /// Pixel data in **BGRA** order (swizzled from the RGBA the Python side
    /// produces), ready for `gpui`'s `RenderImage`.
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

/// Swizzles a flat RGBA byte buffer to BGRA in place semantics (returns a
/// new Vec), which is the pixel format gpui's `RenderImage` expects.
pub fn rgba_to_bgra(rgba: &[u8]) -> Vec<u8> {
    let mut out = rgba.to_vec();
    for px in out.chunks_exact_mut(4) {
        px.swap(0, 2);
    }
    out
}

pub enum Output {
    Plot(Plot),
    MatrixPlot(MatrixPlot),
    Audio(Audio),
    Image(Image),
}

pub enum LevelResult {
    Outputs(Vec<Output>),
    Nested {
        specs: Vec<InputSpec>,
        bindings: Vec<InputBinding>,
        callback: Callback,
    },
    Error(String),
    Discarded,
}

/// Formats a PyErr's message and traceback for display in the UI.
///
/// Deliberately does not print anything: the UI shows the error, and a
/// picoapp's stdout/stderr belong to the user's own callback (nothing by
/// default).
///
/// `PyErr::display` (pyo3 0.22) only prints to stderr and returns `()`, and
/// `PyErr`'s `Display` impl gives just "ExceptionType: message" with no
/// traceback, so the traceback is formatted separately via `PyTraceback::format`
/// and appended when present.
pub fn format_traceback(py: Python<'_>, err: &PyErr) -> String {
    let message = err.to_string();
    match err.traceback_bound(py).and_then(|tb| tb.format().ok()) {
        Some(frames) => format!("{message}\n\n{frames}"),
        None => message,
    }
}

/// Parses a callback's return value. Never returns a `PyErr` to the caller:
/// any parsing failure (including the callback itself having raised) is
/// folded into `LevelResult::Error` so the worker has one uniform result
/// shape.
pub fn parse_level_result(py: Python<'_>, cb_return: PyResult<PyObject>) -> LevelResult {
    let cb_return = match cb_return {
        Ok(v) => v,
        Err(err) => return LevelResult::Error(format_traceback(py, &err)),
    };

    match parse_level_result_inner(py, cb_return) {
        Ok(result) => result,
        Err(err) => LevelResult::Error(format_traceback(py, &err)),
    }
}

fn parse_level_result_inner(py: Python<'_>, cb_return: PyObject) -> PyResult<LevelResult> {
    let cb_return = cb_return.bind(py);
    if cb_return.get_type().name()? == "Outputs" {
        Ok(LevelResult::Outputs(parse_outputs(
            py,
            cb_return.getattr("outputs")?.into(),
        )?))
    } else if cb_return.is_callable() && cb_return.hasattr("inputs")? {
        // Approximate interface of `ReactiveBase` (duck typing, see mod-level
        // note in the original implementation for why nominal typing isn't
        // used here).
        let raw_inputs = cb_return.getattr("inputs")?.getattr("inputs")?;
        let raw_inputs = raw_inputs.downcast::<pyo3::types::PySequence>()?;
        let mut objs = Vec::new();
        for item in raw_inputs.iter()? {
            objs.push(item?);
        }
        let (specs, bindings) = parse_inputs(&objs)?;
        let callback: Callback = cb_return.getattr("__call__")?.extract()?;
        Ok(LevelResult::Nested {
            specs,
            bindings,
            callback,
        })
    } else {
        Err(PyValueError::new_err(format!(
            "Invalid callback return type: {:?}",
            cb_return.get_type().name()?
        )))
    }
}

pub fn parse_outputs(py: Python<'_>, outputs: PyObject) -> PyResult<Vec<Output>> {
    let output = outputs.bind(py);
    let mut results = Vec::new();
    for object in output.iter()? {
        let object = object?;
        let output = parse_output(&object)?;
        results.push(output);
    }
    Ok(results)
}

fn parse_output(object: &Bound<'_, PyAny>) -> PyResult<Output> {
    // TODO: Decide if this should use a nominal type system, or rather structural
    // duck typing. Currently its a pretty bad mix...
    if object.hasattr("xs")?
        && object.hasattr("ys")?
        && object.hasattr("x_limits")?
        && object.hasattr("y_limits")?
    {
        // TODO: This can be improved a lot. Most likely we could leverage the buffer
        // protocol (or https://github.com/PyO3/rust-numpy) to make this zero copy?
        let xs: Vec<f64> = object.getattr("xs")?.extract()?;
        let ys: Vec<f64> = object.getattr("ys")?.extract()?;
        let x_limits: (f64, f64) = object.getattr("x_limits")?.extract()?;
        let y_limits: (f64, f64) = object.getattr("y_limits")?.extract()?;
        Ok(Output::Plot(Plot {
            xs,
            ys,
            x_limits: x_limits.0 as f32..x_limits.1 as f32,
            y_limits: y_limits.0 as f32..y_limits.1 as f32,
        }))
    } else if object.get_type().name()? == "MatrixPlot" {
        let matrix: Vec<Vec<f64>> = object.getattr("matrix")?.extract()?;
        let num_rows: u32 = object.getattr("num_rows")?.extract()?;
        let num_cols: u32 = object.getattr("num_cols")?.extract()?;
        let min_value: f64 = object.getattr("min_value")?.extract()?;
        let max_value: f64 = object.getattr("max_value")?.extract()?;
        Ok(Output::MatrixPlot(MatrixPlot {
            matrix,
            num_rows,
            num_cols,
            min_value,
            max_value,
        }))
    } else if object.get_type().name()? == "Audio" {
        let data: Vec<f32> = object.getattr("data")?.extract()?;
        let sr: u32 = object.getattr("sr")?.extract()?;
        Ok(Output::Audio(Audio { data, sr }))
    } else if object.get_type().name()? == "Image" {
        let data: Vec<u8> = object.getattr("data")?.extract()?;
        let width: u32 = object.getattr("width")?.extract()?;
        let height: u32 = object.getattr("height")?.extract()?;
        Ok(Output::Image(Image {
            data: rgba_to_bgra(&data),
            width,
            height,
        }))
    } else {
        return Err(PyValueError::new_err(format!(
            "Invalid output type: {:?}",
            object.get_type().name()?
        )));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_output_image_field_is_bgra_after_swizzle() {
        // 1x1 red pixel, alpha 128: RGBA = [255, 0, 0, 128]
        let rgba = vec![255u8, 0, 0, 128];
        let bgra = rgba_to_bgra(&rgba);
        assert_eq!(bgra, vec![0, 0, 255, 128]);
    }

    #[test]
    fn swizzle_handles_multiple_pixels() {
        let rgba = vec![10, 20, 30, 40, 50, 60, 70, 80];
        let bgra = rgba_to_bgra(&rgba);
        assert_eq!(bgra, vec![30, 20, 10, 40, 70, 60, 50, 80]);
    }
}
