use xiaopeng_common::{Color, Rect};
use xiaopeng_layout::LayoutBox;
use xiaopeng_layout::layout_box::BoxType;
use xiaopeng_layout::StackingContext;

#[derive(Debug, Clone)]
pub enum DisplayCommand {
    DrawRect { rect: Rect, color: Color },
    DrawBorder { rect: Rect, color: Color, width: f32 },
    DrawText { text: String, rect: Rect, color: Color, font_size: f32 },
}

#[derive(Debug, Default, Clone)]
pub struct DisplayList {
    pub commands: Vec<DisplayCommand>,
}

impl DisplayList {
    pub fn new() -> Self {
        Self { commands: Vec::new() }
    }

    pub fn build(root: &LayoutBox) -> Self {
        let mut list = Self::new();
        let ctx = StackingContext::build(root);
        let boxes = ctx.flatten();
        
        for box_ref in boxes {
            let rect = box_ref.dimensions.border_box();
            
            // Background
            if box_ref.style.background_color.a > 0 {
                list.commands.push(DisplayCommand::DrawRect {
                    rect,
                    color: box_ref.style.background_color,
                });
            }
            
            // Border
            let border_width = box_ref.dimensions.border.top.max(box_ref.dimensions.border.left);
            if border_width > 0.0 && box_ref.style.border_color.a > 0 {
                list.commands.push(DisplayCommand::DrawBorder {
                    rect,
                    color: box_ref.style.border_color,
                    width: border_width,
                });
            }
            
            // Text
            if let BoxType::TextNode(ref text) = box_ref.box_type {
                let font_size = box_ref.style.font_size;
                list.commands.push(DisplayCommand::DrawText {
                    text: text.clone(),
                    rect: box_ref.dimensions.content,
                    color: box_ref.style.color,
                    font_size,
                });
            }
        }
        
        list
    }
}
