# Deferred cleanups

Small things we noticed but chose not to fix immediately — not urgent, not
forgotten. Pick from here when there's slack, or fold an item into a task
that's touching the same area anyway. Delete an entry once it's fixed or no
longer applies; don't let this file grow into a graveyard.

Each entry: where, what, why it can wait, when it was noted.

## `AudioPlayer`'s `sink` is `Arc<Mutex<Sink>>`, but nothing is cross-thread anymore

- **Where**: `src/ui/audio.rs`, `AudioPlayer::sink` and `toggle()`'s lock/drop/relock dance.
- **What**: the `Arc<Mutex<..>>` dates from a design where a separate OS thread polled playback progress. That's gone (`start_progress_polling` runs as a `cx.spawn` task on gpui's own executor), so the mutex just adds ceremony — `toggle()` locks, drops, and relocks the same sink to avoid a borrow conflict with `&mut self`.
- **Why deferred**: cosmetic; works correctly as is. Worth simplifying to a plain `Rc<RefCell<Sink>>` (or no indirection at all, if gpui's executor guarantees single-threaded access) next time this file is touched.
- **Noted**: 2026-09-25, gpui migration review.

## `PlotColors.line` authored differently from the other fields

- **Where**: `src/ui/style.rs`, `plot_colors()`.
- **What**: `panel`/`axis`/`text`/`grid` are written directly as `hsla(...)`; `line` is written as an `Rgba { r, g, b, a }` literal `.into()`'d to `Hsla`. Same resulting type, inconsistent authoring.
- **Why deferred**: purely cosmetic, and converting risks a rounding-induced color shift no one asked for. Fix by hand-converting to an equivalent `hsla(...)` call and eyeballing the plot afterward.
- **Noted**: 2026-09-25, gpui migration review.

## Worker shutdown isn't provably joined before `pa.run()` returns

- **Where**: `src/ui/reactive_view.rs` (`dispatch`'s detached reply task holds an `Arc<WorkerHandle>`), `src/worker.rs` (`WorkerHandle::drop`).
- **What**: if gpui doesn't drop all pending `cx.spawn` tasks before its `run()` call returns, a detached task still holding the `Arc<WorkerHandle>` could keep the worker thread (and an in-flight Python callback) alive after `pa.run()` has returned to the caller.
- **Why deferred**: unconfirmed whether this is reachable in practice — needs a manual check (close the window while a slow callback is running, see if the process/thread actually exits) rather than a speculative code change.
- **Noted**: 2026-09-25, gpui migration review.

## No automated test for `Nested` actually registering a runnable level

- **Where**: `src/worker.rs` tests, `_run_worker_job` (`src/py_module.rs`).
- **What**: `test_worker_job_returns_nested` (Python-side) only checks the `"Nested(1)"` return string; nothing then dispatches a job *against* the newly registered level to confirm its bindings actually work. `_run_worker_job` is a one-shot hook (throwaway `Registry` per call), so there's no way to reach the new level from Python today.
- **Why deferred**: would need a second test hook (e.g. one that keeps the `Registry` alive across two calls) — real but non-trivial scope for a review follow-up.
- **Noted**: 2026-09-25, gpui migration review.

## pyo3 is several majors behind latest (0.22 vs 0.29+)

- **Where**: `Cargo.toml`.
- **What**: `src/py_module.rs` carries a `#![allow(unsafe_op_in_unsafe_fn)]` worked around a pyo3 0.22 / edition 2024 interaction; a newer pyo3 likely doesn't need it. A jump this size is its own migration (API changes across every `Bound`/`FromPyObject` use site), not a quick bump.
- **Why deferred**: no functional problem today; the `allow` is documented and scoped. Worth its own task when there's a concrete reason to upgrade (a needed pyo3 feature/fix), not preemptively.
- **Noted**: 2026-09-25, gpui migration review.
