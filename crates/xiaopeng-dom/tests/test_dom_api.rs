use std::sync::Arc;
use xiaopeng_dom::{ElementData, Node, NodeData, NodePtr};

#[test]
fn test_dom_element_attributes() {
    let mut elem_data = ElementData::new("div".into());
    elem_data.set_attribute("id".into(), "test".into());
    elem_data.set_attribute("class".into(), "container main".into());
    
    assert!(elem_data.has_attribute("id"));
    assert_eq!(elem_data.get_attribute("id"), Some(&"test".into()));
    assert_eq!(elem_data.id(), Some(&"test".into()));
    assert_eq!(elem_data.classes(), vec!["container", "main"]);
    
    elem_data.remove_attribute("id");
    assert!(!elem_data.has_attribute("id"));
}

#[test]
fn test_dom_element_classlist() {
    let mut elem_data = ElementData::new("div".into());
    elem_data.set_attribute("class".into(), "foo bar baz".into());
    
    assert_eq!(elem_data.classes().len(), 3);
    assert!(elem_data.has_class("foo"));
    assert!(elem_data.has_class("bar"));
    assert!(elem_data.has_class("baz"));
    assert!(!elem_data.has_class("qux"));
    
    elem_data.add_class("qux");
    assert!(elem_data.has_class("qux"));
    
    elem_data.remove_class("bar");
    assert!(!elem_data.has_class("bar"));
    assert_eq!(elem_data.classes(), vec!["foo", "baz", "qux"]);
}

#[test]
fn test_dom_element_traversal() {
    // <div><p>First</p><p>Second</p><span>Third</span></div>
    let div = Node::new(NodeData::Element(ElementData::new("div".into())));
    
    let p1 = Node::new(NodeData::Element(ElementData::new("p".into())));
    Node::append_child(&p1, &Node::new(NodeData::Text("First".into())));
    
    let p2 = Node::new(NodeData::Element(ElementData::new("p".into())));
    Node::append_child(&p2, &Node::new(NodeData::Text("Second".into())));
    
    let span = Node::new(NodeData::Element(ElementData::new("span".into())));
    Node::append_child(&span, &Node::new(NodeData::Text("Third".into())));
    
    Node::append_child(&div, &p1);
    Node::append_child(&div, &p2);
    Node::append_child(&div, &span);
    
    assert_eq!(div.read().expect("Lock poisoned").child_element_count(), 3);
    
    let first_child = div.read().expect("Lock poisoned").first_element_child().expect("Unwrap failed");
    assert_eq!(
        first_child.read().expect("Lock poisoned").node_type(),
        xiaopeng_dom::NodeType::Element
    );
    
    let last_child = div.read().expect("Lock poisoned").last_element_child().expect("Unwrap failed");
    assert_eq!(
        last_child.read().expect("Lock poisoned").node_type(),
        xiaopeng_dom::NodeType::Element
    );
    
    let next = Node::next_element_sibling(&first_child).expect("Unwrap failed");
    assert_eq!(
        next.read().expect("Lock poisoned").node_type(),
        xiaopeng_dom::NodeType::Element
    );
    
    let prev = Node::previous_element_sibling(&last_child).expect("Unwrap failed");
    assert_eq!(
        prev.read().expect("Lock poisoned").node_type(),
        xiaopeng_dom::NodeType::Element
    );
}

#[test]
fn test_dom_clone_node() {
    let mut div_data = ElementData::new("div".into());
    div_data.set_attribute("id".into(), "original".into());
    let div = Node::new(NodeData::Element(div_data));
    let p = Node::new(NodeData::Element(ElementData::new("p".into())));
    Node::append_child(&p, &Node::new(NodeData::Text("Text".into())));
    Node::append_child(&div, &p);
    
    let shallow_clone = Node::clone_node(&div, false);
    if let NodeData::Element(ref el) = shallow_clone.read().expect("Lock poisoned").data {
        assert_eq!(el.tag_name, "div");
        assert_eq!(el.id(), Some(&"original".into()));
    } else { panic!("Expected element"); }
    assert_eq!(shallow_clone.read().expect("Lock poisoned").child_element_count(), 0);
    
    let deep_clone = Node::clone_node(&div, true);
    assert_eq!(deep_clone.read().expect("Lock poisoned").child_element_count(), 1);
}

#[test]
fn test_dom_to_html() {
    let mut div_data = ElementData::new("div".into());
    div_data.set_attribute("id".into(), "test".into());
    let div = Node::new(NodeData::Element(div_data));
    let p = Node::new(NodeData::Element(ElementData::new("p".into())));
    Node::append_child(&p, &Node::new(NodeData::Text("Hello".into())));
    Node::append_child(&div, &p);
    
    let html = Node::to_html(&div);
    assert!(html.contains("<div"));
    assert!(html.contains("id=\"test\""));
    assert!(html.contains("<p>"));
    assert!(html.contains("Hello"));
    assert!(html.contains("</p>"));
    assert!(html.contains("</div>"));
}

#[test]
fn test_insert_before_self_move_deadlock() {
    let parent = Node::new(NodeData::Element(ElementData::new("div".into())));
    
    let a = Node::new(NodeData::Element(ElementData::new("a".into())));
    let b = Node::new(NodeData::Element(ElementData::new("b".into())));
    let c = Node::new(NodeData::Element(ElementData::new("c".into())));
    let d = Node::new(NodeData::Element(ElementData::new("d".into())));
    
    Node::append_child(&parent, &a);
    Node::append_child(&parent, &b);
    Node::append_child(&parent, &c);
    Node::append_child(&parent, &d);
    
    // We want to move 'A' to index 2 (before 'C').
    // Since 'A' is already a child of 'parent', its old_parent is 'parent'.
    // If insert_before holds the write lock on 'parent' while trying to write lock 'old_parent', it will deadlock.
    let result = Node::insert_before(&parent, &a, 2);
    assert!(result.is_ok());
    
    let children = parent.read().expect("Lock poisoned").children.clone();
    assert_eq!(children.len(), 4);
    
    // The order should now be B, C, A, D?
    // Let's trace it: 
    // Initial: [A, B, C, D]
    // Retain (remove A): [B, C, D]
    // Insert at 2: [B, C, A, D]
    let tags: Vec<String> = children.iter().map(|c| {
        if let NodeData::Element(ref el) = c.read().expect("Lock poisoned").data {
            el.tag_name.clone()
        } else {
            String::new()
        }
    }).collect();
    
    assert_eq!(tags, vec!["b", "c", "a", "d"]);
}
