//! picoapp's design system: every visual decision that more than one
//! component shares lives here, so styling is changed in one place.
//!
//! Three layers, mirroring how gpui-kit itself is built:
//!
//! 1. **Palette** (`apply_dark_palette`): overrides gpui-kit's dark theme, so
//!    every gpui-kit component (slider, checkbox, radio, buttons, title bar
//!    text, ...) picks up picoapp's colors from the theme tokens. Nothing
//!    else should hard-code a color that a theme token can express.
//! 2. **Tokens** (`GUTTER`, `CARD_RADIUS`, ...): the spacing/size scale.
//! 3. **Semantic helpers** (`card`, `muted_text`, `plot_colors`, ...): named
//!    roles ("an input card", "secondary text") that components use instead
//!    of composing raw colors and paddings themselves.
//!
//! The one deliberate exception to "everything from the theme" is the plot
//! panel: as in the cushy/plotters version, plots are white on the dark UI,
//! so their colors are fixed here (`plot_colors`) rather than themed.

use std::rc::Rc;

use gpui_kit::component::{ActiveTheme, Theme, ThemeMode};
use gpui_kit::{div, hsla, px, App, Div, Hsla, ParentElement, Pixels, Rgba, Styled};

// ---- palette --------------------------------------------------------------

const BACKGROUND: &str = "#1c1c1f";
const FOREGROUND: &str = "#e8e8ea";
const CARD: &str = "#2a2a2e";
const ACCENT: &str = "#9c82f2";
const TITLE_BAR: &str = "#242427";
const BORDER: &str = "#38383d";
/// Outline of checkboxes/radios: must stand out from the card color.
const CONTROL_BORDER: &str = "#77777f";

/// Switches the global theme to picoapp's dark palette (deliberately dark: it
/// suits picoapp's "study an algorithm's behavior on plots" use case).
pub fn apply_dark_palette(cx: &mut App) {
    let theme = Theme::global_mut(cx);
    let mut config = (*theme.dark_theme).clone();
    let colors = &mut config.colors;
    colors.background = Some(BACKGROUND.into());
    colors.foreground = Some(FOREGROUND.into());
    colors.group_box = Some(CARD.into());
    colors.group_box_foreground = Some(FOREGROUND.into());
    colors.primary = Some(ACCENT.into());
    colors.slider_bar = Some(ACCENT.into());
    colors.slider_thumb = Some(ACCENT.into());
    colors.progress_bar = Some(ACCENT.into());
    colors.title_bar = Some(TITLE_BAR.into());
    colors.border = Some(BORDER.into());
    colors.input = Some(CONTROL_BORDER.into());
    theme.dark_theme = Rc::new(config);
    Theme::change(ThemeMode::Dark, None, cx);
}

// ---- tokens ---------------------------------------------------------------

/// Space between, and around, the top-level areas (sidebar, outputs).
pub const GUTTER: Pixels = px(8.);
/// Width of an input sidebar.
pub const SIDEBAR_WIDTH: Pixels = px(300.);
/// Inner padding of a card.
pub const CARD_PADDING: Pixels = px(12.);
/// Corner radius of cards and plot panels.
pub const CARD_RADIUS: Pixels = px(6.);
/// Space between the parts of a control (label above slider, ...).
pub const CONTROL_GAP: Pixels = px(8.);
/// Height of the window header bar.
pub const HEADER_HEIGHT: Pixels = px(46.);

// ---- semantic helpers -----------------------------------------------------

/// A rounded container that groups one control or output on the dark
/// background (an input, the audio player, an image).
pub fn card(cx: &App) -> Div {
    div()
        .p(CARD_PADDING)
        .rounded(CARD_RADIUS)
        .bg(cx.theme().group_box)
        .text_color(cx.theme().group_box_foreground)
}

/// A card-shaped error message.
pub fn error_card(message: String, cx: &App) -> Div {
    div()
        .p(CARD_PADDING)
        .rounded(CARD_RADIUS)
        .bg(cx.theme().danger.opacity(0.15))
        .text_color(cx.theme().danger)
        .child(message)
}

/// Secondary text: units, current values, group titles.
pub fn muted_text(cx: &App) -> Hsla {
    cx.theme().muted_foreground
}

/// Colors of the plot panel (fixed, see the module docs).
pub struct PlotColors {
    pub panel: Hsla,
    pub axis: Hsla,
    pub text: Hsla,
    pub grid: Hsla,
    pub line: Hsla,
}

pub fn plot_colors() -> PlotColors {
    PlotColors {
        panel: hsla(0.0, 0.0, 1.0, 1.0),
        axis: hsla(0.0, 0.0, 0.2, 1.0),
        text: hsla(0.0, 0.0, 0.25, 1.0),
        grid: hsla(0.0, 0.0, 0.86, 1.0),
        line: Rgba {
            r: 0.87,
            g: 0.1,
            b: 0.1,
            a: 1.0,
        }
        .into(),
    }
}
