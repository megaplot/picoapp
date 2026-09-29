use std::collections::HashMap;
use std::sync::mpsc;
use std::thread::JoinHandle;

use pyo3::prelude::*;

use crate::inputs::{InputBinding, InputSpec, InputValue, split_inputs};
use crate::outputs::{CallbackReturn, format_traceback, parse_callback_return};
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

/// What crosses from the worker to the UI: never carries a `Py` value.
pub enum UiLevelResult {
    Outputs(Vec<crate::outputs::Output>),
    Nested {
        level: LevelId,
        specs: Vec<InputSpec>,
    },
    Error(String),
    Discarded,
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
    /// parses the result. Returns `UiLevelResult::Discarded` if `level` isn't
    /// registered (already dropped). On a nested `ReactiveBase` return,
    /// registers the new level's bindings before returning, so the caller
    /// only ever receives specs for it.
    pub fn run_job(
        &mut self,
        py: Python<'_>,
        level: LevelId,
        values: &[InputValue],
    ) -> UiLevelResult {
        let Some(entry) = self.levels.get(&level) else {
            return UiLevelResult::Discarded;
        };

        for (binding, value) in entry.bindings.iter().zip(values) {
            if let Err(err) = binding.set_value(py, value) {
                return UiLevelResult::Error(format_traceback(py, &err));
            }
        }

        let result = entry
            .callback
            .call(py)
            .and_then(|cb_return| parse_callback_return(py, cb_return));
        match result {
            Ok(CallbackReturn::Outputs(outputs)) => UiLevelResult::Outputs(outputs),
            Ok(CallbackReturn::Inputs(inputs, callback)) => {
                let (specs, bindings) = split_inputs(inputs);
                let level = self.register(bindings, callback);
                UiLevelResult::Nested { level, specs }
            }
            Err(err) => UiLevelResult::Error(format_traceback(py, &err)),
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

fn worker_loop(mut registry: Registry, receiver: mpsc::Receiver<Job>) {
    while let Ok(job) = receiver.recv() {
        match job {
            Job::Run {
                level,
                values,
                reply,
            } => {
                let result = Python::attach(|py| registry.run_job(py, level, &values));
                // Ignore a failed send: it only means the UI dropped the
                // receiver (view released while its job was in flight).
                let _ = reply.send(result);
            }
            Job::Drop { level } => {
                Python::attach(|_py| registry.drop_level(level));
            }
        }
    }
    // Channel closed: drop the registry while attached so every Py handle
    // is released correctly.
    Python::attach(|_py| drop(registry));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_job_returns_discarded_for_dropped_level() {
        let mut registry = Registry::new();
        // No easy way to build a real Callback/InputBinding without a live
        // Python interpreter; instead, use a LevelId that was never
        // registered, which is exactly the state a just-dropped level is in
        // from run_job's point of view. `run_job`'s early-return branch
        // doesn't touch Python at all when the level is absent, so this
        // doesn't need a real callback either.
        let level = LevelId(0);
        Python::initialize();
        Python::attach(|py| {
            let result = registry.run_job(py, level, &[]);
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
        Python::initialize();
        let done = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let done_writer = done.clone();
        std::thread::spawn(move || {
            let (worker, _root) = Python::attach(|py| {
                let callback: Callback = py
                    .eval(c"lambda: None", None, None)
                    .unwrap()
                    .extract()
                    .unwrap();
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
