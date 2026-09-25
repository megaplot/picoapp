use gpui_kit::component::checkbox::Checkbox;
use gpui_kit::component::radio::RadioGroup;
use gpui_kit::component::slider::{Slider, SliderScale, SliderState};
use gpui_kit::component::{h_flex, v_flex};
use gpui_kit::{
    div, App, Context, ElementId, Entity, IntoElement, ParentElement, Styled, Window,
};

use crate::inputs::{CheckboxSpec, RadioSpec, SliderSpec};
use crate::ui::style::{muted_text, CONTROL_GAP};

pub fn make_slider_state(spec: &SliderSpec<f64>) -> SliderState {
    // `SliderState`'s own default step is 1.0 (a whole number), which would
    // silently round every value to an integer. Use `decimal_places` when
    // given (matching the precision the label displays), otherwise a step
    // fine enough that the slider feels continuous over its range.
    let step = match spec.decimal_places {
        Some(places) => 10f64.powi(-(places as i32)),
        None => ((spec.max - spec.min) / 1000.0).max(f64::EPSILON),
    };
    let mut state = SliderState::new()
        .min(spec.min as f32)
        .max(spec.max as f32)
        .default_value(spec.init as f32)
        .step(step as f32);
    if spec.log {
        state = state.scale(SliderScale::Logarithmic);
    }
    state
}

pub fn make_int_slider_state(spec: &SliderSpec<i64>) -> SliderState {
    SliderState::new()
        .min(spec.min as f32)
        .max(spec.max as f32)
        .default_value(spec.init as f32)
        .step(1.0)
}

/// The value text next to a float slider's name, with the spec's precision.
pub fn format_slider_value(spec: &SliderSpec<f64>, value: f32) -> String {
    match spec.decimal_places {
        Some(places) => format!("{:.places$}", value, places = places),
        None => format!("{}", value),
    }
}

/// "name  value" above the slider, name in the normal text color and the
/// value dimmed (as in the cushy version).
fn slider_header(name: &str, value: String, cx: &App) -> impl IntoElement {
    h_flex()
        .gap(CONTROL_GAP)
        .child(div().child(name.to_string()))
        .child(div().text_color(muted_text(cx)).child(value))
}

pub fn render_slider_row<T: 'static>(
    spec: &SliderSpec<f64>,
    state: &Entity<SliderState>,
    value: f32,
    cx: &mut Context<T>,
) -> impl IntoElement {
    v_flex()
        .gap(CONTROL_GAP)
        .child(slider_header(&spec.name, format_slider_value(spec, value), cx))
        .child(Slider::new(state))
}

pub fn render_int_slider_row<T: 'static>(
    spec: &SliderSpec<i64>,
    state: &Entity<SliderState>,
    value: f32,
    cx: &mut Context<T>,
) -> impl IntoElement {
    v_flex()
        .gap(CONTROL_GAP)
        .child(slider_header(&spec.name, format!("{}", value as i64), cx))
        .child(Slider::new(state))
}

pub fn render_checkbox(
    id: impl Into<ElementId>,
    spec: &CheckboxSpec,
    checked: bool,
    on_change: impl Fn(&bool, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    Checkbox::new(id)
        .label(spec.name.clone())
        .checked(checked)
        .on_change(on_change)
}

pub fn render_radio(
    id: impl Into<ElementId>,
    spec: &RadioSpec,
    selected: Option<usize>,
    on_change: impl Fn(&usize, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    // `spec.value_names` (`Vec<String>`) converts into `Radio` via gpui-kit's
    // `impl From<String> for Radio`, which sets both the id and the visible
    // label to that string. Building a bare `Radio::new(name)` instead would
    // only set the id, leaving the label blank.
    RadioGroup::vertical(id)
        .children(spec.value_names.iter().cloned())
        .selected_index(selected)
        .on_change(on_change)
}

#[cfg(test)]
mod tests {
    use gpui_kit::{gpui, AppContext, TestAppContext};

    use super::*;

    #[gpui::test]
    fn slider_state_reflects_spec_bounds(cx: &mut TestAppContext) {
        let spec = crate::inputs::SliderSpec {
            name: "a".into(),
            min: -10.0,
            init: 2.5,
            max: 10.0,
            log: false,
            decimal_places: Some(2),
        };
        let state = cx.new(|_| make_slider_state(&spec));
        cx.update(|cx| {
            let s = state.read(cx);
            assert_eq!(s.min_value(), -10.0);
            assert_eq!(s.max_value(), 10.0);
        });
    }

    /// Regression test: `SliderState`'s default step is 1.0 (a whole
    /// number), which silently rounds every float slider's value to an
    /// integer. `make_slider_state` must set a step fine enough that
    /// non-integer values (e.g. the 0.5 init used by example_1.py) are
    /// reachable.
    #[gpui::test]
    fn slider_step_is_not_the_default_whole_number_step(cx: &mut TestAppContext) {
        let spec = crate::inputs::SliderSpec {
            name: "a".into(),
            min: -10.0,
            init: 0.5,
            max: 10.0,
            log: false,
            decimal_places: None,
        };
        let state = cx.new(|_| make_slider_state(&spec));
        cx.update(|cx| {
            assert!(state.read(cx).step_value() < 1.0);
        });
    }

    #[gpui::test]
    fn slider_step_respects_decimal_places(cx: &mut TestAppContext) {
        let spec = crate::inputs::SliderSpec {
            name: "a".into(),
            min: 0.0,
            init: 0.0,
            max: 1.0,
            log: false,
            decimal_places: Some(2),
        };
        let state = cx.new(|_| make_slider_state(&spec));
        cx.update(|cx| {
            assert!((state.read(cx).step_value() - 0.01).abs() < 1e-6);
        });
    }
}
