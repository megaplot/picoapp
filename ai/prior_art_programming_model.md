# Prior art: programming models for picoapp

Research input for `ai/tasks/rethink_programming_model.md`. It surveys how other frameworks answer
the task's open questions, sketches how each would express the task's running use case, and ends
with a synthesis of what picoapp should adopt or avoid.

**Running use case ("dataset × algorithm")**: two selectors (dataset type, algorithm type), each
with type-specific parameters. The UI should show one column per selector, with the selector on top
and its parameters below. One algorithm parameter is a "submode" with submode-specific parameters
of its own. Switching a selector back and forth (A/B) must not reset the parameters of the
inactive branch.

**Questions tracked throughout** (abbreviations used below):

- **ID**: how a widget gets a stable identity across re-evaluations, and what happens to its state
  while it is not shown.
- **LAYOUT**: how layout is expressed and whether it is coupled to the reactive structure.
- **CACHE**: how users avoid recomputing expensive parts (fine-grained reactivity vs. memoization).
- **DIFF**: how the framework avoids rebuilding or re-uploading unchanged outputs.
- **WRITE**: whether code can set input values (presets), and how update cycles are handled.
- **ADV**: tabs and similar container components.

---

## 1. Streamlit

**Model.** The whole script re-runs top to bottom on every widget interaction ("rerun"). Widgets are
*function calls that return their current value* (`x = st.slider("x", 0, 10)`), so it reads as
immediate mode. Elements are emitted into a "delta" stream that the frontend reconciles against the
previous run by position.

- **ID.** A widget's identity is derived from its parameters (label, min, max, …) unless an explicit
  `key=` is passed. Changing a parameter of an unkeyed widget creates a "new" widget and resets it.
  Historically, a widget that is *not rendered in a run is garbage collected*, including its
  `st.session_state[key]` entry. This made exactly our A/B use case painful: hidden branches forget
  their values. The standard workaround (mirror the value into a second, non-widget session state
  key via `on_change`) was one of the most frequently asked forum questions. Streamlit 1.59 (2026)
  finally added `persist_state=None | "page" | "session"` (requires `key`) to keep the value of a
  non-rendered widget. Even with a key, changing `min_value`/`max_value`/`step` resets the widget.
  **Lesson:** identity derived from position/parameters is fragile; Streamlit spent years retrofitting
  explicit, persistent identity. picoapp's "hoist the input object" proposal gets this for free:
  the Python object *is* the identity and its lifetime is controlled by the user.
- **LAYOUT.** Decoupled from reactivity: layout is done with context managers (`with col1:`),
  `st.columns`, `st.tabs`, `st.expander`, `st.sidebar`, `st.container`. Elements can also be written
  to containers created earlier (`placeholder = st.empty()`; later `placeholder.line_chart(...)`),
  so computation order and visual order can differ. Layout is deliberately coarse (columns with
  relative widths, gap, vertical alignment); no general flexbox is exposed, and that has been
  sufficient for the dashboard/prototype niche.
- **CACHE.** Pushed entirely to the user: `@st.cache_data` (hashes arguments, returns a copy) and
  `@st.cache_resource` (returns the same object). Exactly the task's `lru_cache` idea, with one
  important addition: the cache is keyed by *hashing* the arguments, which is costly for big arrays
  (Streamlit hashes numpy arrays content-wise). `st.fragment` (1.37+) is a partial-rerun scope: a
  decorated function re-runs alone when one of *its* widgets changes. It is Streamlit's answer to
  "fine-grained reactivity" and was introduced because full reruns were too slow for larger apps.
  Since 1.63, callbacks can target a named fragment (`@st.fragment(key=...)`).
- **DIFF.** The frontend reconciles element-by-element; a chart whose spec did not change is not
  re-rendered, but the *data still crosses* Python → browser on every run (serialization cost is
  paid regardless). Not a model to copy; picoapp's zero-copy goal is precisely the opposite.
