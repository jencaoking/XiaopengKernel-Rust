use xiaopeng_common::Rect;
use xiaopeng_style::ComputedStyle;
use xiaopeng_layout::layout_box::{LayoutBox, BoxType};
use xiaopeng_layout::hit_test;
use xiaopeng_dom::{Node, NodeData, ElementData};

#[test]
fn test_hit_testing_basic() {
    let mut root_style = ComputedStyle::default();
    root_style.z_index = 0;
    let root_node = Node::new(NodeData::Document);
    let mut root = LayoutBox::new(root_style, BoxType::BlockNode, Some(root_node.clone()));
    root.dimensions.content = Rect::new(0.0, 0.0, 800.0, 600.0);

    let mut c1_style = ComputedStyle::default();
    c1_style.z_index = 0;
    let c1_node = Node::new(NodeData::Element(ElementData::new("div".into())));
    let mut c1 = LayoutBox::new(c1_style, BoxType::BlockNode, Some(c1_node.clone()));
    c1.dimensions.content = Rect::new(10.0, 10.0, 100.0, 100.0);

    let mut c2_style = ComputedStyle::default();
    c2_style.z_index = 10;
    let c2_node = Node::new(NodeData::Element(ElementData::new("span".into())));
    let mut c2 = LayoutBox::new(c2_style, BoxType::BlockNode, Some(c2_node.clone()));
    c2.dimensions.content = Rect::new(50.0, 50.0, 100.0, 100.0); // Overlaps c1

    root.children.push(c1);
    root.children.push(c2);

    // Hit root only
    assert!(std::sync::Arc::ptr_eq(&hit_test(&root, 700.0, 500.0).unwrap(), &root_node));
    
    // Hit c1 (only c1 covers this area)
    assert!(std::sync::Arc::ptr_eq(&hit_test(&root, 20.0, 20.0).unwrap(), &c1_node));
    
    // Hit overlap area: c2 is above c1 (z-index 10 > 0)
    assert!(std::sync::Arc::ptr_eq(&hit_test(&root, 60.0, 60.0).unwrap(), &c2_node));
}
