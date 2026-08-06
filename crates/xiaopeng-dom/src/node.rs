//! DOM Node definitions

use std::collections::HashMap;
use std::sync::{OnceLock, RwLock, RwLockReadGuard, RwLockWriteGuard, Arc};
use tracing::debug;
use crate::event::{Event, EventPhase, EventListenerEntry, EventListener};
use indextree::{Arena, NodeId};

pub fn dom_arena() -> &'static RwLock<Arena<Node>> {
    static ARENA: OnceLock<RwLock<Arena<Node>>> = OnceLock::new();
    ARENA.get_or_init(|| RwLock::new(Arena::new()))
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct NodePtr(pub NodeId);

pub type WeakNodePtr = NodePtr;

pub struct NodeGuard<'a> {
    guard: RwLockReadGuard<'a, Arena<Node>>,
    id: NodeId,
}

impl<'a> std::ops::Deref for NodeGuard<'a> {
    type Target = Node;
    fn deref(&self) -> &Self::Target {
        self.guard.get(self.id).expect("Unwrap failed").get()
    }
}

pub struct NodeWriteGuard<'a> {
    guard: RwLockWriteGuard<'a, Arena<Node>>,
    id: NodeId,
}

impl<'a> std::ops::Deref for NodeWriteGuard<'a> {
    type Target = Node;
    fn deref(&self) -> &Self::Target {
        self.guard.get(self.id).expect("Unwrap failed").get()
    }
}

impl<'a> std::ops::DerefMut for NodeWriteGuard<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.guard.get_mut(self.id).expect("Unwrap failed").get_mut()
    }
}

impl NodePtr {
    pub fn read(&self) -> Result<NodeGuard, ()> {
        Ok(NodeGuard {
            guard: dom_arena().read().expect("Lock poisoned"),
            id: self.0,
        })
    }
    
    pub fn write(&self) -> Result<NodeWriteGuard, ()> {
        Ok(NodeWriteGuard {
            guard: dom_arena().write().expect("Lock poisoned"),
            id: self.0,
        })
    }

    pub fn upgrade(&self) -> Option<NodePtr> {
        Some(*self)
    }

    pub fn clone_ptr(ptr: &NodePtr) -> NodePtr {
        *ptr
    }

    pub fn downgrade(ptr: &NodePtr) -> WeakNodePtr {
        *ptr
    }

    pub fn ptr_eq(a: &NodePtr, b: &NodePtr) -> bool {
        a.0 == b.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub enum NodeType {
    Element = 1,
    Attribute = 2,
    Text = 3,
    CDataSection = 4,
    EntityReference = 5,
    Entity = 6,
    ProcessingInstruction = 7,
    Comment = 8,
    Document = 9,
    DocumentType = 10,
    DocumentFragment = 11,
    Notation = 12,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttrData {
    pub namespace_uri: Option<String>,
    pub prefix: Option<String>,
    pub local_name: String,
    pub value: String,
}

impl AttrData {
    pub fn name(&self) -> String {
        if let Some(prefix) = &self.prefix {
            format!("{}:{}", prefix, self.local_name)
        } else {
            self.local_name.clone()
        }
    }
}

#[derive(Debug, Clone)]
pub struct NamedNodeMap {
    pub items: Vec<AttrData>,
}

impl NamedNodeMap {
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }
    
    pub fn get_named_item(&self, name: &str) -> Option<&AttrData> {
        self.items.iter().find(|a| a.name() == name)
    }

    pub fn get_named_item_ns(&self, namespace_uri: Option<&str>, local_name: &str) -> Option<&AttrData> {
        self.items.iter().find(|a| a.namespace_uri.as_deref() == namespace_uri && a.local_name == local_name)
    }

    pub fn set_named_item(&mut self, attr: AttrData) {
        if let Some(existing) = self.items.iter_mut().find(|a| a.name() == attr.name()) {
            *existing = attr;
        } else {
            self.items.push(attr);
        }
    }

    pub fn set_named_item_ns(&mut self, attr: AttrData) {
        let ns = attr.namespace_uri.clone();
        let local = attr.local_name.clone();
        if let Some(existing) = self.items.iter_mut().find(|a| a.namespace_uri == ns && a.local_name == local) {
            *existing = attr;
        } else {
            self.items.push(attr);
        }
    }

    pub fn remove_named_item(&mut self, name: &str) {
        self.items.retain(|a| a.name() != name);
    }

    pub fn remove_named_item_ns(&mut self, namespace_uri: Option<&str>, local_name: &str) {
        self.items.retain(|a| !(a.namespace_uri.as_deref() == namespace_uri && a.local_name == local_name));
    }
    
    pub fn iter(&self) -> std::slice::Iter<'_, AttrData> {
        self.items.iter()
    }
}

impl<'a> IntoIterator for &'a NamedNodeMap {
    type Item = &'a AttrData;
    type IntoIter = std::slice::Iter<'a, AttrData>;

