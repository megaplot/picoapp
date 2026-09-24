use std::sync::Arc;

use gpui_kit::component::ActiveTheme;
use gpui_kit::gpui::prelude::FluentBuilder;
use gpui_kit::{
    div, px, App, AppContext, Context, Entity, InteractiveElement, StatefulInteractiveElement, IntoElement, ParentElement, Render, Styled, Window,
};

use crate::inputs::{InputSpec, InputValue};
use crate::outputs::Output;
use crate::ui::audio::AudioPlayer;
use crate::ui::image::{build_render_image, drop_images, image_element};
use crate::ui::inputs::{
    make_int_slider_state, make_slider_state, render_checkbox, render_int_slider_row,
    render_radio, render_slider_row,
};
use crate::ui::line_plot::LinePlot;
use crate::ui::matrix_plot::MatrixPlotView;
use crate::ui::run_scheduler::RunScheduler;
use crate::worker::{Job, LevelId, UiLevelResult, WorkerHandle};

enum InputWidgetState {
    Slider {
        spec: crate::inputs::SliderSpec<f64>,
        state: Entity<gpui_kit::component::slider::SliderState>,
    },
    IntSlider {
        spec: crate::inputs::SliderSpec<i64>,
        state: Entity<gpui_kit::component::slider::SliderState>,
    },
    Checkbox {
        spec: crate::inputs::CheckboxSpec,
        checked: bool,
    },
    Radio {
        spec: crate::inputs::RadioSpec,
        selected: usize,
    },
}

/// An output, prepared once when it arrives rather than rebuilt on every
/// render pass. `Image`'s `Arc<RenderImage>` and `Audio`'s `Entity<AudioPlayer>`
/// both hold live GPU/OS resources (a sprite-atlas upload, a playback
/// `Sink`) that must not be recreated every frame: gpui re-renders a view
/// on far more than just its own state changes (any window refresh —
/// dragging any slider, hovering, another entity's `notify` — re-runs
/// `Render::render`), so building either of these inline inside `render()`
/// would upload a fresh image every frame and restart audio playback on
/// every unrelated redraw.
enum PreparedOutput {
    Plot(Arc<crate::outputs::Plot>),
    MatrixPlot(Arc<crate::outputs::MatrixPlot>),
    Image {
        data: crate::outputs::Image,
        render_image: Arc<gpui_kit::RenderImage>,
    },
    Audio(Entity<AudioPlayer>),
}

fn initial_input_values(specs: &[InputSpec]) -> Vec<InputValue> {
    specs
        .iter()
        .map(|spec| match spec {
            InputSpec::Slider(s) => InputValue::F64(s.init),
            InputSpec::IntSlider(s) => InputValue::I64(s.init),
            InputSpec::Checkbox(s) => InputValue::Bool(s.init),
            InputSpec::Radio(s) => InputValue::Index(s.init_index),
        })
        .collect()
}

pub struct ReactiveView {
    level: LevelId,
    worker: Arc<WorkerHandle>,
    scheduler: RunScheduler,
    widgets: Vec<InputWidgetState>,
    outputs: Vec<PreparedOutput>,
    // Images replaced by a newer result, queued to be freed from the
    // sprite atlas on the next `render()` call (which is the first place
    // after `apply_result` that has a `&mut Window` to call
    // `window.drop_image` with).
    pending_image_drops: Vec<Arc<gpui_kit::RenderImage>>,
    child: Option<Entity<ReactiveView>>,
    awaiting: usize,
    busy: bool,
    error: Option<String>,
    _subscriptions: Vec<gpui_kit::Subscription>,
}

