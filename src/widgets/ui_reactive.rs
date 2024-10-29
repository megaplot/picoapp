use cushy::figures::units::Px;
use cushy::value::{Dynamic, Switchable};
use cushy::widget::MakeWidget;
use cushy::widget::WidgetList;
use cushy::widgets::Space;
use pyo3::prelude::*;

use crate::inputs::Input;
use crate::outputs::CallbackReturn;
use crate::utils::Callback;

use super::ui_checkbox::checkbox_widget;
use super::ui_outputs::outputs_widget;
use super::ui_radio::radio_widget;
use super::ui_slider::{int_slider_widget, slider_widget};

pub fn input_widget(
    py: Python,
    input: &Input,
    py_callback: &Callback,
    cb_return_dynamic: &Dynamic<Option<CallbackReturn>>,
) -> impl MakeWidget {
    match input {
        Input::Slider(slider) => {
            slider_widget(py, slider, py_callback, &cb_return_dynamic).make_widget()
        }
        Input::IntSlider(slider) => {
            int_slider_widget(py, slider, py_callback, &cb_return_dynamic).make_widget()
        }
        Input::Checkbox(checkbox) => {
            checkbox_widget(py, checkbox, py_callback, cb_return_dynamic).make_widget()
        }
        Input::Radio(radio) => {
            radio_widget(py, radio, py_callback, cb_return_dynamic).make_widget()
        }
    }
}

pub fn reactive_input_output_widget(
    py: Python,
    inputs: &[Input],
    py_callback: &Callback,
) -> impl MakeWidget {
    let cb_return_dynamic: Dynamic<Option<CallbackReturn>> = Dynamic::new(None);

    // Build the inputs sidebar
    let mut input_widgets = WidgetList::new();
    for input in inputs.iter() {
        input_widgets.push(input_widget(py, input, &py_callback, &cb_return_dynamic));
    }
    let sidebar = input_widgets.into_rows().contain().width(Px::new(300));

    // Build the outputs content
    let content = cb_return_dynamic.switcher(|cb_result, _active| {
        Python::with_gil(|py| {
            let Some(cb_result) = cb_result else {
                return Space::clear().make_widget();
            };
            match cb_result {
                CallbackReturn::Outputs(outputs) => outputs_widget(outputs).make_widget(),
                CallbackReturn::Inputs(inputs, callback) => {
                    reactive_input_output_widget(py, &inputs, callback).make_widget()
                }
            }
        })
    });

    sidebar.and(content.expand()).into_columns().expand()
}
