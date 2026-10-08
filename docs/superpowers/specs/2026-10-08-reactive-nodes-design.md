# Reactive nodes programming model: design

Replaces picoapp's nested `Inputs`/`Outputs`/`Reactive` model with:

- hoisted inputs,
- a `view` function that arranges inputs and outputs into a layout,
- fine-grained `@pa.memoize` nodes with automatic dependency tracking.

This is a breaking change; the old API is removed.

Background and rationale (read these for the "why"; this spec only states the "what"):

- `ai/tasks/rethink_programming_model.md`: the task and the use case.
- `ai/prior_art_programming_model.md`: survey of Streamlit, Dash, Panel, Shiny, marimo, Gradio,
  immediate-mode GUIs, React/signals.
- `ai/flat_view_model.md`: the first proposal (one flat callback). Superseded, but its analysis of
  `has_changed` and latency is the rationale for several decisions here.
- `ai/alternative_reactive_model.md`: the chosen model, its decisions, ported examples, and the
  type-checking experiment (`ai/alternative_reactive_model_typing_sketch.md`).

## Goals

1. The dataset × algorithm use case from the task works with natural UX:
   - one column per selector, with the selector on top and its parameters below;
   - nested submode parameters;
   - switching a selector back and forth keeps every parameter's value.
2. Layout is decoupled from reactivity: `view` can return any arrangement, depending on input
   values.
3. Recomputation is fine-grained by default. Changing an input re-runs only the nodes that read it.
   Outputs that didn't change keep their UI state (audio keeps playing).
4. Newly revealed inputs appear without waiting for unrelated heavy computation.
5. Type-safe public API without `Any`, checked by mypy (repo CI) and verified with pyright.

## Non-goals (deferred, see `ai/alternative_reactive_model.md`)

- `Button`, `on_click`/`on_change` handlers and input writes (presets). The design reserves them:
  only handlers will be allowed to write inputs.
- `Tabs`, titled groups, explicit sizing (`weights=`), responsive wrapping.
- Progress reporting and cooperative cancellation ("Deferred follow-up ideas").
- A manual refresh control.
- `eq=` on nodes to suppress spurious downstream recomputes.
- Zero-copy output data (the existing `rust-numpy` TODO is unaffected).

## Public Python API

```py
import picoapp as pa

# Inputs (unchanged constructors). `.value` is read-only for user code; reads are tracked.
pa.Slider(name, min, init, max, log=False, decimal_places=None)    # Input[float]
pa.IntSlider(name, min, init, max)                                  # Input[int]
pa.Checkbox(name, init=False)                                       # Input[bool]
pa.Radio(name, values, init=None)                                   # Input[T]

# Outputs (unchanged constructors).
pa.Plot(...), pa.MatrixPlot(...), pa.Image(...), pa.Audio(...)

# Layout.
pa.Row(*children: pa.ElementLike)
pa.Column(*children: pa.ElementLike)

# Nodes.
@pa.memoize
def f() -> T: ...                    # f: pa.Memoized[T]; also usable as pa.memoize(fn)
f()                                  # value of the node (computed or cached), tracked
f.map(g)                             # pa.Memoized[U] for g: Callable[[T], U]

pa.run(view)                         # view: Callable[[], pa.Element]

pa.CycleError                        # raised when a node (transitively) calls itself
```

**Type hierarchy** (all public, re-exported from `__init__.py` in the `X as X` form):

- `Element`: the base of everything placeable in a view.
  - `Output(Element)`: `Plot`, `MatrixPlot`, `Image`, `Audio`.
  - `InputBase(Element)`.
    - `Input(InputBase, Generic[T_co])` with `value: T_co`. Covariant, which mypy's list joins
      need (see the typing sketch).
  - `Layout(Element)`: `Row`, `Column`.
- `Memoized(Generic[T_co])`: callable, with `__call__() -> T_co` and `map`.
- `ElementLike = Element | Memoized[Element]`: what `Row`/`Column` accept. `view` returns the
  plain `Element`. Bare callables are not accepted: a lambda created inside `view` would be a new
  object every run.

