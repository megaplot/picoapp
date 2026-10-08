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
./scripts/maturin_develop      # rebuild the Rust extension into the venv (REQUIRED after any src/*.rs change)
./scripts/check_all            # everything CI checks, in the same order — must be green after every change
python examples/example_1.py   # run an example app (needs a GPU/display)
```

`./scripts/maturin_develop && python examples/example_X.py` is the main iteration loop for anything touching Rust.

Every command above is a script, not a hand-typed invocation, and every CI step (`checks.yaml`) calls the same script a human/agent would run locally — this is deliberate: `scripts/*` is the one source of truth for "how do I do X here", and CLAUDE.md, CI and a human's terminal should never drift apart on the same task. `scripts/check_all` delegates to one script per CI step: `check_rust_format` (`cargo fmt --check`), `check_rust_lint` (`cargo check` with warnings denied), `check_rust_test` (`cargo test`), `check_python_types` (`mypy .`), `check_python_lint` (`ruff check .`), `check_python_format` (`ruff format --check .`), `check_python_test` (`pytest`), `check_integration_minimal_install` (installs into a throwaway venv with no dev deps). Run a single one directly while iterating on just that language/tool; run `check_all` before considering a change done.

`scripts/build_wheel [OUT_DIR=dist]` builds a release wheel (host platform only, current Python only) and prints its path; `scripts/measure_wheel_size` builds one via `build_wheel` into a scratch dir, prints its size, and appends a row to `ai/wheel_size_history.md`. See the "Wheel size" AI workflow rule below for when to run the latter. One deliberate exception to the "CI calls the same script" rule: `deploy.yml`'s actual PyPI wheels are built by `PyO3/maturin-action`, which runs `maturin build` inside a manylinux Docker container per target (needed for glibc compatibility and cross-compilation) — a shell script on the runner can't reproduce that, so `build_wheel` is the closest local equivalent (a single-platform, single-interpreter build), not a literal stand-in for the release build.

UI QA without a desktop session runs headlessly: an X11 path (Xvfb; screenshots *and* clicks/drags; the default) and a Wayland path (private GNOME Shell; screenshots only, for window decorations). `scripts/qa/README.md` says which to pick and how. Use it for any UI change instead of "the window opens without crashing" checks, which cannot catch interaction or rendering bugs.

Also: picoapp must print nothing by default (its stdout/stderr belong to the user's callback); `RUST_LOG=info` opts into UI-stack logs.

`.github/workflows/deploy.yml` started from `./scripts/regenerate_maturin_ci` (which wraps `maturin generate-ci github`) but has hand edits since (target matrix trimmed, manylinux version, explicit interpreter list, Linux system dependencies, the release job), each explained by a comment at the point of deviation — regenerating it from scratch will lose them; diff before overwriting, and comment any new deviation the same way.

Releasing: bumping `Cargo.toml`'s version (with its `CHANGELOG.md` entry, see below) is the only manual step. The `release` job in `deploy.yml` runs `scripts/release_if_untagged` on every run. On a push to `main` whose version has no git tag on origin yet, it publishes that run's wheels to PyPI and then creates and pushes the tag. Everywhere else it's a dry run or a no-op. So **merging a version bump to `main` releases it**. Locally, `scripts/release_if_untagged DIST_FILE...` is a safe dry run (`--push` refuses to run outside GitHub Actions on `main`).

## AI Workflow Rules

- **Language**: Keep communication concise and self-critical. Avoid overusing figurative/generic/vague terms that require translation into something concrete specific. Avoid following the LLM langue entry collapse. Avoid overusing terms: once X lands, fold X into, load-bearing
- **Ask before essential decisions.** When an issue could indicate an architectural/design flaw, describe options with pros/cons instead of hacking around it.
- **Checks must be green.** Run `./scripts/check_all` after each code change (or the matching `scripts/check_*` script for a smaller, faster check while iterating) and don't consider the change done until it passes. This is the same suite CI runs; don't rely on CI to catch what a local run would have caught.
- **Code reviews**: `ai/code_reviews.md` lists the review patterns agents are prone to (bad code placement, field/argument ordering, dropped comments, unnecessary renames/rewrites that bloat the diff against `main`). Follow it when writing code, and give it to any reviewer.
- **Deferred cleanups**: `ai/deferred_cleanups.md` tracks minor issues (found during review or otherwise) that were consciously left unfixed — not forgotten, just not worth blocking the current change on. Add an entry instead of silently dropping a minor finding; delete an entry once it's fixed or no longer applies.
- **Wheel size**: after a non-trivial Rust-side change (a dependency bump, a new dependency, anything likely to move the compiled extension's size) run `scripts/measure_wheel_size` and report the before/after in your summary — don't wait to be asked. It appends a row to `ai/wheel_size_history.md` (unstaged; leave it for the human to review and commit like any other change, per the git rules below). Not needed for every commit, just changes where the size could plausibly have moved. See `ai/wheel_size.md` for the cushy-vs-gpui baseline and why `strip = true` isn't a free win. When working on a feature branch, make sure to only introduce a single line from that branch (final size before the merge).
- **Changelog & versioning**: `CHANGELOG.md` tracks user-facing changes, one `## X.Y.Z` heading per version (matching `Cargo.toml`) with one-line bullets underneath. For a change that is user-facing, add (or extend) a bullet and bump `Cargo.toml`'s `version` in the same change (merging it to `main` then releases it, see "Releasing" above). If the top version is already released (a git tag `X.Y.Z` exists on origin — CI creates tags, so check `git ls-remote --tags origin` or fetch first, local tags may be stale), that means a new heading with a bumped version; if it isn't, extend the existing heading and only re-bump if the change warrants a bigger step (e.g. patch → minor). Bump semantic-release style: patch for a fix or maintenance change with no new behavior, minor for a new feature or anything substantial enough to call out (this project bumps minor liberally pre-1.0 — see past entries), major reserved for a breaking change. If it's unclear which, ask (per "Ask before essential decisions" above) rather than guess.
  The test for "user-facing" is **"could this possibly affect the released app artifact?"**, not "is this a code change" — a `CLAUDE.md` edit, a cosmetic refactor, a pure-docs change, or a CI change that only touches comments/documentation (no step behavior changed) gets neither a changelog entry nor a version bump. A change that *does* affect what ships gets both — e.g. a manylinux/target-matrix change, a build flag, or a bump of a dependency that ends up in the wheel (`Cargo.toml`, or `pyproject.toml`'s runtime `dependencies`). Bumping `requirements.txt` doesn't count: it only pins dev tooling and examples. Neither does a pure internal refactor without behavior change. This needs judgment per change, not a fixed include/exclude list — when genuinely unsure whether something could affect the artifact, ask rather than guess either way.
  On a feature branch, think in terms of the *effective diff against `main`*: if the branch introduces and then fixes its own bug before merging, that fix never happened from `main`'s perspective and gets no entry. If a later commit on the same branch refines an already-added entry, edit that entry in place (broaden or correct its wording) instead of appending a second, near-duplicate one. Before calling a branch done, read back everything it added to `CHANGELOG.md` as a whole, as if reviewing the final diff against `main` in one pass, not commit-by-commit — the same discipline `ai/code_reviews.md` asks for the rest of the diff.
- **Git rules**: Never `git push` by yourself or `git commit` on main. Commits on `main` are left for the human companion to give a chance of a last review. Committing on feature branches is fine. Never merge a feature branch into `main` yourself, locally or otherwise — the human companion always reviews a feature branch before it merges, going through GitHub PR's. Almost all AI work should happen on a feature branch, which typically means to create a feature branch unless an appropriate feature branch was already created. The human companion takes care of creating a PR and merging it to main. This also means: when using the `superpowers` finishing-a-development-branch skill (or any equivalent finalize/wrap-up step), skip its menu and its merge/PR actions in this repo — just report the branch is done and stop; don't offer to merge or push.

# Architecture

Hybrid Rust/Python package built with maturin/PyO3:

- `python/picoapp/` — the public Python API (pure Python, typed, `py.typed`).
- `src/` — the Rust extension module, exposed as `picoapp._picoapp` (a `cdylib`).
- The entire FFI surface is one function: `_picoapp.run(engine)` (`src/py_module.rs`). `pa.run(view)` (`_core.py`) wraps `view` in a root `Memoized` node and passes an `Engine` (`_engine.py`). `_parse_slot_content` exists solely so the parsing layer is unit-testable from Python.

## The reactive core (pure Python)

`_tracking.py` keeps a `contextvars` stack of the nodes being evaluated; every `Input.value` read and every `Memoized.__call__` records `(source, version_seen)` in the top frame, in read order. A node's recorded dependencies are replaced after each run, so they follow its `if` branches. Inputs bump their `_version` only when the UI writes a different value (`_write_ui_value`), which also bumps a global epoch: a node checked in the current epoch is known to be fresh without walking its dependencies again.

`_memoize.py`: `Memoized._ensure_fresh` is pull with ordered short-circuit — walk the dependencies in read order, bring node dependencies up to date first, and re-evaluate at the first changed one (no diamond glitch, no evaluation of dependencies only an old branch read). Results and exceptions are cached on the node while it is alive; a cached exception is re-raised with its original traceback. A node calling itself (transitively) raises `CycleError`.

`_engine.py`: a *slot* is a `Memoized` placed in a layout (or the root). Visibility goes from the root through each slot's last *shown* (good) content — an error keeps the previous content on screen, as in the UI. `step()` brings the highest-priority visible slot up to date (root → fragments → leaves, each in tree order) and returns its content or an error string; a slot that becomes visible again is re-sent from the cache. Duplicate inputs/nodes in the visible tree are a slot error. `reject()` restores a slot's previous content when Rust can't parse the new one.

## The Python↔Rust mirroring pattern

Input and output types are *defined in Python* and *re-parsed in Rust* — there are no `#[pyclass]`es. Rust reads the Python objects' **private attributes** (`_id`, `_name`, `_min`, `_value`, `_max`, `_log`, `_decimal_places`, `_values`, `_index`, a layout's `_children`, …) via `FromPyObject` impls. It keeps no Python handle for inputs: the engine writes values by id (`Engine.set_values`).

Consequences to keep in mind:

- Renaming or adding a private field on a Python input/output class silently breaks the Rust extractor. Both sides must change together.
- Type dispatch is by **class-name string** (`obj.get_type().name()? == "Slider"`), not `isinstance`, so class names are part of the contract. Outputs currently mix this with structural duck typing (`Plot` is detected via `hasattr("xs")`, the rest by name) — a known inconsistency flagged in `src/outputs.rs`.

Adding a new **input** type touches: `_types_inputs.py` (a class deriving `Input[T]`, with `_decode`), `__init__.py` (re-export), `src/inputs/<name>.rs` (`FromPyObject` for a `<Name>Spec`), the `InputSpec`/`InputValue` enums and `InputSpec::value` in `src/inputs/mod.rs`, the class-name list in `ViewTree`'s `FromPyObject` and `ViewTree::summary` (`src/view_tree.rs`), a render function in `src/ui/inputs.rs`, and the matches in `src/ui/app_view.rs`.

Adding a new **output** type touches: `_types_outputs.py` (a class deriving `Output`), `__init__.py`, the struct + `parse_output` in `src/outputs.rs`, `ViewTree::summary` in `src/view_tree.rs`, a `src/ui/<name>.rs`, the `PreparedOutput` enum + its match arms and `is_fill` in `src/ui/app_view.rs`.

## The worker thread and the UI loop

No `Py` object ever reaches the UI thread. The worker parses each slot's content into a `ViewTree` (`src/view_tree.rs`: `Row`/`Column`/`Input { id, spec }`/`Output`/`Slot(NodeId)`); an `InputSpec` carries the input's current value. `InputValue` is what the UI sends back, as an `InputChange { input, value }`.

A single `picoapp-worker` thread (`src/worker.rs`) owns the `Engine` and is the only thread that ever calls into Python. It runs *rounds*: apply the coalesced input changes (`Engine.set_values`, latest value per input wins), send the stale visible slots (`WorkerMessage::Stale`), and if there are any, run one `Engine.step` and send its `WorkerMessage::Result`; then drain new changes without blocking and repeat, so a change arriving mid-round re-plans before the next step. The first round starts without any change. A content Rust can't parse becomes `SlotContent::Error(message + traceback)` after calling `Engine.reject`; an exception from the engine itself becomes `WorkerMessage::EngineError`. Neither is ever printed to stderr (picoapp must print nothing by default, see above). Messages go to the UI over a `futures` unbounded channel.

`AppView` (`src/ui/app_view.rs`) is the single gpui view. It keeps `slots: HashMap<NodeId, SlotState>` (last good content, error, busy-since) and one `InputWidgetState` per visible input. After every result it recomputes the visible set from the root (`view_tree::visible`, the same walk as the engine's) and drops the state of invisible slots and inputs; a newly visible input's widget is created from its spec's current value, while a visible one keeps its widget (the UI is the source of truth while dragging). `on_input_changed` sends a change only if the value actually changed (sliders emit events on every mouse move). Rendering is recursive: in a `Row`, compact children (inputs, audio) get `SIDEBAR_WIDTH` and fill children (plots, images) share the rest; in a `Column`, fill children share the height, but a plot or image never gets less than `MIN_FILL_HEIGHT`. Inputs and audio always keep their natural height (`natural_height`), also directly in a row. A column whose height its parent sets (the window, or a row) scrolls when its content is taller, with an always-visible scrollbar (`ScrollbarMode::Always` in `style.rs`); a column inside a column takes its content's height, so the outer one scrolls (`sized`, `is_fill`, `Placement::height_bounded`). `examples/example_many_elements.py` overflows on purpose, for checking layouts.

A slot reported stale is dimmed after ~150ms (so fast updates don't flicker); a slot error shows `error_card` above the slot's last good content. `Output::Image`/`Output::Audio` are built once into `PreparedOutput::Image`/`PreparedOutput::Audio` when a result arrives (`AppView::apply_result`), not on every render pass — gpui re-renders a view on far more than its own state changes, so rebuilding a `RenderImage` or `AudioPlayer` entity inside `Render::render` would re-upload the sprite atlas or restart playback on every unrelated redraw.

Styling: `src/ui/style.rs` is the design system — the dark palette (applied to gpui-kit's theme so its components follow it), spacing/size tokens (`GUTTER`, `CARD_RADIUS`, ...) and semantic helpers (`card`, `error_card`, `muted_text`, `plot_colors`). Components use those instead of composing raw colors/paddings; the plot panel is the one fixed-color exception (white plots on the dark UI, as before). Window frame: the app asks the compositor/OS for its native decorations (`WindowDecorations::Server`, plain native title bar options); `src/ui/header_bar.rs` draws an Adwaita-style header bar only as the fallback when gpui reports client-side decorations (GNOME on Wayland refuses server-side ones by design). gpui-kit needs `application().with_assets(gpui_kit::assets::Assets)` for its icons (checkmarks, window controls).

GIL handling: the worker thread acquires the GIL per engine call (`Python::attach`) and releases it in between; `run_ui` releases the GIL with `py.detach` for the whole gpui event loop, which never touches Python directly.

UI stack: [gpui-kit](https://gpui-kit.com/) (`gpui-kit = "=0.6.6"`, pinned exactly — each release pins a different `gpui` snapshot) on gpui/wgpu; plots (`src/ui/line_plot.rs`, `src/ui/matrix_plot.rs`) are custom `gpui_component::plot::Plot` implementations built on `ScaleLinear`/`Line`/`PlotAxis`/`Grid`, not plotters; audio playback via `rodio`; images go through gpui's `RenderImage` (swizzled to BGRA — see `src/outputs.rs`'s `rgba_to_bgra`, called from `parse_output`).

## Known gaps

- Despite the zero-copy goal, output data is currently *copied* across the boundary via `extract::<Vec<f64>>()`. The `rust-numpy`/buffer-protocol path is a TODO in `src/outputs.rs`.
- `pyproject.toml` duplicates `requirements.in` because maturin does not support dynamic dependencies ([PyO3/maturin#1537](https://github.com/PyO3/maturin/issues/1537)).
- `pa.run()` never returns on macOS: gpui's default `QuitMode` there is `Explicit`, so closing the last window doesn't end the app's `run` loop. Untested on Windows.
- The gpui migration roughly doubled the release wheel size vs. the cushy version (~9.5MB → ~17MB on Linux x86_64) — see `ai/wheel_size.md` for the measurement and why `strip = true` isn't a free fix.

# Conventions

- `python/picoapp/__init__.py` uses explicit `from .x import Y as Y` re-exports — required for the typed public API; keep that form.
- `python/picoapp/_picoapp.pyi` is the hand-written stub for the Rust module; update it whenever the Rust signatures change.
- mypy runs with `disallow_untyped_defs` (relaxed for `tests/`); linting and formatting are both `ruff` (`[tool.ruff]` in `pyproject.toml`), at its default rule set and line-length 88 — `known-first-party = ["picoapp"]` under `[tool.ruff.lint.isort]` keeps import grouping matching what isort used to infer automatically.
- `notes.md` collects research on maturin, PyO3 GIL/callback patterns, and manylinux cross-compilation — check it before re-investigating packaging problems.
- `Cargo.toml` dependencies: since picoapp is a binary/application (a `cdylib`), not a library other crates depend on, pin the exact version currently in use as the lower bound (e.g. `pyo3 = "0.29.2"`, not `"0.29"` or `"0.22"`) rather than leaving room for a caret range to silently pick up a newer minor version. Keep the `[dependencies]` (and `[dev-dependencies]`) list alphabetically sorted.
