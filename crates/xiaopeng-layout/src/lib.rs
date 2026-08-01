//! XiaopengKernel Layout Engine Module (Block/Inline/Flexbox/Grid)

pub mod block;
pub mod flexbox;
pub mod inline;
pub mod layout_box;

pub use block::layout_block;
pub use flexbox::layout_flex;
pub use layout_box::{Dimensions, EdgeSizes, LayoutBox};
use tracing::info;
use xiaopeng_common::XiaopengResult;

pub fn compute_layout() -> XiaopengResult<()> {
    info!("Computing layout");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use xiaopeng_style::ComputedStyle;

    #[test]
    fn test_layout_box_creation() {
        let lbox = LayoutBox::new(ComputedStyle::default(), layout_box::BoxType::BlockNode);
        assert_eq!(lbox.children.len(), 0);
    }
}
