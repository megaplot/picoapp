# Typing sketch for the alternative reactive model

Appendix to `ai/alternative_reactive_model.md`, section "Type checking". It is stored as Markdown
so that `mypy .` doesn't pick up its intentional errors. Lines ending in `# E` must produce a type
error; all other lines must not.

Reproduce (in a scratch dir, with this block saved as `sketch.py`):

```sh
uv venv tcvenv && uv pip install -p tcvenv/bin/python mypy pyright
tcvenv/bin/mypy --strict sketch.py
echo '{"typeCheckingMode":"strict"}' > pyrightconfig.json && tcvenv/bin/pyright sketch.py
```

Result with mypy 2.4.0 (`--strict` plus the repo's `mypy.ini` flags, e.g.
`disallow_any_decorated`) and pyright 1.1.414: every `# E` line errors in both. Besides those, there
are exactly two errors: mypy on `Row(*mixed)` (an unannotated input + node list joins to
`object`) and pyright on `inputs.append(int_slider)` (the list is inferred as the union of its
initial element types). Both go away with a `list[Child]` annotation; see the main doc.

```py
"""Type-checking experiment for the alternative reactive model's API.

Lines ending in `# E` must produce a type error; all other lines must not.
"""

from __future__ import annotations

from collections.abc import Callable, Sequence
from typing import Generic, TypeVar

T = TypeVar("T")
T_co = TypeVar("T_co", covariant=True)


# --- API sketch ---------------------------------------------------------------


class Element:
    """Common base of everything that can appear in a view."""


class Output(Element):
    pass


class Plot(Output):
    pass


class Audio(Output):
    pass


class InputBase(Element):
    pass


class Input(InputBase, Generic[T_co]):
    def __init__(self, value: T_co) -> None:
        self._value = value

    @property
    def value(self) -> T_co:
        return self._value


class Slider(Input[float]):
    def __init__(self) -> None:
        super().__init__(0.0)


class IntSlider(Input[int]):
    def __init__(self) -> None:
        super().__init__(0)


class Checkbox(Input[bool]):
    def __init__(self) -> None:
        super().__init__(False)


class Radio(Input[T]):
    def __init__(self, values: Sequence[T]) -> None:
        super().__init__(values[0])


class Memoized(Generic[T_co]):
    def __init__(self, fn: Callable[[], T_co]) -> None:
        self._fn = fn

    def __call__(self) -> T_co:
        return self._fn()


def memoize(fn: Callable[[], T]) -> Memoized[T]:
    return Memoized(fn)


# What a view slot accepts: a plain element or a node producing an element.
Child = Element | Memoized[Element]


class Layout(Element):
    def __init__(self, *children: Child) -> None:
        self.children = children


class Row(Layout):
    pass


class Column(Layout):
    pass


# --- user code: positive cases ---------------------------------------------------

slider = Slider()
int_slider = IntSlider()
checkbox = Checkbox()
radio = Radio(["a", "b"])
radio_value: str = radio.value


@memoize
def plot_node() -> Plot:
    return Plot()


@memoize
def audio_node() -> Audio:
    return Audio()


@memoize
def dataset() -> list[float]:  # general memoization: non-element value
    return [slider.value]


def plot_fn() -> Plot:
    return Plot()


# Heterogeneous varargs: inputs, outputs, nodes, nested layouts.
Row(
    slider,
    radio,
    Plot(),
    plot_node,
    audio_node,
    memoize(lambda: Plot()),
    memoize(lambda: Audio()),
    memoize(plot_fn),
    Column(checkbox, memoize(lambda: Row(plot_node))),
)

# Heterogeneous list literals, unpacked.
node_list = [memoize(lambda: Plot()), memoize(lambda: Audio())]
Row(*node_list)

nodes = [plot_node, audio_node]
Row(*nodes)

inputs = [slider, radio, checkbox]
inputs.append(int_slider)
Column(*inputs)

mixed = [slider, plot_node]
Row(*mixed)

mixed_annotated: list[Child] = [slider, plot_node, memoize(lambda: Audio())]
mixed_annotated.append(Plot())
Row(*mixed_annotated)

node_dict = {"plot": plot_node, "audio": audio_node}
Row(*node_dict.values())

Row(plot_node if checkbox.value else audio_node)
Row(*(memoize(lambda: Plot()) for _ in range(3)))

# Variance: a Memoized[Plot] is a Memoized[Output].
as_output: Memoized[Output] = plot_node

# --- user code: negative cases ---------------------------------------------------

Row(42)  # E
Row(lambda: Plot())  # E
Row(dataset)  # E
Row(memoize(lambda: "text"))  # E


def int_fn() -> int:
    return 1


Row(int_fn)  # E

bad_lambdas = [lambda: 1, lambda: Plot()]
Row(*bad_lambdas)  # E

bad_dict = {"plot": plot_node, "data": dataset}
Row(*bad_dict.values())  # E
```
