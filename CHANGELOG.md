# Changelog

## 0.3.1

- Bumped Rust dependencies (pyo3, rodio, gpui-kit and others) to their latest versions

## 0.3.0

- Migrated the UI from cushy to gpui-kit (native GPU-rendered UI)
- Added image output, including matplotlib figures
- Added matrix-plot output
- Bumped minimum supported macOS version

## 0.2.1

- Updated the cushy UI dependency

## 0.2.0

- Added checkbox and radio button inputs
- Added log-scale sliders and configurable decimal places for sliders
- Added initial (crude) audio output support
- Added dark mode
- Added initial window-size control

## 0.1.0

- Initial prototype: Python callbacks driving reactive sliders and a live-updating plot, UI built on cushy
- Support for nested reactive callbacks (a callback can itself return another reactive app)
- Basic packaging (maturin/PyO3) and release CI
