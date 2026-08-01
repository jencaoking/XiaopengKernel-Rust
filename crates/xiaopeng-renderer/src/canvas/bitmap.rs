//! Software Rasterizer Bitmap Canvas & PPM Exporter

use xiaopeng_common::{Color, Rect};

pub struct BitmapCanvas {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<Color>,
}

impl BitmapCanvas {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            pixels: vec![Color::WHITE; (width * height) as usize],
        }
    }

    pub fn fill_rect(&mut self, rect: Rect, color: Color) {
        let x_start = rect.x.max(0.0) as u32;
        let y_start = rect.y.max(0.0) as u32;
        let x_end = (rect.x + rect.width).min(self.width as f32) as u32;
        let y_end = (rect.y + rect.height).min(self.height as f32) as u32;

        for y in y_start..y_end {
            for x in x_start..x_end {
                let idx = (y * self.width + x) as usize;
                if idx < self.pixels.len() {
                    self.pixels[idx] = color;
                }
            }
        }
    }

    pub fn export_ppm(&self) -> String {
        let mut ppm = format!("P3\n{} {}\n255\n", self.width, self.height);
        for color in &self.pixels {
            ppm.push_str(&format!("{} {} {} ", color.r, color.g, color.b));
        }
        ppm
    }
}
