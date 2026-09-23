use gpui_kit::component::slider::{Slider, SliderScale, SliderState};
use gpui_kit::{Context, Entity, IntoElement, ParentElement, Styled};

use crate::inputs::SliderSpec;

pub fn make_slider_state(spec: &SliderSpec<f64>) -> SliderState {
    let mut state = SliderState::new()
        .min(spec.min as f32)
        .max(spec.max as f32)
        .default_value(spec.init as f32);
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

pub fn format_slider_label(spec: &SliderSpec<f64>, value: f32) -> String {
    match spec.decimal_places {
        Some(places) => format!("{}: {:.places$}", spec.name, value, places = places),
        None => format!("{}: {}", spec.name, value),
    }
}

pub fn render_slider_row<T: 'static>(
    spec: &SliderSpec<f64>,
    state: &Entity<SliderState>,
    value: f32,
    _cx: &mut Context<T>,
) -> impl IntoElement {
    gpui_kit::component::v_flex()
        .gap_1()
        .child(format_slider_label(spec, value))
        .child(Slider::new(state))
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
}
