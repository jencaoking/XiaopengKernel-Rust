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
    pub id_cache: Option<HashMap<String, WeakNodePtr>>,
    pub tag_cache: Option<HashMap<String, Vec<WeakNodePtr>>>,
    pub class_cache: Option<HashMap<String, Vec<WeakNodePtr>>>,
}

impl Node {
    pub fn new(data: NodeData) -> NodePtr {
        debug!(?data, "Creating new DOM Node");
        Arc::new(RwLock::new(Node {
            parent: None,
            children: Vec::new(),
            data,
            listeners: HashMap::new(),
            id_cache: None,
            tag_cache: None,
            class_cache: None,
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

    pub fn invalidate_caches(node_ptr: &NodePtr) {
        let mut current = Some(Arc::clone(node_ptr));
        while let Some(n) = current {
            let mut n_write = n.write().unwrap();
            let was_dirty = n_write.id_cache.is_none() 
                && n_write.tag_cache.is_none() 
                && n_write.class_cache.is_none();
                
            n_write.id_cache = None;
            n_write.tag_cache = None;
            n_write.class_cache = None;
            
            if was_dirty {
                break;
            }
            let parent = n_write.parent.as_ref().and_then(|w| w.upgrade());
            drop(n_write);
            current = parent;
        }
    }

    pub fn append_child(parent_ptr: &NodePtr, child_ptr: &NodePtr) {
        debug!("Appending child to parent DOM Node");
        
        // Remove from old parent if exists
        if let Some(old_parent_weak) = &child_ptr.read().unwrap().parent {
            if let Some(old_parent) = old_parent_weak.upgrade() {
                old_parent.write().unwrap().children.retain(|c| !Arc::ptr_eq(c, child_ptr));
                Self::invalidate_caches(&old_parent);
            }
        }

        // Set new parent
        child_ptr.write().unwrap().parent = Some(Arc::downgrade(parent_ptr));
        parent_ptr.write().unwrap().children.push(Arc::clone(child_ptr));
        Self::invalidate_caches(parent_ptr);
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
        if let Some(ref old_p) = old_parent {
            old_p.write().unwrap().children.retain(|c| !Arc::ptr_eq(c, child_ptr));
            Self::invalidate_caches(old_p);
        }

        // 3. Insert into the new parent.
        let mut parent = parent_ptr.write().unwrap();
        // If old_parent == parent_ptr, retaining the child above might have shrunk the children vec.
        // We clamp the index to prevent out-of-bounds panics after removal.
        let safe_index = index.min(parent.children.len());
        
        child_ptr.write().unwrap().parent = Some(Arc::downgrade(parent_ptr));
        parent.children.insert(safe_index, Arc::clone(child_ptr));
        drop(parent); // drop write lock before invalidating cache
        Self::invalidate_caches(parent_ptr);
        
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
            drop(parent);
            Self::invalidate_caches(parent_ptr);
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

    /// Clones a node.
    ///
    /// Per the DOM specification:
    /// - Event listeners are NOT cloned.
    /// - The cloned node has no parent (`parent` is `None`) until it is appended to another node.
    /// - If `deep` is true, all descendants are also cloned recursively.
    pub fn clone_node(node_ptr: &NodePtr, deep: bool) -> NodePtr {
        let node = node_ptr.read().unwrap();
        // We clone the inner data, but intentionally do not clone listeners.
        // Node::new will initialize `parent` to None and an empty listeners map.
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
        Self::to_html_inner(node_ptr, None)
    }

    fn to_html_inner(node_ptr: &NodePtr, parent_tag: Option<&str>) -> String {
        let node = node_ptr.read().unwrap();
        match &node.data {
            NodeData::Document => {
                node.children.iter().map(|c| Self::to_html_inner(c, None)).collect::<Vec<_>>().join("")
            }
            NodeData::Element(el) => {
                let mut attrs = String::new();
                for (k, v) in &el.attributes {
                    attrs.push_str(&format!(" {}=\"{}\"", k, Self::escape_html_attr(v)));
                }
                
                let tag_lower = el.tag_name.to_lowercase();
                if Self::is_void_element(&tag_lower) {
                    format!("<{}{}>", el.tag_name, attrs)
                } else {
                    let children_html = node.children.iter().map(|c| Self::to_html_inner(c, Some(&tag_lower))).collect::<Vec<_>>().join("");
                    format!("<{}{}>{}</{}>", el.tag_name, attrs, children_html, el.tag_name)
                }
            }
            NodeData::Text(t) => {
                if let Some(tag) = parent_tag {
                    if matches!(tag, "script" | "style" | "xmp" | "iframe" | "noembed" | "noframes" | "plaintext" | "noscript") {
                        return t.clone();
                    }
                }
                Self::escape_html_text(t)
            }
            NodeData::Comment(c) => format!("<!--{}-->", c),
        }
    }

    fn escape_html_text(text: &str) -> String {
        text.replace("&", "&amp;")
            .replace("<", "&lt;")
            .replace(">", "&gt;")
    }

    fn escape_html_attr(attr: &str) -> String {
        attr.replace("&", "&amp;")
            .replace("\"", "&quot;")
    }

    fn is_void_element(tag: &str) -> bool {
        matches!(
            tag,
            "area" | "base" | "br" | "col" | "embed" | "hr" | "img" | "input" | "link" | "meta" | "param" | "source" | "track" | "wbr"
        )
    }

    /// Recursively searches for an element with the given ID.
    pub fn get_element_by_id(node: &NodePtr, id: &str) -> Option<NodePtr> {
        Self::ensure_id_cache(node);
        let n = node.read().unwrap();
        if let Some(cache) = &n.id_cache {
            if let Some(weak) = cache.get(id) {
                return weak.upgrade();
            }
        }
        None
    }

    fn ensure_id_cache(node: &NodePtr) {
        let is_none = node.read().unwrap().id_cache.is_none();
        if is_none {
            let mut cache = HashMap::new();
            Self::build_id_cache(node, &mut cache);
            node.write().unwrap().id_cache = Some(cache);
        }
    }

    fn build_id_cache(node: &NodePtr, cache: &mut HashMap<String, WeakNodePtr>) {
        let n = node.read().unwrap();
        if let NodeData::Element(ref el) = n.data {
            if let Some(id) = el.id() {
                if !cache.contains_key(id) {
                    cache.insert(id.clone(), Arc::downgrade(node));
                }
            }
        }
        for child in &n.children {
            Self::build_id_cache(child, cache);
        }
    }

    /// Recursively collects all elements matching the given tag name.
    pub fn get_elements_by_tag_name(node: &NodePtr, tag_name: &str) -> Vec<NodePtr> {
        Self::ensure_tag_cache(node);
        let n = node.read().unwrap();
        let mut results = Vec::new();
        if let Some(cache) = &n.tag_cache {
            if let Some(weaks) = cache.get(tag_name) {
                for weak in weaks {
                    if let Some(ptr) = weak.upgrade() {
                        results.push(ptr);
                    }
                }
            }
        }
        results
    }

    fn ensure_tag_cache(node: &NodePtr) {
        let is_none = node.read().unwrap().tag_cache.is_none();
        if is_none {
            let mut cache = HashMap::new();
            Self::build_tag_cache(node, &mut cache);
            node.write().unwrap().tag_cache = Some(cache);
        }
    }

    fn build_tag_cache(node: &NodePtr, cache: &mut HashMap<String, Vec<WeakNodePtr>>) {
        let n = node.read().unwrap();
        if let NodeData::Element(ref el) = n.data {
            cache.entry(el.tag_name.clone()).or_default().push(Arc::downgrade(node));
        }
        for child in &n.children {
            Self::build_tag_cache(child, cache);
        }
    }

    /// Recursively collects all elements containing the given class name.
    pub fn get_elements_by_class_name(node: &NodePtr, class_name: &str) -> Vec<NodePtr> {
        Self::ensure_class_cache(node);
        let n = node.read().unwrap();
        let mut results = Vec::new();
        if let Some(cache) = &n.class_cache {
            if let Some(weaks) = cache.get(class_name) {
                for weak in weaks {
                    if let Some(ptr) = weak.upgrade() {
                        results.push(ptr);
                    }
                }
            }
        }
        results
    }

    fn ensure_class_cache(node: &NodePtr) {
        let is_none = node.read().unwrap().class_cache.is_none();
        if is_none {
            let mut cache = HashMap::new();
            Self::build_class_cache(node, &mut cache);
            node.write().unwrap().class_cache = Some(cache);
        }
    }

    fn build_class_cache(node: &NodePtr, cache: &mut HashMap<String, Vec<WeakNodePtr>>) {
        let n = node.read().unwrap();
        if let NodeData::Element(ref el) = n.data {
            for c in el.classes() {
                cache.entry(c.to_string()).or_default().push(Arc::downgrade(node));
            }
        }
        for child in &n.children {
            Self::build_class_cache(child, cache);
        }
    }

    pub fn add_event_listener(
        node_ptr: &NodePtr,
        event_type: &str,
        listener: Arc<dyn EventListener>,
        use_capture: bool,
    ) {
        let mut node = node_ptr.write().unwrap();
        let entries = node.listeners.entry(event_type.to_string()).or_default();
        entries.push(EventListenerEntry { listener, use_capture });
    }

    pub fn dispatch_event(node_ptr: &NodePtr, event: &mut Event) -> bool {
        event.target = Some(Arc::clone(node_ptr));

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
            event.current_target = Some(Arc::clone(ptr));
            Self::invoke_listeners(ptr, event);
        }

        if !event.propagation_stopped {
            event.phase = EventPhase::AtTarget;
            event.current_target = Some(Arc::clone(node_ptr));
            Self::invoke_listeners(node_ptr, event);
        }

        if event.bubbles && !event.propagation_stopped {
            event.phase = EventPhase::BubblingPhase;
            for ptr in path.iter() {
                if event.propagation_stopped { break; }
                event.current_target = Some(Arc::clone(ptr));
                Self::invoke_listeners(ptr, event);
            }
        }

        event.phase = EventPhase::None;
        event.current_target = None;
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
