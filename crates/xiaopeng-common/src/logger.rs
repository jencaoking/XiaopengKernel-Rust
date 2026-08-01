//! Tracing logging & diagnostics infrastructure for XiaopengKernel

use tracing_subscriber::{fmt, EnvFilter};

#[derive(Debug, Clone)]
pub struct LoggerConfig {
    pub default_level: String,
    pub show_thread_ids: bool,
    pub show_line_numbers: bool,
    pub show_target: bool,
    pub enable_ansi: bool,
}

impl Default for LoggerConfig {
    fn default() -> Self {
        Self {
            default_level: "info,xiaopeng=debug".to_string(),
            show_thread_ids: true,
            show_line_numbers: true,
            show_target: true,
            enable_ansi: true,
        }
    }
}

/// Initialize tracing subscriber with custom configuration
pub fn init_logging_with_config(config: LoggerConfig) {
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(&config.default_level));

    let builder = fmt::Subscriber::builder()
        .with_env_filter(filter)
        .with_target(config.show_target)
        .with_thread_ids(config.show_thread_ids)
        .with_line_number(config.show_line_numbers)
        .with_ansi(config.enable_ansi);

    let _ = builder.try_init();
}

/// Initialize tracing subscriber with default configuration
pub fn init_logging() {
    init_logging_with_config(LoggerConfig::default());
}
