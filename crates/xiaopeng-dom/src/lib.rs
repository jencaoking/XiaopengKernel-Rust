//! XiaopengKernel DOM (Document Object Model) Module

pub mod event;
pub mod node;

pub use event::{Event, EventPhase, EventListener};
pub use node::{ElementData, Node, NodeData, NodePtr, NodeType, DocumentTypeData, ProcessingInstructionData, AttrData, NamedNodeMap};
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

    pub fn create_document_fragment(&self) -> NodePtr {
        Node::new(NodeData::DocumentFragment)
    }

    pub fn create_text_node(&self, data: &str) -> NodePtr {
        Node::new(NodeData::Text(data.to_string()))
    }

    pub fn create_cdata_section(&self, data: &str) -> NodePtr {
        Node::new(NodeData::CDataSection(data.to_string()))
    }

    pub fn create_processing_instruction(&self, target: &str, data: &str) -> NodePtr {
        Node::new(NodeData::ProcessingInstruction(crate::node::ProcessingInstructionData {
            target: target.to_string(),
            data: data.to_string(),
        }))
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

    pub fn get_elements_by_name(&self, name: &str) -> Vec<NodePtr> {
        let mut results = Vec::new();
        Self::collect_by_name(&self.root, name, &mut results);
        results
    }

    fn collect_by_name(node: &NodePtr, name: &str, results: &mut Vec<NodePtr>) {
        if let NodeData::Element(ref el) = node.read().unwrap().data {
            if el.get_attribute("name") == Some(&name.to_string()) {
                results.push(NodePtr::clone_ptr(node));
            }
        }
        let children = node.read().unwrap().children.clone();
        for child in children {
            Self::collect_by_name(&child, name, results);
        }
    }

    pub fn get_elements_by_tag_name_ns(&self, ns: &str, local_name: &str) -> Vec<NodePtr> {
        let mut results = Vec::new();
        Self::collect_by_tag_name_ns(&self.root, ns, local_name, &mut results);
        results
    }

    fn collect_by_tag_name_ns(node: &NodePtr, ns: &str, local_name: &str, results: &mut Vec<NodePtr>) {
        if let NodeData::Element(ref el) = node.read().unwrap().data {
            if el.namespace_uri.as_deref() == Some(ns) && el.local_name == local_name {
                results.push(NodePtr::clone_ptr(node));
            }
        }
        let children = node.read().unwrap().children.clone();
        for child in children {
            Self::collect_by_tag_name_ns(&child, ns, local_name, results);
        }
    }

    pub fn document_element(&self) -> Option<NodePtr> {
        self.root.read().unwrap().first_element_child()
    }

    pub fn head(&self) -> Option<NodePtr> {
        self.get_elements_by_tag_name("head").into_iter().next()
    }

    pub fn body(&self) -> Option<NodePtr> {
        self.get_elements_by_tag_name("body").into_iter().next()
    }

    pub fn create_element_ns(&self, namespace_uri: &str, qualified_name: &str) -> NodePtr {
        let (prefix, local_name) = if let Some(idx) = qualified_name.find(':') {
            (Some(qualified_name[..idx].to_string()), qualified_name[idx + 1..].to_string())
        } else {
            (None, qualified_name.to_string())
        };
        Node::new(NodeData::Element(ElementData::new_with_namespace(Some(namespace_uri.to_string()), prefix, local_name, qualified_name.to_string())))
    }

    pub fn query_selector(&self, selectors: &str) -> Option<NodePtr> {
        Node::query_selector(&self.root, selectors)
    }

    pub fn query_selector_all(&self, selectors: &str) -> Vec<NodePtr> {
        Node::query_selector_all(&self.root, selectors)
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
