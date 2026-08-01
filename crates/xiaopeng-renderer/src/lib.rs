//! XiaopengKernel Render Engine & Rasterizer Module

pub mod canvas;
pub mod font;
pub mod display_list;

pub use canvas::BitmapCanvas;
pub use canvas::gpu::{GpuCanvas, render_display_list_gpu};
pub use display_list::{DisplayList, DisplayCommand};
use tracing::info;
use xiaopeng_common::XiaopengResult;


pub fn render_display_list(display_list: &DisplayList, width: u32, height: u32) -> XiaopengResult<BitmapCanvas> {
    info!("Rendering display list to {}x{} canvas", width, height);
    let mut canvas = BitmapCanvas::new(width, height);

    for command in &display_list.commands {
        match command {
            DisplayCommand::DrawRect { rect, color } => {
                canvas.fill_rect(*rect, *color);
            }
            DisplayCommand::DrawBorder { rect, color, width } => {
                canvas.stroke_rect(*rect, *color, *width);
            }
            DisplayCommand::DrawText { text, rect, color, font_size } => {
                // Wait for FontManager integration, for now just pass None
                canvas.draw_text(text, rect.x, rect.y, *font_size, *color, None);
            }
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

        let mut display_list = DisplayList::new();
        display_list.commands.push(DisplayCommand::DrawRect {
            rect: xiaopeng_common::Rect::new(0.0, 0.0, 100.0, 100.0),
            color: xiaopeng_common::Color::rgb(255, 0, 0),
        });
        let canvas = render_display_list(&display_list, 800, 600).unwrap();
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
