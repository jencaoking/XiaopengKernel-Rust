//! Software Rasterizer Canvas via Tiny-Skia

use tiny_skia::{Color as SkiaColor, Paint, Pixmap, Rect as SkiaRect, Transform};
use xiaopeng_common::{Color, Rect};

pub struct BitmapCanvas {
    pub pixmap: Pixmap,
}

impl BitmapCanvas {
    pub fn new(width: u32, height: u32) -> Self {
        let mut pixmap = Pixmap::new(width, height).expect("Failed to create tiny-skia pixmap");
        pixmap.fill(SkiaColor::WHITE);
        Self { pixmap }
    }

    pub fn fill_rect(&mut self, rect: Rect, color: Color) {
        if rect.width <= 0.0 || rect.height <= 0.0 {
            return;
        }

        let skia_rect = SkiaRect::from_xywh(rect.x, rect.y, rect.width, rect.height);
        if let Some(r) = skia_rect {
            let mut paint = Paint::default();
            paint.set_color_rgba8(color.r, color.g, color.b, color.a);
            paint.anti_alias = true;
            
            self.pixmap.fill_rect(r, &paint, Transform::identity(), None);
        }
    }

    /// Draws a border around a rectangle.
    pub fn stroke_rect(&mut self, rect: Rect, color: Color, width: f32) {
        if rect.width <= 0.0 || rect.height <= 0.0 || width <= 0.0 {
            return;
        }

        let skia_rect = SkiaRect::from_xywh(rect.x, rect.y, rect.width, rect.height);
        if let Some(r) = skia_rect {
            let mut paint = Paint::default();
            paint.set_color_rgba8(color.r, color.g, color.b, color.a);
            paint.anti_alias = true;
            
            let stroke = tiny_skia::Stroke {
                width,
                ..Default::default()
            };
            
            let mut pb = tiny_skia::PathBuilder::new();
            pb.push_rect(r);
            if let Some(path) = pb.finish() {
                self.pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
            }
        }
    }

    pub fn export_png(&self, path: &str) -> Result<(), String> {
        self.pixmap.save_png(path).map_err(|e| e.to_string())
    }

    pub fn draw_text(&mut self, text: &str, x: f32, y: f32, font_size: f32, color: Color, font_manager: Option<&crate::font::FontManager>) {
        let Some(fm) = font_manager else { return; };
        let Ok(shaped) = fm.shape_text(text, font_size) else { return; };
        
        let mut current_x = x;
        let baseline = y + font_size * 0.8; // approximate baseline
        
        for glyph in shaped.glyphs {
            if let Ok(img) = fm.rasterize_glyph(glyph.glyph_id, font_size) {
                if img.placement.width > 0 && img.placement.height > 0 {
                    let mut glyph_pixmap = Pixmap::new(img.placement.width, img.placement.height).unwrap();
                    let pixels = glyph_pixmap.pixels_mut();
                    
                    // Simple assume 1-byte mask from ColorOutline(0) if len matches width*height
                    if img.data.len() == (img.placement.width * img.placement.height) as usize {
                        for (i, &alpha) in img.data.iter().enumerate() {
                            let a = ((alpha as u16 * color.a as u16) / 255) as u8;
                            let c = tiny_skia::ColorU8::from_rgba(color.r, color.g, color.b, a);
                            pixels[i] = c.premultiply();
                        }
                    }
                    
                    let draw_x = current_x + glyph.x_offset + img.placement.left as f32;
                    let draw_y = baseline + glyph.y_offset - img.placement.top as f32;
                    
                    self.pixmap.draw_pixmap(
                        draw_x as i32,
                        draw_y as i32,
                        glyph_pixmap.as_ref(),
                        &tiny_skia::PixmapPaint::default(),
                        tiny_skia::Transform::identity(),
                        None
                    );
                }
            }
            current_x += glyph.x_advance;
        }
    }
}
