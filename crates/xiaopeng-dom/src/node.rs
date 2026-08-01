//! DOM Node definitions

use std::collections::HashMap;
use std::sync::{Arc, RwLock, Weak};
use tracing::debug;

pub type NodePtr = Arc<RwLock<Node>>;
pub type WeakNodePtr = Weak<RwLock<Node>>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeType {
    Document,
    Element,
    Text,
    Comment,
}

#[derive(Debug, Clone)]
pub struct ElementData {
    pub tag_name: String,
    pub attributes: HashMap<String, String>,
}

impl ElementData {
    pub fn new(tag_name: String) -> Self {
        Self {
            tag_name,
            attributes: HashMap::new(),
        }
    }

    pub fn get_attribute(&self, name: &str) -> Option<&String> {
        self.attributes.get(name)
    }

    pub fn set_attribute(&mut self, name: String, value: String) {
        self.attributes.insert(name, value);
    }

    pub fn has_attribute(&self, name: &str) -> bool {
        self.attributes.contains_key(name)
    }

    pub fn remove_attribute(&mut self, name: &str) {
        self.attributes.remove(name);
    }

    pub fn id(&self) -> Option<&String> {
        self.get_attribute("id")
    }

    pub fn classes(&self) -> Vec<&str> {
        self.get_attribute("class")
            .map(|c| c.split_whitespace().collect())
            .unwrap_or_default()
    }
}

#[derive(Debug, Clone)]
pub enum NodeData {
    Document,
    Element(ElementData),
    Text(String),
    Comment(String),
}

#[derive(Debug)]
pub struct Node {
    pub parent: Option<WeakNodePtr>,
    pub children: Vec<NodePtr>,
    pub data: NodeData,
}

impl Node {
    pub fn new(data: NodeData) -> NodePtr {
        debug!(?data, "Creating new DOM Node");
        Arc::new(RwLock::new(Node {
            parent: None,
            children: Vec::new(),
            data,
        }))
    }

    pub fn node_type(&self) -> NodeType {
        match self.data {
            NodeData::Document => NodeType::Document,
            NodeData::Element(_) => NodeType::Element,
            NodeData::Text(_) => NodeType::Text,
            NodeData::Comment(_) => NodeType::Comment,
        }
    }

    pub fn append_child(parent_ptr: &NodePtr, child_ptr: &NodePtr) {
        debug!("Appending child to parent DOM Node");
        
        // Remove from old parent if exists
        if let Some(old_parent_weak) = &child_ptr.read().unwrap().parent {
            if let Some(old_parent) = old_parent_weak.upgrade() {
                old_parent.write().unwrap().children.retain(|c| !Arc::ptr_eq(c, child_ptr));
            }
        }

        // Set new parent
        child_ptr.write().unwrap().parent = Some(Arc::downgrade(parent_ptr));
        parent_ptr.write().unwrap().children.push(Arc::clone(child_ptr));
    }

    pub fn remove_child(parent_ptr: &NodePtr, child_ptr: &NodePtr) -> Option<NodePtr> {
        debug!("Removing child from parent DOM Node");
        let mut parent = parent_ptr.write().unwrap();
        let index = parent.children.iter().position(|c| Arc::ptr_eq(c, child_ptr));
        
        if let Some(idx) = index {
            let removed = parent.children.remove(idx);
            removed.write().unwrap().parent = None;
            Some(removed)
        } else {
            None
        }
    }

    pub fn text_content(&self) -> String {
        match &self.data {
            NodeData::Text(t) => t.clone(),
            NodeData::Comment(_) => String::new(),
            NodeData::Document | NodeData::Element(_) => {
                let mut content = String::new();
                for child in &self.children {
                    content.push_str(&child.read().unwrap().text_content());
                }
                content
            }
        }
    }

    pub fn first_child(&self) -> Option<NodePtr> {
        self.children.first().cloned()
    }

    pub fn last_child(&self) -> Option<NodePtr> {
        self.children.last().cloned()
    }

    /// Recursively searches for an element with the given ID.
    pub fn get_element_by_id(node: &NodePtr, id: &str) -> Option<NodePtr> {
        let n = node.read().unwrap();
        if let NodeData::Element(ref el) = n.data {
            if el.id().map(|s| s.as_str()) == Some(id) {
                return Some(Arc::clone(node));
            }
        }
        for child in &n.children {
            if let Some(found) = Self::get_element_by_id(child, id) {
                return Some(found);
            }
        }
        None
    }

    /// Recursively collects all elements matching the given tag name.
    pub fn get_elements_by_tag_name(node: &NodePtr, tag_name: &str) -> Vec<NodePtr> {
        let mut results = Vec::new();
        Self::collect_elements_by_tag_name(node, tag_name, &mut results);
        results
    }

    fn collect_elements_by_tag_name(node: &NodePtr, tag_name: &str, results: &mut Vec<NodePtr>) {
        let n = node.read().unwrap();
        if let NodeData::Element(ref el) = n.data {
            if el.tag_name == tag_name {
                results.push(Arc::clone(node));
            }
        }
        for child in &n.children {
            Self::collect_elements_by_tag_name(child, tag_name, results);
        }
    }

    /// Recursively collects all elements containing the given class name.
    pub fn get_elements_by_class_name(node: &NodePtr, class_name: &str) -> Vec<NodePtr> {
        let mut results = Vec::new();
        Self::collect_elements_by_class_name(node, class_name, &mut results);
        results
    }

    fn collect_elements_by_class_name(node: &NodePtr, class_name: &str, results: &mut Vec<NodePtr>) {
        let n = node.read().unwrap();
        if let NodeData::Element(ref el) = n.data {
            if el.classes().contains(&class_name) {
                results.push(Arc::clone(node));
            }
        }
        for child in &n.children {
            Self::collect_elements_by_class_name(child, class_name, results);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dom_queries() {
        let root = Node::new(NodeData::Element(ElementData::new("div".into())));
        
        let mut child1_data = ElementData::new("span".into());
        child1_data.set_attribute("id".into(), "test-id".into());
        child1_data.set_attribute("class".into(), "text-bold text-red".into());
        let child1 = Node::new(NodeData::Element(child1_data));
        
        let mut child2_data = ElementData::new("p".into());
        child2_data.set_attribute("class".into(), "text-bold".into());
        let child2 = Node::new(NodeData::Element(child2_data));

        Node::append_child(&root, &child1);
        Node::append_child(&root, &child2);

        // Test get_element_by_id
        let found = Node::get_element_by_id(&root, "test-id");
        assert!(found.is_some());
        assert!(Arc::ptr_eq(&found.unwrap(), &child1));

        // Test get_elements_by_tag_name
        let spans = Node::get_elements_by_tag_name(&root, "span");
        assert_eq!(spans.len(), 1);
        assert!(Arc::ptr_eq(&spans[0], &child1));

        // Test get_elements_by_class_name
        let bolds = Node::get_elements_by_class_name(&root, "text-bold");
        assert_eq!(bolds.len(), 2);
    }
}