- **WRITE.** `st.session_state[key] = v` sets a widget's value, but only *before* the widget is
  instantiated in the run; doing it afterwards raises
  `StreamlitAPIException: st.session_state.x cannot be modified after the widget with key x is
  instantiated`. The sanctioned way is in an `on_change`/`on_click` callback, which runs *before* the
  rerun. `st.rerun()` triggers another full run; infinite loops are the user's problem. Buttons are
  "trigger" widgets: `st.button(...)` returns `True` for exactly one run after a click.
  **Lesson:** writes during the callback are ambiguous ("which value did the code above already
  read?"); Streamlit resolves it with a phase rule (writes allowed before the widget is read, or in
  pre-run callbacks).
- **ADV.** `st.tabs` renders *all* tab bodies on every run (the frontend just hides inactive ones).
  Since 1.55, `st.tabs(..., on_change=...)` can rerun on tab switch and the active tab is readable,
  which enables lazy tabs. Tab/expander open state is preserved across reruns since 1.56.

**Use case.**

```py
ds_col, algo_col, out_col = st.columns([1, 1, 2])
with ds_col:
    ds = st.selectbox("Dataset", ["blobs", "moons"], key="ds")
    if ds == "blobs":
        n = st.slider("n_blobs", 1, 10, key="blobs.n", persist_state="session")
    else:
        noise = st.slider("noise", 0.0, 1.0, key="moons.noise", persist_state="session")
with algo_col:
    ...  # same pattern, nested `if` for the submode
with out_col:
    st.pyplot(run(load(ds, ...), ...))
```

Natural, as long as every conditional widget carries a key and `persist_state`. Our proposal is the
same shape with object identity replacing string keys.

## 2. Plotly Dash

**Model.** Declarative layout tree (`html.Div`, `dcc.Slider`, each with an `id`) plus *callbacks*
that map `Input(id, prop)`/`State(id, prop)` to `Output(id, prop)`. The framework builds a
dependency graph over component properties and only calls callbacks whose inputs changed: genuine
fine-grained reactivity, wired by hand via string IDs.

- **ID.** Explicit string (or dict) IDs. Dynamic sets use *pattern-matching IDs*
  (`{"type": "param", "index": ALL}`), which is clunky but makes dynamic inputs addressable.
