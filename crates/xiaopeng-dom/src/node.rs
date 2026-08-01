//! DOM Node definitions

use std::collections::HashMap;
use std::sync::{Arc, RwLock, Weak};
use tracing::debug;

pub type NodePtr = Arc<RwLock<Node>>;
pub type WeakNodePtr = Weak<RwLock<Node>>;

#[derive(Debug, Clone)]
pub enum NodeType {
    Document,
    Element(ElementData),
    Text(String),
    Comment(String),
}

#[derive(Debug, Clone)]
pub struct ElementData {
    pub tag_name: String,
    pub attributes: HashMap<String, String>,
}

#[derive(Debug)]
pub struct Node {
    pub parent: Option<WeakNodePtr>,
    pub children: Vec<NodePtr>,
    pub node_type: NodeType,
}

impl Node {
    pub fn new(node_type: NodeType) -> NodePtr {
        debug!(?node_type, "Creating new DOM Node");
        Arc::new(RwLock::new(Node {
            parent: None,
            children: Vec::new(),
            node_type,
        }))
    }

    pub fn append_child(parent: &NodePtr, child: &NodePtr) {
        debug!("Appending child to parent DOM Node");
        child.write().unwrap().parent = Some(Arc::downgrade(parent));
        parent.write().unwrap().children.push(Arc::clone(child));
    }
}
