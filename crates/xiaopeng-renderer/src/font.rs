//! Pure Rust Text Shaping (Rustybuzz) and Font Rasterization (Swash)

use rustybuzz::{Face, UnicodeBuffer};
use swash::scale::{ScaleContext, Render};
use swash::scale::image::Image;
use swash::FontRef;

use std::collections::HashMap;
use std::sync::Mutex;

pub struct FontManager {
    font_data: Vec<u8>,
    scaler_cache: Mutex<ScaleCache>,
}

struct ScaleCache {
    context: ScaleContext,
    glyph_cache: HashMap<(u16, u32), Image>,
}

pub struct ShapedText {
    pub width: f32,
    pub height: f32,
    pub glyphs: Vec<ShapedGlyph>,
}

pub struct ShapedGlyph {
    pub glyph_id: u16,
    pub x_offset: f32,
    pub y_offset: f32,
    pub x_advance: f32,
    pub y_advance: f32,
    pub cluster: u32,
}

impl FontManager {
    pub fn new(font_data: Vec<u8>) -> Self {
        Self { 
            font_data,
            scaler_cache: Mutex::new(ScaleCache {
                context: ScaleContext::new(),
                glyph_cache: HashMap::new(),
            }),
        }
    }

    /// Shapes text into glyphs using rustybuzz.
    pub fn shape_text(&self, text: &str, font_size: f32) -> Result<ShapedText, String> {
        let face = Face::from_slice(&self.font_data, 0).ok_or("Failed to parse font")?;
        
        let mut buffer = UnicodeBuffer::new();
        buffer.push_str(text);
        buffer.set_direction(rustybuzz::Direction::LeftToRight);
        
        let glyph_buffer = rustybuzz::shape(&face, &[], buffer);
        
        let positions = glyph_buffer.glyph_positions();
        let infos = glyph_buffer.glyph_infos();
        
        // Font metrics
        let upem = face.units_per_em() as f32;
        let scale = font_size / upem;
        
        let mut shaped_glyphs = Vec::with_capacity(infos.len());
        let mut current_x = 0.0;
        let mut max_y_advance = 0.0_f32;
        
        for (info, pos) in infos.iter().zip(positions.iter()) {
            let x_adv = pos.x_advance as f32 * scale;
            let y_adv = pos.y_advance as f32 * scale;
            
            shaped_glyphs.push(ShapedGlyph {
                glyph_id: info.glyph_id as u16,
                x_offset: pos.x_offset as f32 * scale,
                y_offset: pos.y_offset as f32 * scale,
                x_advance: x_adv,
                y_advance: y_adv,
                cluster: info.cluster,
            });
            
            current_x += x_adv;
            max_y_advance = max_y_advance.max(y_adv);
        }
        
        let height = if max_y_advance > 0.0 { max_y_advance } else { font_size };

        Ok(ShapedText {
            width: current_x,
            height,
            glyphs: shaped_glyphs,
        })
    }

    /// Rasterizes a glyph ID into an image using swash, with caching.
    pub fn rasterize_glyph(&self, glyph_id: u16, font_size: f32) -> Result<Image, String> {
        let cache_key = (glyph_id, font_size.to_bits());
        
        let mut cache = self.scaler_cache.lock().expect("Unwrap failed");
        if let Some(img) = cache.glyph_cache.get(&cache_key) {
            return Ok(img.clone());
        }

        let font = FontRef::from_index(&self.font_data, 0).ok_or("Failed to load swash font")?;
        
        let mut scaler = cache.context
            .builder(font)
            .size(font_size)
            .hint(true)
            .build();
            
        let img = Render::new(&[
            swash::scale::Source::ColorOutline(0),
            swash::scale::Source::Outline,
        ])
        .render(&mut scaler, glyph_id)
        .ok_or_else(|| "Failed to rasterize glyph".to_string())?;
        
        cache.glyph_cache.insert(cache_key, img.clone());
        Ok(img)
    }
}
