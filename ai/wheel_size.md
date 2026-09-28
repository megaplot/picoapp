# Wheel size: cushy vs. gpui

Measured 2026-09-28 on Linux x86_64, CPython 3.12, `maturin build --release`,
comparing `main` (988518b, cushy) against `gpui-migration`.

| | wheel (compressed) | `_picoapp.so` (uncompressed) | `picoapp.libs/` (bundled shared libs) |
|---|---|---|---|
| `main` (cushy) | 9.5 MB | 25 MB | ~4.2 MB (freetype, png, dbus, systemd, brotli, bz2, cap, asound) |
| `gpui-migration` | 17 MB | 51 MB | ~2.1 MB (xkbcommon, xcb, Xdmcp, Xau, asound) |

The migration roughly **doubles the wheel** (~1.8×). The bundled external
libs are actually smaller on the gpui side; the increase is entirely the
extension binary, driven by gpui/wgpu's larger dependency tree. Neither
version's `picoapp.libs/` set is a lever we control much — they're transitive
runtime deps of gpui's winit-like windowing/input stack vs. plotters'
`fontconfig-dlopen` font stack, pulled in by `maturin build`'s auditwheel-
style repair.

## `strip = true`: real size win, but destroys `RUST_BACKTRACE`

Neither branch's `[profile.release]` sets `strip`, so both binaries ship
`not stripped` (checked via `file`). Adding `strip = true` gives a real,
free-looking reduction:

| | wheel, stripped |
|---|---|
| `main` (cushy) | 8.6 MB (from 9.5 MB) |
| `gpui-migration` | 15 MB (from 17 MB) |

**But it is not free.** Verified directly: built a `#[pyfunction]` that
panics inside the extension and called it with `RUST_BACKTRACE=1`.

- Unstripped: full symbolized backtrace, including our own frames
  (`picoapp::py_module::__pyfunction_panic_test`, the `#[pymodule]`
  trampoline, down through `PyEval_EvalCode` to `main`).
- Stripped: `stack backtrace:` followed by *no frames at all* — stripping
  removes what the `backtrace` crate needs to walk the stack, not just the
  human-readable names.

So `strip = true` isn't a pure win: it trades ~10% off the wheel for making
a Rust-side panic (as opposed to a Python exception, which is unaffected —
those go through `format_traceback` in `outputs.rs`, not a Rust panic)
undebuggable via `RUST_BACKTRACE`. Decide deliberately if/when this is
proposed, not as a drive-by "free" optimization.

## Aside: a Rust panic already violates "picoapp prints nothing by default"

Not new to this investigation, but surfaced by triggering the test panic
above: an actual Rust panic (as opposed to a Python exception raised in the
user's callback, which never reaches Rust's panic machinery) prints to
stderr via Rust's default panic hook *before* pyo3's `catch_unwind` turns it
into a `PanicException` — regardless of picoapp's own logging setup
(`logging_setup.rs` only configures `tracing`, not the panic hook). This
only fires on an actual Rust-side bug (an `.unwrap()`/`.expect()`/index panic
in our own code, not anything reachable from normal callback errors), so it
hasn't been a real-world issue, but the "no output by default" claim in
CLAUDE.md is only true as long as no Rust code panics. Not filed as a
cleanup item yet since no known callback path triggers a real panic today.
