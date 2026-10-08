# Task description

I'm considering to re-think the reactivity programming model used by picoapp (breaking change).

## Status quo

Currently the reactivity model requires to pair some "inputs" with "outputs", where the "outputs" can themselves be such an inputs/outputs pair.
This allows for accomplishing some degree of fine-grained reactivity via nesting such pairs.
The core feature that this model enables is that the picoapp framework is able to only call back into inner callbacks.
This means that client code could hoist computations into outer callbacks to avoid their re-computation on inner input modifications.
Essentially the model results in a hierarchical cache.

**Issues**

This programming model has a few issues:

- It's awkward to have "outputs" on some of the outer reactive callbacks. Essentially if an outer callback wants to have any outputs (it currently cannot have any direct outputs, because it must return nested callbacks for reactivity) it would have to pass down its outputs into the nested callbacks it creates. This would result in mixing concerns.
- If the reactivity requires lots of conditionals, a lot of nesting would be necessary.
- Currently picoapp couples the visual representation to the reactivity model (one input column per input nesting). This is limiting, and there is no obvious way how to decouple visual layout from the reactive flow.
- Input controls aren't "stable" -- switching on the outer input level re-creates the input elements, resetting their state on all change instead of keeping the old state.

As an example, think of a picoapp where a user whats to evaluate parameterized "datasets" and "algorithms" pairs, i.e., the user can select from a "dataset" selection and an "algorithm" selection, which produces certain analysis outputs (applying the selected algorithm to the selected dataset).
Let's further assume that both the selected algorithm and selected dataset (think synthetic data generators) have parameters that should be controlled via picoapp inputs, and these parameters/inputs are different/specific for each selected algorithm/dataset.

A natural way to lay this out would be to have 2 input columns:

- one for the dataset, with the dataset name/type first (which branches) and below the selected dataset's parameters/inputs.
- one for the algorithm, with a similar name/type first, followed by specifics.

This is currently not possible.
One way to accomplish it is to have:

- one column for the dataset name/type *and* the algorithm name/type selector (or a separate column?)
- one column for the selected dataset inputs/params
- one column for the selected algorithm inputs/params

This "works" but the parameters get separated weirdly from their primary selectors -- the result app has strange UX.

And things get much more messy if further branching is necessary.
Let's say one of the algorithm parameters itself (some "submode") again requires submode-specific nested parameters/inputs.
These get really awkward to model, because we'd have to further split the app into more columns, some may even be conditional/empty depending on outer selections (submodes without nested params).

Another issues that making a change on the outermost selector re-runs the inner callbacks, and thus inputs get re-created, which will reset their state -- for instance all sliders will "forget" their setting, and if the user wants to switch back and forth (A/B testing like) between two parameter sets, the user would have to re-dial *all* the inputs all the time.
This makes the resulting app currently almost unusable for such use cases.

**Goal**

Find solutions that fix these issues and produce a better UX for this use case example.

## Proposal

This is an idea how to approach the problem, not a direct instruction to go in this direction.
The goal is to challenge the idea and/or propose alternative/variations.

The idea is to:

- Change to a flat (non-nested) callback model that always returns a full "View" object that can be comprised of both inputs and outputs.
- The "View" object could take further data/objects to influence layout, decoupling visual representation from reactivity.
- The problem of caching would be pushed entirely to the client/user writing that app (trading flexibility for responsibility of optimization).

A hello world (example 1) may look like:

```py
# The only important convention client code must stick to is to hoist inputs to give them
# stable identities.
slider_a = pa.Slider("a", -10.0, 0.5, 10.0)
slider_b = pa.Slider("b", -10.0, 0.5, 10.0)
slider_c = pa.Slider("c", -10.0, 0.5, 10.0)
negate = pa.Checkbox("Negate")

def callback() -> pa.View: # or pa.Element?
    print(f"{slider_a.value=} {slider_b.value=} {slider_c.value=} {negate.value=}")
    a = slider_a.value
    b = slider_b.value
    c = slider_c.value

    xs = np.linspace(-10.0, 10.0, 100)
    ys = a * xs**2 + b * xs + c

    if negate:
        ys *= -1

    # The exact details how to accomplish row/column layout in terms of the API
    # are subject to be figured out during the planning. For instance, the pa.View
    # and the second pa.VStack maybe be obsolete if all elements inherit pa.View,
    # we may rather go for a pa.Column(s)/pa.Row(s) naming scheme, and we could stick
    # to a default like standard stacking is vertical (like it currently is), so that
    # only column-based layout would require wrapping.
    return pa.View(
        pa.HStack(
          pa.VStack(slider_a, slider_b, slider_c, negate),
          pa.VStack(pa.Plot(xs, ys, x_limits=(-10, +10), y_limits=(-10, +10))),
        )
    )

pa.run(callback)
```