`Output` and `Input` stop being union type aliases and become base classes. Existing user
annotations like `x: pa.Output` keep working.

`pa.run(view)` wraps `view` in a root node, i.e. `pa.memoize(view)`. A root returning a non-layout
element is treated as a single-child `Column`.

**Removed:** `Inputs`, `Outputs`, `Reactive`, `ReactiveBase`, `Callback`, and the
`pa.run(reactive)` signature.

## Semantics

### Identity

- An input's or node's identity is the Python object. Each instance gets a process-unique integer
  `_id` from a global counter at construction. This is used across FFI instead of `id()`, which
  CPython may reuse after an object is freed.
- Inputs and nodes created inside a node function are new objects on every run. That is allowed,
  but they get fresh state. The docs and examples teach "hoist inputs and nodes".
- An input is immutable except for its value. Its name, bounds and values are fixed; a different
  range means a new input.

### Dependency tracking

- While a node evaluates, it is on top of a `contextvars`-based evaluation stack. Every
  `Input.value` read and every `Memoized.__call__` on that stack records a dependency
  `(source, version_seen)` in read order. The recorded list replaces the node's previous
  dependencies after the run, so dependencies can differ per `if` branch.
- Reads outside any evaluation (e.g. at module level) are untracked and allowed.
- Every input has a `version`, bumped when the worker writes a *different* value. Every node has a
  `version`, bumped on every successful or failing (re-)evaluation. Equal results still bump, so
  there is no equality check.

### Staleness and evaluation

- A node is **stale** if it has never been evaluated, or if one of its recorded dependencies has
  changed. For an input, changed means its version advanced. For a node, it means the node was
  brought up to date and its version advanced.
- **Pull with ordered short-circuit.** To bring node `N` up to date, walk its recorded dependencies
  in read order:
  - for a node dependency, first bring that node up to date recursively;
  - at the first changed dependency, stop walking and re-evaluate `N`.

  This evaluates each node at most once per change (no diamond glitch). It also never evaluates a
  dependency that only the old branch needed.
- **Results are cached** on the node object for as long as it is alive. This holds for hidden
  nodes too (the memory policy).
- **Errors.** If a node raises, the exception is cached as its result and its version bumps.
  Calling it from another node re-raises the cached exception, so dependents fail too, with the
  original traceback, until the failing node's dependencies change.
- **Cycles.** A node that (transitively) calls itself raises `pa.CycleError`, detected via the
  evaluation stack.

### Slots and the view tree

- A **slot** is a `Memoized` placed in a layout: the root, a `Memoized` child of a `Row`/`Column`,
  or a `Memoized` inside a fragment's result.