- **LAYOUT.** Fully decoupled from reactivity: layout is a static tree, callbacks can target any
  property anywhere. Dynamic layout = a callback outputs `children` of a container, and components
  created that way lose their state when re-created (same reset problem as picoapp's nested model)
  unless the user uses `persistence=True` (stores the user-set value keyed by ID in the browser) or
  keeps all branches mounted and toggles `style={"display": "none"}`. The latter is the idiomatic
  answer for A/B use cases.
- **CACHE.** Fine-grained by construction: `foo` depends on (a, common) → only that callback re-runs.
  Plus `flask_caching`/`dcc.Store` for user-side memoization.
- **DIFF.** Per property: only the outputs of the triggered callbacks are sent. `Patch()` (2.9+)
  sends partial property updates ("append to this list", "set this key") instead of the whole
  figure, because re-sending whole figures was the main performance problem. `no_update` lets a
  callback skip an output explicitly. **Lesson:** an explicit "unchanged" sentinel is cheap and
  understandable; this maps directly to the task's "return the previous object instance" idea.
- **WRITE.** Any callback may output to an input's `value` → presets are trivial. Cycles between
  callbacks are rejected at startup (dependency graph must be a DAG), except a *single* callback may
  have the same property as input and output ("circular callbacks", used to sync two widgets,
  disambiguated via `ctx.triggered_id`). `allow_duplicate=True` (2.9+) lets several callbacks target
  the same output.
- **ADV.** `dcc.Tabs`; content either all pre-rendered or produced by a callback on the `value`.

**Use case.** All branch parameter panels live in the static layout, hidden via `style`; one
callback toggles visibility from the selector values; the compute callback takes *all* params as
`State` and only reads the active ones. Works and preserves state, but the user must wire every ID
by hand. This is the boilerplate picoapp wants to avoid.

## 3. Bokeh server and Panel (HoloViz)

**Bokeh server.** Retained mode: a document of model objects (`Slider`, `Column`, `figure`); Python
callbacks are attached with `slider.on_change("value", cb)`; callbacks *mutate* models
(`source.data = ...`, `layout.children = [...]`). Model objects have stable identity because they
are Python objects that the user holds onto (the same mechanism as our hoisting proposal). Swapping
`layout.children` keeps the removed widgets' values, since the objects survive. Data goes through
`ColumnDataSource`, which supports `stream()`/`patch()` for incremental updates. **Lesson:** "hoisted
objects = identity" is proven at scale; the cost of pure retained mode is that the user writes
imperative view-update code.

**Panel** builds on Bokeh with three APIs at different levels:

- `pn.bind(fn, widget_a, widget_b)` / `@pn.depends(...)`: reactive functions that re-run when their
  bound widgets change and return a displayable (plot, layout, …). Dependencies are explicit per
  function → fine-grained, and each bound function is rendered into its own slot of the layout.
- `param.Parameterized` classes: parameters declared as class attributes (`n = param.Integer(5,
  bounds=(1, 10))`); `pn.Param(obj)` generates widgets automatically. Very close to the
  "class-based alternative" in the task.
- Layouts (`pn.Row`, `pn.Column`, `pn.Tabs`, `pn.Card`, `pn.GridSpec`) are separate objects; a
  bound function can return a *layout including widgets*, which is how dynamic parameter panels are
  done. Widgets created *inside* the bound function are recreated (and reset) on each call; widgets
  created outside and merely *placed* by it keep their state (the same rule as our proposal).

`pn.Tabs(dynamic=True)` renders only the active tab. Panel's docs for this use case recommend
`param` classes per dataset/algorithm and a `pn.bind` that returns `pn.Param(selected_obj)`; since
the `Parameterized` instances persist, switching back restores the old values.

**Use case** (Panel):

```py
datasets = {"blobs": Blobs(), "moons": Moons()}      # param.Parameterized, persistent
ds_select = pn.widgets.Select(options=list(datasets))
ds_col = pn.Column(ds_select, pn.bind(lambda k: pn.Param(datasets[k]), ds_select))
```

This is structurally identical to the task's proposal (stable objects, imperative selection of
which ones to show). Panel adds per-function dependency tracking on top.

## 4. Shiny for Python

**Model.** Reactive graph à la spreadsheets: `input.x()` reads are tracked automatically;
`@reactive.calc` (cached derived value, re-evaluated lazily when a dependency changes),
`@render.plot`/`@render.ui` (outputs), `@reactive.effect` (side effects). Dependencies are
discovered at runtime by observing which reactive values were read (the same mechanism as
Solid/Vue/MobX signals).

- **ID.** String IDs in a static UI definition.
- **LAYOUT.** Static layout (`ui.layout_columns`, `ui.card`, `ui.navset_tab`) with output
  placeholders; decoupled from reactivity.
- **CACHE.** `reactive.calc` *is* automatic fine-grained caching: `foo` reads `a` and `common`, so it
  re-runs only when one of them changes. No manual `lru_cache`, no hashing.
- **DIFF.** Only invalidated outputs re-render.
- **WRITE.** `ui.update_slider("x", value=...)` from an effect. Loops are possible
  (effect reads x and writes x) and are the user's problem; `reactive.isolate()` reads without
  creating a dependency, `@reactive.event(input.btn)` restricts an effect to explicit triggers.
  The docs explicitly recommend *update functions over regenerating UI*, because `render.ui`
  regenerates inputs and resets them (the same reset problem again).
- **ADV.** `ui.navset_tab`; inactive tab outputs are not computed (Shiny suspends hidden outputs).
  This is a nice property: outputs that are not visible are not evaluated.

**Use case.** Conditional panels via `ui.panel_conditional("input.ds === 'blobs'", ...)` (JS
expression; keeps hidden inputs alive → A/B works) or `render.ui` (resets). Two columns with the
selector on top are trivial since layout is static.

**Weaknesses relevant to picoapp.**

