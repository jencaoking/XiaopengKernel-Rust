//! Layout Box & Box Model Dimensions

use xiaopeng_common::Rect;
use xiaopeng_style::ComputedStyle;
use xiaopeng_dom::NodePtr;

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BoxType {
    BlockNode,
    InlineNode,
    AnonymousBlock,
    TextNode(String),
}

#[derive(Debug)]
pub struct LayoutBox {
    pub dimensions: Dimensions,
    pub style: ComputedStyle,
    pub children: Vec<LayoutBox>,
    pub box_type: BoxType,
    pub node: Option<NodePtr>,
}

impl LayoutBox {
    pub fn new(style: ComputedStyle, box_type: BoxType, node: Option<NodePtr>) -> Self {
        let mut dimensions = Dimensions::default();
        
        dimensions.margin.top = Self::resolve_length(style.margin_top);
        dimensions.margin.bottom = Self::resolve_length(style.margin_bottom);
        dimensions.margin.left = Self::resolve_length(style.margin_left);
        dimensions.margin.right = Self::resolve_length(style.margin_right);
        
        dimensions.padding.top = Self::resolve_length(style.padding_top);
        dimensions.padding.bottom = Self::resolve_length(style.padding_bottom);
        dimensions.padding.left = Self::resolve_length(style.padding_left);
        dimensions.padding.right = Self::resolve_length(style.padding_right);
        
        dimensions.border.top = Self::resolve_length(style.border_top_width);
        dimensions.border.bottom = Self::resolve_length(style.border_bottom_width);
        dimensions.border.left = Self::resolve_length(style.border_left_width);
        dimensions.border.right = Self::resolve_length(style.border_right_width);

        Self {
            dimensions,
            style,
            children: Vec::new(),
            box_type,
            node,
        }
    }

    fn resolve_length(len: xiaopeng_style::computed_style::CssLength) -> f32 {
        use xiaopeng_style::computed_style::CssLength;
        match len {
            CssLength::Px(v) => v,
            CssLength::Em(v) | CssLength::Rem(v) => v * 16.0, // stub font size assumption
            _ => 0.0,
        }
    }
}
