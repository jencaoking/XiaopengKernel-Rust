//! XiaopengKernel DOM (Document Object Model) Module

pub mod event;
pub mod node;

pub use event::{Event, EventPhase, EventListener};
pub use node::{ElementData, Node, NodeData, NodePtr, NodeType};
use tracing::info;
use xiaopeng_common::XiaopengResult;

pub struct Document {
    pub root: NodePtr,
}

impl Document {
    pub fn new() -> Self {
        info!("Initializing DOM Document");
        Self {
            root: Node::new(NodeData::Document),
        }
    }

    pub fn create_element(&self, tag_name: &str) -> NodePtr {
        Node::new(NodeData::Element(ElementData::new(tag_name.to_string())))
    }

    pub fn create_text_node(&self, data: &str) -> NodePtr {
        Node::new(NodeData::Text(data.to_string()))
    }

    pub fn create_comment(&self, data: &str) -> NodePtr {
        Node::new(NodeData::Comment(data.to_string()))
    }

    pub fn get_element_by_id(&self, id: &str) -> Option<NodePtr> {
        Node::get_element_by_id(&self.root, id)
    }

    pub fn get_elements_by_tag_name(&self, tag_name: &str) -> Vec<NodePtr> {
        Node::get_elements_by_tag_name(&self.root, tag_name)
    }

    pub fn get_elements_by_class_name(&self, class_name: &str) -> Vec<NodePtr> {
        Node::get_elements_by_class_name(&self.root, class_name)
    }
}

impl Default for Document {
    fn default() -> Self {
        Self::new()
    }
}

pub fn init_dom() -> XiaopengResult<()> {
    info!("DOM module initialized");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_document_creation() {
        let doc = Document::new();
        assert!(matches!(
            doc.root.read().unwrap().data,
            NodeData::Document
        ));
    }
}
