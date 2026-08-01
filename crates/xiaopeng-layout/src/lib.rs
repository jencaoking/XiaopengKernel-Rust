//! XiaopengKernel Layout Engine Module (Block/Inline/Flexbox/Grid)

pub mod block;
pub mod flexbox;
pub mod inline;
pub mod layout_box;
pub mod stacking;

pub use block::layout_block;
pub use flexbox::layout_flex;
pub use layout_box::{Dimensions, EdgeSizes, LayoutBox};
pub use stacking::StackingContext;
use tracing::info;
use xiaopeng_common::XiaopengResult;
use xiaopeng_dom::NodePtr;

pub mod builder;
pub use builder::build_layout_tree;

pub fn compute_layout(
    document_node: &NodePtr,
    viewport_width: f32,
    viewport_height: f32,
) -> XiaopengResult<LayoutBox> {
    info!("Computing layout for viewport {}x{}", viewport_width, viewport_height);
    
    // 1. Build layout tree from DOM
    let mut root_box = build_layout_tree(document_node)
        .ok_or_else(|| xiaopeng_common::XiaopengError::LayoutError { component: "Builder".into(), message: "Failed to build layout tree".into() })?;
        
    // 2. Perform layout passes (starting with block layout on root)
    layout_box_recursive(&mut root_box, viewport_width, 0.0, 0.0);
    
    Ok(root_box)
}

pub fn layout_box_recursive(
    node: &mut LayoutBox,
    containing_block_width: f32,
    offset_x: f32,
    offset_y: f32,
) {
    use xiaopeng_style::computed_style::Display;
    
    match node.style.display {
        Display::Block => crate::block::layout_block(node, containing_block_width, offset_x, offset_y),
        Display::Flex => crate::flexbox::layout_flex(node, offset_x, offset_y),
        Display::Inline => crate::inline::layout_inline(node, containing_block_width, offset_x, offset_y),
        _ => crate::block::layout_block(node, containing_block_width, offset_x, offset_y),
    }
}

/// Hit testing: given an (x, y) coordinate, finds the top-most DOM Node at that position.
pub fn hit_test(root: &LayoutBox, x: f32, y: f32) -> Option<NodePtr> {
    let ctx = StackingContext::build(root);
    let display_list = ctx.flatten();
    
    // Iterate from the top-most painted element downwards
    for box_ref in display_list.into_iter().rev() {
        let rect = box_ref.dimensions.border_box();
        if x >= rect.x && x <= rect.x + rect.width && y >= rect.y && y <= rect.y + rect.height {
            if let Some(node) = &box_ref.node {
                return Some(node.clone());
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use xiaopeng_style::ComputedStyle;

    #[test]
    fn test_layout_box_creation() {
        let lbox = LayoutBox::new(ComputedStyle::default(), layout_box::BoxType::BlockNode, None);
        assert_eq!(lbox.children.len(), 0);
    }
}
