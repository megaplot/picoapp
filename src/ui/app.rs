use std::sync::Arc;

use gpui_kit::component::{v_flex, ActiveTheme, Root};
use gpui_kit::{
    App, AppContext, Bounds, Context, Entity, IntoElement, ParentElement, Point, Render, Styled,
    TitlebarOptions, Window, WindowBounds, WindowOptions, div, px, size,
};
use log::info;
use pyo3::prelude::*;

use crate::inputs::parse_inputs;
use crate::logging_setup::setup_logging;
use crate::ui::header_bar::HeaderBar;
use crate::ui::reactive_view::ReactiveView;
use crate::ui::style::GUTTER;
use crate::utils::Callback;
use crate::worker::spawn;

/// gpui-kit 0.6.6 (the version pinned by this crate) does not yet publish
/// the `gpui_kit::open_window` convenience wrapper that a newer,
/// unreleased gpui-kit does (see ledger ruling for Task 1). This
/// reimplements it: open a plain gpui window via `App::open_window` and
/// wrap the built view in `gpui_kit::component::Root`, which is what the
/// newer wrapper does internally — same behavior (dialogs/sheets/
/// notifications/menus work), just not hidden behind a helper function.
fn open_window<V: 'static + Render>(
    options: WindowOptions,
    cx: &mut App,
    build: impl FnOnce(&mut Window, &mut App) -> gpui_kit::Entity<V>,
) -> anyhow::Result<gpui_kit::WindowHandle<Root>> {
    cx.open_window(options, |window, cx| {
        let view = build(window, cx);
        cx.new(|cx| Root::new(view, window, cx))
    })
}

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
/// decorations, see `HeaderBar`) above the root `ReactiveView`, on the
/// dark theme's background with an 8px gutter around the content.
struct AppShell {
    content: Entity<ReactiveView>,
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

pub fn run_ui(py: Python<'_>, input_objs: &[Bound<'_, PyAny>], callback: Callback) -> PyResult<()> {
    setup_logging();

    let (specs, bindings) = parse_inputs(input_objs)?;
    let (worker, root_level) = spawn(bindings, callback);
    info!("Worker spawned, opening window (root's first job dispatches asynchronously)");

    let worker = Arc::new(worker);
    py.allow_threads(move || {
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
                let content =
                    cx.new(|cx| ReactiveView::new(root_level, specs, worker, window, cx));
                cx.new(|_| AppShell { content })
            })
            .expect("failed to open window");
        });
    });

    Ok(())
}