    fn into_iter(self) -> Self::IntoIter {
        self.items.iter()
    }
}

#[derive(Debug, Clone)]
pub struct ElementData {
    pub namespace_uri: Option<String>,
    pub prefix: Option<String>,
    pub local_name: String,
    pub tag_name: String,
    pub attributes: NamedNodeMap,
}

impl ElementData {
    pub fn new(tag_name: String) -> Self {
        Self {
            namespace_uri: None,
            prefix: None,
            local_name: tag_name.clone(),
            tag_name,
            attributes: NamedNodeMap::new(),
        }
    }

    pub fn new_with_namespace(namespace_uri: Option<String>, prefix: Option<String>, local_name: String, tag_name: String) -> Self {
        Self {
            namespace_uri,
            prefix,
            local_name,
            tag_name,
            attributes: NamedNodeMap::new(),
        }
    }

    pub fn get_attribute(&self, name: &str) -> Option<&String> {
        self.attributes.get_named_item(name).map(|a| &a.value)
    }

    pub fn set_attribute(&mut self, name: String, value: String) {
        crate::node::mark_dom_dirty();
        self.attributes.set_named_item(AttrData {
            namespace_uri: None,
            prefix: None,
            local_name: name,
            value,
        });
    }

    pub fn has_attribute(&self, name: &str) -> bool {
        self.attributes.get_named_item(name).is_some()
    }

    pub fn remove_attribute(&mut self, name: &str) {
        crate::node::mark_dom_dirty();
        self.attributes.remove_named_item(name);
    }

    pub fn get_attribute_ns(&self, namespace_uri: Option<&str>, local_name: &str) -> Option<&String> {
        self.attributes.get_named_item_ns(namespace_uri, local_name).map(|a| &a.value)
    }

    pub fn set_attribute_ns(&mut self, namespace_uri: Option<String>, prefix: Option<String>, local_name: String, value: String) {
        crate::node::mark_dom_dirty();
        self.attributes.set_named_item_ns(AttrData {
            namespace_uri,
            prefix,
            local_name,
            value,
        });
    }

    pub fn has_attribute_ns(&self, namespace_uri: Option<&str>, local_name: &str) -> bool {
        self.attributes.get_named_item_ns(namespace_uri, local_name).is_some()
    }

