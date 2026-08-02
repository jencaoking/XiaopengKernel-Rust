use xiaopeng_layout::flexbox::layout_flex;
use xiaopeng_layout::layout_box::{BoxType, LayoutBox};
use xiaopeng_style::computed_style::{ComputedStyle, Display, Position};

#[test]
fn test_absolute_positioning() {
    let mut root_style = ComputedStyle::default();
    root_style.display = Display::Flex; // Use Taffy for root
    let mut root = LayoutBox::new(root_style, BoxType::BlockNode, None);

    // Absolute child positioned at right: 10px, bottom: 20px
    let mut abs_style = ComputedStyle::default();
    abs_style.position = Position::Absolute;
    abs_style.right = xiaopeng_style::computed_style::CssLength::Px(10.0);
    abs_style.bottom = xiaopeng_style::computed_style::CssLength::Px(20.0);
    abs_style.width = xiaopeng_style::computed_style::CssLength::Px(100.0);
    abs_style.height = xiaopeng_style::computed_style::CssLength::Px(50.0);
    let abs_child = LayoutBox::new(abs_style, BoxType::BlockNode, None);

    root.children.push(abs_child);

    layout_flex(&mut root, 0.0, 0.0); // Evaluated with Taffy (available space = 1024xMaxContent, 0.0, 0.0). 
    // Wait, Taffy doesn't know max height if it's absolute, so the container height might be 0, 
    // and absolute bottom 20px from 0 height means y = 0 - 50 - 20 = -70 ?
    // Let's explicitly give root a height so we can assert properly.

    root.style.height = xiaopeng_style::computed_style::CssLength::Px(500.0);
    root.style.width = xiaopeng_style::computed_style::CssLength::Px(500.0);
    layout_flex(&mut root, 0.0, 0.0);

    // Expected position for abs_child:
    // right: 10 -> x = 500(parent) - 100(width) - 10 = 390
    // bottom: 20 -> y = 500(parent) - 50(height, 0.0, 0.0) - 20 = 430
    assert_eq!(root.children[0].dimensions.content.x, 390.0);
    assert_eq!(root.children[0].dimensions.content.y, 430.0);
    assert_eq!(root.children[0].dimensions.content.width, 100.0);
    assert_eq!(root.children[0].dimensions.content.height, 50.0);
}
