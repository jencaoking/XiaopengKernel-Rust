use xiaopeng_layout::inline::layout_inline;
use xiaopeng_layout::layout_box::{BoxType, LayoutBox};
use xiaopeng_style::computed_style::ComputedStyle;

#[test]
fn test_inline_text_wrapping() {
    let mut root_style = ComputedStyle::default();
    let mut root = LayoutBox::new(root_style, BoxType::BlockNode, None); // Container is Block

    // "Hello World Inline Layout Text Wrapping"
    // Each char width is 8.0px.
    // "Hello" = 5 chars = 40px
    // "World" = 5 chars = 40px
    // "Inline" = 6 chars = 48px
    // "Layout" = 6 chars = 48px
    // "Text" = 4 chars = 32px
    // "Wrapping" = 8 chars = 64px
    // Total word width: 272px. Spaces width: 40px. Total: 312px.
    let text = "Hello World Inline Layout Text Wrapping";
    
    let text_node = LayoutBox::new(ComputedStyle::default(), BoxType::TextNode(text.to_string()), None);
    root.children.push(text_node);

    // Layout with containing width 150px
    // Expectation:
    // Line 1: "Hello" (40+8), "World" (40+8) = 96px < 150px
    // "Inline" (48px) would make it 144px + 8 = 152px > 150px.
    // So "Inline" moves to Line 2!
    // Line 2: "Inline" (48+8), "Layout" (48+8) = 112px < 150px
    // "Text" (32px) would make it 144px + 8 = 152px > 150px.
    // So "Text" moves to Line 3!
    // Line 3: "Text" (32+8), "Wrapping" (64+8) = 112px < 150px
    
    layout_inline(&mut root, 150.0);

    // 3 lines * 20px line_height = 60px height.
    assert_eq!(root.dimensions.content.width, 150.0);
    assert_eq!(root.dimensions.content.height, 60.0);
}
