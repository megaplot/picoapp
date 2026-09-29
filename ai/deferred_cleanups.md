# Deferred cleanups

Small things we noticed but chose not to fix immediately — not urgent, not
forgotten. Pick from here when there's slack, or fold an item into a task
that's touching the same area anyway. Delete an entry once it's fixed or no
longer applies; don't let this file grow into a graveyard.

Each entry: where, what, why it can wait, when it was noted.

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

## A Rust panic prints to stderr, unlike everything else

- **Where**: no single file — this is Rust's default panic hook, not anything picoapp installs.
- **What**: `logging_setup.rs` only configures `tracing`'s output; it doesn't touch the panic hook. A genuine Rust-side panic (an `.unwrap()`/`.expect()`/index panic in our own code — not a Python exception raised in the user's callback, which pyo3 turns into a normal `PyErr` and never reaches Rust's panic machinery) prints "thread ... panicked at ..." to stderr via Rust's default hook *before* pyo3's `catch_unwind` turns it into a `PanicException`, regardless of `RUST_LOG`/CLAUDE.md's "prints nothing by default".
- **Why deferred**: only reachable via an actual Rust-side bug, not from any known callback error path today; confirmed while investigating `ai/wheel_size.md`'s `strip = true` question, not something we went looking for. A fix (installing a custom `std::panic::set_hook` that routes through the same silent-by-default machinery) is straightforward but untested and orthogonal to what surfaced it.
- **Noted**: 2026-09-28, wheel-size investigation.
