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

## A Rust panic prints to stderr, unlike everything else

- **Where**: no single file — this is Rust's default panic hook, not anything picoapp installs.
- **What**: `logging_setup.rs` only configures `tracing`'s output; it doesn't touch the panic hook. A genuine Rust-side panic (an `.unwrap()`/`.expect()`/index panic in our own code — not a Python exception raised in the user's callback, which pyo3 turns into a normal `PyErr` and never reaches Rust's panic machinery) prints "thread ... panicked at ..." to stderr via Rust's default hook *before* pyo3's `catch_unwind` turns it into a `PanicException`, regardless of `RUST_LOG`/CLAUDE.md's "prints nothing by default".
- **Why deferred**: only reachable via an actual Rust-side bug, not from any known callback error path today; confirmed while investigating `ai/wheel_size.md`'s `strip = true` question, not something we went looking for. A fix (installing a custom `std::panic::set_hook` that routes through the same silent-by-default machinery) is straightforward but untested and orthogonal to what surfaced it.
- **Noted**: 2026-09-28, wheel-size investigation.

## Closing the window during a slow node blocks until that node finishes

- **Where**: `src/worker.rs` (`WorkerHandle::drop` joins the worker thread), `src/ui/app_view.rs` (`AppView` owns the handle).
- **What**: when the window closes, dropping `AppView` drops its `WorkerHandle`, whose `Drop` joins the worker on the UI thread. The worker stops after the step in flight (`drain_pending` reports the disconnect), but that step — a user node that may run for seconds — cannot be interrupted, so the UI thread and `pa.run()`'s return wait for it.
- **Why deferred**: needs cooperative cancellation of user code (see "Progress reporting" in `ai/alternative_reactive_model.md`) or not joining the thread on shutdown, both design questions of their own. Unconfirmed how it feels in practice (close `example_slow.py` mid-delay and observe).
- **Noted**: 2026-10-08, review of the reactive nodes branch (re-scopes the earlier "worker shutdown isn't provably joined" entry).
