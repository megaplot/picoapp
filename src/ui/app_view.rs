use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use std::sync::Arc;
use std::time::{Duration, Instant};

use futures::StreamExt;
use futures::channel::mpsc::UnboundedReceiver;
use gpui_kit::component::slider::{SliderEvent, SliderState};
use gpui_kit::gpui::prelude::FluentBuilder;
use gpui_kit::{
    AnyElement, App, AppContext, Context, Div, Entity, InteractiveElement, IntoElement,
    ParentElement, Render, RenderImage, SharedString, StatefulInteractiveElement, Styled,
    Subscription, Window, div, px,
};

use crate::inputs::{CheckboxSpec, InputSpec, InputValue, RadioSpec, SliderSpec};
use crate::outputs::{Image as ImageData, MatrixPlot as MatrixPlotData, Output, Plot as PlotData};
use crate::ui::audio::AudioPlayer;
use crate::ui::image::{build_render_image, drop_images, image_element};
use crate::ui::inputs::{
    make_int_slider_state, make_slider_state, render_checkbox, render_int_slider_row, render_radio,
    render_slider_row,
};
use crate::ui::line_plot::LinePlot;
use crate::ui::matrix_plot::MatrixPlotView;
use crate::ui::style::{CONTROL_GAP, GUTTER, SIDEBAR_WIDTH, card, error_card, muted_text};
use crate::view_tree::{InputId, NodeId, SlotContent, SlotResult, Tree, visible};
use crate::worker::{InputChange, WorkerHandle, WorkerMessage};

/// A slot is dimmed only if it stays stale this long, so fast updates don't
/// flicker.
const BUSY_DELAY: Duration = Duration::from_millis(150);

/// Minimum height of a fill element (plot, image) inside a column.
const MIN_FILL_HEIGHT: f32 = 120.;

enum InputWidgetState {
    Slider {
        spec: SliderSpec<f64>,
        state: Entity<SliderState>,
        _subscription: Subscription,
    },
    IntSlider {
        spec: SliderSpec<i64>,
        state: Entity<SliderState>,
        _subscription: Subscription,
    },
    Checkbox {
        spec: CheckboxSpec,
        checked: bool,
    },
    Radio {
        spec: RadioSpec,
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
    Plot(Arc<PlotData>),
    MatrixPlot(Arc<MatrixPlotData>),
    Image {
        data: ImageData,
        render_image: Arc<RenderImage>,
    },
    Audio(Entity<AudioPlayer>),
}

type PreparedTree = Tree<PreparedOutput>;

/// The UI state of one visible slot.
#[derive(Default)]
struct SlotState {
    /// The last good content; `None` until the slot's first good result.
    content: Option<PreparedTree>,
    /// The latest error, shown above `content`.
    error: Option<String>,
    /// Since when the worker reports the slot stale.
    busy_since: Option<Instant>,
}

/// Where an element is placed, which decides how it is sized.
#[derive(Clone, Copy)]
enum Placement {
    /// The root slot: the whole window content.
    Root,
    /// A slot's content: fills the slot.
    SlotContent,
    Row,
    Column,
}

/// The app's single view: renders the slot tree from the root slot.
pub struct AppView {
    root: NodeId,
    worker: WorkerHandle,
    slots: HashMap<NodeId, SlotState>,
    widgets: HashMap<InputId, InputWidgetState>,
    // The value each visible input's widget last reported, so an event that
    // doesn't change the value isn't sent to the worker.
    values: HashMap<InputId, InputValue>,
    // Images replaced by a newer result or dropped with an invisible slot,
    // queued to be freed from the sprite atlas on the next `render()` call
    // (the first place with a `&mut Window` to call `window.drop_image` with).
    pending_image_drops: Vec<Arc<RenderImage>>,
    engine_error: Option<String>,
    _subscriptions: Vec<Subscription>,
}

impl AppView {
    pub fn new(
        root: NodeId,
        worker: WorkerHandle,
        messages: UnboundedReceiver<WorkerMessage>,
        cx: &mut Context<Self>,
    ) -> Self {
        cx.spawn(async move |this, cx| {
            let mut messages = messages;
            while let Some(message) = messages.next().await {
                if this
                    .update(cx, |this, cx| this.on_worker_message(message, cx))
                    .is_err()
                {
                    // View released.
                    break;
                }
            }
        })
        .detach();

        // `render()` only frees images it has a `&mut Window` for; whatever
        // is still held when the view is released — a pending drop that
        // never got a render pass, or a live output — would otherwise leak
        // in the sprite atlas forever. `App`'s `drop_image` (not `Window`'s):
        // no `&mut Window` is available here, and this removes the texture
        // from every window anyway.
        let release_subscription = cx.on_release(|this, app_cx| {
            let mut images = std::mem::take(&mut this.pending_image_drops);
            for slot in std::mem::take(&mut this.slots).into_values() {
                collect_images(slot.content, &mut images);
            }
            for image in images {
                app_cx.drop_image(image, None);
            }
        });

        AppView {
            root,
            worker,
            slots: HashMap::new(),
            widgets: HashMap::new(),
            values: HashMap::new(),
            pending_image_drops: Vec::new(),
            engine_error: None,
            _subscriptions: vec![release_subscription],
        }
    }

