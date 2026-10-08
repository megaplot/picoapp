//! The view tree crossing from the worker to the UI, and the walk that
//! decides which slots and inputs are visible.
//!
//! Mirrors `python/picoapp/_engine.py`: both sides compute visibility the same
//! way from the same sequence of slot results, so they agree on which slots
//! exist without extra messages.

use std::collections::HashSet;

use pyo3::prelude::*;
use pyo3::types::{PySequence, PyString};

use crate::inputs::InputSpec;
use crate::outputs::{Output, parse_output};

/// A `Memoized` node's `_id`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(pub u64);

/// An input's `_id`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct InputId(pub u64);

/// A layout tree. The worker sends `Tree<Output>` (`ViewTree`); the UI keeps
/// `Tree<PreparedOutput>`, see `map_outputs`.
#[derive(Debug)]
pub enum Tree<O> {
    Row(Vec<Tree<O>>),
    Column(Vec<Tree<O>>),
    Input {
        id: InputId,
        spec: InputSpec,
    },
    Output(O),
    /// A `Memoized` child; its content arrives as its own slot result.
    Slot(NodeId),
}

pub type ViewTree = Tree<Output>;

impl<O> Tree<O> {
    pub fn map_outputs<P>(self, f: &mut impl FnMut(O) -> P) -> Tree<P> {
        match self {
            Tree::Row(children) => Tree::Row(Self::map_children(children, f)),
            Tree::Column(children) => Tree::Column(Self::map_children(children, f)),
            Tree::Input { id, spec } => Tree::Input { id, spec },
            Tree::Output(output) => Tree::Output(f(output)),
            Tree::Slot(node) => Tree::Slot(node),
        }
    }

    fn map_children<P>(children: Vec<Tree<O>>, f: &mut impl FnMut(O) -> P) -> Vec<Tree<P>> {
        children
            .into_iter()
            .map(|child| child.map_outputs(f))
            .collect()
    }

    /// Moves all outputs (not descending into slots) into `out`.
    pub fn into_outputs(self, out: &mut Vec<O>) {
        match self {
            Tree::Row(children) | Tree::Column(children) => {
                for child in children {
                    child.into_outputs(out);
                }
            }
            Tree::Output(output) => out.push(output),
            Tree::Input { .. } | Tree::Slot(_) => {}
        }
    }
}

impl<'a, 'py> FromPyObject<'a, 'py> for ViewTree {
    type Error = PyErr;

    fn extract(obj: Borrowed<'a, 'py, PyAny>) -> Result<Self, Self::Error> {
        let type_name = obj.get_type().name()?;
        if type_name == "Row" || type_name == "Column" {
            let children = obj.getattr("_children")?;
            let children = children.cast::<PySequence>()?;
            let mut trees = Vec::new();
            for child in children.try_iter()? {
                trees.push(child?.extract()?);
            }
            Ok(if type_name == "Row" {
                Tree::Row(trees)
            } else {
                Tree::Column(trees)
            })
        } else if type_name == "Memoized" {
            Ok(Tree::Slot(NodeId(obj.getattr("_id")?.extract()?)))
        } else if type_name == "Slider"
            || type_name == "IntSlider"
            || type_name == "Checkbox"
            || type_name == "Radio"
        {
            Ok(Tree::Input {
                id: InputId(obj.getattr("_id")?.extract()?),
                spec: obj.extract()?,
            })
        } else {
            Ok(Tree::Output(parse_output(&obj)?))
        }
    }
}

/// A slot's content as the worker sends it to the UI.
#[derive(Debug)]
pub enum SlotContent {
    Tree(ViewTree),
    /// The node raised, returned a non-element, or its content could not be
    /// parsed (message + traceback).
    Error(String),
}

#[derive(Debug)]
pub struct SlotResult {
    pub node: NodeId,
    pub content: SlotContent,
}

/// Parses an `Engine.step` content: an element, or an error string.
///
/// Returns `Err` if the element can't be parsed; the worker then tells the
/// engine (`Engine.reject`) and sends the error instead.
pub fn parse_slot_content(content: &Bound<'_, PyAny>) -> PyResult<SlotContent> {
    if let Ok(message) = content.cast::<PyString>() {
        Ok(SlotContent::Error(message.to_string()))
    } else {
        Ok(SlotContent::Tree(content.extract()?))
    }
}

impl SlotContent {
    /// A compact description for tests, e.g. `Row(Slider#3(a=0.5), Slot#4)`.
    pub fn summary(&self) -> String {
        match self {
            SlotContent::Tree(tree) => tree.summary(),
            SlotContent::Error(message) => format!("Error({message})"),
        }
    }
}

