use cushy::value::{Destination, Dynamic, Source};
use cushy::widget::{MakeWidget, Widget};
use cushy::widgets::slider::Slidable;
use pyo3::prelude::*;

use crate::inputs::Slider;
use crate::outputs::{parse_callback_return, CallbackReturn};
use crate::utils::Callback;

#[derive(Copy, Clone)]
struct LinLogTransformer {
    log: bool,
}

impl LinLogTransformer {
    fn fwd(&self, x: f64) -> f64 {
        if self.log {
            2.0_f64.powf(x)
        } else {
            x
        }
    }
    fn bwd(&self, x: f64) -> f64 {
        if self.log {
            x.log2()
        } else {
            x
        }
    }
}

pub fn slider_widget(
    py: Python,
    slider: &Slider<f64>,
    py_callback: &Callback,
    cb_return_dynamic: &Dynamic<Option<CallbackReturn>>,
) -> impl Widget {
    let py_slider = slider.py_slider.clone_ref(py);
    let py_callback = py_callback.clone_ref(py);
    let cb_return_dynamic = cb_return_dynamic.clone();

    let transformer = LinLogTransformer { log: slider.log };

    let value = Dynamic::new(transformer.bwd(slider.init));
    value
        .for_each(move |value: &f64| {
            let result = Python::with_gil(|py| -> PyResult<()> {
                py_slider.set_value(py, transformer.fwd(*value))?;

                let cb_return = py_callback.call(py)?;
                let cb_return = parse_callback_return(py, cb_return)?;

                cb_return_dynamic.set(Some(cb_return));
                Ok(())
            });
            if let Err(e) = result {
                println!("Error on calling callback: {}", e);
            }
        })
        .persist();

    let decimal_places = slider.decimal_places;
    let label_row = slider
        .name
        .clone()
        .small()
        .and(
            value
                .map_each(move |x| {
                    if let Some(decimal_places) = decimal_places {
                        format!("{:.prec$}", transformer.fwd(*x), prec = decimal_places)
                    } else {
                        format!("{}", transformer.fwd(*x))
                    }
                })
                .small(),
        )
        .into_columns();

    let slider = value
        .clone()
        .slider_between(transformer.bwd(slider.min), transformer.bwd(slider.max));
    label_row.and(slider).into_rows().contain()
}

pub fn int_slider_widget(
    py: Python,
    slider: &Slider<i64>,
    py_callback: &Callback,
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

                let cb_return = py_callback.call(py)?;
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
        .and(value.map_each(move |x| format!("{}", x)).small())
        .into_columns();

    let slider = value.clone().slider_between(slider.min, slider.max);
    label_row.and(slider).into_rows()
}
