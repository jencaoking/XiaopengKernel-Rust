//! XiaopengKernel Render Engine & Rasterizer Module

pub mod canvas;
pub mod font;

pub use canvas::BitmapCanvas;
pub use canvas::gpu::{GpuCanvas, render_display_list_gpu};
use tracing::info;
use xiaopeng_common::XiaopengResult;

use xiaopeng_layout::LayoutBox;

pub fn render_display_list(display_list: &[&LayoutBox], width: u32, height: u32) -> XiaopengResult<BitmapCanvas> {
    info!("Rendering display list to {}x{} canvas", width, height);
    let mut canvas = BitmapCanvas::new(width, height);

    for box_ in display_list {
        let content_rect = box_.dimensions.border_box();
        
        // 1. Draw Background
        canvas.fill_rect(content_rect, box_.style.background_color);

        // 2. Draw Borders (simple border implementation)
        // A complete engine would handle individual border widths and colors.
        let border_width = box_.dimensions.border.top.max(box_.dimensions.border.left);
        if border_width > 0.0 {
            canvas.stroke_rect(content_rect, box_.style.color, border_width); // Using foreground color as border color for now
        }

        // 3. Draw Text (stubbed - tiny_skia doesn't do text natively, we'd need rusttype/fontdue)
        if let xiaopeng_layout::layout_box::BoxType::TextNode(ref text) = box_.box_type {
            // Text rendering is skipped in this basic software rasterizer, 
            // but we could draw text-bounding debug boxes if needed.
            let _ = text;
        }
    }

    Ok(canvas)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_display_list() {
        use xiaopeng_layout::layout_box::{LayoutBox, BoxType};
        use xiaopeng_style::computed_style::ComputedStyle;

        let mut style1 = ComputedStyle::default();
        style1.background_color = xiaopeng_common::Color::rgb(255, 0, 0);
        let mut box1 = LayoutBox::new(style1, BoxType::BlockNode, None);
        box1.dimensions.content.width = 100.0;
        box1.dimensions.content.height = 100.0;

        let list = vec![&box1];
        let canvas = render_display_list(&list, 800, 600).unwrap();
        assert_eq!(canvas.pixmap.width(), 800);
        assert_eq!(canvas.pixmap.height(), 600);
    }

    #[test]
    fn test_render_display_list_gpu() {
        use xiaopeng_layout::layout_box::{LayoutBox, BoxType};
        use xiaopeng_style::computed_style::ComputedStyle;

        let mut style1 = ComputedStyle::default();
        style1.background_color = xiaopeng_common::Color::rgb(0, 255, 0); // Green
        let mut box1 = LayoutBox::new(style1, BoxType::BlockNode, None);
        box1.dimensions.content.width = 50.0;
        box1.dimensions.content.height = 50.0;

        let list = vec![&box1];
        let gpu_canvas = render_display_list_gpu(&list, 200, 200).unwrap();
        assert_eq!(gpu_canvas.width, 200);
        assert_eq!(gpu_canvas.height, 200);
        assert_eq!(gpu_canvas.pixels.len(), 200 * 200 * 4);
    }
}