    pub fn remove_attribute_ns(&mut self, namespace_uri: Option<&str>, local_name: &str) {
        crate::node::mark_dom_dirty();
        self.attributes.remove_named_item_ns(namespace_uri, local_name);
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
pub struct DocumentTypeData {
    pub name: String,
    pub public_id: String,
    pub system_id: String,
}

#[derive(Debug, Clone)]
pub struct ProcessingInstructionData {
    pub target: String,
    pub data: String,
}

#[derive(Debug, Clone)]
pub enum NodeData {
    Document,
    DocumentType(DocumentTypeData),
    DocumentFragment,
    Element(ElementData),
    Attr(AttrData),
    Text(String),
    CDataSection(String),
    ProcessingInstruction(ProcessingInstructionData),
    Comment(String),
    Entity(String),
    EntityReference(String),
    Notation(String),
}

use std::sync::atomic::{AtomicBool, Ordering};

pub static IS_DOM_DIRTY: AtomicBool = AtomicBool::new(false);

pub fn mark_dom_dirty() {
    IS_DOM_DIRTY.store(true, Ordering::SeqCst);
}

pub fn take_dom_dirty() -> bool {
    IS_DOM_DIRTY.swap(false, Ordering::SeqCst)
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
        let mut arena = dom_arena().write().expect("Lock poisoned");
        let id = arena.new_node(Node {
            parent: None,
            children: Vec::new(),
            data,
            listeners: HashMap::new(),
            id_cache: None,
            tag_cache: None,
            class_cache: None,
        });
        NodePtr(id)
    }

    pub fn node_type(&self) -> NodeType {
        match self.data {
            NodeData::Document => NodeType::Document,
            NodeData::DocumentType(_) => NodeType::DocumentType,
            NodeData::DocumentFragment => NodeType::DocumentFragment,
            NodeData::Element(_) => NodeType::Element,
            NodeData::Attr(_) => NodeType::Attribute,
            NodeData::Text(_) => NodeType::Text,
            NodeData::CDataSection(_) => NodeType::CDataSection,
            NodeData::ProcessingInstruction(_) => NodeType::ProcessingInstruction,
            NodeData::Comment(_) => NodeType::Comment,
            NodeData::Entity(_) => NodeType::Entity,
            NodeData::EntityReference(_) => NodeType::EntityReference,
            NodeData::Notation(_) => NodeType::Notation,
        }
    }

    pub fn invalidate_caches(node_ptr: &NodePtr) {
        crate::node::mark_dom_dirty();
        let mut current = Some(NodePtr::clone_ptr(node_ptr));
        while let Some(n) = current {
            let mut n_write = n.write().expect("Lock poisoned");
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
        
        let old_parent = child_ptr.read().expect("Lock poisoned").parent.clone().and_then(|w| w.upgrade());
        if let Some(old_parent) = old_parent {
            old_parent.write().expect("Lock poisoned").children.retain(|c| !NodePtr::ptr_eq(c, child_ptr));
            Self::invalidate_caches(&old_parent);
        }

        // Set new parent
        child_ptr.write().expect("Lock poisoned").parent = Some(NodePtr::downgrade(parent_ptr));
        parent_ptr.write().expect("Lock poisoned").children.push(NodePtr::clone_ptr(child_ptr));
        Self::invalidate_caches(parent_ptr);
    }

    pub fn insert_before(parent_ptr: &NodePtr, child_ptr: &NodePtr, index: usize) -> Result<(), &'static str> {
        debug!("Inserting child into parent DOM Node at index {}", index);
        
        // 1. Strict boundary check (read lock only, released immediately)
        {
            let parent = parent_ptr.read().expect("Lock poisoned");
            if index >= parent.children.len() {
                return Err("IndexOutOfBounds: insert_before requires index < children.len()");
            }
        }

        // 2. Remove from old parent if exists.
        // We do this BEFORE acquiring parent_ptr's write lock to avoid deadlock if old_parent == parent_ptr
        let old_parent = child_ptr.read().expect("Lock poisoned").parent.as_ref().and_then(|w| w.upgrade());
        if let Some(ref old_p) = old_parent {
            old_p.write().expect("Lock poisoned").children.retain(|c| !NodePtr::ptr_eq(c, child_ptr));
            Self::invalidate_caches(old_p);
        }

        // 3. Insert into the new parent.
        let mut parent = parent_ptr.write().expect("Lock poisoned");
        // If old_parent == parent_ptr, retaining the child above might have shrunk the children vec.
        // We clamp the index to prevent out-of-bounds panics after removal.
        let safe_index = index.min(parent.children.len());
        
        child_ptr.write().expect("Lock poisoned").parent = Some(NodePtr::downgrade(parent_ptr));
        parent.children.insert(safe_index, NodePtr::clone_ptr(child_ptr));
        drop(parent); // drop write lock before invalidating cache
        Self::invalidate_caches(parent_ptr);
        
        Ok(())
    }

    pub fn insert_before_node(parent_ptr: &NodePtr, child_ptr: &NodePtr, reference_ptr: &NodePtr) -> Result<(), &'static str> {
        let index = {
            let parent = parent_ptr.read().expect("Lock poisoned");
            parent.children.iter().position(|c| NodePtr::ptr_eq(c, reference_ptr))
        };
        
        if let Some(idx) = index {
            Self::insert_before(parent_ptr, child_ptr, idx)
        } else {
            Err("ReferenceNodeNotFound: The reference node is not a child of the parent")
        }
    }

    pub fn remove_child(parent_ptr: &NodePtr, child_ptr: &NodePtr) -> Option<NodePtr> {
        debug!("Removing child from parent DOM Node");
        let mut parent = parent_ptr.write().expect("Lock poisoned");
        let index = parent.children.iter().position(|c| NodePtr::ptr_eq(c, child_ptr));
        
        if let Some(idx) = index {
            let removed = parent.children.remove(idx);
            removed.write().expect("Lock poisoned").parent = None;
            drop(parent);
            Self::invalidate_caches(parent_ptr);
            Some(removed)
        } else {
            None
        }
    }

