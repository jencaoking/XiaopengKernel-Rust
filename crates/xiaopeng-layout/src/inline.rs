//! Inline Formatting Context (IFC) and Text Layout

use crate::layout_box::{LayoutBox, BoxType};

pub fn layout_inline(node: &mut LayoutBox, containing_block_width: f32, offset_x: f32, offset_y: f32) {
    let mut current_x = offset_x;
    let mut current_y = offset_y;
    let mut max_line_height = 0.0;

    // A very simple font metric stub
    let char_width = 8.0; // 8px per character
    let line_height = 20.0; // 20px line height

    for child in &mut node.children {
        match &child.box_type {
            BoxType::TextNode(text) => {
                // Perform text wrapping
                let words: Vec<&str> = text.split_whitespace().collect();
                for word in words {
                    let word_width = word.len() as f32 * char_width;
                    let space_width = char_width; // Space between words

                    // Wrap if word doesn't fit on this line (unless it's the first word on the line)
                    if current_x - offset_x + word_width > containing_block_width && current_x > offset_x {
                        current_x = offset_x;
                        current_y += line_height;
                    }

                    // Advance x
                    current_x += word_width + space_width;
                    if line_height > max_line_height {
                        max_line_height = line_height;
                    }
                }
            }
            BoxType::InlineNode => {
                // Layout inline children recursively
                layout_inline(child, containing_block_width - (current_x - offset_x), current_x, current_y);
                // Advance x based on the child's width
                child.dimensions.content.x = current_x;
                child.dimensions.content.y = current_y;
                current_x += child.dimensions.margin_box().width;
                if child.dimensions.margin_box().height > max_line_height {
                    max_line_height = child.dimensions.margin_box().height;
                }
            }
            _ => {}
        }
    }

    // Set the container dimensions
    node.dimensions.content.x = offset_x;
    node.dimensions.content.y = offset_y;
    node.dimensions.content.width = containing_block_width;
    node.dimensions.content.height = (current_y - offset_y) + max_line_height;
}
