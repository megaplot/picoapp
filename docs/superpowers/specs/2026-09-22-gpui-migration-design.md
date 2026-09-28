# Migrate picoapp's UI from cushy to gpui / gpui-kit

Date: 2026-09-22
Status: approved design, pending implementation plan
Branch: `gpui-migration`

## Goal

Replace cushy (apparently unmaintained) with [gpui](https://github.com/zed-industries/zed) via
[gpui-kit](https://gpui-kit.com/) (`github.com/longbridge/gpui-kit`) as picoapp's only UI framework.
The work starts as a feasibility POC, but all code is production code that stays; there is no
throwaway prototype and no period with two UI frameworks side by side.

As part of the migration, the Python callback moves off the UI thread (a long-standing TODO): slow
callbacks must no longer freeze the window.

### Success criteria

1. cushy, kludgine and plotters are removed from `Cargo.toml`; the app builds against `gpui-kit =0.6.6`.
2. All 8 existing examples run on Linux x86_64 with behavior matching the cushy version.
3. With a slow callback (`examples/example_slow.py`), the UI stays responsive, slider changes coalesce
   (latest value wins), a busy indicator appears, and a Python exception is shown in the UI.
4. The Python API (`picoapp` package, `pa.run`, input/output classes) is unchanged.
5. CI builds wheels for Linux x86_64, macOS x86_64/aarch64 and Windows x64; `cargo test` and `pytest` pass.

### Platform scope

- Linux x86_64: required; verified manually.
- macOS and Windows 64-bit: best-effort; must build in CI, not verified at runtime, do not block merge.
- 32-bit targets (Linux x86, Windows x86): dropped.

## Research findings that shape the design

- gpui-kit 0.6.6 (released 2026-09-21, Apache-2.0) pins gpui through exact snapshot crates
  (`gpui-pre =0.3.6`). Every gpui-kit release pins a new snapshot, so we pin gpui-kit exactly and
  upgrade deliberately. gpui-kit uses Rust edition 2024.
- gpui-kit has `Slider`/`SliderState` (f32 values, built-in `SliderScale::Logarithmic`, emits
  `SliderEvent::Change` while dragging), `Checkbox`, `RadioGroup`, `Button`, `Progress`, `Spinner`.
- gpui-kit's `chart::LineChart` uses a categorical x-axis (`X: Into<SharedString>`, point scale) and is
  unsuitable for numeric `xs/ys` data. The lower-level `plot` module (`Plot` trait with
  `paint(bounds, window, cx)`, `ScaleLinear`, `Line` shape, `PlotAxis`, `Grid`) is suitable. It has no tick
  generator.
- gpui renderers: wgpu 29 on Linux, Metal on macOS, DirectX on Windows. Elements can only emit a closed
  set of primitives (quads, paths, sprites, shadows, underlines; `Surface` is macOS-only). **There is no
  custom-shader hook**, so the shader-based line renderer from megaplot-rs cannot currently plug in.
- gpui images (`RenderImage`) are BGRA frames in a sprite atlas, sampled with **linear filtering**.
  Atlas entries must be freed explicitly (`Window::drop_image`).
- On Linux, `Application::run` returns after the app quits. On macOS, gpui quits through
  `[NSApp terminate:]`, which ends the process, so `pa.run` never returns there.

## Architecture

### Separating UI data from Python handles

Today `Input` mixes the widget description with a `PyObject` handle, and `CallbackReturn::Inputs` carries
`Py` objects into the UI. In the new design **no `Py` object reaches the UI thread**, so the UI thread never
needs the GIL.

- `src/inputs/`: parsing an input produces
  - `InputSpec` (pure Rust: `SliderSpec<f64>`, `SliderSpec<i64>`, `CheckboxSpec`, `RadioSpec`) for the UI, and
  - `InputBinding` (Python handle used to write `_value`) that stays on the worker.
  - `InputValue` enum (`F64`, `I64`, `Bool`, `Index`) is what the UI sends back.
- `src/outputs.rs`: the `Output` enum and its structs stay pure Rust. `Image` additionally carries a
  ready-to-render `Arc<RenderImage>` (built on the worker, see below). `CallbackReturn` is replaced by
  ```rust
  enum LevelResult {
      Outputs(Vec<Output>),
      Nested { level: LevelId, specs: Vec<InputSpec> },
      Error(String),   // message + formatted Python traceback
      Discarded,       // level was dropped before the job ran
  }
  ```
- The Python-side contract (private attribute names, class-name dispatch) is unchanged.

### Module layout

```
src/
  lib.rs, logging_setup.rs, utils.rs     (unchanged)
  py_module.rs                            (+ test hook `_run_worker_job`)
  inputs/                                 (spec/binding split, InputValue)
  outputs.rs                              (LevelResult, BGRA image prep)
  worker.rs                               (new: Python worker thread; no gpui app/entity/window dependency, only the `RenderImage` data type)
  ui/                                     (replaces src/widgets/)
    app.rs                                (run_ui: worker startup, gpui app/window, shutdown)
    reactive_view.rs                      (ReactiveView entity per reactive level)
    run_scheduler.rs                      (coalescing state machine, no gpui types)
    inputs.rs                             (slider, checkbox, radio)
    outputs.rs                            (dispatch Output -> element)
    line_plot.rs, matrix_plot.rs          (custom gpui-kit Plot impls)
    ticks.rs                              (1-2-5 "nice numbers" tick helper)
    image.rs, audio.rs, color_utils.rs
```

`src/main_run_ui.rs` and `src/widgets/` are removed. The Python package and the `pa.run` / `_picoapp.run`
signatures are unchanged.

## Threads and worker protocol

- **Main thread**: runs the gpui event loop with the GIL released (`py.allow_threads`, as today).
- **`picoapp-worker` thread**: performs every Python interaction (value writes, callback calls, output
  parsing). It acquires the GIL per job and releases it between jobs. A single thread gives deterministic
  ordering and thread affinity for libraries like matplotlib that are not safe to call from changing threads.

UI → worker: `std::sync::mpsc` channel of

```rust
enum Job {
    Run { level: LevelId, values: Vec<InputValue>, reply: futures::channel::oneshot::Sender<LevelResult> },
    Drop { level: LevelId },
}
```

The worker owns a registry `LevelId -> (Vec<InputBinding>, Callback)`.

Processing `Run`, under one GIL acquisition:
1. If `level` is not registered, reply `Discarded`.
2. Write `values[i]` into `bindings[i]._value`.
3. Call the callback.
4. Parse the return:
   - `Outputs` -> `LevelResult::Outputs` (images are swizzled RGBA -> BGRA into `Arc<RenderImage>` here, off
     the UI thread).
   - `ReactiveBase` (existing duck typing) -> parse its inputs into specs + bindings, allocate a new
     `LevelId`, register it, reply `Nested`.
   - Any `PyErr` -> print it with traceback to stderr and reply `Error(message + traceback)`.

`Drop { level }` removes the registry entry (drops the `Py` handles under the GIL).

Worker -> UI: the view awaits the `oneshot` receiver inside a `cx.spawn` foreground task and applies the
result via `this.update(cx, ..)` + `cx.notify()`. No polling.

### Startup

`run_ui` parses the root inputs with the GIL held, registers them as `LevelId(0)`, spawns the worker, then
enters gpui inside `py.allow_threads`. Each `ReactiveView` dispatches its initial values on creation
(replaces cushy's implicit initial `for_each` firing).

### Coalescing (`RunScheduler`)

Pure Rust, no gpui types, unit-tested. State: current `values: Vec<InputValue>`, `in_flight: bool`,
`dirty: bool`.

- `on_change(i, v)`: set `values[i] = v`; if not in flight, dispatch a `Run` with a clone of `values`
  and set `in_flight`; otherwise set `dirty`.
- `on_result(result)`: clear `in_flight`; the view applies the result; if `dirty`, clear it and dispatch
  the current `values`.

Slider value labels update immediately on the UI side, independent of the callback.

### Nested levels

A `ReactiveView` holds `child: Option<Entity<ReactiveView>>`. A `Nested` result creates a new child view
and replaces the old one. The old child's `on_release` sends `Drop { level }`. A reply that arrives for a
released view is discarded; if that reply was `Nested`, its newly created level is dropped too, so the
worker never leaks registrations. As today, each parent re-run builds a fresh nested level (child inputs
reset to their initial values).

### Shutdown

- Linux/Windows: when `Application::run` returns, drop the `Job` sender. The worker loop exits, drops its
  registry under the GIL, and `run_ui` joins the worker before returning to Python. If a callback is still
  running, closing the window waits for it to finish.
- macOS: the process terminates on quit; `pa.run` does not return (known limitation).
- If the worker panics, pending `oneshot`s resolve to `Canceled` and views display "worker terminated".

## UI

### Layout and states (`reactive_view.rs`)

- Window 1600x1000, title "pico app", gpui-kit dark theme; `gpui_kit::init` + `gpui_kit::open_window`.
- Each level: fixed 300px sidebar (vertically scrollable `v_flex` of inputs) next to an expanding content
  area. A nested level renders its sidebar + content inside the parent's content area.
- **Busy**: when a job has been in flight for more than ~150 ms, the previous outputs are dimmed and a
  `Spinner` is shown. Inputs stay interactive. The delay avoids flicker for fast callbacks.
- **Error**: a red, selectable text block with the message and traceback above the previous outputs,
  which stay visible. Cleared by the next successful result.

### Inputs (`ui/inputs.rs`)

Each input owns its gpui-kit state entity; changes call `RunScheduler::on_change(i, value)`.

- **Slider / IntSlider**: `SliderState` with `min`, `max`, `default_value = init`;
  `SliderScale::Logarithmic` when `log` is set (replaces `LinLogTransformer`); `step(1.0)` for int
  sliders. Subscribe to `SliderEvent::Change`. Label row: name + value formatted with `decimal_places`.
  Known limitation: `SliderState` is f32 (~7 significant digits; int sliders exact within ±2^24). Accepted
  for now.
- **Checkbox**: `Checkbox::new(id).label(name).checked(v).on_change(..)`.
- **Radio**: `RadioGroup::vertical(id).children(..).selected_index(..).on_change(..)` -> `InputValue::Index`.

### Line plot (`ui/line_plot.rs`)

Custom `Plot` impl: `ScaleLinear` for x/y over `x_limits`/`y_limits`, `Line` shape for the polyline,
`PlotAxis` + `Grid` with ticks from `ticks.rs`. Line clipped to the plot area; colors from the gpui-kit
theme. Minimum size 400x400, otherwise expands (as today).

### Matrix plot (`ui/matrix_plot.rs`)

Custom `Plot` impl: one `paint_quad` per cell with the existing viridis mapping, same axes/ticks. Cells
stay crisp. An image-based variant was rejected because gpui's linear sprite filtering blurs upscaled
cells. Large matrices mean many quads; decimation or a texture path is a follow-up.

### Image (`ui/image.rs`)

Rendered at native size via `img(ImageSource::Render(arc))` using the `Arc<RenderImage>` prepared on the
worker. When a level's outputs are replaced, the view calls `window.drop_image` on the previous images
so the sprite atlas does not grow per callback.

### Audio (`ui/audio.rs`)

`rodio` 0.19 and the thread-local `OutputStream` stay. An `AudioPlayer` entity owns the `Sink` and renders
a play/pause `Button` and a `Progress` bar. A gpui foreground task (`cx.spawn` with a ~16 ms timer)
replaces the monitor thread: it updates progress while playing and ends when playback ends. Replacing
the outputs drops the player, which stops playback.

## Packaging and CI

- `Cargo.toml`: edition 2024; `gpui-kit = "=0.6.6"` (check that its default features do not pull in the
  webview / webkit2gtk); remove `cushy`, `kludgine`, `plotters` and the fontconfig note; add `futures`
  for `oneshot` if gpui-kit does not re-export it. `pyo3` stays at 0.22.
- New Linux build dependencies (documented in CLAUDE.md):
  `libxkbcommon-x11-dev libwayland-dev libx11-xcb-dev libfontconfig-dev clang` in addition to
  `libasound2-dev libdbus-1-dev`.
- `deploy.yml` (already hand-edited despite being generated; fix the CLAUDE.md note accordingly):
  - drop Linux `x86` and Windows `x86`;
  - Linux x86_64: `manylinux: 2_28` (AlmaLinux 8) with the matching `yum install ... -devel` packages;
  - replace the deprecated `macos-13` runner with a currently available Intel runner or cross-build from
    `macos-14`.
  - Risk: auditwheel must bundle or whitelist libxkbcommon / libwayland-client / libxcb. If bundling
    breaks at runtime, exclude them and rely on the system libraries (present on any desktop Linux).
- `checks.yaml`: new apt list, `ubuntu-24.04`, `actions/checkout@v4`, fix the `dpkq-query` typo.

## Testing

- **Rust unit tests** (`cargo test`, no GPU): `RunScheduler` transitions (idle -> in flight -> dirty ->
  redispatch; many changes in flight yield exactly one follow-up run; results for dropped levels),
  tick helper, viridis, RGBA -> BGRA swizzle.
- **Worker tests via pytest**: new test hook `_picoapp._run_worker_job(inputs, callback, values)` (same
  rationale as `_parse_input`; declared in `_picoapp.pyi`) runs one `Run` job through the real worker code
  without gpui and returns a summary. Covers `_value` writes per input type, `Outputs` parsing, nested
  registration, exception -> `Error` with traceback.
- **Manual acceptance on Linux**: all 8 examples; new `examples/example_slow.py` (callback sleeps ~1 s,
  raises above a slider threshold) for coalescing, delayed spinner, dimming, responsiveness and error
  display.
- macOS/Windows: CI wheel builds only.

## Out of scope (follow-ups)

- Shader-based plotting (megaplot-rs): needs a custom-primitive / render-callback API in gpui, per
  backend (wgpu, Metal, DirectX). Separate spec.
- Zero-copy output data (numpy / buffer protocol).
- f64 slider precision.
- Line and matrix decimation for large data.
- Making `pa.run` return on macOS.
- pyo3 upgrade.
- 32-bit targets.