- *Stringly typed input access.* An input is declared with a string ID (`ui.input_slider("n",
  ...)`) and consumed as an attribute of a dynamic `input` object (`input.n()`). The two are only
  linked by the string, so typos and type errors are not caught by a type checker, and the value
  type of `input.n()` is not inferred. picoapp's "the input object *is* the handle" (`n.value`,
  typed via the class) avoids this entirely.
- *Layout is static by default, dynamic only via `render.ui`.* The compute graph makes the logical
  structure easy to read, but the visual structure is a separate static UI tree with output
  placeholders. Changing the layout fundamentally depending on an input (e.g. one vs. two columns,
  or a completely different set of panels) requires `@render.ui`, which returns a fresh UI subtree
  and thereby re-creates (and resets) every input inside it. Outputs inside a `render.ui` subtree
  must still be pre-declared as separate `@render.*` functions referenced by string ID. The
  alternative is `panel_conditional` with a JavaScript expression string. In the flat picoapp
  proposal, the callback can return an arbitrary view on every run, and hoisted inputs keep their
  state regardless.

## 5. marimo

**Model.** Reactive notebook: cells form a DAG via the global variables they define/read. A UI
element (`mo.ui.slider`) must be assigned to a *global variable*; interacting with it re-runs the
cells that read that variable. Elements created inside functions or not bound to a global are not
reactive. Dynamic sets of elements go into `mo.ui.array`/`mo.ui.dictionary`/`mo.ui.batch`, which
are themselves elements. `mo.state` exists for cycles/sync between elements and is documented as
"you almost never need this".

- **ID.** The global variable name (the cell that defines the element re-creating it resets it).
- **LAYOUT.** `mo.hstack`/`mo.vstack`/`mo.ui.tabs`/`mo.accordion` compose elements and outputs
  freely; layout is a value returned by a cell, decoupled from which cell owns the element. An
  element can be *displayed* in a different cell than where it is defined.
- **CACHE.** The cell DAG (one cell = one cache unit) plus `mo.cache`/`mo.persistent_cache`.
- **WRITE.** Via `mo.state` setters; a cell that sets state it reads is not re-triggered by its own
  set (configurable `allow_self_loops`). **Lesson:** a built-in "a run doesn't re-trigger itself by
  default" rule.

**Lesson for picoapp.** marimo is the closest *Python-native* analog to "hoisted input objects +
a function that composes elements into a layout". Its "element must live at a stable outer place
to keep state" rule is the same convention our proposal asks users to follow, and the docs/FAQ show
users understand it once it is stated explicitly.

## 6. Gradio (Blocks + `@gr.render`)

Blocks: static layout (`with gr.Row(): ...`) + event listeners (`slider.change(fn, inputs,
outputs)`), Dash-like. Dynamic UI via `@gr.render(inputs=[...])`: the decorated function re-runs and
re-creates its components. To keep values across re-renders, components need an explicit
`key=` (and parent layout blocks need matching keys too); only `value` is preserved by default,
other props via `preserved_by_key=`. Event listeners must be re-declared inside the render function.
**Lesson:** another framework that started with "re-create = reset" and retrofitted key-based
identity, with sharp edges (parent keys must match) that object identity would not have.

## 7. Solara / Reflex (React model in Python)

Solara reimplements React in Python: `@solara.component` functions, `solara.use_state`,
`use_memo(fn, dependencies=[...])`, reactive variables (`solara.reactive(0)`) that can live at
module level. State is keyed by *position in the component tree* (React rules of hooks, including
"don't call hooks conditionally"). Widgets whose state is a module-level `solara.reactive` survive
being unmounted; widgets with `use_state` local state do not. Reflex: state classes on the server,
compiled React frontend. **Lesson:** React-style positional identity forces "rules of hooks" on the
user; for an explorative-script audience, explicit objects are simpler.

## 8. ipywidgets `interact`

`interact(f, a=(0, 10))` auto-generates widgets from a function's signature and re-runs `f` on
change. Simplest possible model, no layout control, no conditional inputs, no caching. It is the
baseline picoapp's *current* flat case resembles. Worth noting: `interact_manual` adds a "Run"
button (explicit trigger) for slow functions, a simple answer to high-latency callbacks that
picoapp currently handles via coalescing instead.

