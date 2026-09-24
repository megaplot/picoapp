# Project description

Picoapp is a (prototypical) minimal, opinionated Python app framework offering a super fast/responsive UI.

It does not try to be general purpose UI framework -- it offers neither low-level, general purpose UI components, nor a fine-grained reactivity framework.
Instead it offers a simple reactivity framework with a strong emphasis on type-safety that allows to write simple, explorative apps in a quick and simple way.

Typical use cases: You have some algorithmic Python code and you want to study how it behaves depending on the algorithm parameters.
The UI allows to control parameters via simple UI elements like sliders, checkboxes, comboboxes etc.
Changes to these input components result in a re-evaluation of the Python code.
How fast the Python function re-evaluates does not matter -- the goal is that picoapp can deal with both low-latency (UI responds immediately) and high-latency (UI will indicate that it re-runs) use cases.
The Python callback are able to return certain output elements.
These output elements can get visualized by picoapp.

**What makes picoapp special**

What makes picoapp special compared to other approaches like e.g. streamlit:
Picoapp does not rely on web technologies (no browser, no webview) for its UI, and thus, does not require the typical "Python -> serialization -> deserialization -> Web UI" indirection.
Instead it uses a native GPU-powered UI.
This allows to dramatically reduce the overhead and latency of all the UI -> Python -> UI roundtrips.
It also allows to avoid memory duplication of having to hold potentially large data like numpy arrays both in memory in Python and in memory in the browser for the UI.
Instead, the UI can make directly access the underlying Python data and render visualization off of it with zero overhead.

**Project status**

The majority here is work-in-progress, far from stable.

# Development

## Environment

`. env.sh` activates `./venv` and puts `./scripts` on `$PATH`. All commands below assume it has been sourced.

```sh
. env.sh
./scripts/venv_create              # uv venv (first time only)
./scripts/venv_install             # uv pip install -r requirements.txt -e . --no-deps
./scripts/venv_compile_requirements  # re-pin requirements.txt from requirements*.in
```

System dependencies (Linux): `libasound2-dev`, `libdbus-1-dev`, `libxkbcommon-x11-dev`, `libwayland-dev`, `libx11-xcb-dev`, `libfontconfig-dev`, `clang`.

## Build / test / check

```sh
maturin develop --uv          # rebuild the Rust extension into the venv (REQUIRED after any src/*.rs change)
cargo test                    # Rust side (no #[test]s exist yet, but CI runs it)
pytest                        # Python tests
pytest tests/test_input_parsing.py::test_parse_slider   # single test
mypy . && flake8 && black --check . && isort --check .  # what CI checks
python examples/example_1.py  # run an example app (needs a GPU/display)
```

`maturin develop --uv && python examples/example_X.py` is the main iteration loop for anything touching Rust.

UI QA without a desktop session runs headlessly: an X11 path (Xvfb; screenshots *and* clicks/drags; the default) and a Wayland path (private GNOME Shell; screenshots only, for window decorations). `scripts/qa/README.md` says which to pick and how. Use it for any UI change instead of "the window opens without crashing" checks, which cannot catch interaction or rendering bugs.