    fn on_worker_message(&mut self, message: WorkerMessage, cx: &mut Context<Self>) {
        match message {
            WorkerMessage::Stale(nodes) => self.set_stale(&nodes, cx),
            WorkerMessage::Result(result) => self.apply_result(result, cx),
            WorkerMessage::EngineError(message) => self.engine_error = Some(message),
        }
        cx.notify();
    }

    fn set_stale(&mut self, nodes: &[NodeId], cx: &mut Context<Self>) {
        let stale: HashSet<NodeId> = nodes.iter().copied().collect();
        let now = Instant::now();
        let mut newly_busy = false;
        for node in nodes {
            let slot = self.slots.entry(*node).or_default();
            if slot.busy_since.is_none() {
                slot.busy_since = Some(now);
                newly_busy = true;
            }
        }
        for (node, slot) in &mut self.slots {
            if !stale.contains(node) {
                slot.busy_since = None;
            }
        }
        if newly_busy {
            // Redraw once the delay has passed, so the dim appears even if
            // nothing else triggers a redraw.
            cx.spawn(async move |this, cx| {
                cx.background_executor().timer(BUSY_DELAY).await;
                let _ = this.update(cx, |_, cx| cx.notify());
            })
            .detach();
        }
    }

    fn apply_result(&mut self, result: SlotResult, cx: &mut Context<Self>) {
        self.engine_error = None;
        let slot = self.slots.entry(result.node).or_default();
        match result.content {
            SlotContent::Tree(tree) => {
                let tree = tree.map_outputs(&mut |output| prepare_output(output, cx));
                collect_images(slot.content.replace(tree), &mut self.pending_image_drops);
                slot.error = None;
            }
            SlotContent::Error(message) => {
                // Keep the previous content: it stays visible below the error.
                slot.error = Some(message);
            }
        }
        self.prune_invisible(cx);
    }

    /// Drops the state of slots and inputs that are no longer visible, and
    /// creates widgets for newly visible inputs.
    fn prune_invisible(&mut self, cx: &mut Context<Self>) {
        let visible = visible(self.root, |node| {
            self.slots.get(&node).and_then(|slot| slot.content.as_ref())
        });
        let visible_slots: HashSet<NodeId> = visible.slots.iter().copied().collect();
        let visible_inputs: HashSet<InputId> = visible.inputs.iter().map(|(id, _)| *id).collect();
        let new_inputs: Vec<(InputId, InputSpec)> = visible
            .inputs
            .iter()
            .filter(|(id, _)| !self.widgets.contains_key(id))
            .map(|(id, spec)| (*id, (*spec).clone()))
            .collect();

        for slot in prune(&mut self.slots, &visible_slots) {
            collect_images(slot.content, &mut self.pending_image_drops);
        }
        prune(&mut self.widgets, &visible_inputs);
        prune(&mut self.values, &visible_inputs);
        for (id, spec) in new_inputs {
            self.add_widget(id, spec, cx);
        }
    }