## 9. Immediate-mode GUIs: Dear ImGui, egui

**Model.** The UI is code executed every frame: `if ImGui::SliderFloat("a", &a, 0, 1) { ... }`.
Widgets return "changed"/"clicked" booleans in the same call that draws them; the *application*
owns the value (`&a`), the library only stores transient interaction state (hover, active drag,
open/closed tree nodes, scroll).

- **ID.** Transient state is keyed by a hash of an *ID stack* (window/parent IDs + label, or
  `##suffix`/`PushID(i)` to disambiguate). Duplicate labels collide, a classic pitfall. Since the
  *value* lives in user variables, a hidden widget's value is never lost: it lives in `a`.
  **Lesson:** separating "value owned by the program" from "widget is drawn this frame" is exactly
  what makes A/B switching trivial. Our hoisted `pa.Slider` objects play the role of `a`.
- **LAYOUT.** The known weakness: layout is computed in a single pass while drawing, so a container
  cannot size itself by children it has not yet seen. egui works around it with previous-frame
  sizes and discard/sizing passes (`Grid` guesses column widths the first frame, then re-runs);
  Dear ImGui has `Columns`/`Tables` with similar caveats. **This limitation does not apply to
  picoapp's proposal**: picoapp's callback returns a *tree description*; layout happens later in
  gpui/taffy with full knowledge of the tree. We get immediate-mode *authoring* with retained-mode
  *layout*.
- **CACHE/DIFF.** Users cache explicitly (immediate mode re-runs every frame, so caching discipline is
  part of the culture). egui's `Context::memory` / `ui.data()` lets widgets keep data keyed by `Id`.
  Textures are explicitly retained handles (`ctx.load_texture` → `TextureHandle`), re-uploaded only
  when the user replaces them. This is the egui answer to DIFF: *expensive GPU resources are
  explicit retained handles; cheap things are re-emitted every frame*.
- **WRITE.** Trivial: the program writes `a = 5.0` and the next frame shows it. No cycle problem
  because there is no dependency graph; a frame just runs again (and egui only repaints on input
  or `request_repaint()`).
- **ADV.** `BeginTabBar`/`BeginTabItem` only execute the active tab's body; state of the inactive
  tab's widgets survives because values live in program variables.

**Use case** (egui):

```rust
ui.columns(2, |cols| {
    combo(&mut cols[0], &mut app.ds_kind);
    match app.ds_kind { Blobs => slider(&mut cols[0], &mut app.blobs.n), Moons => ... }
    combo(&mut cols[1], &mut app.algo_kind);
    match app.algo_kind { Kmeans => { ...; if app.kmeans.submode == X { ... } } ... }
});
```

Nested `if`/`match` without any extra structure, and all branch params persist because they live in
`app`. This is what the task's proposal reproduces in Python.

## 10. React / Svelte / Solid (and Elm/iced, SwiftUI/Compose)