impl ReactiveView {
    /// Builds a view's widgets and initial scheduler state, with empty
    /// outputs/error and no child — shared by `new` (the root) and
    /// `apply_result`'s `Nested` handling (a fresh child). Neither has run
    /// its first job yet; the caller dispatches it via
    /// `scheduler.start()` + `dispatch()` right after construction.
    fn new_child(
        level: LevelId,
        specs: Vec<InputSpec>,
        worker: Arc<WorkerHandle>,
        cx: &mut Context<Self>,
    ) -> Self {
        let initial_values = initial_input_values(&specs);

        let mut widgets = Vec::with_capacity(specs.len());
        let mut subscriptions = Vec::new();
        for (i, spec) in specs.into_iter().enumerate() {
            match spec {
                InputSpec::Slider(s) => {
                    let state = cx.new(|_| make_slider_state(&s));
                    let sub = cx.subscribe(&state, move |this: &mut ReactiveView, _state, event, cx| {
                        if let gpui_kit::component::slider::SliderEvent::Change(value) = event {
                            this.on_input_changed(i, InputValue::F64(value.start() as f64), cx);
                        }
                    });
                    subscriptions.push(sub);
                    widgets.push(InputWidgetState::Slider { spec: s, state });
                }
                InputSpec::IntSlider(s) => {
                    let state = cx.new(|_| make_int_slider_state(&s));
                    let sub = cx.subscribe(&state, move |this: &mut ReactiveView, _state, event, cx| {
                        if let gpui_kit::component::slider::SliderEvent::Change(value) = event {
                            this.on_input_changed(i, InputValue::I64(value.start() as i64), cx);
                        }
                    });
                    subscriptions.push(sub);
                    widgets.push(InputWidgetState::IntSlider { spec: s, state });
                }
                InputSpec::Checkbox(s) => {
                    widgets.push(InputWidgetState::Checkbox {
                        checked: s.init,
                        spec: s,
                    });
                }
                InputSpec::Radio(s) => {
                    widgets.push(InputWidgetState::Radio {
                        selected: s.init_index,
                        spec: s,
                    });
                }
            }
        }

        // Drop this level's worker registration when this view's entity is
        // released — whether because a parent replaced it (see
        // `apply_result`'s `Nested` arm, which just overwrites `self.child`
        // and lets normal Rust drop do the rest) or the whole app is
        // torn down. This also covers grandchildren recursively: dropping
        // `child` below drops its `Entity<ReactiveView>`, and if that was
        // its last reference, gpui fires *its* on_release the same way.
        let level_for_release = level;
        let worker_for_release = worker.clone();
        let release_sub = cx.on_release(move |_this, _app_cx| {
            worker_for_release.send(Job::Drop {
                level: level_for_release,
            });
        });
        subscriptions.push(release_sub);

        ReactiveView {
            level,
            worker,
            scheduler: RunScheduler::new(initial_values),
            widgets,
            outputs: Vec::new(),
            pending_image_drops: Vec::new(),
            child: None,
            awaiting: 0,
            busy: false,
            error: None,
            _subscriptions: subscriptions,
        }
    }

    pub fn new(
        level: LevelId,
        specs: Vec<InputSpec>,
        worker: Arc<WorkerHandle>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let mut view = Self::new_child(level, specs, worker, cx);
        // The root's first job is dispatched here, through the same worker
        // channel as every later job — no job runs synchronously on the
        // calling thread, so the window can appear immediately and a slow
        // first callback shows the busy indicator like any other.
        let values = view.scheduler.start();
        view.dispatch(values, cx);
        view
    }

