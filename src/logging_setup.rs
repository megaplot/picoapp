use tracing_subscriber::EnvFilter;

/// Silent by default: a picoapp's stdout/stderr belong to the user's Python
/// callback, so nothing from the UI stack (gpui, wgpu, winit-style platform
/// warnings) may leak into them. Set `RUST_LOG` (e.g. `RUST_LOG=info`) to
/// opt back into log output when debugging picoapp itself.
pub fn setup_logging() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("off"));
    // `try_init`: calling `pa.run` twice in one process must not panic on
    // an already-installed global subscriber.
    let _ = tracing_subscriber::fmt().with_env_filter(filter).try_init();
}
