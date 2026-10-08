use std::sync::mpsc;
use std::thread::JoinHandle;

use futures::channel::mpsc::UnboundedSender;
use pyo3::prelude::*;

use crate::inputs::InputValue;
use crate::outputs::format_traceback;
use crate::view_tree::{InputId, NodeId, SlotContent, SlotResult, parse_slot_content};

/// A UI input change, sent to the worker.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InputChange {
    pub input: InputId,
    pub value: InputValue,
}

/// What crosses from the worker to the UI: never carries a `Py` value.
#[derive(Debug)]
pub enum WorkerMessage {
    /// The visible slots that the following steps will update (the busy set).
    /// Sent before every step, and once more (usually empty) at the end of a
    /// round.
    Stale(Vec<NodeId>),
    Result(SlotResult),
    /// The engine itself raised: a picoapp bug, not an error of a user node.
    EngineError(String),
}

pub struct WorkerHandle {
    // `Option` so `Drop` can explicitly drop the sender (closing the
    // channel) *before* joining the thread, rather than relying on Rust's
    // automatic field drop order, which only runs after `Drop::drop`
    // returns — too late, since the join inside it would block forever
    // waiting for a channel that hasn't closed yet.
    sender: Option<mpsc::Sender<InputChange>>,
    thread: Option<JoinHandle<()>>,
}

impl WorkerHandle {
    pub fn send(&self, change: InputChange) {
        // The receiving end only goes away when the worker thread itself
        // panicked; the change is then simply dropped.
        if let Some(sender) = &self.sender {
            let _ = sender.send(change);
        }
    }
}

