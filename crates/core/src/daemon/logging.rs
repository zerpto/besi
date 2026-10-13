use tracing_subscriber::EnvFilter;

/// Log to stderr, keeping stdout for command output. Level comes from `RUST_LOG` (default `info`), e.g.
/// `RUST_LOG=debug` or `RUST_LOG=tower_http=debug,info`.
pub(crate) fn init() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    // Ignore the error if a subscriber is already installed.
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .try_init();
}
