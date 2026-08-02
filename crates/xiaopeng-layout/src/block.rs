//! Block Formatting Context (BFC)

use crate::layout_box::LayoutBox;

pub fn layout_block(node: &mut LayoutBox, containing_block_width: f32, offset_x: f32, offset_y: f32) {
    // 1. Calculate width based on containing block
    calculate_block_width(node, containing_block_width);

    // 2. Calculate absolute position
    calculate_block_position(node, offset_x, offset_y);

    // 3. Layout children
    layout_block_children(node);

    // 4. Calculate height based on children
    calculate_block_height(node);
}

fn calculate_block_width(node: &mut LayoutBox, containing_block_width: f32) {
    let style = &node.style;
    
    // Default width is the containing block width (simulating width: auto)
    // If a specific width is set, use it.
    let mut width = style.width.to_px(containing_block_width).unwrap_or(containing_block_width);
    
    // For now, margin/padding are 0 unless implemented in style resolver
    // In a full implementation we'd subtract margin/padding from width if box-sizing: content-box
    
    if width > containing_block_width {
        width = containing_block_width;
    }
    
    node.dimensions.content.width = width;
}

fn calculate_block_position(node: &mut LayoutBox, offset_x: f32, offset_y: f32) {
    let d = &mut node.dimensions;
    // Absolute position x includes parent's absolute x + node's left margin + parent's padding/border (passed as offset_x)
    d.content.x = offset_x + d.margin.left;
    d.content.y = offset_y + d.margin.top;
}

fn layout_block_children(node: &mut LayoutBox) {
    let d = &mut node.dimensions;
    let mut current_y = d.content.y; // Start at parent's absolute content Y
    
    let parent_content_width = d.content.width;
    let parent_content_x = d.content.x;

    for child in &mut node.children {
        crate::layout_box_recursive(
            child, 
            parent_content_width, 
            parent_content_x, // offset_x for child is parent's content x
            current_y         // offset_y for child is the current vertical cursor
        );
        
        // Advance current_y by the child's total height (margin box)
        current_y += child.dimensions.margin_box().height;
    }
}

fn calculate_block_height(node: &mut LayoutBox) {
    if let Some(explicit_height) = node.style.height.to_px(0.0) {
        node.dimensions.content.height = explicit_height;
    } else {
        // Height is the sum of children heights (already accumulated in layout_block_children)
        let mut h = 0.0;
        for child in &node.children {
            h += child.dimensions.margin_box().height;
        }
        node.dimensions.content.height = h;
    }
}

