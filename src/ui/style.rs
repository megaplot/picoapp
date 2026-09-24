//! picoapp's look: a dark palette (deliberate — it suits picoapp's "study
//! an algorithm's behavior on plots" use case) with the purple accent of the
//! cushy version, layered over gpui-kit's default dark theme.

use std::rc::Rc;

use gpui_kit::component::{Theme, ThemeMode};
use gpui_kit::App;

const BACKGROUND: &str = "#1c1c1f";
const FOREGROUND: &str = "#e8e8ea";
const CARD: &str = "#2a2a2e";
const ACCENT: &str = "#9c82f2";
const TITLE_BAR: &str = "#151517";
const BORDER: &str = "#38383d";
const CONTROL_BORDER: &str = "#77777f";

/// Switches the global theme to picoapp's dark palette.
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
    // Outline of checkboxes/radios: must stand out from the card color.
    colors.input = Some(CONTROL_BORDER.into());
    theme.dark_theme = Rc::new(config);
    Theme::change(ThemeMode::Dark, None, cx);
}
