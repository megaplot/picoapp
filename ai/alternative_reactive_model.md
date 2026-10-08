# Alternative: view callback + fine-grained reactive nodes

The competing direction to `ai/flat_view_model.md`. Inputs **and outputs** are long-lived objects.
A cheap `view` callback only arranges them into a layout. Each output is computed by its own
function, and picoapp re-runs it only when an input it depends on has changed. This is a Shiny-like
reactive graph, but with typed object handles instead of string IDs, and with a fully dynamic view.
Prior art: `ai/prior_art_programming_model.md`, sections 3 (Panel `pn.bind`), 4 (Shiny), 5 (marimo)
and 10 (Solid/signals).

## Motivation

- In the flat model, avoiding recomputation means the user ends up hoisting or caching *outputs*
  as well as inputs (`ai/flat_view_model.md`, "Observation: outputs need hoisting too"). The naive
  style (fresh outputs on every run) behaves badly: audio restarts, and large plots are re-prepared.
  Here, stable outputs are the default.
- `has_changed` is error-prone for conditional computations (`ai/flat_view_model.md`, "Pitfall").
  Per-node dependency tracking handles conditional computations correctly by construction.
- Because the view callback is cheap, newly revealed inputs appear immediately, and each output can
  be delivered and shown as soon as it is ready.

## Sketch

```py
ds_select = pa.Radio("Dataset", ["blobs", "moons"])
blobs_n = pa.IntSlider("n_blobs", 1, 3, 10)
moons_noise = pa.Slider("noise", 0.0, 0.1, 1.0)

algo_select = pa.Radio("Algorithm", ["kmeans", "dbscan"])
kmeans_k = pa.IntSlider("k", 1, 3, 10)
kmeans_init = pa.Radio("init", ["random", "k-means++", "custom"])   # the "submode"
custom_seed = pa.IntSlider("seed", 0, 0, 100)                       # only for init == "custom"
dbscan_eps = pa.Slider("eps", 0.01, 0.5, 2.0)

@pa.computed
def dataset() -> Dataset:              # reads only the active branch's inputs
    if ds_select.value == "blobs":
        return make_blobs(blobs_n.value)
    return make_moons(moons_noise.value)

@pa.computed
def clustering() -> pa.Plot:
    data = dataset()                   # depends on the dataset node
    if algo_select.value == "kmeans":
        seed = custom_seed.value if kmeans_init.value == "custom" else None
        labels = kmeans(data, kmeans_k.value, kmeans_init.value, seed)
    else:
        labels = dbscan(data, dbscan_eps.value)
    return pa.Plot(...)

def view() -> pa.View:                 # cheap: reads selectors, arranges objects
    ds_params = [blobs_n] if ds_select.value == "blobs" else [moons_noise]
    if algo_select.value == "kmeans":
        algo_params = [kmeans_k, kmeans_init]
        if kmeans_init.value == "custom":
            algo_params.append(custom_seed)
    else:
        algo_params = [dbscan_eps]
    return pa.Row(
        pa.Column(ds_select, *ds_params),
        pa.Column(algo_select, *algo_params),
        clustering,                    # a node reference, not a value
    )

pa.run(view)
```

Moving `dbscan_eps` re-runs `clustering` only. Neither `dataset` nor `view` re-runs. Switching
`kmeans_init` to `"custom"` re-runs `view` (it reads `kmeans_init`), so the seed slider appears
right away. `clustering` re-runs afterwards, because it reads `kmeans_init` too.

## The answers to the questions raised

### Does the graph have to be static? No.

- **Nodes** are ordinary Python objects (`Computed[T]`). They can be created anywhere at any time,
  including lazily, the same way hoisted inputs can.
- **Edges** are discovered at runtime (automatic dependency tracking). While a node evaluates,
  picoapp records every `input.value` read and every `node()` call. The recorded set replaces the
  node's previous dependencies, so dependencies may differ per evaluation, e.g. per `if` branch.
  This is how Shiny, Solid, MobX and Vue work.
