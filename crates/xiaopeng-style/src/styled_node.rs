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

    pub fn build(
        node: &NodePtr, 
        resolver: &StyleResolver,
        parent_style: Option<&ComputedStyle>,
        root_font_size: f32,
        viewport_width: f32,
        viewport_height: f32,
    ) -> Option<StyledNode> {
        let style = resolver.resolve_style(node, parent_style, root_font_size, viewport_width, viewport_height);
        
        // If display is None, the element and all its children are removed from the render tree
        if style.display == Display::None {
            return None;
        }
        
        let mut children = Vec::new();
        
        let node_ref = node.read().unwrap();
        
        // If this is the root node (html), its font size becomes the root font size for children
        let current_root_font_size = if parent_style.is_none() { style.font_size } else { root_font_size };

        for child in &node_ref.children {
            if let Some(styled_child) = Self::build(child, resolver, Some(&style), current_root_font_size, viewport_width, viewport_height) {
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
