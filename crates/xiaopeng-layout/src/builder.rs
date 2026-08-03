//! DOM to LayoutTree Builder

use xiaopeng_dom::{NodePtr, NodeData, NodeType};
use xiaopeng_style::{ComputedStyle, computed_style::Display};
use crate::layout_box::{LayoutBox, BoxType};

pub fn build_layout_tree(node: &NodePtr) -> Option<LayoutBox> {
    let node_ref = node.read().unwrap();
    
    if node_ref.node_type() == NodeType::Document {
        let children = node_ref.children.clone();
        drop(node_ref);
        let mut root_box = LayoutBox::new(ComputedStyle::default(), BoxType::BlockNode, Some(node.clone()));
        for child in &children {
            if let Some(child_box) = build_layout_tree(child) {
                root_box.children.push(child_box);
            }
        }
        return Some(root_box);
    }
    
    if node_ref.node_type() == NodeType::Comment {
        return None;
    }
    
    if node_ref.node_type() == NodeType::Text {
        if let NodeData::Text(ref t) = node_ref.data {
            let trimmed = t.trim();
            if trimmed.is_empty() {
                return None;
            }
            return Some(LayoutBox::new(ComputedStyle::default(), BoxType::TextNode(trimmed.to_string()), Some(node.clone())));
        }
    }
    
    let children = node_ref.children.clone();
    drop(node_ref);
    
    let style = xiaopeng_style::resolve_style(node);
    
    if style.display == Display::None {
        return None;
    }
    
    let box_type = match style.display {
        Display::Inline => BoxType::InlineNode,
        _ => BoxType::BlockNode,
    };
    
    let mut layout_box = LayoutBox::new(style, box_type, Some(node.clone()));
    
    for child in &children {
        if let Some(child_box) = build_layout_tree(child) {
            layout_box.children.push(child_box);
        }
    }
    
    Some(layout_box)
}
