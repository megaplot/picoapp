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
