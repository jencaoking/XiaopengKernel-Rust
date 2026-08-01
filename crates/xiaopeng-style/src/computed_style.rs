//! Computed Style Data Structure

use xiaopeng_common::Color;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Display {
    Block,
    Inline,
    Flex,
    Grid,
    None,
}

#[derive(Debug, Clone)]
pub struct ComputedStyle {
    pub display: Display,
    pub color: Color,
    pub background_color: Color,
    pub width: Option<f32>,
    pub height: Option<f32>,
    pub z_index: i32,
}

impl Default for ComputedStyle {
    fn default() -> Self {
        Self {
            display: Display::Block,
            color: Color::BLACK,
            background_color: Color::TRANSPARENT,
            width: None,
            height: None,
            z_index: 0,
        }
    }
}