The input hoisting is what enables to have stable identities of input elements.
The old "nested func" example (which would have to be renamed) would turn into something like this:

```py
def main() -> None:

    max_sliders = 10

    master_slider = pa.IntSlider("Number of sliders", 1, 5, max_sliders)
    sliders = [
        pa.Slider(f"coefficient of x^{i}", -10.0, 0.5, 10.0)
        for i in range(max_sliders)
    ]

    def callback() -> pa.View:

        order = master_slider.value
        print(f"Polynomial order: {order}")

        xs = np.linspace(-10.0, 10.0, 100)
        ys = np.zeros_like(xs)
        for k in range(order + 1):
            ys += sliders[k].value * xs**k

        # Here we'd be flexible in how to arrange the inputs vs the one plot output now.
        # - keep master slider in a column, sub-sliders in a column, and one (wider) column for the output plot, as with the old model.
        # - combine the master slider and the sub-sliders into one column (not possible with old programming model), plus the plot output column.
        # - combine everything into one column (or even rows, but that'd look silly).
        return ...

    pa.run(callback)
```

Note that it is not even necessary to initialize all inputs statically and eagerly.
In principle the input instances could be fully dynamic and lazily initialized, as long as the instances are e.g. put into a collection (dict, list, etc.) that lives on the outer level to hold on to the stable instances.

And of course, if preferred, a class based approach would still be possible as an alternative to using a closure over the inputs:

- `__init__` would initialize the inputs and attach them to `self` to replace hoisting.
- `__call__` can make use of the `self` instance inputs.

Coming back to the use case discussed above:
Multiple nesting/conditional levels of inputs should be easily possible with such a programming model, because any conditional input element could just be included or omitted via imperative `if` branch logic.
Even really weird things could be expressed relatively easily:

```py
def callback() -> pa.View:

    active_inputs = [some_slider, some_radio]

    # `some_rarely_used_conditional_slider` appears only under very special conditions:
    if some_slider.value = 42 and some_radio.value == "some specific value":
        active_inputs.append(some_rarely_used_conditional_slider)

    # ...

    return pa.View(pa.Column(active_inputs), outputs)
```

Since `some_rarely_used_conditional_slider` is a stable instance (hoisted) its state is maintained whether the element is active or not, i.e., if it becomes active again, it would take exactly the old value it had before becoming inactive.

**Caching**

Responsibility of dealing with fine grained reactivity is now entirely pushed onto the clients.

Let's take an example scenario where
- output `foo` depends on `slider_a` and `slider_common`,
- output `bar` depends on `slider_b` and `slider_common`,
and we'd like to avoid re-computing outputs `foo` and `bar` if possible (assuming costly).

Clients could simply take care of that manually by putting the computations of `foo` and `bar` under some `functools.lru_cache(maxsize=1)` decorated compute functions.
The callback would call e.g. `compute_foo(slider_a.value, slider_common.value)`, and thanks to the LRU caching the computations could be re-used across callback calls if the inputs for foo or bar stay identical.

This is simple and likely sufficient but has minor drawbacks:
- The optimization "dictates" to wrap all cache-worthy computations into function calls.
- The client side value diffing is somewhat unnecessary overhead, despite typically negligible unless a computation depends on *many* inputs (hard with our scalar input elements).

In principle we could do slightly better because we likely already know *which* input has changed from the ui framework side.
We could forward such `has_changed` information into the callback input instances, allowing to write optimized apps like this:

```py
slider_a = pa.Slider("a", -10.0, 0.5, 10.0)
slider_b = pa.Slider("b", -10.0, 0.5, 10.0)
slider_common = pa.Slider("common", -10.0, 0.5, 10.0)

output_foo: pa.Output | None = None
output_bar: pa.Output | None = None

def callback() -> pa.View:

    if slider_a.has_changed or slider_common.has_changed:
        output_foo = pa.Output(...) # use their `.value` here

    if slider_b.has_changed or slider_common.has_changed:
        output_bar = pa.Output(...) # use their `.value` here

    assert output_foo is not None
    assert output_bar is not None

    return pa.View(slider_a, slider_b, slider_common, output_foo, output_bar)
```

