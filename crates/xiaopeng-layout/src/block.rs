//! Block Formatting Context (BFC)

use crate::layout_box::LayoutBox;

pub fn layout_block(root: &mut LayoutBox, _containing_block_width: f32) {
    // Block layout algorithm stub
    root.dimensions.content.width = _containing_block_width;
}