    fn dispatch(&mut self, values: Vec<InputValue>, cx: &mut Context<Self>) {
        let (tx, rx) = futures::channel::oneshot::channel();
        self.worker.send(Job::Run {
            level: self.level,
            values,
            reply: tx,
        });

        // Only show the busy indicator if the job is still running after
        // ~150ms, so fast callbacks don't flicker.
        cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(std::time::Duration::from_millis(150))
                .await;
            let _ = this.update(cx, |this, cx| {
                if this.awaiting > 0 {
                    this.busy = true;
                    cx.notify();
                }
            });
        })
        .detach();

        self.awaiting += 1;
        // Captured separately from `self` so a `Nested` result can still be
        // cleaned up on the worker even if this view is released before
        // the reply arrives (see below).
        let worker_for_stale_reply = self.worker.clone();
        cx.spawn(async move |this, cx| {
            let Ok(result) = rx.await else {
                // Worker panicked mid-job: pending reply never arrives as
                // Ok. Show "worker terminated" instead of hanging.
                let _ = this.update(cx, |this, cx| {
                    this.awaiting -= 1;
                    this.busy = this.awaiting > 0;
                    this.error = Some("worker terminated".to_string());
                    cx.notify();
                });
                return;
            };

            // A `Nested` result already registered a brand-new child level
            // on the worker (see `Registry::run_job_for_ui`) before this
            // reply was even sent. If this view has been released by the
            // time we get here, nothing will ever build the ReactiveView
            // that would drop that level on its own release, so note it
            // now, before `result` is (maybe) moved into the view below.
            let orphaned_nested_level = match &result {
                UiLevelResult::Nested { level, .. } => Some(*level),
                _ => None,
            };

            let applied = this.update(cx, |this, cx| {
                this.awaiting -= 1;
                this.busy = this.awaiting > 0;
                this.apply_result(result, cx);
                let followup = this.scheduler.on_result();
                cx.notify();
                if let Some(values) = followup {
                    this.dispatch(values, cx);
                }
            });

            if applied.is_err() {
                if let Some(level) = orphaned_nested_level {
                    worker_for_stale_reply.send(Job::Drop { level });
                }
            }
        })
        .detach();
    }

    fn apply_result(&mut self, result: UiLevelResult, cx: &mut Context<Self>) {
        match result {
            UiLevelResult::Outputs(outputs) => {
                self.set_outputs(outputs, cx);
                self.error = None;
                // Overwriting `self.child` with `None` drops the old
                // `Entity<ReactiveView>` (if any); that in turn triggers
                // its own on_release, which drops its worker registration.
                self.child = None;
            }
            UiLevelResult::Nested { level, specs } => {
                self.error = None;
                let worker = self.worker.clone();
                let child = cx.new(|cx| {
                    let mut view = ReactiveView::new_child(level, specs, worker, cx);
                    let values = view.scheduler.start();
                    view.dispatch(values, cx);
                    view
                });
                // Overwriting `Some(old_child)` here drops it the same way
                // as the `Outputs` arm above.
                self.child = Some(child);
            }
            UiLevelResult::Error(msg) => {
                self.error = Some(msg);
                // Keep self.outputs (and self.child) as-is: the previous
                // outputs stay visible underneath the error.
            }
            UiLevelResult::Discarded => {}
        }
    }

    /// Prepares each raw `Output` exactly once (building the GPU image /
    /// audio player), replacing `self.outputs`. Images being replaced are
    /// queued in `pending_image_drops` for `render()` to free.
    fn set_outputs(&mut self, outputs: Vec<Output>, cx: &mut Context<Self>) {
        for old in self.outputs.drain(..) {
            if let PreparedOutput::Image { render_image, .. } = old {
                self.pending_image_drops.push(render_image);
            }
        }
        self.outputs = outputs
            .into_iter()
            .map(|output| match output {
                Output::Plot(plot) => PreparedOutput::Plot(Arc::new(plot)),
                Output::MatrixPlot(matrix) => PreparedOutput::MatrixPlot(Arc::new(matrix)),
                Output::Image(data) => {
                    let render_image = build_render_image(&data);
                    PreparedOutput::Image { data, render_image }
                }
                Output::Audio(audio) => {
                    PreparedOutput::Audio(cx.new(|cx| AudioPlayer::new(audio, cx)))
                }
            })
            .collect();
    }

    fn on_input_changed(&mut self, index: usize, value: InputValue, cx: &mut Context<Self>) {
        // Update the widget's own displayed state immediately, independent
        // of the (possibly coalesced, possibly slow) worker round trip:
        // gpui-component's Checkbox/RadioGroup are controlled components,
        // so without this the click always redraws with the pre-click
        // value and a checkbox can never visibly toggle.
        match (&mut self.widgets[index], &value) {
            (InputWidgetState::Checkbox { checked, .. }, InputValue::Bool(v)) => *checked = *v,
            (InputWidgetState::Radio { selected, .. }, InputValue::Index(v)) => *selected = *v,
            _ => {}
        }

        if let Some(values) = self.scheduler.on_change(index, value) {
            self.dispatch(values, cx);
        } else {
            // Coalesced: no dispatch, but the checkbox/radio state above
            // still needs to reach the screen.
            cx.notify();
        }
    }
}

