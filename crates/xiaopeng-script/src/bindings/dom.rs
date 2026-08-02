//! DOM API Bindings for Boa
//! 
//! Exposes DOM manipulation via native functions and sets up JS wrappers.

use boa_engine::{Context, JsResult, JsValue, NativeFunction, Source};
use lazy_static::lazy_static;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tracing::{info, warn};
use xiaopeng_common::XiaopengResult;
use xiaopeng_dom::{NodeData, NodePtr, ElementData, Node};

lazy_static! {
    /// Global registry mapping JS IDs (which are raw pointer addresses) to NodePtrs.
    /// This keeps nodes alive and accessible from JS.
    static ref JS_NODES: RwLock<HashMap<usize, NodePtr>> = RwLock::new(HashMap::new());
}

/// Register a node in the global registry and return its unique ID (address).
pub fn expose_node(node: NodePtr) -> usize {
    let id = Arc::as_ptr(&node) as usize;
    JS_NODES.write().unwrap().insert(id, node);
    id
}

/// Retrieve a node from the global registry by its ID.
pub fn get_node(id: usize) -> Option<NodePtr> {
    JS_NODES.read().unwrap().get(&id).cloned()
}

// ---------------------------------------------------------------------------
// Native Builtin Functions
// ---------------------------------------------------------------------------

fn dom_get_document_id(_this: &JsValue, _args: &[JsValue], _ctx: &mut Context) -> JsResult<JsValue> {
    // For now, if we don't have a document set yet, we just return 0.
    // The engine should set the document into the JS_NODES manually when it starts.
    // We will retrieve it by assuming there is a special ID 0 for the document, OR
    // we let the BrowserEngine call a function to set it.
    Ok(JsValue::from(0_i32)) // Placeholder, will be replaced by dynamic injection
}

fn dom_get_element_by_id(_this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let parent_id = args.get(0).and_then(|v| v.as_number()).unwrap_or(0.0) as usize;
    let target_id = args.get(1).unwrap_or(&JsValue::undefined()).to_string(ctx)?.to_std_string_escaped();
    
    if let Some(parent) = get_node(parent_id) {
        if let Some(found) = Node::get_element_by_id(&parent, &target_id) {
            let id = expose_node(found);
            return Ok(JsValue::from(id as f64));
        }
    }
    Ok(JsValue::null())
}

fn dom_create_element(_this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let tag = args.get(0).unwrap_or(&JsValue::undefined()).to_string(ctx)?.to_std_string_escaped();
    let node = Node::new(NodeData::Element(ElementData::new(tag)));
    let id = expose_node(node);
    Ok(JsValue::from(id as f64))
}

fn dom_append_child(_this: &JsValue, args: &[JsValue], _ctx: &mut Context) -> JsResult<JsValue> {
    let parent_id = args.get(0).and_then(|v| v.as_number()).unwrap_or(0.0) as usize;
    let child_id = args.get(1).and_then(|v| v.as_number()).unwrap_or(0.0) as usize;
    
    if let (Some(parent), Some(child)) = (get_node(parent_id), get_node(child_id)) {
        Node::append_child(&parent, &child);
    }
    Ok(JsValue::undefined())
}

fn dom_get_text_content(_this: &JsValue, args: &[JsValue], _ctx: &mut Context) -> JsResult<JsValue> {
    let node_id = args.get(0).and_then(|v| v.as_number()).unwrap_or(0.0) as usize;
    if let Some(node) = get_node(node_id) {
        let text = node.read().unwrap().text_content();
        return Ok(JsValue::from(boa_engine::JsString::from(text.as_str())));
    }
    Ok(JsValue::from(boa_engine::JsString::from("")))
}

fn dom_set_text_content(_this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let node_id = args.get(0).and_then(|v| v.as_number()).unwrap_or(0.0) as usize;
    let text = args.get(1).unwrap_or(&JsValue::undefined()).to_string(ctx)?.to_std_string_escaped();
    
    if let Some(node) = get_node(node_id) {
        let mut n = node.write().unwrap();
        // Overwrite children with a single text node
        n.children.clear();
        let text_node = Node::new(NodeData::Text(text));
        text_node.write().unwrap().parent = Some(Arc::downgrade(&node));
        n.children.push(text_node);
    }
    Ok(JsValue::undefined())
}

fn dom_get_attribute(_this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let node_id = args.get(0).and_then(|v| v.as_number()).unwrap_or(0.0) as usize;
    let name = args.get(1).unwrap_or(&JsValue::undefined()).to_string(ctx)?.to_std_string_escaped();
    
    if let Some(node) = get_node(node_id) {
        if let NodeData::Element(ref el) = node.read().unwrap().data {
            if let Some(val) = el.get_attribute(&name) {
                return Ok(JsValue::from(boa_engine::JsString::from(val.as_str())));
            }
        }
    }
    Ok(JsValue::null())
}

