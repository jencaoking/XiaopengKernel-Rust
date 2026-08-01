//! Layout Box & Box Model Dimensions

use xiaopeng_common::Rect;
use xiaopeng_style::ComputedStyle;

#[derive(Debug, Clone, Default)]
pub struct EdgeSizes {
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
}

#[derive(Debug, Clone, Default)]
pub struct Dimensions {
    pub content: Rect,
    pub padding: EdgeSizes,
    pub border: EdgeSizes,
    pub margin: EdgeSizes,
}

impl Dimensions {
    pub fn padding_box(&self) -> Rect {
        Rect::new(
            self.content.x - self.padding.left,
            self.content.y - self.padding.top,
            self.content.width + self.padding.left + self.padding.right,
            self.content.height + self.padding.top + self.padding.bottom,
        )
    }

    pub fn border_box(&self) -> Rect {
        let p_box = self.padding_box();
        Rect::new(
            p_box.x - self.border.left,
            p_box.y - self.border.top,
            p_box.width + self.border.left + self.border.right,
            p_box.height + self.border.top + self.border.bottom,
        )
    }

    pub fn margin_box(&self) -> Rect {
        let b_box = self.border_box();
        Rect::new(
            b_box.x - self.margin.left,
            b_box.y - self.margin.top,
            b_box.width + self.margin.left + self.margin.right,
            b_box.height + self.margin.top + self.margin.bottom,
        )
    }
}

#[derive(Debug)]
pub struct LayoutBox {
    pub dimensions: Dimensions,
    pub style: ComputedStyle,
    pub children: Vec<LayoutBox>,
}

impl LayoutBox {
    pub fn new(style: ComputedStyle) -> Self {
        Self {
            dimensions: Dimensions::default(),
            style,
            children: Vec::new(),
        }
    }
}
