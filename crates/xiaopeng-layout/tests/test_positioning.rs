use xiaopeng_layout::flexbox::layout_flex;
use xiaopeng_layout::layout_box::{BoxType, LayoutBox};
use xiaopeng_style::computed_style::{ComputedStyle, Display, Position};

#[test]
fn test_absolute_positioning() {
    let mut root_style = ComputedStyle::default();
    root_style.display = Display::Flex; // Use Taffy for root
    let mut root = LayoutBox::new(root_style, BoxType::BlockNode);

    // Absolute child positioned at right: 10px, bottom: 20px
    let mut abs_style = ComputedStyle::default();
    abs_style.position = Position::Absolute;
    abs_style.right = Some(10.0);
    abs_style.bottom = Some(20.0);
    abs_style.width = Some(100.0);
    abs_style.height = Some(50.0);
    let abs_child = LayoutBox::new(abs_style, BoxType::BlockNode);

    root.children.push(abs_child);

    layout_flex(&mut root); // Evaluated with Taffy (available space = 1024xMaxContent). 
    // Wait, Taffy doesn't know max height if it's absolute, so the container height might be 0, 
    // and absolute bottom 20px from 0 height means y = 0 - 50 - 20 = -70 ?
    // Let's explicitly give root a height so we can assert properly.

    root.style.height = Some(500.0);
    root.style.width = Some(500.0);
    layout_flex(&mut root);

    // Expected position for abs_child:
    // right: 10 -> x = 500(parent) - 100(width) - 10 = 390
    // bottom: 20 -> y = 500(parent) - 50(height) - 20 = 430
    assert_eq!(root.children[0].dimensions.content.x, 390.0);
    assert_eq!(root.children[0].dimensions.content.y, 430.0);
    assert_eq!(root.children[0].dimensions.content.width, 100.0);
    assert_eq!(root.children[0].dimensions.content.height, 50.0);
}
