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
}
