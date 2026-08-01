//! Console log binding stubs

use tracing::info;

pub fn console_log(message: &str) {
    info!("[JS Console]: {}", message);
}
