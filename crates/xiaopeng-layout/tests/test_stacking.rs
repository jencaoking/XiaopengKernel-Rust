use xiaopeng_layout::layout_box::{BoxType, LayoutBox};
use xiaopeng_layout::stacking::StackingContext;
use xiaopeng_style::computed_style::ComputedStyle;

#[test]
fn test_z_index_sorting() {
    let mut root_style = ComputedStyle::default();
    let mut root = LayoutBox::new(root_style, BoxType::BlockNode);

    let mut z_neg2_style = ComputedStyle::default();
    z_neg2_style.z_index = -2;
    let z_neg2 = LayoutBox::new(z_neg2_style, BoxType::BlockNode);

    let mut z_10_style = ComputedStyle::default();
    z_10_style.z_index = 10;
    let mut z_10 = LayoutBox::new(z_10_style, BoxType::BlockNode);
    
    let mut z_5_style = ComputedStyle::default();
    z_5_style.z_index = 5;
    let z_5 = LayoutBox::new(z_5_style, BoxType::BlockNode);

    let mut z_0_style = ComputedStyle::default();
    z_0_style.z_index = 0;
    let z_0 = LayoutBox::new(z_0_style, BoxType::BlockNode);

    // Tree structure:
    // Root
    //  |- Z: 10
    //  |   |- Z: 5
    //  |- Z: 0
    //  |- Z: -2
    z_10.children.push(z_5);
    root.children.push(z_10);
    root.children.push(z_0);
    root.children.push(z_neg2);

    let context = StackingContext::build(&root);
    let display_list = context.flatten();

    // Flattening order expected:
    // 1. Root (0)
    // 2. Z: -2
    // 3. Z: 0
    // 4. Z: 10
    //    |- Z: 5 (inside Z: 10's context)
    
    assert_eq!(display_list.len(), 5);
    assert_eq!(display_list[0].style.z_index, 0); // root
    assert_eq!(display_list[1].style.z_index, -2);
    assert_eq!(display_list[2].style.z_index, 0); // z_0
    assert_eq!(display_list[3].style.z_index, 10);
    assert_eq!(display_list[4].style.z_index, 5); // z_5 is a child of z_10's stacking context
}
