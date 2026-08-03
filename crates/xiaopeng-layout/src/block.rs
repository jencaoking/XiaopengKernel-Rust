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

    // Resolve horizontal edges from style
    let margin_l = LayoutBox::resolve_length_pub(style.margin_left);
    let margin_r = LayoutBox::resolve_length_pub(style.margin_right);
    let padding_l = LayoutBox::resolve_length_pub(style.padding_left);
    let padding_r = LayoutBox::resolve_length_pub(style.padding_right);
    let border_l = LayoutBox::resolve_length_pub(style.border_left_width);
    let border_r = LayoutBox::resolve_length_pub(style.border_right_width);

    // Store to dimensions (will be overwritten if style changed, but good for now)
    node.dimensions.margin.left = margin_l;
    node.dimensions.margin.right = margin_r;
    node.dimensions.padding.left = padding_l;
    node.dimensions.padding.right = padding_r;
    node.dimensions.border.left = border_l;
    node.dimensions.border.right = border_r;

    let total_fixed = margin_l + margin_r + padding_l + padding_r + border_l + border_r;
    let available = (containing_block_width - total_fixed).max(0.0);

    let width = style.width.to_px(containing_block_width).unwrap_or(available);
    node.dimensions.content.width = width.min(available);
}

fn calculate_block_position(node: &mut LayoutBox, offset_x: f32, offset_y: f32) {
    let style = &node.style;
    let d = &mut node.dimensions;

    // Resolve vertical edges
    d.margin.top = LayoutBox::resolve_length_pub(style.margin_top);
    d.margin.bottom = LayoutBox::resolve_length_pub(style.margin_bottom);
    d.padding.top = LayoutBox::resolve_length_pub(style.padding_top);
    d.padding.bottom = LayoutBox::resolve_length_pub(style.padding_bottom);
    d.border.top = LayoutBox::resolve_length_pub(style.border_top_width);
    d.border.bottom = LayoutBox::resolve_length_pub(style.border_bottom_width);

    // Absolute position: offset includes parent's content x/y
    // content.x/y starts at the padding edge of the containing block
    d.content.x = offset_x + d.padding.left + d.border.left + d.margin.left;
    d.content.y = offset_y + d.padding.top + d.border.top + d.margin.top;
}

fn layout_block_children(node: &mut LayoutBox) {
    let parent_content_width = node.dimensions.content.width;
    let parent_content_x = node.dimensions.content.x;
    let mut current_y = node.dimensions.content.y;

    for child in &mut node.children {
        crate::layout_box_recursive(
            child,
            parent_content_width,
            parent_content_x,
            current_y,
        );
        current_y += child.dimensions.margin_box().height;
    }
}

fn calculate_block_height(node: &mut LayoutBox) {
    if let Some(explicit_height) = node.style.height.to_px(0.0) {
        node.dimensions.content.height = explicit_height;
    } else {
        let mut h = 0.0f32;
        for child in &node.children {
            h += child.dimensions.margin_box().height;
        }
        node.dimensions.content.height = h;
    }
}

