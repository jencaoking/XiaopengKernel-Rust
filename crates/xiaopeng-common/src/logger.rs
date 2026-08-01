use tracing_subscriber::{fmt, EnvFilter};

/// Initialize tracing subscriber with RUST_LOG environment filter support
pub fn init_logging() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,xiaopeng=debug"));

    let _ = fmt::Subscriber::builder()
        .with_env_filter(filter)
        .with_target(true)
        .with_thread_ids(true)
        .with_line_number(true)
        .try_init();
}
