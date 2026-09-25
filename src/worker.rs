use std::collections::HashMap;
use std::sync::mpsc;
use std::thread::JoinHandle;

use pyo3::prelude::*;

use crate::inputs::{InputBinding, InputSpec, InputValue};
use crate::outputs::{parse_level_result, LevelResult};
use crate::utils::Callback;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LevelId(u64);

impl LevelId {
    /// Stable numeric id, usable as a gpui element id component.
    pub fn index(self) -> usize {
        self.0 as usize
    }
}

struct Level {
    bindings: Vec<InputBinding>,
    callback: Callback,
}

/// Owns every Python handle. Lives entirely on the worker thread.
pub struct Registry {
    next_id: u64,
    levels: HashMap<LevelId, Level>,
}

impl Registry {
    pub fn new() -> Self {
        Registry {
            next_id: 0,
            levels: HashMap::new(),
        }
    }

    pub fn register(&mut self, bindings: Vec<InputBinding>, callback: Callback) -> LevelId {
        let id = LevelId(self.next_id);
        self.next_id += 1;
        self.levels.insert(id, Level { bindings, callback });
        id
    }

    pub fn drop_level(&mut self, level: LevelId) {
        self.levels.remove(&level);
    }

    /// Writes `values` into `level`'s bindings, calls its callback, and
    /// parses the result. Returns `LevelResult::Discarded` if `level` isn't
    /// registered (already dropped). On `Nested`, allocates and registers
    /// the new level's bindings before returning, so the caller only ever
    /// receives specs for it (see `run_job`'s caller in `spawn_worker`,
    /// which turns `Nested { specs, bindings, callback }` into a freshly
    /// registered `LevelId` before forwarding a UI-safe result).
    pub fn run_job(&mut self, py: Python<'_>, level: LevelId, values: &[InputValue]) -> LevelResult {
        let Some(entry) = self.levels.get(&level) else {
            return LevelResult::Discarded;
        };

        for (binding, value) in entry.bindings.iter().zip(values) {
            if let Err(err) = binding.set_value(py, value) {
                return LevelResult::Error(crate::outputs::format_traceback(py, &err));
            }
        }

        let cb_return = entry.callback.call(py);
        parse_level_result(py, cb_return)
    }
}

/// What crosses from the worker to the UI: never carries a `Py` value.
pub enum UiLevelResult {
    Outputs(Vec<crate::outputs::Output>),
    Nested { level: LevelId, specs: Vec<InputSpec> },
    Error(String),
    Discarded,
}

impl Registry {
    /// Like `run_job`, but resolves a `Nested` result into a freshly
    /// registered `LevelId`, so nothing but `InputSpec`s and plain data
    /// escape to the UI.
    pub fn run_job_for_ui(
        &mut self,
        py: Python<'_>,
        level: LevelId,
        values: &[InputValue],
    ) -> UiLevelResult {
        match self.run_job(py, level, values) {
            LevelResult::Outputs(outputs) => UiLevelResult::Outputs(outputs),
            LevelResult::Nested {
                specs,
                bindings,
                callback,
            } => {
                let new_level = self.register(bindings, callback);
                UiLevelResult::Nested {
                    level: new_level,
                    specs,
                }
            }
            LevelResult::Error(msg) => UiLevelResult::Error(msg),
            LevelResult::Discarded => UiLevelResult::Discarded,
        }
    }
}

pub enum Job {
    Run {
        level: LevelId,
        values: Vec<InputValue>,
        reply: futures::channel::oneshot::Sender<UiLevelResult>,
    },
    Drop {
        level: LevelId,
    },
}

pub struct WorkerHandle {
    // `Option` so `Drop` can explicitly drop the sender (closing the
    // channel) *before* joining the thread, rather than relying on Rust's
    // automatic field drop order, which only runs after `Drop::drop`
    // returns — too late, since the join inside it would block forever
    // waiting for a channel that hasn't closed yet.
    sender: Option<mpsc::Sender<Job>>,
    thread: Option<JoinHandle<()>>,
}

impl WorkerHandle {
    pub fn send(&self, job: Job) {
        // The receiving end only goes away when the worker thread itself
        // panicked; in that case the Job is simply dropped, and any
        // `oneshot::Sender` inside it drops too, which resolves the
        // matching receiver to `Canceled` (see ui/reactive_view.rs).
        if let Some(sender) = &self.sender {
            let _ = sender.send(job);
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

/// Spawns the worker thread and registers the root level. Does not run any
/// job synchronously: the caller dispatches the root's first job through
/// the same channel as every later job (see `ReactiveView::new`), so every
/// Python call — including the very first — runs on the dedicated worker
/// thread, and the window can appear before that first call finishes.
pub fn spawn(bindings: Vec<InputBinding>, callback: Callback) -> (WorkerHandle, LevelId) {
    let mut registry = Registry::new();
    let root = registry.register(bindings, callback);

    let (sender, receiver) = mpsc::channel::<Job>();
    let thread = std::thread::Builder::new()
        .name("picoapp-worker".to_string())
        .spawn(move || worker_loop(registry, receiver))
        .expect("failed to spawn picoapp-worker thread");

    (
        WorkerHandle {
            sender: Some(sender),
            thread: Some(thread),
        },
        root,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_job_for_ui_returns_discarded_for_dropped_level() {
        let mut registry = Registry::new();
        // No easy way to build a real Callback/InputBinding without a live
        // Python interpreter; instead, exercise drop_level directly against a
        // LevelId that was never registered, which is exactly the state a
        // just-dropped level is in from run_job_for_ui's point of view.
        let level = LevelId(0);
        // No `Python::with_gil` available outside `pyo3::prepare_freethreaded_python()`
        // in a plain `cargo test`; this test only needs `run_job`'s early-return
        // branch, which doesn't touch Python at all when the level is absent.
        pyo3::prepare_freethreaded_python();
        Python::with_gil(|py| {
            let result = registry.run_job_for_ui(py, level, &[]);
            assert!(matches!(result, UiLevelResult::Discarded));
        });
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
        pyo3::prepare_freethreaded_python();
        let done = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let done_writer = done.clone();
        std::thread::spawn(move || {
            let (worker, _root) = Python::with_gil(|py| {
                let callback: Callback = py.eval_bound("lambda: None", None, None).unwrap().extract().unwrap();
                spawn(vec![], callback)
            });
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

fn worker_loop(mut registry: Registry, receiver: mpsc::Receiver<Job>) {
    while let Ok(job) = receiver.recv() {
        match job {
            Job::Run {
                level,
                values,
                reply,
            } => {
                let result = Python::with_gil(|py| registry.run_job_for_ui(py, level, &values));
                // Ignore a failed send: it only means the UI dropped the
                // receiver (view released while its job was in flight).
                let _ = reply.send(result);
            }
            Job::Drop { level } => {
                Python::with_gil(|_py| registry.drop_level(level));
            }
        }
    }
    // Channel closed: drop the registry under the GIL so every Py handle
    // is released correctly.
    Python::with_gil(|_py| drop(registry));
}
