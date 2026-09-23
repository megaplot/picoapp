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

`.github/workflows/deploy.yml` is generated — regenerate with `./scripts/regenerate_maturin_ci`, don't hand-edit.

## AI Workflow Rules

- **Language**: Keep communication concise and self-critical. Avoid overusing figurative/generic/vague terms that require translation into something concrete specific. Avoid following the LLM langue entry collapse. Avoid overusing terms: once X lands, fold X into, load-bearing
- **Ask before essential decisions.** When an issue could indicate an architectural/design flaw, describe options with pros/cons instead of hacking around it.
- **Git rules**: Never `git push` by yourself or `git commit` on main. Commits on `main` are left for the human companion to give a chance of a last review. Committing on feature branches is fine.

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
- `CallbackReturn` is also duck-typed: anything callable with an `inputs` attribute is treated as a `ReactiveBase`.

Adding a new **input** type touches: `_types_inputs.py` (class + `Input` union), `__init__.py` (re-export), `src/inputs/<name>.rs`, the `Input` enum + `FromPyObject` in `src/inputs/mod.rs`, a `src/widgets/ui_<name>.rs`, and the match in `input_widget`.

Adding a new **output** type touches: `_types_outputs.py` (class + `Output` union), `__init__.py`, the structs + `parse_output` in `src/outputs.rs`, a `src/widgets/ui_<name>.rs`, and the match in `outputs_widget`.

## Reactivity and the UI loop

`reactive_input_output_widget` (`src/widgets/ui_reactive.rs`) is the heart of the app: a fixed 300px input sidebar plus a content area driven by a `Dynamic<Option<CallbackReturn>>`.

Each input widget registers a `for_each` on its cushy `Dynamic` that, under the GIL: writes the new value into the Python object → calls the Python callback → parses the return → sets `cb_return_dynamic`, which re-renders the content. If the callback returns another `ReactiveBase` instead of `Outputs`, the switcher recursively builds a *nested* sidebar+content (see `examples/example_nested_func.py` and `example_nested_inheritance.py`).

`CallbackReturn::eq` deliberately always returns `false` so every callback invocation triggers a UI update.

GIL handling: `run_ui` releases the GIL with `py.allow_threads` for the whole event loop and re-acquires it via `Python::with_gil` inside widget callbacks.

UI stack: [cushy](https://github.com/khonsulabs/cushy) (pinned to a git rev) on kludgine/wgpu; plots are rendered with `plotters` into a cushy `Canvas`; audio playback via `rodio`; images go straight to a wgpu `Texture`.

## Known gaps

- Despite the zero-copy goal, output data is currently *copied* across the boundary via `extract::<Vec<f64>>()`. The `rust-numpy`/buffer-protocol path is a TODO in `src/outputs.rs`.
- `pyproject.toml` duplicates `requirements.in` because maturin does not support dynamic dependencies ([PyO3/maturin#1537](https://github.com/PyO3/maturin/issues/1537)).

# Conventions

- `python/picoapp/__init__.py` uses explicit `from .x import Y as Y` re-exports — required for the typed public API; keep that form.
- `python/picoapp/_picoapp.pyi` is the hand-written stub for the Rust module; update it whenever the Rust signatures change.
- mypy runs with `disallow_untyped_defs` (relaxed for `tests/`); formatting is black + isort at their defaults, flake8 at max-line-length 120.
- `notes.md` collects research on maturin, PyO3 GIL/callback patterns, and manylinux cross-compilation — check it before re-investigating packaging problems.