impl ViewTree {
    fn summary(&self) -> String {
        let join = |children: &[ViewTree]| {
            children
                .iter()
                .map(ViewTree::summary)
                .collect::<Vec<_>>()
                .join(", ")
        };
        match self {
            Tree::Row(children) => format!("Row({})", join(children)),
            Tree::Column(children) => format!("Column({})", join(children)),
            Tree::Input { id, spec } => match spec {
                InputSpec::Slider(s) => format!("Slider#{}({}={})", id.0, s.name, s.value),
                InputSpec::IntSlider(s) => format!("IntSlider#{}({}={})", id.0, s.name, s.value),
                InputSpec::Checkbox(s) => format!("Checkbox#{}({}={})", id.0, s.name, s.value),
                InputSpec::Radio(s) => format!("Radio#{}({}={})", id.0, s.name, s.index),
            },
            Tree::Output(Output::Plot(_)) => "Plot".to_string(),
            Tree::Output(Output::MatrixPlot(_)) => "MatrixPlot".to_string(),
            Tree::Output(Output::Audio(_)) => "Audio".to_string(),
            Tree::Output(Output::Image(_)) => "Image".to_string(),
            Tree::Slot(node) => format!("Slot#{}", node.0),
        }
    }
}

/// What is visible, in tree order.
#[derive(Debug)]
pub struct Visible<'a> {
    pub slots: Vec<NodeId>,
    pub inputs: Vec<(InputId, &'a InputSpec)>,
}

/// Walks from `root` through each slot's shown content (`shown`, `None` for a
/// slot without content yet). A slot is visited at most once, so a malformed
/// tree can't recurse forever.
pub fn visible<'a, O: 'a>(
    root: NodeId,
    shown: impl Fn(NodeId) -> Option<&'a Tree<O>>,
) -> Visible<'a> {
    let mut visible = Visible {
        slots: Vec::new(),
        inputs: Vec::new(),
    };
    let mut seen = HashSet::new();
    visit_slot(root, &shown, &mut seen, &mut visible);
    visible
}

fn visit_slot<'a, O: 'a>(
    node: NodeId,
    shown: &impl Fn(NodeId) -> Option<&'a Tree<O>>,
    seen: &mut HashSet<NodeId>,
    visible: &mut Visible<'a>,
) {
    if !seen.insert(node) {
        return;
    }
    visible.slots.push(node);
    if let Some(tree) = shown(node) {
        visit_tree(tree, shown, seen, visible);
    }
}

fn visit_tree<'a, O: 'a>(
    tree: &'a Tree<O>,
    shown: &impl Fn(NodeId) -> Option<&'a Tree<O>>,
    seen: &mut HashSet<NodeId>,
    visible: &mut Visible<'a>,
) {
    match tree {
        Tree::Row(children) | Tree::Column(children) => {
            for child in children {
                visit_tree(child, shown, seen, visible);
            }
        }
        Tree::Input { id, spec } => visible.inputs.push((*id, spec)),
        Tree::Output(_) => {}
        Tree::Slot(node) => visit_slot(*node, shown, seen, visible),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::inputs::CheckboxSpec;

    fn checkbox(id: u64) -> Tree<()> {
        Tree::Input {
            id: InputId(id),
            spec: InputSpec::Checkbox(CheckboxSpec {
                name: format!("c{id}"),
                value: false,
            }),
        }
    }

    fn visible_ids(root: u64, shown: &HashMap<NodeId, Tree<()>>) -> (Vec<u64>, Vec<u64>) {
        let visible = visible(NodeId(root), |node| shown.get(&node));
        (
            visible.slots.iter().map(|node| node.0).collect(),
            visible.inputs.iter().map(|(id, _)| id.0).collect(),
        )
    }

    #[test]
    fn root_without_content_is_the_only_visible_slot() {
        let shown = HashMap::new();
        assert_eq!(visible_ids(1, &shown), (vec![1], vec![]));
    }

    #[test]
    fn walks_through_slots_in_tree_order() {
        let shown = HashMap::from([
            (
                NodeId(1),
                Tree::Row(vec![
                    Tree::Column(vec![checkbox(10), Tree::Slot(NodeId(2))]),
                    checkbox(11),
                    Tree::Slot(NodeId(3)),
                ]),
            ),
            (NodeId(2), Tree::Column(vec![checkbox(12)])),
            (NodeId(3), Tree::Output(())),
        ]);
        assert_eq!(visible_ids(1, &shown), (vec![1, 2, 3], vec![10, 12, 11]));
    }

    #[test]
    fn slots_not_reachable_from_the_root_are_invisible() {
        let shown = HashMap::from([
            (NodeId(1), Tree::Row(vec![checkbox(10)])),
            (NodeId(2), Tree::Column(vec![checkbox(11)])),
        ]);
        assert_eq!(visible_ids(1, &shown), (vec![1], vec![10]));
    }

    #[test]
    fn a_slot_containing_itself_is_visited_once() {
        let shown = HashMap::from([(NodeId(1), Tree::Row(vec![Tree::Slot(NodeId(1))]))]);
        assert_eq!(visible_ids(1, &shown), (vec![1], vec![]));
    }

    #[test]
    fn map_outputs_keeps_structure() {
        let tree: Tree<i32> = Tree::Row(vec![Tree::Output(1), Tree::Slot(NodeId(5))]);
        let mapped = tree.map_outputs(&mut |x| x * 10);
        let mut outputs = Vec::new();
        mapped.into_outputs(&mut outputs);
        assert_eq!(outputs, vec![10]);
    }
}
