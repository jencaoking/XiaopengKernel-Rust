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
}
