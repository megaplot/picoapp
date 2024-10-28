use cushy::value::{Destination, Dynamic, Source};
use cushy::widget::{MakeWidget, Widget};
use cushy::widgets::slider::Slidable;
use pyo3::prelude::*;
use pyo3::types::PyFunction;

use crate::inputs::Slider;
use crate::outputs::{parse_callback_return, CallbackReturn};

pub fn slider_widget(
    py: Python,
    slider: &Slider<f64>,
    py_callback: &Py<PyFunction>,
    cb_return_dynamic: &Dynamic<Option<CallbackReturn>>,
) -> impl Widget {
    let py_slider = slider.py_slider.clone_ref(py);
    let py_callback = py_callback.clone_ref(py);
    let cb_return_dynamic = cb_return_dynamic.clone();

    let value = Dynamic::new(slider.init);
    value
        .for_each(move |value: &f64| {
            let result = Python::with_gil(|py| -> PyResult<()> {
                py_slider.set_value(py, *value)?;

                let cb_return = py_callback.call_bound(py, (), None)?;
                let cb_return = parse_callback_return(py, cb_return)?;

                cb_return_dynamic.set(Some(cb_return));
                Ok(())
            });
            if let Err(e) = result {
                println!("Error on calling callback: {}", e);
            }
        })
        .persist();

    let label_row = slider
        .name
        .clone()
        .small()
        .and(value.map_each(|x| format!("{}", x)).small())
        .into_columns();

    let slider = value.clone().slider_between(slider.min, slider.max);
    label_row.and(slider).into_rows().contain()
}

pub fn int_slider_widget(
    py: Python,
    slider: &Slider<i64>,
    py_callback: &Py<PyFunction>,
    cb_return_dynamic: &Dynamic<Option<CallbackReturn>>,
) -> impl Widget {
    let py_slider = slider.py_slider.clone_ref(py);
    let py_callback = py_callback.clone_ref(py);
    let cb_return_dynamic = cb_return_dynamic.clone();

    let value = Dynamic::new(slider.init);
    value
        .for_each(move |value: &i64| {
            let result = Python::with_gil(|py| -> PyResult<()> {
                py_slider.set_value(py, *value)?;

                let cb_return = py_callback.call_bound(py, (), None)?;
                let cb_return = parse_callback_return(py, cb_return)?;

                cb_return_dynamic.set(Some(cb_return));
                Ok(())
            });
            if let Err(e) = result {
                println!("Error on calling callback: {}", e);
            }
        })
        .persist();

    let label_row = slider
        .name
        .clone()
        .small()
        .and(value.map_each(|x| format!("{}", x)).small())
        .into_columns();

    let slider = value.clone().slider_between(slider.min, slider.max);
    label_row.and(slider).into_rows()
}
