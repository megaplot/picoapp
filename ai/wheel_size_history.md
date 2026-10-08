# Wheel size history

A crude, ad-hoc measurement log — not automated per-commit, just enough to
spot trends and regressions over time via `git log -p -- ai/wheel_size_history.md`
(or `git blame`, to find which change a jump landed in). Append a row with
`scripts/measure_wheel_size` after a non-trivial change to `src/` or to
`Cargo.toml`'s dependencies; see CLAUDE.md for when to suggest it.

No git commit/branch column: a feature branch's own commits don't survive a
squash merge, so a recorded SHA could point at nothing. The timestamp is
what a row's own commit already carries in `git log`, on whichever branch
that commit ends up landing on.

Same-platform rows are the only ones directly comparable — see
`ai/wheel_size.md` for why (bundled shared libs differ a lot by OS).

| timestamp (UTC) | picoapp version | platform (rustc host) | size | bytes |
|---|---|---|---|---|
| 2026-09-29T13:54Z | 0.3.0 | x86_64-unknown-linux-gnu | 17M | 17379730 |
| 2026-10-08T16:45Z | 0.4.0 | x86_64-unknown-linux-gnu | 18M | 17987139 |
