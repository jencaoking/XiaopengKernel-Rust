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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Position {
    #[default]
    Static,
    Relative,
    Absolute,
    Fixed,
    Sticky,
}

#[derive(Debug, Clone)]
pub struct ComputedStyle {
    pub display: Display,
    pub position: Position,
    pub color: Color,
    pub background_color: Color,
    pub width: Option<f32>,
    pub height: Option<f32>,
    pub top: Option<f32>,
    pub left: Option<f32>,
    pub right: Option<f32>,
    pub bottom: Option<f32>,
    pub z_index: i32,
    
    // Margins
    pub margin_top: f32,
    pub margin_bottom: f32,
    pub margin_left: f32,
    pub margin_right: f32,

    // Padding
    pub padding_top: f32,
    pub padding_bottom: f32,
    pub padding_left: f32,
    pub padding_right: f32,

    // Border Widths
    pub border_top_width: f32,
    pub border_bottom_width: f32,
    pub border_left_width: f32,
    pub border_right_width: f32,
    
    // Border Colors
    pub border_color: Color,
    
    // Typography
    pub font_size: f32,
}

impl Default for ComputedStyle {
    fn default() -> Self {
        Self {
            display: Display::Block,
            position: Position::Static,
            color: Color::BLACK,
            background_color: Color::TRANSPARENT,
            width: None,
            height: None,
            top: None,
            left: None,
            right: None,
            bottom: None,
            z_index: 0,
            
            margin_top: 0.0,
            margin_bottom: 0.0,
            margin_left: 0.0,
            margin_right: 0.0,
            
            padding_top: 0.0,
            padding_bottom: 0.0,
            padding_left: 0.0,
            padding_right: 0.0,
            
            border_top_width: 0.0,
            border_bottom_width: 0.0,
            border_left_width: 0.0,
            border_right_width: 0.0,
            
            border_color: Color::TRANSPARENT,
            font_size: 16.0,
        }
    }
}