    pub fn text_content(&self) -> String {
        match &self.data {
            NodeData::Text(t) | NodeData::CDataSection(t) => t.clone(),
            NodeData::Attr(attr) => attr.value.clone(),
            NodeData::Comment(_) | NodeData::ProcessingInstruction(_) | NodeData::DocumentType(_) 
            | NodeData::Notation(_) | NodeData::Entity(_) | NodeData::EntityReference(_) => String::new(),
            NodeData::Document | NodeData::DocumentFragment | NodeData::Element(_) => {
                let mut content = String::new();
                for child in &self.children {
                    content.push_str(&child.read().expect("Lock poisoned").text_content());
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
        self.children.iter().find(|c| c.read().expect("Lock poisoned").node_type() == NodeType::Element).cloned()
    }

    pub fn last_element_child(&self) -> Option<NodePtr> {
        self.children.iter().rev().find(|c| c.read().expect("Lock poisoned").node_type() == NodeType::Element).cloned()
    }

    pub fn next_element_sibling(node_ptr: &NodePtr) -> Option<NodePtr> {
        let parent = {
            let node = node_ptr.read().expect("Lock poisoned");
            node.parent.as_ref().and_then(|w| w.upgrade())
        };
        if let Some(parent) = parent {
            let p = parent.read().expect("Lock poisoned");
            let pos = p.children.iter().position(|c| NodePtr::ptr_eq(c, node_ptr))?;
            for sibling in p.children.iter().skip(pos + 1) {
                if sibling.read().expect("Lock poisoned").node_type() == NodeType::Element {
                    return Some(NodePtr::clone_ptr(sibling));
                }
            }
        }
        None
    }

    pub fn previous_element_sibling(node_ptr: &NodePtr) -> Option<NodePtr> {
        let parent = {
            let node = node_ptr.read().expect("Lock poisoned");
            node.parent.as_ref().and_then(|w| w.upgrade())
        };
        if let Some(parent) = parent {
            let p = parent.read().expect("Lock poisoned");
            let pos = p.children.iter().position(|c| NodePtr::ptr_eq(c, node_ptr))?;
            for sibling in p.children.iter().take(pos).rev() {
                if sibling.read().expect("Lock poisoned").node_type() == NodeType::Element {
                    return Some(NodePtr::clone_ptr(sibling));
                }
            }
        }
        None
    }

    pub fn child_element_count(&self) -> usize {
        self.children.iter().filter(|c| c.read().expect("Lock poisoned").node_type() == NodeType::Element).count()
    }

    /// Clones a node.
    ///
    /// Per the DOM specification:
    /// - Event listeners are NOT cloned.
    /// - The cloned node has no parent (`parent` is `None`) until it is appended to another node.
    /// - If `deep` is true, all descendants are also cloned recursively.
    pub fn clone_node(node_ptr: &NodePtr, deep: bool) -> NodePtr {
        let node = node_ptr.read().expect("Lock poisoned");
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
        let node = node_ptr.read().expect("Lock poisoned");
        match &node.data {
            NodeData::Document | NodeData::DocumentFragment => {
                node.children.iter().map(|c| Self::to_html_inner(c, None)).collect::<Vec<_>>().join("")
            }
            NodeData::DocumentType(dt) => {
                let mut html = format!("<!DOCTYPE {}", dt.name);
                if !dt.public_id.is_empty() {
                    html.push_str(&format!(" PUBLIC \"{}\"", dt.public_id));
                    if !dt.system_id.is_empty() {
                        html.push_str(&format!(" \"{}\"", dt.system_id));
                    }
                } else if !dt.system_id.is_empty() {
                    html.push_str(&format!(" SYSTEM \"{}\"", dt.system_id));
                }
                html.push_str(">");
                html
            }
            NodeData::Element(el) => {
                let mut attrs = String::new();
                for attr in &el.attributes {
                    attrs.push_str(&format!(" {}=\"{}\"", attr.name(), Self::escape_html_attr(&attr.value)));
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
            NodeData::CDataSection(c) => format!("<![CDATA[{}]]>", c),
            NodeData::ProcessingInstruction(pi) => format!("<?{} {}?>", pi.target, pi.data),
            NodeData::Comment(c) => format!("<!--{}-->", c),
            _ => String::new(),
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
        let n = node.read().expect("Lock poisoned");
        if let Some(cache) = &n.id_cache {
            if let Some(weak) = cache.get(id) {
                return weak.upgrade();
            }
        }
        None
    }

    fn ensure_id_cache(node: &NodePtr) {
        let is_none = node.read().expect("Lock poisoned").id_cache.is_none();
        if is_none {
            let mut cache = HashMap::new();
            Self::build_id_cache(node, &mut cache);
            node.write().expect("Lock poisoned").id_cache = Some(cache);
        }
    }

    fn build_id_cache(node: &NodePtr, cache: &mut HashMap<String, WeakNodePtr>) {
        let n = node.read().expect("Lock poisoned");
        if let NodeData::Element(ref el) = n.data {
            if let Some(id) = el.id() {
                if !cache.contains_key(id) {
                    cache.insert(id.clone(), NodePtr::downgrade(node));
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
        let n = node.read().expect("Lock poisoned");
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
        let is_none = node.read().expect("Lock poisoned").tag_cache.is_none();
        if is_none {
            let mut cache = HashMap::new();
            Self::build_tag_cache(node, &mut cache);
            node.write().expect("Lock poisoned").tag_cache = Some(cache);
        }
    }

    fn build_tag_cache(node: &NodePtr, cache: &mut HashMap<String, Vec<WeakNodePtr>>) {
        let n = node.read().expect("Lock poisoned");
        if let NodeData::Element(ref el) = n.data {
            cache.entry(el.tag_name.clone()).or_default().push(NodePtr::downgrade(node));
        }
        for child in &n.children {
            Self::build_tag_cache(child, cache);
        }
    }

    /// Recursively collects all elements containing the given class name.
    pub fn get_elements_by_class_name(node: &NodePtr, class_name: &str) -> Vec<NodePtr> {
        Self::ensure_class_cache(node);
        let n = node.read().expect("Lock poisoned");
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
        let is_none = node.read().expect("Lock poisoned").class_cache.is_none();
        if is_none {
            let mut cache = HashMap::new();
            Self::build_class_cache(node, &mut cache);
            node.write().expect("Lock poisoned").class_cache = Some(cache);
        }
    }

    fn build_class_cache(node: &NodePtr, cache: &mut HashMap<String, Vec<WeakNodePtr>>) {
        let n = node.read().expect("Lock poisoned");
        if let NodeData::Element(ref el) = n.data {
            for c in el.classes() {
                cache.entry(c.to_string()).or_default().push(NodePtr::downgrade(node));
            }
        }
        for child in &n.children {
            Self::build_class_cache(child, cache);
        }
    }

    pub fn query_selector(_node: &NodePtr, _selectors: &str) -> Option<NodePtr> {
        tracing::warn!("querySelector is not natively implemented in xiaopeng-dom. Use xiaopeng-engine for CSS querying.");
        None
    }

    pub fn query_selector_all(_node: &NodePtr, _selectors: &str) -> Vec<NodePtr> {
        tracing::warn!("querySelectorAll is not natively implemented in xiaopeng-dom. Use xiaopeng-engine for CSS querying.");
        Vec::new()
    }

    pub fn add_event_listener(
        node_ptr: &NodePtr,
        event_type: &str,
        listener: Arc<dyn EventListener>,
        use_capture: bool,
    ) {
        let mut node = node_ptr.write().expect("Lock poisoned");
        let entries = node.listeners.entry(event_type.to_string()).or_default();
        entries.push(EventListenerEntry { listener, use_capture });
    }

    pub fn dispatch_event(node_ptr: &NodePtr, event: &mut Event) -> bool {
        event.target = Some(NodePtr::clone_ptr(node_ptr));

        let mut path = Vec::new();
        let mut current = NodePtr::clone_ptr(node_ptr);
        loop {
            let parent = current.read().expect("Lock poisoned").parent.as_ref().and_then(|w| w.upgrade());
            if let Some(p) = parent {
                path.push(NodePtr::clone_ptr(&p));
                current = p;
            } else {
                break;
            }
        }

        event.phase = EventPhase::CapturingPhase;
        for ptr in path.iter().rev() {
            if event.propagation_stopped { break; }
            event.current_target = Some(NodePtr::clone_ptr(ptr));
            Self::invoke_listeners(ptr, event);
        }

        if !event.propagation_stopped {
            event.phase = EventPhase::AtTarget;
            event.current_target = Some(NodePtr::clone_ptr(node_ptr));
            Self::invoke_listeners(node_ptr, event);
        }

        if event.bubbles && !event.propagation_stopped {
            event.phase = EventPhase::BubblingPhase;
            for ptr in path.iter() {
                if event.propagation_stopped { break; }
                event.current_target = Some(NodePtr::clone_ptr(ptr));
                Self::invoke_listeners(ptr, event);
            }
        }

        event.phase = EventPhase::None;
        event.current_target = None;
        !event.default_prevented
    }

    fn invoke_listeners(node_ptr: &NodePtr, event: &mut Event) {
        let listeners_to_run = {
            let node = node_ptr.read().expect("Lock poisoned");
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
        assert!(NodePtr::ptr_eq(&found.expect("Unwrap failed"), &child1));

        // Test get_elements_by_tag_name
        let spans = Node::get_elements_by_tag_name(&root, "span");
        assert_eq!(spans.len(), 1);
        assert!(NodePtr::ptr_eq(&spans[0], &child1));

        // Test get_elements_by_class_name
        let bolds = Node::get_elements_by_class_name(&root, "text-bold");
        assert_eq!(bolds.len(), 2);
    }
}
