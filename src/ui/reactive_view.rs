use std::sync::Arc;

use gpui_kit::gpui::prelude::FluentBuilder;
use gpui_kit::{
    div, px, AppContext, Context, Entity, IntoElement, ParentElement, Render, Styled, Window,
};

use crate::inputs::{InputSpec, InputValue};
use crate::outputs::Output;
use crate::ui::inputs::{
    make_int_slider_state, make_slider_state, render_checkbox, render_radio, render_slider_row,
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

pub struct ReactiveView {
    level: LevelId,
    worker: Arc<WorkerHandle>,
    scheduler: RunScheduler,
    widgets: Vec<InputWidgetState>,
    outputs: Vec<Output>,
    child: Option<Entity<ReactiveView>>,
    awaiting: usize,
    busy: bool,
    error: Option<String>,
    _subscriptions: Vec<gpui_kit::Subscription>,
}

impl ReactiveView {
    /// Builds a view's widgets and initial scheduler state, with empty
    /// outputs/error and no child — shared by `new` (the root, which then
    /// applies its already-computed first result) and `Nested` handling in
    /// `apply_result` (a fresh child, whose first result hasn't run yet).
    fn new_child(
        level: LevelId,
        specs: Vec<InputSpec>,
        worker: Arc<WorkerHandle>,
        cx: &mut Context<Self>,
    ) -> Self {
        let initial_values: Vec<InputValue> = specs
            .iter()
            .map(|spec| match spec {
                InputSpec::Slider(s) => InputValue::F64(s.init),
                InputSpec::IntSlider(s) => InputValue::I64(s.init),
                InputSpec::Checkbox(s) => InputValue::Bool(s.init),
                InputSpec::Radio(s) => InputValue::Index(s.init_index),
            })
            .collect();

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

        ReactiveView {
            level,
            worker,
            scheduler: RunScheduler::new(initial_values),
            widgets,
            outputs: Vec::new(),
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
        initial: UiLevelResult,
        worker: Arc<WorkerHandle>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let mut view = Self::new_child(level, specs, worker, cx);
        // Applies the root's first computed result (Outputs/Nested/Error)
        // the same way a later dispatch's result would be applied, so a
        // callback that raises (or returns a nested ReactiveBase) on its
        // very first invocation behaves identically to a later one.
        view.apply_result(initial, cx);
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
            let _ = this.update(cx, |this, cx| {
                this.awaiting -= 1;
                this.busy = this.awaiting > 0;
                this.apply_result(result, cx);
                let followup = this.scheduler.on_result();
                cx.notify();
                if let Some(values) = followup {
                    this.dispatch(values, cx);
                }
            });
        })
        .detach();
    }

    fn apply_result(&mut self, result: UiLevelResult, cx: &mut Context<Self>) {
        match result {
            UiLevelResult::Outputs(outputs) => {
                self.outputs = outputs;
                self.error = None;
                if let Some(old_child) = self.child.take() {
                    self.worker.send(Job::Drop {
                        level: old_child.read(cx).level,
                    });
                }
            }
            UiLevelResult::Nested { level, specs } => {
                self.error = None;
                if let Some(old_child) = self.child.take() {
                    self.worker.send(Job::Drop {
                        level: old_child.read(cx).level,
                    });
                }
                // A nested level's initial values come straight from its
                // specs' init values (mirrors run_ui's root-level bootstrap
                // in Task 6/15); its first Outputs is dispatched right away.
                let initial_values: Vec<InputValue> = specs
                    .iter()
                    .map(|spec| match spec {
                        InputSpec::Slider(s) => InputValue::F64(s.init),
                        InputSpec::IntSlider(s) => InputValue::I64(s.init),
                        InputSpec::Checkbox(s) => InputValue::Bool(s.init),
                        InputSpec::Radio(s) => InputValue::Index(s.init_index),
                    })
                    .collect();
                let worker = self.worker.clone();
                let child = cx.new(|cx| {
                    let mut view = ReactiveView::new_child(level, specs, worker, cx);
                    view.dispatch(initial_values, cx);
                    view
                });
                self.child = Some(child);
            }
            UiLevelResult::Error(msg) => {
                self.error = Some(msg);
                // Keep self.outputs as-is: previous outputs stay visible.
            }
            UiLevelResult::Discarded => {}
        }
    }

    fn on_input_changed(&mut self, index: usize, value: InputValue, cx: &mut Context<Self>) {
        if let Some(values) = self.scheduler.on_change(index, value) {
            self.dispatch(values, cx);
        }
    }
}

impl Render for ReactiveView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let sidebar = div()
            .flex()
            .flex_col()
            .w(px(300.))
            .gap_2()
            .children(self.widgets.iter().enumerate().map(|(i, widget)| {
                match widget {
                    InputWidgetState::Slider { spec, state } => {
                        let value = state.read(cx).value().start();
                        render_slider_row(spec, state, value, cx).into_any_element()
                    }
                    InputWidgetState::IntSlider { spec, state } => {
                        let value = state.read(cx).value().start();
                        div().child(format!("{}: {}", spec.name, value as i64)).into_any_element()
                    }
                    InputWidgetState::Checkbox { spec, checked } => {
                        render_checkbox(("input", i), spec, *checked, {
                            let entity = cx.entity();
                            move |v, _window, cx| {
                                entity.update(cx, |this, cx| {
                                    this.on_input_changed(i, InputValue::Bool(*v), cx);
                                });
                            }
                        })
                        .into_any_element()
                    }
                    InputWidgetState::Radio { spec, selected } => {
                        render_radio(("input", i), spec, Some(*selected), {
                            let entity = cx.entity();
                            move |v, _window, cx| {
                                entity.update(cx, |this, cx| {
                                    this.on_input_changed(i, InputValue::Index(*v), cx);
                                });
                            }
                        })
                        .into_any_element()
                    }
                }
            }));

        let error_block = self.error.as_ref().map(|msg| {
            div()
                .p_2()
                .text_color(gpui_kit::red())
                .child(msg.clone())
                .into_any_element()
        });

        let content_area = if let Some(child) = &self.child {
            div().flex_1().child(child.clone()).into_any_element()
        } else {
            div()
                .flex_1()
                .flex()
                .flex_col()
                .gap_4()
                .when(self.busy, |el| el.opacity(0.5))
                .children(error_block)
                .children(self.outputs.iter().map(|output| match output {
                    Output::Plot(plot) => LinePlot { data: plot.clone() }.into_any_element(),
                    Output::MatrixPlot(matrix) => {
                        MatrixPlotView { data: matrix.clone() }.into_any_element()
                    }
                    Output::Image(image) => {
                        let render_image = crate::ui::image::build_render_image(image);
                        crate::ui::image::image_element(image, render_image).into_any_element()
                    }
                    Output::Audio(audio) => {
                        let player =
                            cx.new(|cx| crate::ui::audio::AudioPlayer::new(audio.clone(), cx));
                        div().child(player).into_any_element()
                    }
                }))
                .into_any_element()
        };

        div()
            .flex()
            .flex_row()
            .size_full()
            .child(sidebar)
            .child(content_area)
    }
}