Also: picoapp must print nothing by default (its stdout/stderr belong to the user's callback); `RUST_LOG=info` opts into UI-stack logs.

`.github/workflows/deploy.yml` started from `./scripts/regenerate_maturin_ci` (which wraps `maturin generate-ci github`) but has hand edits since (target matrix trimmed, manylinux version, Linux system dependencies) — regenerating it from scratch will lose them; diff before overwriting.

## AI Workflow Rules

- **Language**: Keep communication concise and self-critical. Avoid overusing figurative/generic/vague terms that require translation into something concrete specific. Avoid following the LLM langue entry collapse. Avoid overusing terms: once X lands, fold X into, load-bearing
- **Ask before essential decisions.** When an issue could indicate an architectural/design flaw, describe options with pros/cons instead of hacking around it.
- **Git rules**: Never `git push` by yourself or `git commit` on main. Commits on `main` are left for the human companion to give a chance of a last review. Committing on feature branches is fine. Never merge a feature branch into `main` yourself, locally or otherwise — the human companion always reviews a feature branch before it merges. This also means: when using the `superpowers` finishing-a-development-branch skill (or any equivalent finalize/wrap-up step), skip its menu and its merge/PR actions in this repo — just report the branch is done and stop; don't offer to merge or push.

# Architecture

Hybrid Rust/Python package built with maturin/PyO3:

- `python/picoapp/` — the public Python API (pure Python, typed, `py.typed`).
- `src/` — the Rust extension module, exposed as `picoapp._picoapp` (a `cdylib`).
- The entire FFI surface is one function: `_picoapp.run(inputs, callback)` (`src/py_module.rs`). `pa.run()` (`_core.py`) just unwraps a `ReactiveBase` into `(reactive.inputs.inputs, reactive.__call__)`. `_parse_input` exists solely so the parsing layer is unit-testable from Python.

## The Python↔Rust mirroring pattern

Input and output types are *defined in Python* and *re-parsed in Rust* — there are no `#[pyclass]`es. Rust reads the Python objects' **private attributes** (`_name`, `_min`, `_init`, `_max`, `_log`, `_decimal_places`, `_values`, `_init_index`, …) via `FromPyObject` impls, and keeps a `PyObject` handle (`PySlider`/`PyCheckbox`/`PyRadio` newtypes) to write the user-visible `_value` back when the widget changes.

Consequences to keep in mind:

- Renaming or adding a private field on a Python input/output class silently breaks the Rust extractor. Both sides must change together.
- Type dispatch is by **class-name string** (`obj.get_type().name()? == "Slider"`), not `isinstance`, so class names are part of the contract. Outputs currently mix this with structural duck typing (`Plot` is detected via `hasattr("xs")`, the rest by name) — a known inconsistency flagged in `src/outputs.rs`.
- `LevelResult`'s `Nested` case is also duck-typed: anything callable with an `inputs` attribute is treated as a `ReactiveBase`.

Adding a new **input** type touches: `_types_inputs.py` (class + `Input` union), `__init__.py` (re-export), `src/inputs/<name>.rs` (the `<Name>Spec`/`<Name>Binding`/`parse_<name>` split — see below), the `InputSpec`/`InputBinding`/`InputValue` enums in `src/inputs/mod.rs`, a render function in `src/ui/inputs.rs`, and the matches in `src/ui/reactive_view.rs`.

Adding a new **output** type touches: `_types_outputs.py` (class + `Output` union), `__init__.py`, the struct + `parse_output` in `src/outputs.rs`, a `src/ui/<name>.rs`, the `PreparedOutput` enum + its match arms in `src/ui/reactive_view.rs`.

## The worker thread, the scheduler, and the UI loop

No `Py` object ever reaches the UI thread. Parsing an input produces two halves: an `InputSpec` (plain Rust — name, min/max/init, …) sent to the UI, and an `InputBinding` (holds the `PyObject` handle used to write `_value`) that stays on the worker. `InputValue` is what the UI sends back to the worker to write.

A single `picoapp-worker` thread (`src/worker.rs`) owns a `Registry` of `LevelId -> (bindings, callback)` and processes `Job::Run`/`Job::Drop` messages sent over an `mpsc` channel from `WorkerHandle`. It's the only thread that ever calls into Python — it writes each input's `_value`, calls the callback, and parses the return via `parse_level_result` (`src/outputs.rs`), which never surfaces a raw `PyErr`: a raised exception becomes `LevelResult::Error(message + traceback)` (also printed to stderr) and a callback that returns another `ReactiveBase` becomes `LevelResult::Nested`, registering the new level's bindings before anything crosses to the UI as `UiLevelResult`.

`ReactiveView` (`src/ui/reactive_view.rs`) is one gpui `Entity` per reactive level — the root and each nested `ReactiveBase` — each holding a `RunScheduler` (`src/ui/run_scheduler.rs`, pure Rust, no gpui/pyo3 types). An input change goes through `RunScheduler::on_change`: if nothing is in flight it dispatches a `Job::Run` immediately; if a job is already running, the change is coalesced (latest value wins, no queue) and redispatched from `RunScheduler::on_result` once the in-flight job's reply arrives. The very first job for a level (root or nested) goes through `RunScheduler::start()` + `dispatch()` the same way, so every Python call — including the first — runs on the worker thread, not on the thread that constructed the view.

A `Nested` reply replaces `self.child: Option<Entity<ReactiveView>>`; the previous child's own `Context::on_release` callback (registered in `new_child`) drops its worker registration when its `Entity` is released, which recurses naturally for grandchildren since dropping a parent's `child` field drops that child's `Entity`, and so on. A reply that arrives after its view was released is simply dropped, except a `Nested` reply also gets an explicit `Job::Drop` for its (already-worker-registered) new level, since no `ReactiveView` will ever exist to release it otherwise.

A dispatch shows a busy dim after ~150ms (so fast callbacks don't flicker) and displays `LevelResult::Error`'s message in the content area without discarding the previous outputs. `Output::Image`/`Output::Audio` are built once into `PreparedOutput::Image`/`PreparedOutput::Audio` when a result arrives (`ReactiveView::set_outputs`), not on every render pass — gpui re-renders a view on far more than its own state changes, so rebuilding a `RenderImage` or `AudioPlayer` entity inside `Render::render` would re-upload the sprite atlas or restart playback on every unrelated redraw.

Styling: `src/ui/style.rs` is the design system — the dark palette (applied to gpui-kit's theme so its components follow it), spacing/size tokens (`GUTTER`, `CARD_RADIUS`, ...) and semantic helpers (`card`, `error_card`, `muted_text`, `plot_colors`). Components use those instead of composing raw colors/paddings; the plot panel is the one fixed-color exception (white plots on the dark UI, as before). Window frame: the app asks the compositor/OS for its native decorations (`WindowDecorations::Server`, plain native title bar options); `src/ui/header_bar.rs` draws an Adwaita-style header bar only as the fallback when gpui reports client-side decorations (GNOME on Wayland refuses server-side ones by design). gpui-kit needs `application().with_assets(gpui_kit::assets::Assets)` for its icons (checkmarks, window controls).

GIL handling: the worker thread acquires the GIL per job (`Python::with_gil`) and releases it between jobs; `run_ui` releases the GIL with `py.allow_threads` for the whole gpui event loop, which never touches Python directly.

UI stack: [gpui-kit](https://gpui-kit.com/) (`gpui-kit = "=0.6.6"`, pinned exactly — each release pins a different `gpui` snapshot) on gpui/wgpu; plots (`src/ui/line_plot.rs`, `src/ui/matrix_plot.rs`) are custom `gpui_component::plot::Plot` implementations built on `ScaleLinear`/`Line`/`PlotAxis`/`Grid`, not plotters; audio playback via `rodio`; images go through gpui's `RenderImage` (swizzled to BGRA — see `src/outputs.rs`'s `rgba_to_bgra` and its ledger note in `docs/superpowers/specs/2026-09-22-gpui-migration-design.md`).

## Known gaps

- Despite the zero-copy goal, output data is currently *copied* across the boundary via `extract::<Vec<f64>>()`. The `rust-numpy`/buffer-protocol path is a TODO in `src/outputs.rs`.
- `pyproject.toml` duplicates `requirements.in` because maturin does not support dynamic dependencies ([PyO3/maturin#1537](https://github.com/PyO3/maturin/issues/1537)).

# Conventions

- `python/picoapp/__init__.py` uses explicit `from .x import Y as Y` re-exports — required for the typed public API; keep that form.
- `python/picoapp/_picoapp.pyi` is the hand-written stub for the Rust module; update it whenever the Rust signatures change.
- mypy runs with `disallow_untyped_defs` (relaxed for `tests/`); formatting is black + isort at their defaults, flake8 at max-line-length 120.
- `notes.md` collects research on maturin, PyO3 GIL/callback patterns, and manylinux cross-compilation — check it before re-investigating packaging problems.
