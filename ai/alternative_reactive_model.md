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

Users start at level 0 and optimize locally. The first spec could implement only level 0, with the
contract designed so that nodes can be added later without breaking changes. That requires, from
the start: the callback takes no arguments, inputs are hoisted objects, the view tree can contain
placeholders delivered separately, and output identity is defined. Alternatively, the first spec
ships both levels.

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

- **Two concepts** (`view` and `computed`) instead of one. This is mitigated by level 0 needing
  only `view`.
- **Tracking "magic".** Reads of untracked state (globals, files, a mutable object shared between
  nodes) don't invalidate anything, so results can silently go stale. This is the same contract as
  Shiny/Solid and must be documented. A node that *mutates* a shared object breaks it, too.
- **Debuggability.** "Why did X (not) recompute?" needs an answer. For example, an env-var-gated
  log (picoapp prints nothing by default).
- **Code shape.** Algorithm code gets split into decorated functions. That is less script-like,
  but optional thanks to level 0.
- **Rust side.** Partial updates per slot and a multi-step worker loop. Today: one job, one result.
- **Spurious recomputes.** A recomputed upstream node always invalidates its dependents, since
  there is no cheap equality.

## Open questions

1. One design with levels 0 and 1, or two competing designs? This document argues for one.
2. Should the first spec ship level 1, or only level 0 with a forward-compatible contract?
3. Automatic tracking vs. explicit dependencies (recommendation: automatic).
4. Node API shape: a decorator (`@pa.computed`) vs. output-specific constructors
   (`pa.Plot.computed(fn)`). Do nodes have to return `Output`s, or can they also return arbitrary
   values (like `dataset` above), i.e. general memoization?
5. Nodes returning `View` subtrees (fragments): in or out of the first spec?
6. Memory policy for cached results of hidden nodes: keep them while the node is alive (the
   proposal), or evict them?
