use xiaopeng_layout::flexbox::layout_flex;
use xiaopeng_layout::layout_box::{BoxType, LayoutBox};
use xiaopeng_style::computed_style::{ComputedStyle, Display};

#[test]
fn test_flexbox_horizontal_stack() {
    let mut root_style = ComputedStyle::default();
    root_style.display = Display::Flex;
    // We don't set width explicitly, Taffy will use available_space = 1024
    let mut root = LayoutBox::new(root_style, BoxType::BlockNode, None);

    // Child 1: width 200, height 50
    let mut c1_style = ComputedStyle::default();
    c1_style.width = Some(200.0);
    c1_style.height = Some(50.0);
    let c1 = LayoutBox::new(c1_style, BoxType::BlockNode, None);

    // Child 2: width 300, height 80
    let mut c2_style = ComputedStyle::default();
    c2_style.width = Some(300.0);
    c2_style.height = Some(80.0);
    let c2 = LayoutBox::new(c2_style, BoxType::BlockNode, None);

    root.children.push(c1);
    root.children.push(c2);

    layout_flex(&mut root);

    // Flex container (row by default) should place c2 next to c1
    // Child 1: (0, 0)
    assert_eq!(root.children[0].dimensions.content.x, 0.0);
    assert_eq!(root.children[0].dimensions.content.y, 0.0);
    assert_eq!(root.children[0].dimensions.content.width, 200.0);
    assert_eq!(root.children[0].dimensions.content.height, 50.0);

    // Child 2: (200, 0)
    assert_eq!(root.children[1].dimensions.content.x, 200.0);
    assert_eq!(root.children[1].dimensions.content.y, 0.0);
    assert_eq!(root.children[1].dimensions.content.width, 300.0);
    assert_eq!(root.children[1].dimensions.content.height, 80.0);

    // Root size should encompass both horizontally (200 + 300 = 500 width) and max height (80)
    assert_eq!(root.dimensions.content.width, 500.0);
    assert_eq!(root.dimensions.content.height, 80.0);
}
