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

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CssLength {
    Auto,
    Px(f32),
    Percent(f32),
    Em(f32),
    Rem(f32),
    Vh(f32),
    Vw(f32),
    Inherit,
}

impl Default for CssLength {
    fn default() -> Self {
        CssLength::Px(0.0)
    }
}

impl CssLength {
    pub fn to_px(&self, parent_size: f32) -> Option<f32> {
        match self {
            CssLength::Px(v) => Some(*v),
            CssLength::Percent(v) => Some(*v / 100.0 * parent_size),
            CssLength::Em(v) | CssLength::Rem(v) => Some(*v * 16.0),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ComputedStyle {
    pub display: Display,
    pub position: Position,
    pub color: Color,
    pub background_color: Color,
    pub width: CssLength,
    pub height: CssLength,
    pub top: CssLength,
    pub left: CssLength,
    pub right: CssLength,
    pub bottom: CssLength,
    pub z_index: i32,
    
    // Margins
    pub margin_top: CssLength,
    pub margin_bottom: CssLength,
    pub margin_left: CssLength,
    pub margin_right: CssLength,

    // Padding
    pub padding_top: CssLength,
    pub padding_bottom: CssLength,
    pub padding_left: CssLength,
    pub padding_right: CssLength,

    // Border Widths
    pub border_top_width: CssLength,
    pub border_bottom_width: CssLength,
    pub border_left_width: CssLength,
    pub border_right_width: CssLength,
    
    // Border Colors
    pub border_color: Color,
    
    // Typography
    pub font_size: f32, // Font size is usually resolved to px during style resolution
}

impl Default for ComputedStyle {
    fn default() -> Self {
        Self {
            display: Display::Block,
            position: Position::Static,
            color: Color::BLACK,
            background_color: Color::TRANSPARENT,
            width: CssLength::Auto,
            height: CssLength::Auto,
            top: CssLength::Auto,
            left: CssLength::Auto,
            right: CssLength::Auto,
            bottom: CssLength::Auto,
            z_index: 0,
            
            margin_top: CssLength::Px(0.0),
            margin_bottom: CssLength::Px(0.0),
            margin_left: CssLength::Px(0.0),
            margin_right: CssLength::Px(0.0),
            
            padding_top: CssLength::Px(0.0),
            padding_bottom: CssLength::Px(0.0),
            padding_left: CssLength::Px(0.0),
            padding_right: CssLength::Px(0.0),
            
            border_top_width: CssLength::Px(0.0),
            border_bottom_width: CssLength::Px(0.0),
            border_left_width: CssLength::Px(0.0),
            border_right_width: CssLength::Px(0.0),
            
            border_color: Color::TRANSPARENT,
            font_size: 16.0,
        }
    }
}
