//! XiaopengKernel Render Engine & Rasterizer Module

pub mod canvas;

pub use canvas::BitmapCanvas;
use tracing::info;
use xiaopeng_common::XiaopengResult;

pub fn render_frame() -> XiaopengResult<()> {
    info!("Rendering frame");
    let mut canvas = BitmapCanvas::new(800, 600);
    canvas.fill_rect(
        xiaopeng_common::Rect::new(10.0, 10.0, 100.0, 100.0),
        xiaopeng_common::Color::rgb(255, 0, 0),
    );
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
