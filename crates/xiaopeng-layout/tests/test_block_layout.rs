use xiaopeng_layout::block::layout_block;
use xiaopeng_layout::layout_box::LayoutBox;
use xiaopeng_style::computed_style::{ComputedStyle, Display};

#[test]
fn test_block_stacking() {
    // Parent Block
    let mut root_style = ComputedStyle::default();
    root_style.display = Display::Block;
    let mut root = LayoutBox::new(root_style, xiaopeng_layout::layout_box::BoxType::BlockNode, None);

    // Child 1: height 100
    let mut c1_style = ComputedStyle::default();
    c1_style.display = Display::Block;
    c1_style.height = xiaopeng_style::computed_style::CssLength::Px(100.0);
    let c1 = LayoutBox::new(c1_style, xiaopeng_layout::layout_box::BoxType::BlockNode, None);

    // Child 2: height 200
    let mut c2_style = ComputedStyle::default();
    c2_style.display = Display::Block;
    c2_style.height = xiaopeng_style::computed_style::CssLength::Px(200.0);
    let c2 = LayoutBox::new(c2_style, xiaopeng_layout::layout_box::BoxType::BlockNode, None);

    root.children.push(c1);
    root.children.push(c2);

    // Run block layout
    layout_block(&mut root, 800.0, 0.0, 0.0);

    // Verify root width
    assert_eq!(root.dimensions.content.width, 800.0);

    // Verify child 1 positioned at y=0, height 100
    assert_eq!(root.children[0].dimensions.content.y, 0.0);
    assert_eq!(root.children[0].dimensions.content.height, 100.0);
    assert_eq!(root.children[0].dimensions.content.width, 800.0);

    // Verify child 2 positioned at y=100 (below child 1), height 200
    assert_eq!(root.children[1].dimensions.content.y, 100.0);
    assert_eq!(root.children[1].dimensions.content.height, 200.0);
    assert_eq!(root.children[1].dimensions.content.width, 800.0);

    // Verify root height sum
    assert_eq!(root.dimensions.content.height, 300.0);
}
