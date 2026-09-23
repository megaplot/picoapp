use std::collections::HashMap;
use std::sync::mpsc;
use std::thread::JoinHandle;

use pyo3::prelude::*;

use crate::inputs::{InputBinding, InputSpec, InputValue};
use crate::outputs::{parse_level_result, LevelResult};
use crate::utils::Callback;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LevelId(u64);

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
    sender: mpsc::Sender<Job>,
    thread: Option<JoinHandle<()>>,
}

impl WorkerHandle {
    pub fn send(&self, job: Job) {
        // The receiving end only goes away when the worker thread itself
        // panicked; in that case the Job is simply dropped, and any
        // `oneshot::Sender` inside it drops too, which resolves the
        // matching receiver to `Canceled` (see ui/reactive_view.rs).
        let _ = self.sender.send(job);
    }
}

impl Drop for WorkerHandle {
    fn drop(&mut self) {
        // Dropping `sender` (by not cloning it elsewhere past this point)
        // closes the channel; the worker loop's `recv()` then returns Err
        // and the thread exits. Take the JoinHandle so this can only run
        // once.
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

/// Spawns the worker thread, registers the root level and runs its first
/// job synchronously (blocking this call) so callers get the initial
/// output without a separate round trip.
pub fn spawn_root(
    py: Python<'_>,
    bindings: Vec<InputBinding>,
    callback: Callback,
    initial_values: Vec<InputValue>,
) -> (WorkerHandle, LevelId, UiLevelResult) {
    let mut registry = Registry::new();
    let root = registry.register(bindings, callback);
    let initial_result = registry.run_job_for_ui(py, root, &initial_values);

    let (sender, receiver) = mpsc::channel::<Job>();
    let thread = std::thread::Builder::new()
        .name("picoapp-worker".to_string())
        .spawn(move || worker_loop(registry, receiver))
        .expect("failed to spawn picoapp-worker thread");

    (
        WorkerHandle {
            sender,
            thread: Some(thread),
        },
        root,
        initial_result,
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
