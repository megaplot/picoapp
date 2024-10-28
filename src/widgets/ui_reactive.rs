use cushy::figures::units::Px;
use cushy::value::{Dynamic, Switchable};
use cushy::widget::MakeWidget;
use cushy::widget::WidgetList;
use cushy::widgets::Space;
use pyo3::prelude::*;
use pyo3::types::PyFunction;

use crate::inputs::Input;
use crate::outputs::CallbackReturn;

use super::ui_outputs::outputs_widget;
use super::ui_slider::{int_slider_widget, slider_widget};

pub fn reactive_input_output_widget(
    py: Python,
    inputs: &[Input],
    py_callback: Py<PyFunction>,
) -> impl MakeWidget {
    let cb_return_dynamic: Dynamic<Option<CallbackReturn>> = Dynamic::new(None);

    // Build the inputs sidebar
    let mut input_widgets = WidgetList::new();
    for input in inputs.iter() {
        if let Input::Slider(slider) = input {
            let input_widget = slider_widget(py, slider, &py_callback, &cb_return_dynamic);
            input_widgets = input_widgets.and(input_widget);
        } else if let Input::IntSlider(slider) = input {
            let input_widget = int_slider_widget(py, slider, &py_callback, &cb_return_dynamic);
            input_widgets = input_widgets.and(input_widget);
        }
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
                    reactive_input_output_widget(py, &inputs, callback.clone_ref(py)).make_widget()
                }
            }
        })
    });

    sidebar.and(content.expand()).into_columns().expand()
}