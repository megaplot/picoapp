//! The window header bar, in the style of a GNOME/Adwaita header bar
//! (centered title, round minimize/maximize/close buttons on the right).
//!
//! GNOME's compositor draws no window decorations for Wayland windows, so
//! the application has to; gpui-kit's own `TitleBar` draws flat
//! Windows-style controls, which look foreign next to GNOME apps (and the
//! cushy version, whose winit backend drew an Adwaita-like bar). Where the
//! compositor *does* draw decorations (server-side, e.g. KDE or X11), this
//! renders nothing so there is never a second title bar.
//!
//! Dragging, double-click-to-maximize and the window menu follow gpui-kit's
//! `TitleBar` (same gpui window calls).

use gpui_kit::component::{ActiveTheme, Icon, IconName, InteractiveElementExt as _, Sizable};
use gpui_kit::{
    div, prelude::FluentBuilder, px, App, ClickEvent, Context, Decorations, InteractiveElement,
    IntoElement, MouseButton, ParentElement, Render, RenderOnce, SharedString,
    StatefulInteractiveElement, Styled, Window,
};

use crate::ui::style::{CONTROL_GAP, HEADER_HEIGHT};

const BUTTON_SIZE: f32 = 28.0;

#[derive(IntoElement)]
pub struct HeaderBar {
    title: SharedString,
}

impl HeaderBar {
    pub fn new(title: impl Into<SharedString>) -> Self {
        HeaderBar {
            title: title.into(),
        }
    }
}

struct DragState {
    should_move: bool,
}

impl Render for DragState {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

#[derive(Clone, Copy)]
enum Control {
    Minimize,
    Maximize,
    Close,
}

fn control_button(id: &'static str, icon: IconName, control: Control, cx: &App) -> impl IntoElement {
    let theme = cx.theme();
    let (hover_bg, hover_fg) = match control {
        Control::Close => (theme.danger, theme.danger_foreground),
        _ => (theme.secondary_hover, theme.foreground),
    };
    div()
        .id(id)
        .flex()
        .items_center()
        .justify_center()
        .size(px(BUTTON_SIZE))
        .rounded_full()
        .bg(theme.foreground.opacity(0.08))
        .text_color(theme.foreground)
        .hover(move |style| style.bg(hover_bg).text_color(hover_fg))
        .on_mouse_down(MouseButton::Left, |_, window, cx| {
            // Don't start a window drag from a button press.
            window.prevent_default();
            cx.stop_propagation();
        })
        .on_click(move |_: &ClickEvent, window, cx| {
            cx.stop_propagation();
            match control {
                Control::Minimize => window.minimize_window(),
                Control::Maximize => window.zoom_window(),
                Control::Close => window.remove_window(),
            }
        })
        .child(Icon::new(icon).small())
}

impl RenderOnce for HeaderBar {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        // Server-side decorated: the compositor already draws a title bar.
        if !matches!(window.window_decorations(), Decorations::Client { .. }) {
            return div().id("header-bar");
        }

        let state = window.use_state(cx, |_, _| DragState { should_move: false });
        let supported = window.window_controls();
        let maximized = window.is_maximized();
        let controls_width = px(3.0 * BUTTON_SIZE) + CONTROL_GAP * 2.0;

        let controls = div()
            .flex()
            .items_center()
            .gap(CONTROL_GAP)
            .w(controls_width)
            .justify_end()
            .when(supported.minimize, |el| {
                el.child(control_button("minimize", IconName::WindowMinimize, Control::Minimize, cx))
            })
            .when(supported.maximize, |el| {
                let icon = if maximized {
                    IconName::WindowRestore
                } else {
                    IconName::WindowMaximize
                };
                el.child(control_button("maximize", icon, Control::Maximize, cx))
            })
            .child(control_button("close", IconName::WindowClose, Control::Close, cx));

        div()
            .id("header-bar")
            .flex()
            .flex_row()
            .flex_shrink_0()
            .items_center()
            .h(HEADER_HEIGHT)
            .px(CONTROL_GAP)
            .bg(cx.theme().title_bar)
            .border_b_1()
            .border_color(cx.theme().border)
            .on_double_click(|_, window, _| window.zoom_window())
            .on_mouse_down_out(window.listener_for(&state, |state, _, _, _| {
                state.should_move = false;
            }))
            .on_mouse_down(
                MouseButton::Left,
                window.listener_for(&state, |state, _, _, _| {
                    state.should_move = true;
                }),
            )
            .on_mouse_down(MouseButton::Right, |event, window, _| {
                window.show_window_menu(event.position)
            })
            .on_mouse_up(
                MouseButton::Left,
                window.listener_for(&state, |state, _, _, _| {
                    state.should_move = false;
                }),
            )
            .on_mouse_move(window.listener_for(&state, |state, _, window, _| {
                if state.should_move {
                    state.should_move = false;
                    window.start_window_move();
                }
            }))
            // An empty block as wide as the controls keeps the title
            // centered, as in Adwaita.
            .child(div().w(controls_width))
            .child(
                div()
                    .flex_1()
                    .flex()
                    .justify_center()
                    .font_weight(gpui_kit::FontWeight::BOLD)
                    .child(self.title),
            )
            .child(controls)
    }
}