    /// Creates the widget for a newly visible input from its spec's current
    /// value. An input that stays visible keeps its widget: while the user
    /// drags, the widget, not the spec, is the source of truth.
    fn add_widget(&mut self, id: InputId, spec: InputSpec, cx: &mut Context<Self>) {
        self.values.insert(id, spec.value());
        let widget = match spec {
            InputSpec::Slider(s) => {
                let state = cx.new(|_| make_slider_state(&s));
                let subscription =
                    cx.subscribe(&state, move |this: &mut AppView, _state, event, cx| {
                        if let SliderEvent::Change(value) = event {
                            this.on_input_changed(id, InputValue::F64(value.start() as f64), cx);
                        }
                    });
                InputWidgetState::Slider {
                    spec: s,
                    state,
                    _subscription: subscription,
                }
            }
            InputSpec::IntSlider(s) => {
                let state = cx.new(|_| make_int_slider_state(&s));
                let subscription =
                    cx.subscribe(&state, move |this: &mut AppView, _state, event, cx| {
                        if let SliderEvent::Change(value) = event {
                            this.on_input_changed(id, InputValue::I64(value.start() as i64), cx);
                        }
                    });
                InputWidgetState::IntSlider {
                    spec: s,
                    state,
                    _subscription: subscription,
                }
            }
            InputSpec::Checkbox(s) => InputWidgetState::Checkbox {
                checked: s.value,
                spec: s,
            },
            InputSpec::Radio(s) => InputWidgetState::Radio {
                selected: s.index,
                spec: s,
            },
        };
        self.widgets.insert(id, widget);
    }

    fn on_input_changed(&mut self, id: InputId, value: InputValue, cx: &mut Context<Self>) {
        // Update the widget's own displayed state immediately, independent
        // of the (possibly coalesced, possibly slow) worker round trip:
        // gpui-component's Checkbox/RadioGroup are controlled components,
        // so without this the click always redraws with the pre-click
        // value and a checkbox can never visibly toggle.
        match (self.widgets.get_mut(&id), &value) {
            (Some(InputWidgetState::Checkbox { checked, .. }), InputValue::Bool(v)) => {
                *checked = *v
            }
            (Some(InputWidgetState::Radio { selected, .. }), InputValue::Index(v)) => {
                *selected = *v
            }
            _ => {}
        }

        // Widgets report events, not changes: a slider emits one on every
        // mouse move even while its (integer-rounded) value stays put.
        // Re-running nodes for an unchanged value is pure waste.
        if self.values.get(&id) != Some(&value) {
            self.values.insert(id, value);
            self.worker.send(InputChange { input: id, value });
        }
        cx.notify();
    }

    fn render_slot(
        &self,
        node: NodeId,
        placement: Placement,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let fill = self.slot_is_fill(node);
        let Some(slot) = self.slots.get(&node) else {
            return sized(div(), fill, placement).into_any_element();
        };
        let busy = slot
            .busy_since
            .is_some_and(|since| since.elapsed() >= BUSY_DELAY);
        let error_block = slot
            .error
            .as_ref()
            .map(|msg| error_card(msg.clone(), cx).flex_shrink_0());
        let content = slot
            .content
            .as_ref()
            .map(|tree| self.render_tree(tree, Placement::SlotContent, &format!("{}", node.0), cx));
        sized(div().flex().flex_col().gap(GUTTER), fill, placement)
            .when(busy, |el| el.opacity(0.5))
            .children(error_block)
            .children(content)
            .into_any_element()
    }

