//! XiaopengKernel CSS Style & Cascade Resolver Module

use tracing::info;
use xiaopeng_common::XiaopengResult;

pub fn init_style() -> XiaopengResult<()> {
    info!("Style module initialized");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init_style() {
        assert!(init_style().is_ok());
    }
}
