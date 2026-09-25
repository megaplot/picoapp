//! Shared look of the plot outputs: a white panel (as in the cushy/plotters
//! version — plots stay white on picoapp's dark UI), a left/bottom margin
//! that holds the tick labels, and tick-label formatting.

use gpui_kit::{fill, px, size, App, Bounds, Pixels, Point, Window};

use crate::ui::style::{plot_colors, CARD_RADIUS};

const MARGIN_LEFT: f32 = 48.0;
const MARGIN_TOP: f32 = 10.0;
const MARGIN_RIGHT: f32 = 16.0;
const MARGIN_BOTTOM: f32 = 28.0;

/// Fills the whole element with the rounded white panel and returns the
/// inner area the data is drawn in (the margins hold the tick labels).
pub fn paint_panel(bounds: Bounds<Pixels>, window: &mut Window, _cx: &mut App) -> Bounds<Pixels> {
    window.paint_quad(
        fill(bounds, plot_colors().panel).corner_radii(CARD_RADIUS),
    );
    Bounds {
        origin: bounds.origin + Point::new(px(MARGIN_LEFT), px(MARGIN_TOP)),
        size: size(
            bounds.size.width - px(MARGIN_LEFT + MARGIN_RIGHT),
            bounds.size.height - px(MARGIN_TOP + MARGIN_BOTTOM),
        ),
    }
}

/// Tick label text: up to 3 decimals, trailing zeros trimmed ("2", "0.5").
pub fn format_tick(value: f64) -> String {
    let s = format!("{value:.3}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s.is_empty() || s == "-0" {
        "0".to_string()
    } else {
        s.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_tick_trims_trailing_zeros() {
        assert_eq!(format_tick(2.0), "2");
        assert_eq!(format_tick(0.5), "0.5");
        assert_eq!(format_tick(-10.0), "-10");
        assert_eq!(format_tick(5000.0), "5000");
        assert_eq!(format_tick(0.02), "0.02");
    }

    #[test]
    fn format_tick_has_no_negative_zero() {
        assert_eq!(format_tick(-0.0), "0");
        assert_eq!(format_tick(-0.0004), "0");
    }
}