/// One input in the sidebar: a rounded card, like the cushy version's
/// containers, so inputs read as separate controls on the dark background.
fn input_card(cx: &App) -> gpui_kit::Div {
    div()
        .p_3()
        .rounded_md()
        .bg(cx.theme().group_box)
        .text_color(cx.theme().group_box_foreground)
}

impl Render for ReactiveView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.pending_image_drops.is_empty() {
            drop_images(window, std::mem::take(&mut self.pending_image_drops));
        }

        let sidebar = div()
            .id(("sidebar", self.level.index()))
            .w(px(300.))
            .flex_shrink_0()
            .flex()
            .flex_col()
            .gap_2()
            .overflow_y_scroll()
            .children(self.widgets.iter().enumerate().map(|(i, widget)| {
                match widget {
                    InputWidgetState::Slider { spec, state } => {
                        let value = state.read(cx).value().start();
                        input_card(cx)
                            .child(render_slider_row(spec, state, value, cx))
                            .into_any_element()
                    }
                    InputWidgetState::IntSlider { spec, state } => {
                        let value = state.read(cx).value().start();
                        input_card(cx)
                            .child(render_int_slider_row(spec, state, value, cx))
                            .into_any_element()
                    }
                    InputWidgetState::Checkbox { spec, checked } => input_card(cx)
                        .child(render_checkbox(("input", i), spec, *checked, {
                            let entity = cx.entity();
                            move |v, _window, cx| {
                                entity.update(cx, |this, cx| {
                                    this.on_input_changed(i, InputValue::Bool(*v), cx);
                                });
                            }
                        }))
                        .into_any_element(),
                    InputWidgetState::Radio { spec, selected } => input_card(cx)
                        .flex()
                        .flex_col()
                        .gap_2()
                        .child(div().text_sm().text_color(cx.theme().muted_foreground).child(spec.name.clone()))
                        .child(render_radio(("input", i), spec, Some(*selected), {
                            let entity = cx.entity();
                            move |v, _window, cx| {
                                entity.update(cx, |this, cx| {
                                    this.on_input_changed(i, InputValue::Index(*v), cx);
                                });
                            }
                        }))
                        .into_any_element(),
                }
            }));

        let error_block = self.error.as_ref().map(|msg| {
            div()
                .p_3()
                .rounded_md()
                .bg(cx.theme().danger.opacity(0.15))
                .text_color(cx.theme().danger)
                .child(msg.clone())
                .into_any_element()
        });

        let outputs_or_child = if let Some(child) = &self.child {
            div().flex_1().min_w_0().child(child.clone()).into_any_element()
        } else {
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .gap_2()
                .children(self.outputs.iter().map(|output| match output {
                    PreparedOutput::Plot(plot) => div()
                        .flex_1()
                        .min_h(px(120.))
                        .child(LinePlot { data: plot.clone() })
                        .into_any_element(),
                    PreparedOutput::MatrixPlot(matrix) => div()
                        .flex_1()
                        .min_h(px(120.))
                        .child(MatrixPlotView { data: matrix.clone() })
                        .into_any_element(),
                    PreparedOutput::Image { data, render_image } => input_card(cx)
                        .child(image_element(data, render_image.clone()))
                        .into_any_element(),
                    PreparedOutput::Audio(player) => div()
                        .flex()
                        .justify_center()
                        .child(player.clone())
                        .into_any_element(),
                }))
                .into_any_element()
        };

        // The busy dim and the error block apply above the outputs *or*
        // the child (a parent can be busy re-running, or show an error,
        // while a nested level from its previous run is still shown).
        let content = div()
            .flex_1()
            .min_w_0()
            .flex()
            .flex_col()
            .gap_2()
            .when(self.busy, |el| el.opacity(0.5))
            .children(error_block)
            .child(outputs_or_child);

        div()
            .flex()
            .flex_row()
            .gap_2()
            .size_full()
            .child(sidebar)
            .child(content)
    }
}
