//! Styled Node Tree Definition

use crate::computed_style::{ComputedStyle, Display};
use crate::resolver::StyleResolver;
use std::sync::Arc;
use xiaopeng_dom::NodePtr;

#[derive(Debug)]
pub struct StyledNode {
    pub node: NodePtr,
    pub style: ComputedStyle,
    pub children: Vec<StyledNode>,
}

impl StyledNode {
    /// Recursively builds a tree of StyledNodes starting from a DOM node.
    /// Elements with `display: none` are completely omitted from the tree.
    pub fn build(node: &NodePtr, resolver: &StyleResolver) -> Option<StyledNode> {
        let style = resolver.resolve_style(node);
        
        // If display is None, the element and all its children are removed from the render tree
        if style.display == Display::None {
            return None;
        }
        
        let mut children = Vec::new();
        
        let node_ref = node.read().unwrap();
        for child in &node_ref.children {
            if let Some(styled_child) = Self::build(child, resolver) {
                children.push(styled_child);
            }
        }
        
        Some(StyledNode {
            node: Arc::clone(node),
            style,
            children,
        })
    }
}
