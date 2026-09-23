use gpui_kit::component::Root;
use gpui_kit::{
    App, AppContext, Bounds, Context, IntoElement, ParentElement, Point, Render, TitlebarOptions,
    Window, WindowBounds, WindowOptions, div, px, size,
};
use log::info;
use pyo3::prelude::*;

use crate::inputs::{parse_inputs, InputValue};
use crate::logging_setup::setup_logging;
use crate::utils::Callback;
use crate::worker::spawn_root;

struct RootPlaceholder {
    summary: String,
}

impl Render for RootPlaceholder {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().child(self.summary.clone())
    }
}

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

    let summary = match &initial_result {
        crate::worker::UiLevelResult::Outputs(outputs) => format!("{} output(s)", outputs.len()),
        crate::worker::UiLevelResult::Nested { specs, .. } => {
            format!("nested, {} input(s)", specs.len())
        }
        crate::worker::UiLevelResult::Error(msg) => format!("error: {msg}"),
        crate::worker::UiLevelResult::Discarded => "discarded".to_string(),
    };
    info!("Initial root result: {summary}");

    py.allow_threads(move || {
        gpui_kit::application().run(move |cx: &mut App| {
            gpui_kit::init(cx);
            open_window(window_options(), cx, move |_window, cx| {
                cx.new(|_cx| RootPlaceholder { summary })
            })
            .expect("failed to open window");
        });
    });

    // `worker`'s Drop impl joins the thread once it goes out of scope here,
    // after the gpui event loop has returned (Linux/Windows). On macOS the
    // process terminates before this line runs (known limitation, see spec).
    drop(worker);
    let _ = root_level;
    Ok(())
}
