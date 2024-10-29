use cushy::value::{Destination, Dynamic, Source};
use cushy::widget::{MakeWidget, Widget};
use cushy::widgets::checkbox::Checkable;
use pyo3::prelude::*;
use pyo3::types::PyFunction;

use crate::inputs::Checkbox;
use crate::outputs::{parse_callback_return, CallbackReturn};

pub fn checkbox_widget(
    py: Python,
    checkbox: &Checkbox,
    py_callback: &Py<PyFunction>,
    cb_return_dynamic: &Dynamic<Option<CallbackReturn>>,
) -> impl Widget {
    let py_slider = checkbox.py_checkbox.clone_ref(py);
    let py_callback = py_callback.clone_ref(py);
    let cb_return_dynamic = cb_return_dynamic.clone();

    let checkbox_state = Dynamic::new(checkbox.init);

    checkbox_state
        .for_each(move |value: &bool| {
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

    checkbox_state
        .clone()
        .to_checkbox(&checkbox.name)
        .small()
        .contain()
}
