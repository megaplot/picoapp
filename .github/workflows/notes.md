Error when using `container: off`

```
error: failed to run custom build command for `yeslogic-fontconfig-sys v5.0.0`

Caused by:
  process didn't exit successfully: `/home/runner/work/picoapp/picoapp/target/release/build/yeslogic-fontconfig-sys-c85507faacfc0d52/build-script-build` (exit status: 101)
  --- stdout
  cargo:rerun-if-env-changed=RUST_FONTCONFIG_DLOPEN
  cargo:rerun-if-env-changed=FONTCONFIG_NO_PKG_CONFIG
  cargo:rerun-if-env-changed=PKG_CONFIG_ALLOW_CROSS_i686-unknown-linux-gnu
  cargo:rerun-if-env-changed=PKG_CONFIG_ALLOW_CROSS_i686_unknown_linux_gnu
  cargo:rerun-if-env-changed=TARGET_PKG_CONFIG_ALLOW_CROSS
  cargo:rerun-if-env-changed=PKG_CONFIG_ALLOW_CROSS
  cargo:rerun-if-env-changed=PKG_CONFIG_i686-unknown-linux-gnu
  cargo:rerun-if-env-changed=PKG_CONFIG_i686_unknown_linux_gnu
  cargo:rerun-if-env-changed=TARGET_PKG_CONFIG
  cargo:rerun-if-env-changed=PKG_CONFIG
  cargo:rerun-if-env-changed=PKG_CONFIG_SYSROOT_DIR_i686-unknown-linux-gnu
  cargo:rerun-if-env-changed=PKG_CONFIG_SYSROOT_DIR_i686_unknown_linux_gnu
  cargo:rerun-if-env-changed=TARGET_PKG_CONFIG_SYSROOT_DIR
  cargo:rerun-if-env-changed=PKG_CONFIG_SYSROOT_DIR

  --- stderr
  thread 'main' panicked at /home/runner/.cargo/registry/src/index.crates.io-6f17d22bba15001f/yeslogic-fontconfig-sys-5.0.0/build.rs:8:48:
  called `Result::unwrap()` on an `Err` value: "pkg-config has not been configured to support cross-compilation.\n\nInstall a sysroot for the target platform and configure it via\nPKG_CONFIG_SYSROOT_DIR and PKG_CONFIG_PATH, or install a\ncross-compiling wrapper for pkg-config and set it via\nPKG_CONFIG environment variable."
  note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
warning: build failed, waiting for other jobs to finish...
💥 maturin failed
  Caused by: Failed to build a native library through cargo
  Caused by: Cargo build finished with "exit status: 101": `env -u CARGO PYO3_CONFIG_FILE="/home/runner/work/picoapp/picoapp/target/maturin/pyo3-config-i686-unknown-linux-gnu-3.8.txt" "cargo" "rustc" "--features" "pyo3/extension-module" "--target" "i686-unknown-linux-gnu" "--message-format" "json-render-diagnostics" "--manifest-path" "/home/runner/work/picoapp/picoapp/Cargo.toml" "--release" "--lib"`
Error: The process '/home/runner/work/_temp/325b8d5e-3745-46f9-a2ae-e7ef1539c859/maturin' failed with exit code 1
    at ExecState._setResult (/home/runner/work/_actions/PyO3/maturin-action/v1/dist/index.js:1702:25)
    at ExecState.CheckComplete (/home/runner/work/_actions/PyO3/maturin-action/v1/dist/index.js:1685:18)
    at ChildProcess.<anonymous> (/home/runner/work/_actions/PyO3/maturin-action/v1/dist/index.js:1579:27)
    at ChildProcess.emit (node:events:519:28)
    at maybeClose (node:internal/child_process:1105:16)
    at ChildProcess._handle.onexit (node:internal/child_process:305:5)
```