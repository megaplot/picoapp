use tracing_subscriber::EnvFilter;

pub fn setup_logging() {
    // Note that filtering on "warn" is not sufficient, because there seem to be
    // a number of warnings "by design". In principle, we could set it to "error"
    // only during app setup, and change to "warn" later, but I the reload mechanism
    // seems broken and/or has poor DX.
    // Something like the following could work:
    // https://github.com/tokio-rs/tracing/issues/3025
    tracing_subscriber::fmt()
        // For valid patterns see: https://docs.rs/tracing-subscriber/latest/tracing_subscriber/filter/struct.EnvFilter.html#directives
        .with_env_filter(
            EnvFilter::from_default_env()
                .add_directive("info".parse().unwrap())
                .add_directive("winit=warn".parse().unwrap())
                .add_directive("naga=warn".parse().unwrap())
                .add_directive("wgpu=error".parse().unwrap()),
        )
        .init();
}
