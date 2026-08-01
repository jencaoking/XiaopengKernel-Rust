//! Block Formatting Context (BFC)

use crate::layout_box::LayoutBox;
use xiaopeng_style::computed_style::Display;

pub fn layout_block(node: &mut LayoutBox, containing_block_width: f32) {
    // 1. Calculate width based on containing block
    calculate_block_width(node, containing_block_width);

    // 2. Calculate initial position
    calculate_block_position(node);

    // 3. Layout children
    layout_block_children(node);

    // 4. Calculate height based on children
    calculate_block_height(node);
}

fn calculate_block_width(node: &mut LayoutBox, containing_block_width: f32) {
    let style = &node.style;
    
    // Default width is the containing block width (simulating width: auto)
    // If a specific width is set, use it.
    let mut width = style.width.unwrap_or(containing_block_width);
    
    // For now, margin/padding are 0 unless implemented in style resolver
    // In a full implementation we'd subtract margin/padding from width if box-sizing: content-box
    
    if width > containing_block_width {
        width = containing_block_width;
    }
    
    node.dimensions.content.width = width;
}

fn calculate_block_position(node: &mut LayoutBox) {
    let d = &mut node.dimensions;
    // Position x includes parent's padding + node's left margin (stubs for now)
    d.content.x = d.margin.left + d.padding.left;
    d.content.y = d.margin.top + d.padding.top;
}

fn layout_block_children(node: &mut LayoutBox) {
    let d = &mut node.dimensions;
    let mut current_y = 0.0;
    
    let parent_content_width = d.content.width;

    for child in &mut node.children {
        if child.style.display == Display::Block {
            // First, run the layout for the child to calculate its width, height, and intrinsic padding/margin offsets.
            layout_block(child, parent_content_width);
            
            // Now position the child vertically based on the previous children
            // The child's y position relative to the parent's content box is:
            // current_y + the child's top margin, border, and padding.
            // Wait, content.y represents the origin of the CONTENT box relative to parent's content box.
            child.dimensions.content.y = current_y + child.dimensions.margin.top + child.dimensions.border.top + child.dimensions.padding.top;
            
            // Advance current_y by the child's total height (including margins)
            current_y += child.dimensions.margin_box().height;
        } else {
            layout_block(child, parent_content_width);
            child.dimensions.content.y = current_y;
            current_y += child.dimensions.margin_box().height;
        }
    }
}

fn calculate_block_height(node: &mut LayoutBox) {
    if let Some(explicit_height) = node.style.height {
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

