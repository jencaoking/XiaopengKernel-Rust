//! DOM Node definitions

use std::collections::HashMap;
use std::sync::{Arc, RwLock, Weak};
use tracing::debug;
use crate::event::{Event, EventPhase, EventListenerEntry, EventListener};

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

    pub fn has_class(&self, class_name: &str) -> bool {
        self.classes().contains(&class_name)
    }

    pub fn add_class(&mut self, class_name: &str) {
        if !self.has_class(class_name) {
            let current = self.get_attribute("class").cloned().unwrap_or_default();
            let new_class = if current.is_empty() {
                class_name.to_string()
            } else {
                format!("{} {}", current, class_name)
            };
            self.set_attribute("class".into(), new_class);
        }
    }

    pub fn remove_class(&mut self, class_name: &str) {
        let classes: Vec<&str> = self.classes().into_iter().filter(|c| *c != class_name).collect();
        if classes.is_empty() {
            self.remove_attribute("class");
        } else {
            self.set_attribute("class".into(), classes.join(" "));
        }
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
    pub listeners: HashMap<String, Vec<EventListenerEntry>>,
}

impl Node {
    pub fn new(data: NodeData) -> NodePtr {
        debug!(?data, "Creating new DOM Node");
        Arc::new(RwLock::new(Node {
            parent: None,
            children: Vec::new(),
            data,
            listeners: HashMap::new(),
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

    pub fn insert_before(parent_ptr: &NodePtr, child_ptr: &NodePtr, index: usize) -> Result<(), &'static str> {
        debug!("Inserting child into parent DOM Node at index {}", index);
        
        // 1. Strict boundary check (read lock only, released immediately)
        {
            let parent = parent_ptr.read().unwrap();
            if index >= parent.children.len() {
                return Err("IndexOutOfBounds: insert_before requires index < children.len()");
            }
        }

        // 2. Remove from old parent if exists.
        // We do this BEFORE acquiring parent_ptr's write lock to avoid deadlock if old_parent == parent_ptr
        let old_parent = child_ptr.read().unwrap().parent.as_ref().and_then(|w| w.upgrade());
        if let Some(old_parent) = old_parent {
            old_parent.write().unwrap().children.retain(|c| !Arc::ptr_eq(c, child_ptr));
        }

        // 3. Insert into the new parent.
        let mut parent = parent_ptr.write().unwrap();
        // If old_parent == parent_ptr, retaining the child above might have shrunk the children vec.
        // We clamp the index to prevent out-of-bounds panics after removal.
        let safe_index = index.min(parent.children.len());
        
        child_ptr.write().unwrap().parent = Some(Arc::downgrade(parent_ptr));
        parent.children.insert(safe_index, Arc::clone(child_ptr));
        
        Ok(())
    }

    pub fn insert_before_node(parent_ptr: &NodePtr, child_ptr: &NodePtr, reference_ptr: &NodePtr) -> Result<(), &'static str> {
        let index = {
            let parent = parent_ptr.read().unwrap();
            parent.children.iter().position(|c| Arc::ptr_eq(c, reference_ptr))
        };
        
        if let Some(idx) = index {
            Self::insert_before(parent_ptr, child_ptr, idx)
        } else {
            Err("ReferenceNodeNotFound: The reference node is not a child of the parent")
        }
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

    pub fn first_element_child(&self) -> Option<NodePtr> {
        self.children.iter().find(|c| c.read().unwrap().node_type() == NodeType::Element).cloned()
    }

    pub fn last_element_child(&self) -> Option<NodePtr> {
        self.children.iter().rev().find(|c| c.read().unwrap().node_type() == NodeType::Element).cloned()
    }

    pub fn next_element_sibling(node_ptr: &NodePtr) -> Option<NodePtr> {
        let parent = {
            let node = node_ptr.read().unwrap();
            node.parent.as_ref().and_then(|w| w.upgrade())
        };
        if let Some(parent) = parent {
            let p = parent.read().unwrap();
            let pos = p.children.iter().position(|c| Arc::ptr_eq(c, node_ptr))?;
            for sibling in p.children.iter().skip(pos + 1) {
                if sibling.read().unwrap().node_type() == NodeType::Element {
                    return Some(Arc::clone(sibling));
                }
            }
        }
        None
    }

    pub fn previous_element_sibling(node_ptr: &NodePtr) -> Option<NodePtr> {
        let parent = {
            let node = node_ptr.read().unwrap();
            node.parent.as_ref().and_then(|w| w.upgrade())
        };
        if let Some(parent) = parent {
            let p = parent.read().unwrap();
            let pos = p.children.iter().position(|c| Arc::ptr_eq(c, node_ptr))?;
            for sibling in p.children.iter().take(pos).rev() {
                if sibling.read().unwrap().node_type() == NodeType::Element {
                    return Some(Arc::clone(sibling));
                }
            }
        }
        None
    }

    pub fn child_element_count(&self) -> usize {
        self.children.iter().filter(|c| c.read().unwrap().node_type() == NodeType::Element).count()
    }

    pub fn clone_node(node_ptr: &NodePtr, deep: bool) -> NodePtr {
        let node = node_ptr.read().unwrap();
        let cloned_data = node.data.clone();
        let new_node = Node::new(cloned_data);
        
        if deep {
            for child in &node.children {
                let cloned_child = Self::clone_node(child, true);
                Node::append_child(&new_node, &cloned_child);
            }
        }
        new_node
    }

    pub fn to_html(node_ptr: &NodePtr) -> String {
        let node = node_ptr.read().unwrap();
        match &node.data {
            NodeData::Document => {
                node.children.iter().map(Self::to_html).collect::<Vec<_>>().join("")
            }
            NodeData::Element(el) => {
                let mut attrs = String::new();
                for (k, v) in &el.attributes {
                    attrs.push_str(&format!(" {}=\"{}\"", k, v));
                }
                let children_html = node.children.iter().map(Self::to_html).collect::<Vec<_>>().join("");
                format!("<{}{}>{}</{}>", el.tag_name, attrs, children_html, el.tag_name)
            }
            NodeData::Text(t) => t.clone(),
            NodeData::Comment(c) => format!("<!--{}-->", c),
        }
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

    pub fn add_event_listener(
        node_ptr: &NodePtr,
        event_type: &str,
        listener: Arc<dyn EventListener>,
        use_capture: bool,
    ) {
        let mut node = node_ptr.write().unwrap();
        let entries = node.listeners.entry(event_type.to_string()).or_insert_with(Vec::new);
        entries.push(EventListenerEntry { listener, use_capture });
    }

    pub fn dispatch_event(node_ptr: &NodePtr, event: &mut Event) -> bool {
        let mut path = Vec::new();
        let mut current = Arc::clone(node_ptr);
        loop {
            let parent = current.read().unwrap().parent.as_ref().and_then(|w| w.upgrade());
            if let Some(p) = parent {
                path.push(Arc::clone(&p));
                current = p;
            } else {
                break;
            }
        }

        event.phase = EventPhase::CapturingPhase;
        for ptr in path.iter().rev() {
            if event.propagation_stopped { break; }
            Self::invoke_listeners(ptr, event);
        }

        if !event.propagation_stopped {
            event.phase = EventPhase::AtTarget;
            Self::invoke_listeners(node_ptr, event);
        }

        if event.bubbles && !event.propagation_stopped {
            event.phase = EventPhase::BubblingPhase;
            for ptr in path.iter() {
                if event.propagation_stopped { break; }
                Self::invoke_listeners(ptr, event);
            }
        }

        event.phase = EventPhase::None;
        !event.default_prevented
    }

    fn invoke_listeners(node_ptr: &NodePtr, event: &mut Event) {
        let listeners_to_run = {
            let node = node_ptr.read().unwrap();
            if let Some(entries) = node.listeners.get(&event.event_type) {
                entries.clone()
            } else {
                Vec::new()
            }
        };

        for entry in listeners_to_run {
            if event.immediate_propagation_stopped { break; }
            
            let should_run = match event.phase {
                EventPhase::CapturingPhase => entry.use_capture,
                EventPhase::BubblingPhase => !entry.use_capture,
                EventPhase::AtTarget => true,
                _ => false,
            };

            if should_run {
                entry.listener.handle_event(event);
            }
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
