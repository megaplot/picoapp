# Flat view model: brainstorming notes

Working notes for the proposal in `ai/tasks/rethink_programming_model.md`: one flat callback
returns a full view, built from hoisted inputs and outputs. Prior art lives in
`ai/prior_art_programming_model.md`. The competing direction is in
`ai/alternative_reactive_model.md`. Once a direction is chosen, these notes feed the superpowers spec.

## Decisions so far

- **Scope of the first spec ("B").** Flat callback returning a view; hoisted inputs with object
  identity; `Row`/`Column` layout; keep the last good view when the callback raises; `has_changed`
  (but see the open question below); identity-based reuse of prepared outputs; port all examples;
  **remove the old `Inputs`/`Outputs`/`Reactive`/`ReactiveBase` API** (no compatibility adapter).
  Deferred to follow-up specs: input writes/presets with a re-run cap, `Button`, `Tabs`, fragments.
- **Status: paused, likely superseded.** The alternative reactive model
  (`ai/alternative_reactive_model.md`) contains this model as a special case: a `view` that builds
  all outputs inline. The current direction is to ship that model with `@pa.computed` nodes, see
  its "Decision: go straight for level 1". The scope items above carry over, except `has_changed`
  (replaced by tracking and handlers) and identity-based output reuse (replaced by per-node
  versions). The analysis in this file stays as the rationale.

## Latency of revealed inputs

With a flat callback, a newly revealed input (e.g. submode parameters) is shown only when the
whole callback returns. Analysis:

- If the revealing input (the submode selector) itself affects an output, that output must be
  recomputed anyway. There is nothing to win, except from a user-side cache hit.
- If it doesn't, a user-side `lru_cache` or `has_changed` skip makes the run cheap, so the new
  inputs appear quickly anyway.
- The current nested model does not avoid the recomputation either. It only shows the new inputs
  earlier, by the duration of the leaf callback (see the prior art doc, "Challenges", item 2).

Conclusion: no special mechanism is needed for the flat model. Fragments (prior art doc, "Streamlit
fragments mapped onto the flat model") are a possible additive extension.

## `has_changed` semantics

### Definition

**Value-based.** `x.has_changed` is true if `x.value` differs from its value at the start of the
previous callback invocation. Before the first invocation that sees `x`, it is true; this covers
inputs created lazily mid-session. The alternative definition, "a UI event touched `x` since the
last run", gives a wrong answer under coalescing. If the user drags a slider away and back while a
job is in flight, the value is unchanged but the event-based flag would be true.

Implementation: Python-side bookkeeping only. Each input keeps a `_prev_value` snapshot. picoapp
refreshes the snapshots of all live inputs (tracked in a `WeakSet`) after each invocation.

### Inactive inputs

An input that wasn't part of the previous view cannot be changed by the UI, so `has_changed` is
false for it. That is correct, not a special case. "Becoming visible" is not a value change. A
value the previous run computed with is still valid.

### Pitfall: `has_changed` is relative to the previous *invocation*, not the previous *computation*

```py
plot_a: pa.Plot | None = None

def callback() -> pa.View:
    global plot_a
    if tab.value == "A" and (s.has_changed or plot_a is None):
        plot_a = pa.Plot(..., f(s.value))
    ...
```

1. The user is on tab B and moves `s`.
2. The run skips `plot_a`, because tab A is hidden.
3. The user switches to A. Now `s.has_changed` is false (`s` changed one invocation earlier), and
   `plot_a` is stale.

So `has_changed` is only correct for a computation that is evaluated on *every* invocation. As soon
as the computation is conditional (tabs, branches, the use case's dataset/algorithm switch), the
user has to track "which values was this output computed with". That is exactly what a value-keyed
cache (`functools.lru_cache`) does, and it is always correct. Inputs are scalars, so keying on
values is cheap.

**Consequence.** For output caching, `lru_cache` is the robust primary tool, and `has_changed` is a
trap. `has_changed` remains meaningful for *events*: a button click, or "run this side effect when
X changes". Open question: drop `has_changed` from scope B and only introduce event semantics later
together with `Button.clicked`?

## Reading inactive inputs

Recommendation: **allow it, no warning.** The values are owned by the user's program, just like an
immediate-mode `&x`. Valid uses:

- a parameter shown in only one tab but used by outputs in several;
- an "advanced settings" section that is collapsed but still in effect;
- comparing the current branch against the remembered parameters of the other branch (A/B diff).

A warning would also have to be opt-in, because picoapp prints nothing by default.

## Observation: outputs need hoisting too

Returning fresh output objects from every run is the natural way to write the callback. With
identity-based reuse, though, it means every output is rebuilt on every run. An `Audio` output
restarts playback whenever any slider moves. A plot with millions of points is re-prepared even
though only an unrelated output changed. To get stable behavior, users will in practice hoist or
cache their outputs as well (`lru_cache` returning the `pa.Audio` object gives a stable identity
for free). This is the main motivation for looking at the alternative model, where outputs are
long-lived nodes by construction.