Effectively this would let the API go a bit in the direction of immediate mode UI frameworks and would fit nicely with the envisioned way how to offer simple buttons in the UI.
Sooner or later (perhaps even as part of this task) we want to offer functionality for buttons, which could then look like this:

```py
some_button = pa.Button("Run side-effect", -10.0, 0.5, 10.0)

def callback() -> pa.View:

    if some_button.clicked:
        print("Running side effect...")
        run_side_effect()

    return pa.View(...)
```

Or perhaps a better way to describe the programming model would be to speak of a hybrid of retained mode and immediate mode, because inputs are retained resulting from hoisting, but in terms of the in terms of the interface, it very much feels like an immediate mode UI.

## Open questions

- How exactly should the layout API look like?
  What does gpui offer in terms of "flex" like layout?
  Can we avoid tailoring our API too specifically to gpui -- in case we have to migrate away from gpui it would be nice not having to break our API again.
  How much of a flex interface should we already expose to users?
  Immediate mode UI's are known for being limited in terms of layouting -- are these limitations a concern in our case?
- Are there any implications in terms of caching and diffing outputs at the framework side?
  Caching may not only be a necessary optimization on the Python (client) side, but also on the Rust (ui) side.
  Think of React-like prop updates that flow into the actual UI components.
  We may want to avoid having to re-build (component re-render in terms of React) all the UI (output) components after the Python callback returns.
  If some of the outputs haven't changed, the ui components don't need to be updated.
  Maybe this optimization is less of an concern with a non-DOM based UI (less state?), but this depends on how gpui really works internally.
  If such optimizations are indeed relevant, we may use a convention like:
  Object identities are used as high performant way to indicate to the framework whether an output element should be updated.
  Fresh object instances update, returning the previous object instance skips updating the ui element.
  Motivation: We'd like to avoid having to diff huge numpy array on the framework side to decide if e.g. plot output component must change (think of millions of values).
  This would mean that users have to be aware of this convention though -- mutating a numpy array (changing e.g. a single value) would fail to update the ui component unless the numpy array or its output wrapper is cloned.
  To avoid unnecessary cloning, output components could get marked as "must be updated" by the client callback.
  But such optimization could get dirty quickly and ideally aren't required, but we should discuss these aspects in the planning phase.
- Should we allow inputs to be assigned to?
  In general it would be nice to have some sort of "preset" system, where e.g. clicking a button sets *many* inputs to specific values *from* the callback itself.
  How should such "update from an update" be handled?
  In general the callback must be called again, because changed inputs requires recomputation of the outputs.
  Should we do anything to prevent infinite re-triggers or is it just up to the callback to not re-trigger itself repeatedly?
  Should the callback be allowed to control re-triggering to break signal cycles?
  Are there any standard examples from reactive frameworks of such "retrigger loops" that we should think through?
- Can our programming model deal with advanced components?
  Any limitations?
  How would we e.g. offer a "multi tab" view with the proposed model?

## Prior art

There is a lot of prior art for basically everything discussed above.
Before jumping to any conclusion we should conduct a thorough research of how such questions are solved elsewhere.

We should at least look into:

- Streamlit, which is likely the closest related app framework to what we're doing here (main differences: not browser, same process, GPU optimized).
- Plotly/Dash and Bokeh app interfaces
- Immediate mode UIs like Dear Imgui and egui
- React/Svelte/Solid etc. although their reactivity models have some fundamental difference to what we're trying to accomplish because picoapp (1) does not try to be general purpose UI, but intentionally offers only higher abstractions for specific prototype apps, and (2) the DOM target puts a much stronger emphasis on fine-grained reactivity due the the inherently required state syncing.
- Any other UI framework I'm not familiar with, but has a different model/scope/use-case as what we're doing here?

For each of them we should look into:
- What we can learn about the questions raised above?
- How would they handle the use case example at hand specifically?
- Anything else we can learn or adopt from them in terms of design?

The output of the prior art analysis should be written to `ai/prior_art_programming_model.md`.