impl Drop for WorkerHandle {
    fn drop(&mut self) {
        // Drop the sender first so the worker loop's `recv()` returns Err
        // and the thread can exit; only then join it.
        self.sender.take();
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

/// Spawns the worker thread, which owns `engine` (a `picoapp._engine.Engine`)
/// and is the only thread that ever calls into Python. It starts the first
/// round right away, without waiting for a UI change, so the window can
/// appear before the first (possibly slow) evaluation finishes.
pub fn spawn(engine: Py<PyAny>, to_ui: UnboundedSender<WorkerMessage>) -> WorkerHandle {
    let (sender, receiver) = mpsc::channel::<InputChange>();
    let thread = std::thread::Builder::new()
        .name("picoapp-worker".to_string())
        .spawn(move || worker_loop(engine, receiver, to_ui))
        .expect("failed to spawn picoapp-worker thread");

    WorkerHandle {
        sender: Some(sender),
        thread: Some(thread),
    }
}

fn worker_loop(
    engine: Py<PyAny>,
    receiver: mpsc::Receiver<InputChange>,
    to_ui: UnboundedSender<WorkerMessage>,
) {
    // The first round starts without any change.
    let mut changes = Vec::new();
    loop {
        run_round(&engine, &receiver, &to_ui, changes);
        // Block until the next change. The channel closes when the UI drops
        // its `WorkerHandle`.
        let Ok(first) = receiver.recv() else {
            break;
        };
        changes = drain_pending(&receiver, vec![first]);
    }
    // Channel closed: drop the engine while attached so every Py handle is
    // released correctly.
    Python::attach(|_py| drop(engine));
}

/// Applies `changes`, then steps until no visible slot is stale. Changes that
/// arrive in between are applied before the next step, which then picks the
/// highest-priority stale slot of the new state (re-plan instead of finishing
/// a stale plan).
fn run_round(
    engine: &Py<PyAny>,
    receiver: &mpsc::Receiver<InputChange>,
    to_ui: &UnboundedSender<WorkerMessage>,
    mut changes: Vec<InputChange>,
) {
    loop {
        let stale = Python::attach(|py| apply_changes(engine.bind(py), &changes));
        let stale = match stale {
            Ok(stale) => stale,
            Err(message) => {
                let _ = to_ui.unbounded_send(WorkerMessage::EngineError(message));
                return;
            }
        };
        let idle = stale.is_empty();
        if to_ui.unbounded_send(WorkerMessage::Stale(stale)).is_err() || idle {
            // Done, or the UI is gone.
            return;
        }

        let result = Python::attach(|py| step(engine.bind(py)));
        let message = match result {
            Ok(Some(result)) => WorkerMessage::Result(result),
            // Can't happen after a non-empty stale set; end the round rather
            // than loop.
            Ok(None) => return,
            Err(message) => WorkerMessage::EngineError(message),
        };
        let failed = matches!(message, WorkerMessage::EngineError(_));
        if to_ui.unbounded_send(message).is_err() || failed {
            return;
        }

        changes = drain_pending(receiver, Vec::new());
    }
}

/// Writes `changes` (if any) and returns the visible stale slots afterwards.
fn apply_changes(
    engine: &Bound<'_, PyAny>,
    changes: &[InputChange],
) -> Result<Vec<NodeId>, String> {
    let call = || -> PyResult<Vec<NodeId>> {
        if !changes.is_empty() {
            let pairs: Vec<(u64, InputValue)> = changes
                .iter()
                .map(|change| (change.input.0, change.value))
                .collect();
            engine.call_method1("set_values", (pairs,))?;
        }
        let stale: Vec<u64> = engine.call_method0("stale_visible_slots")?.extract()?;
        Ok(stale.into_iter().map(NodeId).collect())
    };
    call().map_err(|err| format_traceback(engine.py(), &err))
}

/// Runs one `Engine.step` and parses its result.
fn step(engine: &Bound<'_, PyAny>) -> Result<Option<SlotResult>, String> {
    let call = || -> PyResult<Option<SlotResult>> {
        let result = engine.call_method0("step")?;
        if result.is_none() {
            return Ok(None);
        }
        let (node, _version, content): (u64, u64, Bound<'_, PyAny>) = result.extract()?;
        let content = match parse_slot_content(&content) {
            Ok(content) => content,
            Err(err) => {
                // The UI keeps showing the slot's previous content, so the
                // engine must treat that one as shown, too.
                engine.call_method1("reject", (node,))?;
                SlotContent::Error(format_traceback(engine.py(), &err))
            }
        };
        Ok(Some(SlotResult {
            node: NodeId(node),
            content,
        }))
    };
    call().map_err(|err| format_traceback(engine.py(), &err))
}

/// Collects all changes already queued behind `changes`, without blocking.
fn drain_pending(
    receiver: &mpsc::Receiver<InputChange>,
    mut changes: Vec<InputChange>,
) -> Vec<InputChange> {
    while let Ok(change) = receiver.try_recv() {
        changes.push(change);
    }
    coalesce(changes)
}

/// Keeps the latest value per input (latest value wins, no queue), in the
/// order in which the inputs first changed.
fn coalesce(changes: Vec<InputChange>) -> Vec<InputChange> {
    let mut coalesced: Vec<InputChange> = Vec::new();
    for change in changes {
        match coalesced.iter_mut().find(|c| c.input == change.input) {
            Some(existing) => existing.value = change.value,
            None => coalesced.push(change),
        }
    }
    coalesced
}

#[cfg(test)]
mod tests {
    use futures::channel::mpsc::unbounded;
    use pyo3::types::PyDict;

    use super::*;

    fn change(input: u64, value: f64) -> InputChange {
        InputChange {
            input: InputId(input),
            value: InputValue::F64(value),
        }
    }

    /// Builds an engine stand-in from Python source defining `engine`.
    fn python_engine(code: &std::ffi::CStr) -> Py<PyAny> {
        Python::initialize();
        Python::attach(|py| {
            let locals = PyDict::new(py);
            py.run(code, None, Some(&locals)).unwrap();
            locals.get_item("engine").unwrap().unwrap().unbind()
        })
    }

    #[test]
    fn coalesce_keeps_the_latest_value_per_input() {
        let changes = vec![
            change(1, 1.0),
            change(2, 5.0),
            change(1, 2.0),
            change(1, 3.0),
        ];
        assert_eq!(coalesce(changes), vec![change(1, 3.0), change(2, 5.0)]);
    }

    #[test]
    fn coalesce_keeps_distinct_inputs_in_first_change_order() {
        let changes = vec![change(2, 1.0), change(1, 1.0)];
        assert_eq!(coalesce(changes), vec![change(2, 1.0), change(1, 1.0)]);
    }

    #[test]
    fn first_round_sends_stale_set_result_and_final_stale_set() {
        let engine = python_engine(
            c"
class Engine:
    def __init__(self):
        self.pending = True
    def set_values(self, changes):
        raise AssertionError('the first round has no changes')
    def stale_visible_slots(self):
        return [7] if self.pending else []
    def step(self):
        self.pending = False
        return (7, 1, 'boom')
engine = Engine()
",
        );
        let (to_ui, mut from_worker) = unbounded();
        let worker = spawn(engine, to_ui);
        // Dropping the handle closes the change channel and joins the thread
        // after the first round.
        drop(worker);

        let mut messages = Vec::new();
        while let Ok(message) = from_worker.try_recv() {
            messages.push(format!("{message:?}"));
        }
        assert_eq!(
            messages,
            vec![
                "Stale([NodeId(7)])".to_string(),
                "Result(SlotResult { node: NodeId(7), content: Error(\"boom\") })".to_string(),
                "Stale([])".to_string(),
            ]
        );
    }

    #[test]
    fn engine_exception_becomes_an_engine_error() {
        let engine = python_engine(
            c"
class Engine:
    def stale_visible_slots(self):
        raise RuntimeError('engine bug')
engine = Engine()
",
        );
        let (to_ui, mut from_worker) = unbounded();
        drop(spawn(engine, to_ui));

        let message = from_worker.try_recv().unwrap();
        assert!(
            matches!(&message, WorkerMessage::EngineError(m) if m.contains("engine bug")),
            "{message:?}"
        );
    }

    #[test]
    fn unparsable_content_is_rejected_and_sent_as_an_error() {
        let engine = python_engine(
            c"
class Engine:
    def __init__(self):
        self.pending = True
        self.rejected = []
    def stale_visible_slots(self):
        return [3] if self.pending else []
    def step(self):
        self.pending = False
        return (3, 1, object())
    def reject(self, node_id):
        self.rejected.append(node_id)
engine = Engine()
",
        );
        let engine_for_check = Python::attach(|py| engine.clone_ref(py));
        let (to_ui, mut from_worker) = unbounded();
        drop(spawn(engine, to_ui));

        let messages: Vec<WorkerMessage> =
            std::iter::from_fn(|| from_worker.try_recv().ok()).collect();
        assert!(
            matches!(
                &messages[1],
                WorkerMessage::Result(SlotResult { node: NodeId(3), content: SlotContent::Error(m) })
                    if m.contains("Invalid output type")
            ),
            "{messages:?}"
        );
        let rejected: Vec<u64> = Python::attach(|py| {
            engine_for_check
                .getattr(py, "rejected")
                .unwrap()
                .extract(py)
                .unwrap()
        });
        assert_eq!(rejected, vec![3]);
    }

    /// Regression test for a deadlock: `WorkerHandle::drop` used to call
    /// `thread.join()` while `self.sender` was still alive (a struct's
    /// fields are only dropped *after* its custom `Drop::drop` body
    /// returns), so the worker's `recv()` never saw a closed channel and
    /// the join blocked forever. Runs the drop on a background thread and
    /// polls a flag with a bounded timeout instead of joining directly, so
    /// a regression here fails this test instead of hanging the suite.
    #[test]
    fn dropping_the_last_worker_handle_does_not_hang() {
        let engine = python_engine(
            c"
class Engine:
    def stale_visible_slots(self):
        return []
engine = Engine()
",
        );
        let done = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let done_writer = done.clone();
        std::thread::spawn(move || {
            let (to_ui, _from_worker) = unbounded();
            let worker = spawn(engine, to_ui);
            drop(worker);
            done_writer.store(true, std::sync::atomic::Ordering::SeqCst);
        });

        let start = std::time::Instant::now();
        while !done.load(std::sync::atomic::Ordering::SeqCst) {
            assert!(
                start.elapsed() < std::time::Duration::from_secs(5),
                "dropping WorkerHandle did not complete within 5s (deadlock regression)"
            );
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
    }
}