- **Visible slots** are those reachable from the root through the *latest results* of slot nodes.
- A slot whose result is a `Layout` is a **fragment**. A slot whose result is an `Output` or an
  `Input` is a leaf. Any other result type produces a slot error ("node `f` returned `int`, expected
  an `Element`").
- **Duplicates.** The same input object or node object appearing twice in the visible tree is an
  error, shown in the slot that introduces the second occurrence.
- **Value nodes** (results that aren't elements, like `dataset`) are never slots themselves. They
  are evaluated when a slot node pulls them.

### The evaluation round

A stateful `Engine` (pure Python, `python/picoapp/_engine.py`) runs on the worker thread:

```py
class Engine:
    def __init__(self, root: Memoized[Element]) -> None: ...
    def set_values(self, changes: Sequence[tuple[int, object]]) -> None: ...
    def stale_visible_slots(self) -> list[int]: ...
    def step(self) -> SlotResult | None: ...
```

- **`set_values`** writes `_value` and bumps `version` for changed values. Input ids are resolved
  through a `weakref.WeakValueDictionary`. Unknown ids (input already collected) are ignored.
- **`stale_visible_slots`** returns the ids of visible slots that are stale. The UI uses it for the
  busy dim.
- **`step`** brings up to date the first stale visible slot in priority order and returns its
  `SlotResult`. It returns `None` when nothing visible is stale. Pulled value nodes are evaluated
  inside the same step. Priority order:
  1. the root;
  2. fragments, top-down in tree order (so revealed inputs appear first);
  3. leaves in tree order.
- **`SlotResult`** is `(node_id, version, content)`, where `content` is the element tree or output,
  or an error string (message + traceback).

**Worker loop (Rust).**

1. Block until a UI message arrives.
2. Drain all pending messages, so the latest value per input wins (coalescing).
3. Call `set_values`, then send the UI the stale slot set.
4. Loop: call `step`, parse the result into Rust types, send it, then drain new messages without
   blocking. If there are new messages, apply them as in steps 2–3 and continue. The next `step`
   picks the highest-priority stale slot of the new state; this is "option A, re-plan". When
   `step` returns `None`, go back to 1.

The first round starts without any UI message. Python is only ever called from this thread, and
the GIL is held per call, as today.

**Consistency.** Inputs are written only between steps, so every node evaluation sees one
consistent snapshot. Different slots may briefly show results from different snapshots. Stale and
in-flight slots are dimmed, which keeps this honest.

## Python ↔ Rust contract

The pattern stays the same: Python classes, re-parsed in Rust via `FromPyObject`, dispatch by class
name, private attributes as the contract.

**Python → Rust** (`SlotResult.content`, parsed on the worker thread):

```rust
enum ViewTree {
    Row(Vec<ViewTree>),
    Column(Vec<ViewTree>),
    Input { id: InputId, spec: InputSpec },   // spec carries the *current* value
    Output(Output),                           // existing Output enum
    Slot(NodeId),                             // a Memoized child
}
enum SlotContent { Tree(ViewTree), Error(String) }
```

- A leaf slot's content is a one-element tree (`ViewTree::Output` or `ViewTree::Input`).
- `InputSpec` gains the current value (today it carries `init`). This is needed because a re-shown
  input is re-created from the spec.
- `InputBinding`/`PySlider`/... handles are no longer needed: the engine writes values by id, so no
  `Py` object reaches the UI and the worker keeps no per-level bindings.

**Rust → Python:** `Engine.set_values([(input_id, value), ...])`. `InputValue` maps to a Python
`float`/`int`/`bool`, and to an index for `Radio`; `Engine` maps the index to `values[index]`.

**FFI surface:**

- `_picoapp.run(engine: Engine)`; `pa.run(view)` constructs the `Engine`.
- The test hooks `_parse_input` and `_run_worker_job` are replaced by `_parse_slot_content(obj) ->
  str`: a summary of the parsed `SlotContent`, for pytest.
- `_picoapp.pyi` is updated accordingly.

## UI (Rust/gpui)

One `AppView` entity replaces the per-level `ReactiveView` entities. Its state:

- **`slots: HashMap<NodeId, SlotState>`**, where `SlotState` holds the latest content, its
  version, `busy_since: Option<Instant>`, and the error (if any) shown over the last good content.
  - `PreparedOutput` (image upload, `AudioPlayer` entity, plot data) is built when a slot result
    arrives, as today. It is kept until the slot's next result or until the slot becomes invisible.
- **`widgets: HashMap<InputId, InputWidgetState>`**: one gpui widget state per *visible* input.
  - It is created from the spec's current value when the input appears.
  - If the input is already visible, it is kept and the spec value is ignored. The UI is the
    source of truth while a user drags.
  - It is dropped when the input becomes invisible. Its value lives in Python, so it comes back
    from the next spec.
- **After every received result**, recompute the visible slot/input sets from the root. Drop
  invisible `SlotState`s (dropping `RenderImage`s through the existing `pending_image_drops` path)
  and invisible widgets.
- **Rendering** is recursive from the root slot:
  - `Row` → `flex_row`, `Column` → `flex_col`.
  - *compact* (inputs, `Audio`): natural size. A `Row` child containing only compact elements gets
    `SIDEBAR_WIDTH` and `flex_shrink_0`.
  - *fill* (`Plot`, `MatrixPlot`, `Image`): `flex_1`, with the 120px minimum height inside columns.
    A `Row` child containing any fill element gets `flex_1` + `min_w_0`.
  - Overflowing columns scroll vertically.
  - Inputs keep today's card rendering (`style.rs` helpers).
- **Busy dim and errors.**
  - The busy dim is per slot, after the existing ~150ms delay, for slots in the last stale set.
  - A slot error renders `error_card` above the slot's last good content.
  - A root error leaves the last good view in place, with the error card on top.
- `run_scheduler.rs` is deleted. Its coalescing moves into the worker loop, and its busy timing into
  `SlotState`.

## Use case example

`examples/example_dataset_algorithm.py` is new and implements the task's use case. It follows the
sketch in `ai/alternative_reactive_model.md`, "Sketch", using synthetic datasets and simple
numpy-only algorithms, so it needs no new dependencies. Every existing example is ported:

- `example_nested_func.py` → `example_dynamic_inputs.py` (master and coefficient sliders in one
  column);
- `example_nested_inheritance.py` → `example_class_based.py`;
- the others keep their names; `example_filter.py` and `example_freq.py` use nodes as in the ported
  versions in the alternative doc.

## Testing

- **Engine unit tests** (`tests/test_engine.py`, pure Python, no UI):
  - tracking records reads in order, and dependencies change with branches;
  - only nodes that read a changed input re-run;
  - a diamond evaluates its sink once;
  - the ordered short-circuit does not evaluate an old-branch dependency;
  - hidden nodes are not evaluated, keep their cached result, and recompute on re-show only if
    stale;
  - errors are cached, propagate to dependents, and clear when dependencies change;
  - cycle detection;
  - step priority (root → fragments → leaves);
  - re-planning after `set_values` mid-round;
  - `stale_visible_slots`;
  - duplicates are rejected;
  - a collected input id is ignored by `set_values`.
- **Parsing tests** (`tests/test_slot_parsing.py`, through `_parse_slot_content`): every
  element/layout/slot combination, a non-element node result, a duplicate input.
- **Typing tests** (`tests/test_typing.py`, checked by `mypy .` in CI): the positive cases from
  the typing sketch, plus the negative cases as `# type: ignore[arg-type]` lines. The repo's
  `warn_unused_ignores = True` fails CI if an expected error disappears. pyright is verified once
  manually; it is not added to CI.
- **Rust unit tests:** the visible-set computation and slot/widget pruning (pure functions over
  `ViewTree`), and the worker loop's drain/coalesce logic, in the style of today's
  `run_scheduler` tests.
- **UI QA** (`scripts/qa`, X11 path) on `example_dataset_algorithm.py`:
  - screenshot of the two-column layout;
  - drag a dataset parameter, switch the dataset selector away and back, and check that the
    parameter kept its value;
  - toggle the submode and check that the nested parameter appears;
  - drag an algorithm parameter and check that only the plot re-renders.
  - Plus a smoke screenshot of every ported example.
- `./scripts/check_all` green.

## Release

User-facing breaking change: bump `Cargo.toml` to **0.4.0** (decided: pre-1.0, breaking changes
bump the minor version) with a new `## 0.4.0` `CHANGELOG.md` heading.

## Risks

- **Tracking "magic":** untracked state (globals, files, RNG) silently doesn't invalidate. Mitigated
  by documentation; a manual refresh is deferred.
- **Size of the Rust change:** the worker protocol, `ReactiveView` → `AppView` and slot rendering
  are all rewritten. Mitigated by keeping parsing/output rendering code (`outputs.rs`,
  `line_plot.rs`, `inputs.rs` render functions) and by the pure-function unit tests above.
- **Wheel size:** no new dependencies, so no expected movement. `scripts/measure_wheel_size` runs
  at the end anyway, per the AI workflow rules.
