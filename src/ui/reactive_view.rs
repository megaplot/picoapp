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
    awaiting: usize,
    busy: bool,
    error: Option<String>,
    _subscriptions: Vec<gpui_kit::Subscription>,
}

impl ReactiveView {
    pub fn new(
        level: LevelId,
        specs: Vec<InputSpec>,
        initial: UiLevelResult,
        worker: Arc<WorkerHandle>,
        _window: &mut Window,
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

        let mut view = ReactiveView {
            level,
            worker,
            scheduler: RunScheduler::new(initial_values),
            widgets,
            outputs: Vec::new(),
            awaiting: 0,
            busy: false,
            error: None,
            _subscriptions: subscriptions,
        };
        // Applies the root's first computed result (Outputs or Error) the
        // same way a later dispatch's result would be applied, so a
        // callback that raises on its very first invocation shows the
        // error immediately instead of silently starting with no output.
        // Nested is handled starting Task 17.
        view.apply_result(initial);
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
                this.apply_result(result);
                let followup = this.scheduler.on_result();
                cx.notify();
                if let Some(values) = followup {
                    this.dispatch(values, cx);
                }
            });
        })
        .detach();
    }

    fn apply_result(&mut self, result: UiLevelResult) {
        match result {
            UiLevelResult::Outputs(outputs) => {
                self.outputs = outputs;
                self.error = None;
            }
            UiLevelResult::Error(msg) => {
                self.error = Some(msg);
                // Keep self.outputs as-is: previous outputs stay visible.
            }
            // Nested handled in Task 17.
            UiLevelResult::Nested { .. } | UiLevelResult::Discarded => {}
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

        let content = div()
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
                // Audio wired up in Task 18.
                Output::Audio(_) => div().child("audio (not yet wired)").into_any_element(),
            }));

        div().flex().flex_row().size_full().child(sidebar).child(content)
    }
}