- **React.** `view = f(state)`; the whole component function re-runs; reconciliation diffs the
  returned element tree. Identity is *position + type + optional `key`*; a component removed from
  the tree loses its state (the canonical fix is "lift state up", i.e. hoisting, which is what our
  proposal does). Memoization is user-side (`useMemo(fn, deps)`, `React.memo` skipping re-render
  when props are *referentially* equal). Referential equality as a cheap "unchanged" check is the
  standard idiom, and its pitfall (mutating an array in place doesn't trigger an update) is
  well-known to React users and solved by convention ("treat state as immutable"). That is the
  task's DIFF proposal, with a known teaching story. Setting state during render loops are caught
  with "Too many re-renders"/"Maximum update depth exceeded" (a hard cap of ~50 nested updates).
- **Solid / Svelte 5 runes / Vue / MobX.** Fine-grained signals with automatic dependency tracking;
  components run once, only effects re-run. Equivalent to Shiny's `reactive.calc`. Elegant, but
  requires all reads to go through tracked accessors and makes control flow inside the "callback"
  subtle (`<Show>`/`<For>` instead of `if`/`for`). Conflicts with picoapp's "plain Python function"
  goal; only worth borrowing as an *optional* memo layer.
- **Elm / iced.** `update(msg, model)` + `view(model)`. Presets are just a message that updates many
  model fields; no cycles possible because view is pure and updates are messages. Relevant as the
  clean answer to WRITE: *input writes are allowed only in a well-defined phase*.
- **SwiftUI / Jetpack Compose / Flutter.** Identity is structural (call-site position) plus explicit
  `id()`/`key()`; `@State`/`remember` state dies when the view leaves the tree, `rememberSaveable`/
  hoisted `@StateObject`/`@Observable` models survive. Same lesson as React, again: *the state
  that must survive branch switches must be hoisted out of the view tree*. Compose's skipping of
  recomposition for "stable" unchanged parameters is the DIFF idea, done by equality on stable
  types.

---

## Synthesis

### ID: stable identity and hidden-state preservation

Every framework that derives identity from position or parameters (Streamlit, Gradio, React,
Compose, Shiny's `render.ui`, Dash dynamic children, picoapp today) has the "branch switch resets
inputs" problem and has retrofitted explicit keys plus persistence (Streamlit's `persist_state`
arrived only in 2026). Frameworks where the *user holds the object/value* (Bokeh, Panel, marimo,
Dear ImGui, egui) never had it.

→ **The proposal's "hoist inputs, object identity = widget identity" is the strongest choice
available.** It needs no string keys and no persistence flag. The cost is one convention ("an input
created inside the callback is a new input every run"), which marimo/Panel document successfully.
picoapp can *detect* violations cheaply (an input object seen for the first time on a run that was
triggered by a change, repeatedly) and could warn in a debug mode, though "picoapp prints nothing by
default" applies.

Rust-side implication: a registry `input identity (e.g. Python id() or a picoapp-assigned counter)
→ gpui Entity<SliderState>` that survives across callback runs; entities for inputs that are absent
from a returned view are retained as long as the Python object is alive (they hold transient UI
state like drag/focus only; the value lives in Python).

### LAYOUT

- Decoupling layout from reactivity is universal outside picoapp; all Python app frameworks offer
  `Row/Column` (or `columns`), `Tabs`, an expander/card, and stop there. None of the prototype-app
  frameworks (Streamlit, marimo, Gradio, Panel) expose full flexbox; they expose *relative widths*
  (`st.columns([1, 2])`, Gradio `scale=`), gaps, and alignment. That has been sufficient for the
  niche for years.
- Naming: `Row`/`Column` (Streamlit, Gradio, Panel, Dash bootstrap, Compose, Flutter) is far more
  common than `HStack`/`VStack` (SwiftUI, marimo).
- Immediate-mode layout limitations do *not* apply, since picoapp builds a full tree before layout.
- gpui offers taffy 0.13: full flexbox (grow/shrink/basis/wrap/gap/align) and a *simplified* grid
  (N equal tracks with spans). A small backend-neutral vocabulary (`Row`, `Column`, a per-child
  relative weight, maybe `Tabs`/`Group`) maps onto any of gpui, egui, Qt or the web. Exposing CSS
  terms directly would tie the API to flexbox semantics.

### CACHE

Three families: (1) explicit memoization with hashing (Streamlit `cache_data`, `lru_cache`),
(2) per-function declared deps (Dash, Panel `pn.bind`, React `useMemo`), (3) automatic dependency
tracking (Shiny `reactive.calc`, Solid, marimo cells).

For picoapp's audience, (1) plus a cheap change signal fits best:

- `lru_cache(maxsize=1)` on scalar inputs is cheap; hashing is only a problem for array args, which
  in picoapp come from user computations, not from inputs (inputs are scalars).
- `has_changed`/`clicked` per input is the immediate-mode idiom (egui `response.changed()`,
  ImGui's return value, Streamlit's button). It costs picoapp nothing, since the scheduler already
  knows which input changed. Caveat to resolve in the spec: with coalescing, *several* inputs may
  have changed between two runs, so `has_changed` must mean "changed since the previous callback
  invocation", not "the triggering event". On the first run everything has "changed".
- A built-in helper is cheap to add later and not required up front (e.g. `pa.memo(fn, *deps)`
  keyed by call site, similar to `useMemo`). Automatic tracking (family 3) is unnecessary given
  scalar inputs and conflicts with plain-Python control flow.

### DIFF

- In gpui, elements are rebuilt on every render anyway (like immediate mode), and only `Entity`s
  and GPU resources persist. Rebuilding the *element tree* after a callback is therefore free in
  practice; what matters is not re-doing *expensive preparation*: image upload (`RenderImage`),
  audio player restart, and potentially plot path building for very large series. picoapp already
  separates those in `PreparedOutput` (built once per result, not per frame).
- The established cheap "unchanged" signals are (a) referential equality (React.memo, egui texture
  handles, our proposal), (b) an explicit sentinel (Dash `no_update`), (c) explicit retained handles
  (egui `TextureHandle`, Bokeh `ColumnDataSource`). None of the frameworks content-hashes large
  arrays on the hot path.
- → Identity-based reuse of `PreparedOutput` (same Python output object as last run → reuse the
  prepared Rust value) is the established pattern. It also gives "audio keeps playing when an
  unrelated slider moves" for free, which is a user-visible improvement over today, where every run
  restarts audio. The in-place-mutation pitfall exists but is the same, well-known React convention;
  outputs are cheap wrappers, so "make a new `pa.Plot`" is the natural thing users do anyway.
  Needs care: CPython can reuse an `id()` after an object is freed, so identity must be compared on
  objects the worker keeps alive (hold a reference to the last run's outputs), not on raw ids.

### WRITE: presets and update cycles

- Allowed in every framework, with a phase rule to remove ambiguity: Streamlit (only before the
  widget is read, or in pre-run callbacks), Elm (only in `update`), Shiny/Dash (from
  effects/callbacks, i.e. *between* renders), ImGui (anywhere, takes effect next frame).
- Cycle handling: Dash rejects multi-callback cycles statically; marimo doesn't re-trigger a cell
  on its own state writes by default; React caps at ~50 nested updates; Streamlit/Shiny/ImGui leave
  it to the user.
- For picoapp: setting `slider.value = x` inside the callback can be defined as "takes effect for
  the *next* run; picoapp schedules exactly one follow-up run if any value was written". If the
  follow-up run writes again, run again, up to a cap (React-style), and surface an error
  in the UI when exceeded. A write equal to the current value should not count. This mirrors
  Streamlit `st.rerun()` + React's cap without introducing a graph. A `Button` (trigger input,
  `clicked` true for exactly one run) is the natural place to issue preset writes from.

### ADV: tabs and containers

- Two strategies: render all tab bodies and hide (Streamlit default, Dash, Gradio) vs. evaluate only
  the active tab (ImGui, Shiny suspension, Panel `dynamic=True`, Streamlit `on_change`).
- In the proposed model, the active tab is just another input (`pa.Tabs` exposing `.value` /
  `.active`), and the callback can choose to compute only the active tab's outputs, which is the
  immediate-mode answer and needs no special machinery. Hidden tab inputs keep their state because
  they are hoisted. The gpui-component crate already ships `tab`, `accordion`, `collapsible`, and
  resizable `dock` panels.

### Streamlit fragments mapped onto the flat model

**Streamlit semantics.** On a full rerun, a `@st.fragment` function runs as part of the script, and
its arguments are captured. Interacting with a widget *created inside* the fragment re-runs only
that function, with the captured arguments, and its elements replace the fragment's previous
elements in place. The main script does not run. If main-script code also reads the fragment's
widget value (via `st.session_state`), that code is stale until the next full rerun. Streamlit
accepts this staleness. `st.rerun(scope="app")` from inside the fragment forces a full run.

**A picoapp equivalent** could look like this:

```py
def algo_panel(dataset: Dataset) -> pa.View:   # dataset computed by the main callback
    params = algo_params[algo_select.value]
    result = run_algo(dataset, params)         # expensive
    return pa.Column(algo_select, *params.inputs, pa.Plot(...result...))

def callback() -> pa.View:
    dataset = make_dataset(...)
    return pa.Row(dataset_panel(dataset), pa.Fragment(algo_panel, dataset))
```

- `pa.Fragment(fn, *args)` is a view element. On a full run, picoapp calls `fn(*args)` and records
  which inputs appear in the returned subtree: the fragment *owns* those inputs. The worker keeps
  `(fn, args)` per fragment, like the old `Registry` keeps nested levels.
- On a change, if every changed input (after coalescing) is owned by the same fragment, picoapp
  sends a `Job::RunFragment(id)`. It runs `fn(*captured_args)` and splices the new subtree into the
  last view. Otherwise it does a full run.
- The staleness hazard is the same as in Streamlit: the main callback may read a fragment-owned
  input. picoapp could detect this cheaply by recording `.value` reads during the main run (a
  property hook). A change to such an input then forces a full run. That is automatic dependency
  tracking in disguise, which is where the alternative model in `ai/alternative_reactive_model.md`
  starts.

**Observations.**

- A fragment is the old nested `Reactive` level turned inside out. It still gives a partial re-run
  scope, but layout is no longer tied to nesting, and the inputs stay stable because they are
  hoisted.
- Fragments are additive to the flat model. A later spec can add them without breaking the API.
  They are a coarse-grained version of the per-output nodes in the alternative model.

### Use-case verdict

| Framework | Columns per selector with params below | A/B keeps params | Boilerplate |
|---|---|---|---|
| picoapp today | no (one column per nesting level) | no | high |
| Streamlit | yes | only with `key` + `persist_state` (2026) | low |
| Dash | yes | via hide/show or `persistence` | high (IDs) |
| Panel | yes | yes (persistent `Parameterized`) | low–medium |
| Shiny | yes | via `panel_conditional` | medium |
| marimo | yes | yes (global elements) | low |
| Gradio `@gr.render` | yes | with `key` on element and parents | medium |
| egui / ImGui | yes | yes (values in app struct) | low |
| **Proposal** | yes | yes (hoisted objects) | low |

### Challenges to the proposal / variations to discuss in brainstorming

1. **Input placement vs. callback.** Every surveyed framework lets an input appear in the layout
   without being re-declared per run; the proposal makes the callback return inputs every run. That
   is fine (it is what lets layout be conditional), but it means a callback that raises renders
   *no* inputs. The UI must keep the last successful view (today it keeps the last outputs) or the
   user could get stuck without the input that would fix the error.
2. **Latency.** With a full-view return, *showing* a newly revealed input (e.g. the submode's params)
   waits for the whole callback. Whether there is anything to gain depends on whether the
   revealing input itself affects an output: if it does, the output must be recomputed anyway (or
   come from a user-side cache); if it doesn't, a user-side `lru_cache`/`has_changed` skip makes the
   run fast anyway. Shiny avoids the issue structurally (per-output graph); Streamlit with
   fragments (see the mapping below). picoapp's *current* nesting does **not** avoid the
   recomputation: a `Nested` reply makes the child's inputs visible once the outer callback returns,
   but the child's first job is dispatched right away (`reactive_view.rs`, `apply_result`), and
   since the child inputs are fresh, the leaf outputs are always recomputed. It only shows the new
   inputs earlier, by the duration of the leaf callback. Discussed further in
   `ai/flat_view_model.md` and `ai/alternative_reactive_model.md`.
3. **Coalescing semantics of `has_changed`** (see CACHE).
4. ~~`Column` name collision~~ with an "input column" concept: a false alarm. The old API
   never names columns; the term only came from the task description's prose.
5. **Input identity is a Python object, but options like `min`/`max`/`values` may change at
   runtime**: Streamlit resets on bound changes; the simplest picoapp rule is "inputs are immutable
   except `value`; create a new input for a new range".
6. **Migration**: whether the old `Inputs`/`Outputs`/`Reactive` API is removed outright (breaking,
   pre-1.0, simplest) or kept as a thin adapter for one release.