    fn render_tree(
        &self,
        tree: &PreparedTree,
        placement: Placement,
        path: &str,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let fill = self.is_fill(tree);
        match tree {
            Tree::Row(children) => {
                let children: Vec<AnyElement> = children
                    .iter()
                    .enumerate()
                    .map(|(i, child)| {
                        self.render_tree(child, Placement::Row, &format!("{path}/{i}"), cx)
                    })
                    .collect();
                sized(div().flex().flex_row().gap(GUTTER), fill, placement)
                    .children(children)
                    .into_any_element()
            }
            Tree::Column(children) => {
                let children: Vec<AnyElement> = children
                    .iter()
                    .enumerate()
                    .map(|(i, child)| {
                        self.render_tree(child, Placement::Column, &format!("{path}/{i}"), cx)
                    })
                    .collect();
                let column =
                    sized(div().flex().flex_col().gap(GUTTER), fill, placement).children(children);
                if fill {
                    column.into_any_element()
                } else {
                    // A column of compact elements (e.g. inputs) scrolls when
                    // it overflows, like a sidebar.
                    column
                        .id(SharedString::from(format!("column-{path}")))
                        .overflow_y_scroll()
                        .into_any_element()
                }
            }
            Tree::Input { id, .. } => match self.widgets.get(id) {
                Some(widget) => {
                    sized(self.render_input(*id, widget, cx), false, placement).into_any_element()
                }
                None => div().into_any_element(),
            },
            Tree::Output(output) => {
                sized(render_output(output, cx), fill, placement).into_any_element()
            }
            Tree::Slot(node) => self.render_slot(*node, placement, cx),
        }
    }

    fn render_input(&self, id: InputId, widget: &InputWidgetState, cx: &mut Context<Self>) -> Div {
        let index = id.0 as usize;
        match widget {
            InputWidgetState::Slider { spec, state, .. } => {
                let value = state.read(cx).value().start();
                card(cx).child(render_slider_row(spec, state, value, cx))
            }
            InputWidgetState::IntSlider { spec, state, .. } => {
                let value = state.read(cx).value().start();
                card(cx).child(render_int_slider_row(spec, state, value, cx))
            }
            InputWidgetState::Checkbox { spec, checked } => {
                card(cx).child(render_checkbox(("input", index), spec, *checked, {
                    let entity = cx.entity();
                    move |v, _window, cx| {
                        entity.update(cx, |this, cx| {
                            this.on_input_changed(id, InputValue::Bool(*v), cx);
                        });
                    }
                }))
            }
            InputWidgetState::Radio { spec, selected } => card(cx)
                .flex()
                .flex_col()
                .gap(CONTROL_GAP)
                .child(
                    div()
                        .text_sm()
                        .text_color(muted_text(cx))
                        .child(spec.name.clone()),
                )
                .child(render_radio(("input", index), spec, Some(*selected), {
                    let entity = cx.entity();
                    move |v, _window, cx| {
                        entity.update(cx, |this, cx| {
                            this.on_input_changed(id, InputValue::Index(*v), cx);
                        });
                    }
                })),
        }
    }

    /// Whether `tree` grows to fill space (it contains a plot or image) or
    /// keeps its natural size (inputs, audio).
    fn is_fill(&self, tree: &PreparedTree) -> bool {
        match tree {
            Tree::Row(children) | Tree::Column(children) => {
                children.iter().any(|child| self.is_fill(child))
            }
            Tree::Input { .. } => false,
            Tree::Output(PreparedOutput::Audio(_)) => false,
            Tree::Output(_) => true,
            Tree::Slot(node) => self.slot_is_fill(*node),
        }
    }

    /// A slot without content yet counts as fill: it is most likely a slow
    /// output, which shouldn't be squeezed into a sidebar's width meanwhile.
    fn slot_is_fill(&self, node: NodeId) -> bool {
        self.slots
            .get(&node)
            .and_then(|slot| slot.content.as_ref())
            .is_none_or(|tree| self.is_fill(tree))
    }
}

impl Render for AppView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.pending_image_drops.is_empty() {
            drop_images(window, std::mem::take(&mut self.pending_image_drops));
        }