fn dom_set_attribute(_this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let node_id = args.get(0).and_then(|v| v.as_number()).unwrap_or(0.0) as usize;
    let name = args.get(1).unwrap_or(&JsValue::undefined()).to_string(ctx)?.to_std_string_escaped();
    let value = args.get(2).unwrap_or(&JsValue::undefined()).to_string(ctx)?.to_std_string_escaped();
    
    if let Some(node) = get_node(node_id) {
        let mut n = node.write().unwrap();
        if let NodeData::Element(ref mut el) = n.data {
            el.set_attribute(name, value);
        }
    }
    Ok(JsValue::undefined())
}

fn dom_classlist_add(_this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let node_id = args.get(0).and_then(|v| v.as_number()).unwrap_or(0.0) as usize;
    let cls = args.get(1).unwrap_or(&JsValue::undefined()).to_string(ctx)?.to_std_string_escaped();
    
    if let Some(node) = get_node(node_id) {
        let mut n = node.write().unwrap();
        if let NodeData::Element(ref mut el) = n.data {
            el.add_class(&cls);
        }
    }
    Ok(JsValue::undefined())
}

fn dom_classlist_remove(_this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let node_id = args.get(0).and_then(|v| v.as_number()).unwrap_or(0.0) as usize;
    let cls = args.get(1).unwrap_or(&JsValue::undefined()).to_string(ctx)?.to_std_string_escaped();
    
    if let Some(node) = get_node(node_id) {
        let mut n = node.write().unwrap();
        if let NodeData::Element(ref mut el) = n.data {
            el.remove_class(&cls);
        }
    }
    Ok(JsValue::undefined())
}

fn dom_classlist_contains(_this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let node_id = args.get(0).and_then(|v| v.as_number()).unwrap_or(0.0) as usize;
    let cls = args.get(1).unwrap_or(&JsValue::undefined()).to_string(ctx)?.to_std_string_escaped();
    
    if let Some(node) = get_node(node_id) {
        if let NodeData::Element(ref el) = node.read().unwrap().data {
            return Ok(JsValue::from(el.has_class(&cls)));
        }
    }
    Ok(JsValue::from(false))
}

// ---------------------------------------------------------------------------
// Registration
// ---------------------------------------------------------------------------

pub fn register_dom_api(ctx: &mut Context) -> XiaopengResult<()> {
    let mut reg = |name: &str, func: fn(&JsValue, &[JsValue], &mut Context) -> JsResult<JsValue>| -> XiaopengResult<()> {
        ctx.register_global_builtin_callable(boa_engine::JsString::from(name), 2, NativeFunction::from_fn_ptr(func))
            .map_err(|e| crate::runtime::map_boa_err(e))?;
        Ok(())
    };

    reg("____dom_get_document_id", dom_get_document_id)?;
    reg("____dom_get_element_by_id", dom_get_element_by_id)?;
    reg("____dom_create_element", dom_create_element)?;
    reg("____dom_append_child", dom_append_child)?;
    reg("____dom_get_text_content", dom_get_text_content)?;
    reg("____dom_set_text_content", dom_set_text_content)?;
    reg("____dom_get_attribute", dom_get_attribute)?;
    reg("____dom_set_attribute", dom_set_attribute)?;
    reg("____dom_classlist_add", dom_classlist_add)?;
    reg("____dom_classlist_remove", dom_classlist_remove)?;
    reg("____dom_classlist_contains", dom_classlist_contains)?;

    // 2. Define JS classes that wrap the IDs
    let js_classes = r#"
class Node {
    constructor(id) {
        this.__id = id;
    }
    appendChild(child) {
        ____dom_append_child(this.__id, child.__id);
        return child;
    }
    get textContent() {
        return ____dom_get_text_content(this.__id);
    }
    set textContent(val) {
        ____dom_set_text_content(this.__id, String(val));
    }
}

class DOMTokenList {
    constructor(id) {
        this.__id = id;
    }
    add(cls) {
        ____dom_classlist_add(this.__id, String(cls));
    }
    remove(cls) {
        ____dom_classlist_remove(this.__id, String(cls));
    }
    contains(cls) {
        return ____dom_classlist_contains(this.__id, String(cls));
    }
}

class Element extends Node {
    constructor(id) {
        super(id);
    }
    getAttribute(name) {
        return ____dom_get_attribute(this.__id, String(name));
    }
    setAttribute(name, value) {
        ____dom_set_attribute(this.__id, String(name), String(value));
    }
    get id() {
        return this.getAttribute('id') || '';
    }
    set id(val) {
        this.setAttribute('id', val);
    }
    get classList() {
        return new DOMTokenList(this.__id);
    }
}

class Document extends Node {
    constructor(id) {
        super(id);
    }
    getElementById(id) {
        let nodeId = ____dom_get_element_by_id(this.__id, String(id));
        if (nodeId !== null) return new Element(nodeId);
        return null;
    }
    createElement(tag) {
        let nodeId = ____dom_create_element(String(tag));
        return new Element(nodeId);
    }
}

// We don't instantiate `document` immediately here because the engine 
// might need to inject the document ID after JS initialization.
// The engine will call `__init_document(id)` to set up the global document.
function ____init_document(id) {
    globalThis.document = new Document(id);
}
    "#;

    ctx.eval(Source::from_bytes(js_classes))
        .map_err(|e| crate::runtime::map_boa_err(e))?;

    info!("DOM API JS bindings registered");
    Ok(())
}
