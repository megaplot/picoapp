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

## `Memoized._maybe_stale` re-walks shared dependencies

- **Where**: `python/picoapp/_memoize.py`, `_maybe_stale` (called by `Engine.stale_visible_slots` and `_next_slot` for every visible slot).
- **What**: unlike `_ensure_fresh`, it caches nothing per epoch, so a dependency reachable over several paths (diamonds) is walked once per path. Stacked diamonds make it exponential in the depth.
- **Why deferred**: the examples have dependency graphs a few nodes deep; no measurable cost. Fix by caching the answer per node for the current epoch (cleared when an input is written).
- **Noted**: 2026-10-08, review of the reactive nodes branch.

## Busy dims stay after an engine error

- **Where**: `src/ui/app_view.rs`, `AppView::on_worker_message` (`WorkerMessage::EngineError`).
- **What**: the worker ends the round on an `EngineError`, but the UI keeps the slots it reported stale dimmed until the next input change starts a new round.
- **Why deferred**: an engine error is a picoapp bug, not a user-code error, so this only shows next to another bug's error card. The fix is one loop (clear `busy_since` on every slot), but testing it needs an `AppView` under `TestAppContext`, which nothing sets up yet.
- **Noted**: 2026-10-08, review of the reactive nodes branch.

## A slider's f64 value goes through f32 in the UI

- **Where**: `src/ui/inputs.rs` (gpui-kit's `SliderState` is `f32`), `src/ui/app_view.rs`, `on_input_changed`.
- **What**: an initial value like `0.3` comes back from the widget as `0.30000001192092896`. The first slider event that doesn't move the value (e.g. a click on the thumb) therefore sends a "change" and re-runs the nodes that read it once.
- **Why deferred**: one extra run, once per slider; values are otherwise right. A fix compares in f32 (or rounds to the slider's step) before sending.
- **Noted**: 2026-10-08, review of the reactive nodes branch.