        let engine_error = self
            .engine_error
            .as_ref()
            .map(|msg| error_card(msg.clone(), cx).flex_shrink_0());

        div()
            .size_full()
            .flex()
            .flex_col()
            .gap(GUTTER)
            .children(engine_error)
            .child(self.render_slot(self.root, Placement::Root, cx))
    }
}

/// Applies the size an element gets from its placement.
///
/// In a `Row`, a compact child gets the sidebar width and fill children
/// share the rest; in a `Column`, fill children share the height (with a
/// minimum) and compact children keep their natural height.
fn sized(el: Div, fill: bool, placement: Placement) -> Div {
    match (placement, fill) {
        (Placement::Root, _) => el.size_full(),
        (Placement::SlotContent, _) => el.flex_1().min_w_0().min_h_0(),
        (Placement::Row, true) => el.flex_1().min_w_0(),
        (Placement::Row, false) => el.w(SIDEBAR_WIDTH).flex_shrink_0(),
        (Placement::Column, true) => el.flex_1().min_h(px(MIN_FILL_HEIGHT)),
        (Placement::Column, false) => el.flex_shrink_0(),
    }
}

fn render_output(output: &PreparedOutput, cx: &App) -> Div {
    match output {
        PreparedOutput::Plot(plot) => div().child(LinePlot { data: plot.clone() }),
        PreparedOutput::MatrixPlot(matrix) => div().child(MatrixPlotView {
            data: matrix.clone(),
        }),
        PreparedOutput::Image { data, render_image } => {
            card(cx).child(image_element(data, render_image.clone()))
        }
        PreparedOutput::Audio(player) => div().flex().justify_center().child(player.clone()),
    }
}

/// Prepares a raw `Output` exactly once (building the GPU image / audio
/// player), see `PreparedOutput`.
fn prepare_output(output: Output, cx: &mut Context<AppView>) -> PreparedOutput {
    match output {
        Output::Plot(plot) => PreparedOutput::Plot(Arc::new(plot)),
        Output::MatrixPlot(matrix) => PreparedOutput::MatrixPlot(Arc::new(matrix)),
        Output::Image(data) => {
            let render_image = build_render_image(&data);
            PreparedOutput::Image { data, render_image }
        }
        Output::Audio(audio) => PreparedOutput::Audio(cx.new(|_| AudioPlayer::new(audio))),
    }
}

/// Moves the images of a replaced or dropped slot content into `images`, to
/// be freed from the sprite atlas.
fn collect_images(content: Option<PreparedTree>, images: &mut Vec<Arc<RenderImage>>) {
    let mut outputs = Vec::new();
    if let Some(tree) = content {
        tree.into_outputs(&mut outputs);
    }
    for output in outputs {
        if let PreparedOutput::Image { render_image, .. } = output {
            images.push(render_image);
        }
    }
}

/// Removes the entries whose key is not in `keep`, returning their values.
fn prune<K: Copy + Eq + Hash, V>(map: &mut HashMap<K, V>, keep: &HashSet<K>) -> Vec<V> {
    let removed: Vec<K> = map
        .keys()
        .filter(|key| !keep.contains(key))
        .copied()
        .collect();
    removed
        .into_iter()
        .filter_map(|key| map.remove(&key))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prune_removes_and_returns_entries_not_kept() {
        let mut map = HashMap::from([(1, "a"), (2, "b"), (3, "c")]);
        let mut removed = prune(&mut map, &HashSet::from([2]));
        removed.sort();
        assert_eq!(removed, vec!["a", "c"]);
        assert_eq!(map, HashMap::from([(2, "b")]));
    }

    #[test]
    fn prune_keeps_everything_when_all_are_kept() {
        let mut map = HashMap::from([(1, "a")]);
        assert!(prune(&mut map, &HashSet::from([1, 2])).is_empty());
        assert_eq!(map.len(), 1);
    }
}
