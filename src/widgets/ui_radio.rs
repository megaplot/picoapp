use cushy::value::{Destination, Dynamic, Source};
use cushy::widget::{MakeWidget, Widget, WidgetList};
use pyo3::prelude::*;

use crate::inputs::Radio;
use crate::outputs::{parse_callback_return, CallbackReturn};
use crate::utils::Callback;

pub fn radio_widget(
    py: Python,
    radio: &Radio,
    py_callback: &Callback,
    cb_return_dynamic: &Dynamic<Option<CallbackReturn>>,
) -> impl Widget {
    let py_slider = radio.py_radio.clone_ref(py);
    let py_callback = py_callback.clone_ref(py);
    let cb_return_dynamic = cb_return_dynamic.clone();

    let option = Dynamic::new(radio.init_index);

    option
        .for_each(move |index: &usize| {
            let result = Python::with_gil(|py| -> PyResult<()> {
                py_slider.set_to_index(py, *index)?;

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

    let mut options_widget_list = WidgetList::new();
    options_widget_list.push(radio.name.clone().small());

    for (idx, name) in radio.value_names.iter().enumerate() {
        options_widget_list.push(option.new_radio(idx, name).small());
    }

    options_widget_list.into_rows().contain()
}
