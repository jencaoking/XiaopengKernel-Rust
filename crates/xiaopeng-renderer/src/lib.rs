//! XiaopengKernel Render Engine & Rasterizer Module

use tracing::info;
use xiaopeng_common::XiaopengResult;

pub fn render_frame() -> XiaopengResult<()> {
    info!("Rendering frame");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_frame() {
        assert!(render_frame().is_ok());
    }
}