- The alternative is explicit dependencies, e.g. `pa.computed(fn, slider_a, slider_b)`, where `fn`
  receives the values. The graph becomes static, and conditional dependencies have to be
  over-approximated (an inactive branch's slider would trigger recomputes). It is also not
  type-safe for variadic inputs: Python has no `Map[Input, T]` over a `TypeVarTuple`, so it would
  need N overloads. **Recommendation: automatic tracking.**

### "Unbounded" inputs and outputs: create them lazily

Evaluation is **pull-based**. A node is evaluated only if the current view references it, directly
or through other nodes. Nodes that are not referenced cost nothing beyond the Python object. For
unbounded families, create nodes on demand:

```py
@functools.cache
def coefficient_slider(i: int) -> pa.Slider:
    return pa.Slider(f"coefficient of x^{i}", -10.0, 0.5, 10.0)

@functools.cache
def residual_plot(i: int) -> pa.Computed[pa.Plot]:
    return pa.computed(lambda: pa.Plot(...coefficient_slider(i).value...))
```

Nothing has to be created eagerly for the worst case.

### Staleness and caching

- Every input has a version counter, incremented when its value changes.
- A node remembers the versions of the dependencies it read. It is stale if any of them advanced,
  or, for a computed dependency, if that node was recomputed.
- A node's last result is kept while the node object is alive, even while it is hidden.
  Consequences:
  - The `has_changed` pitfall disappears. A hidden output whose input changed stays stale and
    recomputes when it becomes visible again.
  - A/B switching can be instant without recomputation, *if* the user models the branches as
    separate nodes (e.g. one `clustering` node per algorithm). With a single node that reads the
    selector, switching back recomputes. Users choose the granularity.
- Equality: a recomputed node always counts as changed. Comparing large arrays is too expensive,
  consistent with the prior art doc's DIFF section. An optional `eq=` could be added later.

### `has_changed` and `button.clicked`

`has_changed` disappears. Nodes don't need it, and event-style logic moves to handlers:

- `Button` has a tracked `clicks: int` counter (like Dash's `n_clicks`).
- **"Compute on demand"** (e.g. an expensive run that should start only on click): the node reads
  `run_button.clicks` normally and the parameters untracked, e.g. `pa.untracked(lambda:
  slider.value)` or a `slider.peek()` (Shiny `isolate`, Solid `untrack`). The node then recomputes
  only on click.
- **Side effects** (save a file, print, apply a preset): `run_button.on_click(fn)` registers an
  event handler. It runs on the worker *outside* the graph, before re-evaluation.

### Input writes (presets) and cycles

- **Only handlers may write inputs.** Writing `slider.value = 3` inside a node (or inside `view`)
  raises, because nodes must be pure.
- A handler's writes bump the version counters, and the normal re-evaluation follows.
- The graph is acyclic by construction: a node cannot write what it reads. So the re-run cap from
  the flat model becomes unnecessary. This is the Elm rule ("state changes only in `update`") and
  the cleanest answer to the WRITE question in the prior art doc.
- Open: may a handler write inputs and also trigger another handler (`on_change` chains)? Simplest
  rule: writes from handlers do not fire handlers.

### Other aspects of the discussion so far

| Topic | Flat model | This model |
|---|---|---|
| Latency of revealed inputs | after the whole callback | after `view` (cheap); outputs arrive one by one |
| Output stability (audio, big plots) | user hoists or `lru_cache`s outputs | default |
| Tabs | callback computes the active tab | `view` references the active tab's nodes; hidden ones are not evaluated, and their results are kept |
| Fragments | additive extension | subsumed: a node returning a `View` subtree is a fragment |
| Errors | keep the last view | per node: an error card at that node's slot with its last value kept; `view` error → keep the last view |
| DIFF on the Rust side | identity of output objects | per-node result versions; the UI rebuilds a slot only when its node's version changes |
| Coalescing | one run in flight, latest values | between node evaluations, re-plan: skip nodes that became stale or unreferenced |

### The flat model is a special case: "hybrid"

`view` may also build outputs inline (`pa.Plot(xs, ys)` instead of a node). Those behave exactly
like the flat model: rebuilt whenever `view` re-runs. Since `view` is itself a tracked node, a
`view` that reads every input re-runs on every change, which is the flat model. So the two
proposals form one design:

- **Level 0:** `pa.run(view)` with everything inline. Immediate-mode feel; a script-like hello world.
- **Level 1:** lift expensive parts into `@pa.computed` nodes. Fine-grained reactivity, stable
  outputs.

Users start at level 0 and optimize locally.

**Clarification: level 0 needs no lambdas.** Outputs are built inline in `view` as plain objects
(`pa.Plot(xs, ys)`), like in the flat proposal. Lambdas or `@pa.computed` appear only at level 1.

**But level 0 is not really immediate mode either.** It is the flat proposal *minus* `has_changed`
and `clicked`, which this model replaces with tracking and handlers. Combined with hoisting, that
is far from immediate mode. Its only remaining value is a minimal hello world.

**Decision: go straight for level 1.** Level 0 on its own offers no convenient way to avoid
recomputation. Manual caching also fits this API badly:

- The obvious move is to put `functools.cache` on a unary node function (`def plot() -> pa.Plot`).
  That is wrong: it has no arguments, so it caches the first result forever.
- Correct manual caching needs "double functions": a cached function that takes the values as
  parameters, plus a unary wrapper that reads `.value` and calls it.

`@pa.computed` makes this unnecessary. Level 0 stays as a style (inline outputs in `view`), not as
a separate deliverable.

## Architecture sketch

- **Reactive core in pure Python.** Version counters, dependency recording (via a `contextvars`
  stack of "currently evaluating node"), staleness and planning. It is unit-testable with pytest,
  and the class names stay part of the Rust contract as before.
- **Worker loop (Rust)**, per round:
  1. drain UI events and write input values;
  2. run handlers;
  3. evaluate `view` if it is stale;
  4. send the view tree (node slots as IDs) to the UI;
  5. evaluate the stale referenced nodes one by one, sending each result as soon as it is ready
     and checking for new input events between nodes (re-plan instead of finishing a stale plan).
- **UI.** A persistent map `input object → gpui Entity` (the same as in the flat model) plus
  `node id → (version, PreparedOutput)`. A slot whose node is being recomputed shows the busy dim
  individually.
- The GIL/threading model is unchanged: a single worker thread is the only one calling Python.

## Costs and risks

- **Two concepts** (`view` and `computed`) instead of one. Partly mitigated by allowing inline
  outputs in `view` for cheap cases (the "level 0" style), so a hello world needs only `view`.
- **Tracking "magic".** Reads of untracked state (globals, files, a mutable object shared between
  nodes) don't invalidate anything, so results can silently go stale. This is the same contract as
  Shiny/Solid and must be documented. A node that *mutates* a shared object breaks it, too.
- **Debuggability.** "Why did X (not) recompute?" needs an answer. For example, an env-var-gated
  log (picoapp prints nothing by default).
- **Code shape.** Algorithm code gets split into decorated functions, which is less script-like.
  The ported examples (see "Examples ported from `examples/`") suggest it stays readable.
- **Rust side.** Partial updates per slot and a multi-step worker loop. Today: one job, one result.
- **Spurious recomputes.** A recomputed upstream node always invalidates its dependents, since
  there is no cheap equality.

## Examples ported from `examples/`

All examples use `@pa.computed` nodes. Layout names (`Row`/`Column`) are placeholders until the
layout API is settled.

### `example_1.py`: trivial, everything inline

Cheap compute, so no node is needed. The plot is rebuilt whenever `view` re-runs, i.e. on every
input change, which is the desired behavior here.

```py
slider_a = pa.Slider("a", -10.0, 0.5, 10.0)
slider_b = pa.Slider("b", -10.0, 0.5, 10.0)
slider_c = pa.Slider("c", -10.0, 0.5, 10.0)
negate = pa.Checkbox("Negate")

def view() -> pa.View:
    xs = np.linspace(-10.0, 10.0, 100)
    ys = slider_a.value * xs**2 + slider_b.value * xs + slider_c.value
    if negate.value:
        ys *= -1
    return pa.Row(
        pa.Column(slider_a, slider_b, slider_c, negate),
        pa.Plot(xs, ys, x_limits=(-10, +10), y_limits=(-10, +10)),
    )

pa.run(view)
```

The same code with the plot lifted into a node. `view` then reads no inputs and runs only once:

```py
@pa.computed
def plot() -> pa.Plot:
    ...  # the same computation
    return pa.Plot(xs, ys, x_limits=(-10, +10), y_limits=(-10, +10))

def view() -> pa.View:
    return pa.Row(pa.Column(slider_a, slider_b, slider_c, negate), plot)
```

### `example_freq.py`: stable audio

To show the benefit, this version adds a hypothetical `zoom` slider that only affects the plot.
Moving `zoom` re-runs `plot` only; `audio` keeps playing. In the flat model, every run creates a
new `pa.Audio`, which restarts playback.

```py
slider_freq = pa.Slider("Frequency", 20.0, 440.0, 10_000.0, log=True, decimal_places=2)
slider_zoom = pa.IntSlider("Samples shown", 100, 1000, _SAMPLE_RATE)

@pa.computed
def sine() -> np.ndarray:                          # value node, shared by both outputs
    return create_sine(n=_SAMPLE_RATE, freq=slider_freq.value)

@pa.computed
def plot() -> pa.Plot:
    n = slider_zoom.value
    return pa.Plot(xs=np.arange(n), ys=sine()[:n])

@pa.computed
def audio() -> pa.Audio:
    return pa.Audio(sine(), sr=_SAMPLE_RATE)

def view() -> pa.View:
    return pa.Row(pa.Column(slider_freq, slider_zoom), pa.Column(plot, audio))
```

### `example_filter.py`: shared intermediates, partial recomputation

In the flat version, every input change recomputes everything. Here, the kernel inputs and the
signal inputs invalidate disjoint parts of the graph:

- Moving a *signal* slider re-runs `signal`, `convolved` and the four plots that depend on them.
  `kernel` and the two kernel plots are untouched.
- Moving `Window` re-runs `kernel`, its two plots, `convolved` and the three convolution plots.

```py
@pa.computed
def kernel() -> np.ndarray:
    ...  # body of the old callback up to `kernel *= window`

@pa.computed
def signal() -> np.ndarray:
    ...  # uses slider_wavelen_signal / slider_repeat_signal and len(kernel())

@pa.computed
def convolved() -> np.ndarray:
    return np.convolve(signal(), kernel(), mode="same")

def padded_plot(f: Callable[[np.ndarray], np.ndarray]) -> pa.Computed[pa.Plot]:
    def fn() -> pa.Plot:
        n_max = max(len(signal()), len(kernel()))
        return pa.Plot(xs=np.arange(n_max), ys=np.pad(f(kernel()), (0, n_max - len(kernel()))))
    return pa.computed(fn)

kernel_re, kernel_im = padded_plot(np.real), padded_plot(np.imag)
signal_plot = pa.computed(lambda: pa.Plot(xs=np.arange(len(signal())), ys=signal()))
conv_plots = [
    pa.computed(lambda f=f: pa.Plot(xs=np.arange(len(convolved())), ys=f(convolved())))
    for f in (np.real, np.imag, np.abs)
]

def view() -> pa.View:
    return pa.Row(
        pa.Column(slider_wavelen_signal, slider_repeat_signal, slider_wavelen_filter,
                  slider_repeat_filter, radio_window, radio_complex_mode),
        pa.Column(kernel_re, kernel_im, signal_plot, *conv_plots),
    )
```

Observations:

- Node factories (`padded_plot`) and comprehensions work naturally. The usual closure-in-a-loop
  pitfall applies: note the `lambda f=f:`.
- The kernel plots depend on `signal()` only through its length (padding). Tracking cannot see
  that, so they re-run on signal changes too. That is a correct over-approximation. A user who
  cares could lift `n_max` into its own node; a node with an `eq` check (open question) would make
  that cheap.

### `example_nested_func.py`: dynamic number of inputs, one column

The master slider and the coefficient sliders now share one column, which the nested model can't
express. Coefficients beyond `order` keep their values when the order is reduced and increased
again.

```py
max_order = 10
master = pa.IntSlider("Polynomial order", 0, 4, max_order)
coefficients = [pa.Slider(f"coefficient of x^{i}", -10.0, 0.5, 10.0) for i in range(max_order + 1)]

@pa.computed
def plot() -> pa.Plot:
    xs = np.linspace(-10.0, 10.0, 100)
    ys = sum(coefficients[k].value * xs**k for k in range(master.value + 1))
    return pa.Plot(xs, ys, y_limits=(-10, +10))

def view() -> pa.View:
    return pa.Row(pa.Column(master, *coefficients[: master.value + 1]), plot)
```

The coefficients could also be created lazily (`functools.cache` on a factory `coefficient(i)`,
see "Unbounded inputs and outputs" above). Note: `plot` only depends on the coefficients it reads,
so moving a hidden coefficient (impossible via the UI, but possible from a preset handler) would
not re-run it.

### `example_nested_inheritance.py`: class-based style

```py
class App:
    def __init__(self) -> None:
        self.master = pa.IntSlider("Polynomial order", 0, 4, 10)
        self.coefficients = [pa.Slider(f"x^{i}", -10.0, 0.5, 10.0) for i in range(11)]
        self.plot = pa.computed(self._plot)

    def _plot(self) -> pa.Plot:
        ...

    def __call__(self) -> pa.View:
        return pa.Row(pa.Column(self.master, *self.coefficients[: self.master.value + 1]),
                      self.plot)

pa.run(App())
```

A `pa.computed` *method decorator* would need a descriptor that creates one node per instance.
That is possible, but `self.plot = pa.computed(self._plot)` is explicit and needs no magic.

### `example_slow.py`: per-node errors

The single plot node either renders or shows an error card in its slot, keeping its previous
value. The sliders stay usable, because `view` doesn't call the failing code. This is better than
the flat model, where a raising callback produces no view at all.

## Type checking

**Question.** Do heterogeneous children (inputs, inline outputs, nodes of different output types,
nested layouts) type-check without `Any`? Arguments that are not elements, or nodes that don't
produce elements, must be rejected. This must hold in both mypy and pyright.

**Experiment.** I wrote a sketch of the proposed API and checked it with mypy 2.4 `--strict` and
pyright 1.1.414 in strict mode. Lines marked as expected errors had to fail; all other lines had to
pass. The full sketch and the commands to reproduce it are in
`ai/alternative_reactive_model_typing_sketch.md`. The core of the API:

```py
T_co = TypeVar("T_co", covariant=True)

class Element: ...                         # common base of everything in a view
class Output(Element): ...                 # Plot, Audio, ...
class InputBase(Element): ...
class Input(InputBase, Generic[T_co]):     # covariant, see below
    @property
    def value(self) -> T_co: ...

class Computed(Generic[T_co]):             # covariant: Computed[Plot] <: Computed[Output]
    def __call__(self) -> T_co: ...

def computed(fn: Callable[[], T]) -> Computed[T]: ...

Child = Element | Computed[Element]
class Row(Element):
    def __init__(self, *children: Child) -> None: ...
```

**Results.**

| Case | mypy | pyright |
|---|---|---|
| `Row(slider, radio, Plot(), plot_node, audio_node, Column(...))`, heterogeneous varargs | ok | ok |
| `Row(*[plot_node, audio_node])`, list of nodes with different output types | ok | ok |
| `Row(*{"p": plot_node, "a": audio_node}.values())` | ok | ok |
| `Row(plot_node if c else audio_node)`; `Row(*(computed(...) for ...))` | ok | ok |
| `x: Computed[Output] = plot_node` (variance) | ok | ok |
| `Row(42)`, `Row(computed(lambda: "text"))`, `Row(dataset)` (a `Computed[list[float]]`) | error ✓ | error ✓ |
| `Row(lambda: Plot())`, `Row(int_fn)`: bare callables | error ✓ | error ✓ |
| `Row(*{"p": plot_node, "d": dataset}.values())`, `Row(*[lambda: 1, lambda: Plot()])`: a bad item in a heterogeneous collection | error ✓ | error ✓ |
| `Row(*[slider, checkbox, radio])`, unannotated list of inputs | ok once `Input` is covariant ¹ | ok |
| `ins = [slider, radio]; ins.append(int_slider)` | ok | **error** ² |
| `Row(*[slider, plot_node])`, unannotated mix of an input and a node | **error** ³ | ok |
| the same lists annotated as `list[pa.Child]` | ok | ok |

¹ mypy infers list literals by *joining* element types. The join of `Input[float]` and
`Input[bool]` with an invariant type parameter is `object`, not the common base `InputBase`. A
covariant `Input` joins to `Input[float]` or `Input[object]`, which fixes it. Covariance is sound
because `value` is read-only for callbacks. A later setter for presets cannot take a covariant
`T_co` parameter, so it would be defined on the concrete classes (`Slider.value: float`, and a
`Radio` with its own invariant `T`), which is allowed.

² pyright infers the list as `list[Slider | Radio[str]]`, so appending an `IntSlider` fails. This is
general pyright list inference and applies to today's API in the same way. The fix is an
annotation (`list[pa.Child]` or `list[pa.InputBase]`).

³ `Element` and `Computed` have no common base, so mypy's join is `object`. Making `Computed` an
`Element` would fix the join, but then `Computed[str]` would be accepted as a child too. That
loses exactly the error we want, so the annotation is the right fix.

**Conclusion.** The model type-checks without `Any`. Variance is not a problem, because `Callable`
and a covariant `Computed` both propagate subtypes. The only friction is unannotated mixed lists,
which need a `list[pa.Child]` annotation. That should be documented. `Child` and `InputBase` must
therefore be public names.

**Design consequence: no bare callables as children.** A first version of the sketch also
accepted `Child = Element | Computed[Element] | Callable[[], Element]` (bare lambdas and `def`s).
That type-checks equally well, positive and negative: `Callable` is covariant in its return type,
so heterogeneous lambdas pass and lambdas returning non-elements fail. Semantically, though, a lambda created inside
`view` is a new object on every `view` run. picoapp can neither cache it nor give it a stable slot.
So only `Computed` nodes should be accepted, and a lambda is wrapped explicitly with
`pa.computed(lambda: ...)`, at a place where it is created once.

## Open questions

1. ~~One design with levels 0 and 1, or two competing designs?~~ One design; ship level 1
   (see "The flat model is a special case").
2. ~~Ship only level 0 first?~~ No, see 1.
3. Automatic tracking vs. explicit dependencies (recommendation: automatic).
4. Node API shape: a decorator (`@pa.computed`) vs. output-specific constructors
   (`pa.Plot.computed(fn)`). Do nodes have to return `Output`s, or can they also return arbitrary
   values (like `dataset` above), i.e. general memoization?
5. Nodes returning `View` subtrees (fragments): in or out of the first spec?
6. Memory policy for cached results of hidden nodes: keep them while the node is alive (the
   proposal), or evict them?
