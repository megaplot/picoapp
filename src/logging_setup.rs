use tracing_subscriber::EnvFilter;

/// Silent by default: a picoapp's stdout/stderr belong to the user's Python
/// callback, so nothing from the UI stack (gpui, wgpu, winit-style platform
/// warnings) may leak into them. Set `RUST_LOG` (e.g. `RUST_LOG=info`) to
/// opt back into log output when debugging picoapp itself.
///
/// Filtering on "warn" instead of turning logging off entirely would not be
/// enough anyway: a number of these warnings are by design (see
/// https://github.com/tokio-rs/tracing/issues/3025).
pub fn setup_logging() {
    // For valid `RUST_LOG` patterns see: https://docs.rs/tracing-subscriber/latest/tracing_subscriber/filter/struct.EnvFilter.html#directives
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("off"));
    // `try_init`: calling `pa.run` twice in one process must not panic on
    // an already-installed global subscriber.
    let _ = tracing_subscriber::fmt().with_env_filter(filter).try_init();
}
