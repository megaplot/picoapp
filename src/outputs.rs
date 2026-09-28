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

pub enum Output {
    Plot(Plot),
    MatrixPlot(MatrixPlot),
    Audio(Audio),
    Image(Image),
}

pub enum CallbackReturn {
    Outputs(Vec<Output>),
    Inputs(Inputs, Callback),
}

/// Formats a `PyErr`'s message and traceback for display in the UI.
///
/// Deliberately does not print anything: the UI shows the error, and
/// picoapp's stdout/stderr belong to the user's own callback (nothing by
/// default).
///
/// `PyErr::display` (pyo3 0.22) only prints to stderr and returns `()`, and
/// `PyErr`'s `Display` impl gives just "ExceptionType: message" with no
/// traceback, so the traceback is formatted separately via
/// `PyTraceback::format` and appended when present.
pub fn format_traceback(py: Python<'_>, err: &PyErr) -> String {
    let message = err.to_string();
    match err.traceback_bound(py).and_then(|tb| tb.format().ok()) {
        Some(frames) => format!("{message}\n\n{frames}"),
        None => message,
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
        // nice to be able to use the equivalent of `instance(cb_return, ReactiveBase)`. The
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
    for object in output.iter()? {
        let object = object?;
        let output = parse_output(&object)?;
        results.push(output);
    }
    Ok(results)
}

/// Swizzles a flat RGBA byte buffer to BGRA, the pixel format gpui's
/// `RenderImage` expects. Returns a new `Vec`.
fn rgba_to_bgra(rgba: &[u8]) -> Vec<u8> {
    let mut out = rgba.to_vec();
    for px in out.chunks_exact_mut(4) {
        px.swap(0, 2);
    }
    out
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
        // Validated here, on the worker, so a mismatched Image surfaces as
        // an `Error` in the UI instead of panicking on the UI thread when
        // `build_render_image` (`ui/image.rs`) later builds the `RenderImage`.
        let expected_len = width as usize * height as usize * 4;
        if data.len() != expected_len {
            return Err(PyValueError::new_err(format!(
                "Image data has {} bytes, but a {width}x{height} RGBA image needs {expected_len}",
                data.len()
            )));
        }
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
    fn rgba_to_bgra_swaps_red_and_blue() {
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
