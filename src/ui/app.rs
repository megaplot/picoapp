use std::sync::Arc;

use gpui_kit::component::Root;
use gpui_kit::{
    App, AppContext, Bounds, Point, Render, TitlebarOptions, Window, WindowBounds, WindowOptions,
    px, size,
};
use log::info;
use pyo3::prelude::*;

use crate::inputs::{parse_inputs, InputValue};
use crate::logging_setup::setup_logging;
use crate::ui::reactive_view::ReactiveView;
use crate::utils::Callback;
use crate::worker::spawn_root;

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

fn window_options() -> WindowOptions {
    WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: Point::default(),
            size: size(px(1600.), px(1000.)),
        })),
        titlebar: Some(TitlebarOptions {
            title: Some("pico app".into()),
            ..Default::default()
        }),
        ..Default::default()
    }
}

pub fn run_ui(py: Python<'_>, input_objs: &[Bound<'_, PyAny>], callback: Callback) -> PyResult<()> {
    setup_logging();

    let (specs, bindings) = parse_inputs(input_objs)?;
    let initial_values: Vec<InputValue> = specs
        .iter()
        .map(|spec| match spec {
            crate::inputs::InputSpec::Slider(s) => InputValue::F64(s.init),
            crate::inputs::InputSpec::IntSlider(s) => InputValue::I64(s.init),
            crate::inputs::InputSpec::Checkbox(s) => InputValue::Bool(s.init),
            crate::inputs::InputSpec::Radio(s) => InputValue::Index(s.init_index),
        })
        .collect();

    let (worker, root_level, initial_result) = spawn_root(py, bindings, callback, initial_values);
    info!("Initial root result computed, opening window");

    let worker = Arc::new(worker);
    py.allow_threads(move || {
        gpui_kit::application().run(move |cx: &mut App| {
            gpui_kit::init(cx);
            open_window(window_options(), cx, move |window, cx| {
                cx.new(|cx| {
                    ReactiveView::new(root_level, specs, initial_result, worker, window, cx)
                })
            })
            .expect("failed to open window");
        });
    });

    Ok(())
}
