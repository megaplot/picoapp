use gpui_kit::component::{ActiveTheme, v_flex};
use gpui_kit::{
    App, AppContext, Bounds, Context, Entity, IntoElement, ParentElement, Point, Render, Styled,
    TitlebarOptions, Window, WindowBounds, WindowOptions, div, open_window, px, size,
};
use log::info;
use pyo3::prelude::*;

use crate::logging_setup::setup_logging;
use crate::ui::app_view::AppView;
use crate::ui::header_bar::HeaderBar;
use crate::ui::style::GUTTER;
use crate::view_tree::NodeId;
use crate::worker::spawn;

const APP_TITLE: &str = "pico app";

fn window_options() -> WindowOptions {
    WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: Point::default(),
            size: size(px(1600.), px(1000.)),
        })),
        window_min_size: Some(size(px(640.), px(400.))),
        // The platform's own title bar: native on macOS and Windows, and on
        // Linux wherever the compositor draws window decorations (X11 window
        // managers, KDE, wlroots compositors, ...).
        titlebar: Some(TitlebarOptions {
            title: Some(APP_TITLE.into()),
            ..Default::default()
        }),
        // Ask the compositor for its decorations. gpui falls back to
        // client-side decorations when the compositor has none to offer
        // (GNOME on Wayland refuses by design); only then does `HeaderBar`
        // draw one of its own.
        #[cfg(target_os = "linux")]
        window_decorations: Some(gpui_kit::WindowDecorations::Server),
        ..Default::default()
    }
}

/// The window's content: a header bar (only if the compositor draws no
/// decorations, see `HeaderBar`) above the `AppView`, on the
/// dark theme's background with an 8px gutter around the content.
struct AppShell {
    content: Entity<AppView>,
}

impl Render for AppShell {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .child(HeaderBar::new(APP_TITLE))
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .p(GUTTER)
                    .child(self.content.clone()),
            )
    }
}

pub fn run_ui(py: Python<'_>, engine: Py<PyAny>) -> PyResult<()> {
    setup_logging();

    let root = NodeId(engine.getattr(py, "root_id")?.extract(py)?);
    let (to_ui, from_worker) = futures::channel::mpsc::unbounded();
    let worker = spawn(engine, to_ui);
    info!("Worker spawned, opening window (the first round runs asynchronously)");

    py.detach(move || {
        gpui_kit::application()
            // Icons (checkmarks, window controls) are embedded SVG assets.
            .with_assets(gpui_kit::assets::Assets)
            .run(move |cx: &mut App| {
                gpui_kit::init(cx);
                // picoapp is deliberately dark: it suits its use case (studying
                // algorithms on plots) and matches the cushy version's look.
                crate::ui::style::apply_dark_palette(cx);
                open_window(window_options(), cx, move |window, cx| {
                    window.set_window_title(APP_TITLE);
                    let content = cx.new(|cx| AppView::new(root, worker, from_worker, cx));
                    cx.new(|_| AppShell { content })
                })
                .expect("failed to open window");
            });
    });

    Ok(())
}
