//! DOM API Bindings for Boa
//!
//! Exposes DOM manipulation via native functions and sets up JS wrappers.

use boa_engine::{Context, JsResult, JsValue, NativeFunction, Source};
use boa_engine::object::builtins::JsFunction;
use lazy_static::lazy_static;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tracing::info;
use xiaopeng_common::XiaopengResult;
use xiaopeng_dom::{NodeData, NodePtr, ElementData, Node};
use crate::bindings::events;

lazy_static! {
    /// Global registry mapping JS IDs (raw pointer addresses) to NodePtrs.
    /// This keeps nodes alive and accessible from JS.
    static ref JS_NODES: RwLock<HashMap<usize, NodePtr>> = RwLock::new(HashMap::new());
}

/// Register a node in the global registry and return its unique ID.
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
// Native Builtin Functions — DOM Manipulation
// ---------------------------------------------------------------------------

fn dom_get_document_id(_this: &JsValue, _args: &[JsValue], _ctx: &mut Context) -> JsResult<JsValue> {
    Ok(JsValue::from(0_i32))
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

fn dom_query_selector(_this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let parent_id = args.get(0).and_then(|v| v.as_number()).unwrap_or(0.0) as usize;
    let selector = args.get(1).unwrap_or(&JsValue::undefined()).to_string(ctx)?.to_std_string_escaped();

    if let Some(parent) = get_node(parent_id) {
        if let Some(node) = xiaopeng_style::query_selector(&parent, &selector) {
            let id = expose_node(node);
            return Ok(JsValue::from(id as f64));
        }
    }
    Ok(JsValue::null())
}

fn dom_query_selector_all(_this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let parent_id = args.get(0).and_then(|v| v.as_number()).unwrap_or(0.0) as usize;
    let selector = args.get(1).unwrap_or(&JsValue::undefined()).to_string(ctx)?.to_std_string_escaped();

    if let Some(parent) = get_node(parent_id) {
        let nodes = xiaopeng_style::query_selector_all(&parent, &selector);

        // Return the IDs as a JS array
        let ids: Vec<JsValue> = nodes.into_iter().map(|n| {
            let id = expose_node(n);
            JsValue::from(id as f64)
        }).collect();

        let arr = boa_engine::object::builtins::JsArray::from_iter(ids, ctx);
        return Ok(JsValue::from(arr));
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

fn dom_remove_child(_this: &JsValue, args: &[JsValue], _ctx: &mut Context) -> JsResult<JsValue> {
    let parent_id = args.get(0).and_then(|v| v.as_number()).unwrap_or(0.0) as usize;
    let child_id = args.get(1).and_then(|v| v.as_number()).unwrap_or(0.0) as usize;

    if let (Some(parent), Some(child)) = (get_node(parent_id), get_node(child_id)) {
        Node::remove_child(&parent, &child);
    }
    Ok(JsValue::undefined())
}

fn dom_get_parent_node(_this: &JsValue, args: &[JsValue], _ctx: &mut Context) -> JsResult<JsValue> {
    let node_id = args.get(0).and_then(|v| v.as_number()).unwrap_or(0.0) as usize;
    if let Some(node) = get_node(node_id) {
        let parent = node.read().unwrap().parent.as_ref().and_then(|w| w.upgrade());
        if let Some(p) = parent {
            let id = expose_node(p);
            return Ok(JsValue::from(id as f64));
        }
    }
    Ok(JsValue::null())
}

fn dom_get_children(_this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let node_id = args.get(0).and_then(|v| v.as_number()).unwrap_or(0.0) as usize;
    if let Some(node) = get_node(node_id) {
        let children = node.read().unwrap().children.clone();
        let ids: Vec<JsValue> = children.into_iter().map(|c| {
            let id = expose_node(c);
            JsValue::from(id as f64)
        }).collect();
        let arr = boa_engine::object::builtins::JsArray::from_iter(ids, ctx);
        return Ok(JsValue::from(arr));
    }
    Ok(JsValue::null())
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
        n.children.clear();
        let text_node = Node::new(NodeData::Text(text));
        text_node.write().unwrap().parent = Some(Arc::downgrade(&node));
        n.children.push(text_node);
    }
    Ok(JsValue::undefined())
}

fn dom_get_inner_html(_this: &JsValue, args: &[JsValue], _ctx: &mut Context) -> JsResult<JsValue> {
    let node_id = args.get(0).and_then(|v| v.as_number()).unwrap_or(0.0) as usize;
    if let Some(node) = get_node(node_id) {
        // Build innerHTML from children
        let children = node.read().unwrap().children.clone();
        let html: String = children.iter().map(Node::to_html).collect();
        return Ok(JsValue::from(boa_engine::JsString::from(html.as_str())));
    }
    Ok(JsValue::from(boa_engine::JsString::from("")))
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

fn dom_remove_attribute(_this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let node_id = args.get(0).and_then(|v| v.as_number()).unwrap_or(0.0) as usize;
    let name = args.get(1).unwrap_or(&JsValue::undefined()).to_string(ctx)?.to_std_string_escaped();

    if let Some(node) = get_node(node_id) {
        let mut n = node.write().unwrap();
        if let NodeData::Element(ref mut el) = n.data {
            el.remove_attribute(&name);
        }
    }
    Ok(JsValue::undefined())
}

fn dom_has_attribute(_this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let node_id = args.get(0).and_then(|v| v.as_number()).unwrap_or(0.0) as usize;
    let name = args.get(1).unwrap_or(&JsValue::undefined()).to_string(ctx)?.to_std_string_escaped();

    if let Some(node) = get_node(node_id) {
        if let NodeData::Element(ref el) = node.read().unwrap().data {
            return Ok(JsValue::from(el.has_attribute(&name)));
        }
    }
    Ok(JsValue::from(false))
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

fn dom_classlist_toggle(_this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let node_id = args.get(0).and_then(|v| v.as_number()).unwrap_or(0.0) as usize;
    let cls = args.get(1).unwrap_or(&JsValue::undefined()).to_string(ctx)?.to_std_string_escaped();

    if let Some(node) = get_node(node_id) {
        let mut n = node.write().unwrap();
        if let NodeData::Element(ref mut el) = n.data {
            if el.has_class(&cls) {
                el.remove_class(&cls);
                return Ok(JsValue::from(false));
            } else {
                el.add_class(&cls);
                return Ok(JsValue::from(true));
            }
        }
    }
    Ok(JsValue::from(false))
}

// ---------------------------------------------------------------------------
// Native Builtin Functions — Event API
// ---------------------------------------------------------------------------

fn dom_add_event_listener(_this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let node_id = args.get(0).and_then(|v| v.as_number()).unwrap_or(0.0) as usize;
    let event_type = args.get(1).unwrap_or(&JsValue::undefined()).to_string(ctx)?.to_std_string_escaped();
    let func = match args.get(2) {
        Some(v) if v.is_callable() => {
            v.as_object().and_then(|o| JsFunction::from_object(o.clone()))
        }
        _ => None,
    };

    if let Some(func) = func {
        let lid = events::add_js_listener(node_id, event_type, func);
        return Ok(JsValue::from(lid as f64));
    }
    Ok(JsValue::from(-1_i32))
}

fn dom_remove_event_listener(_this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let node_id = args.get(0).and_then(|v| v.as_number()).unwrap_or(0.0) as usize;
    let event_type = args.get(1).unwrap_or(&JsValue::undefined()).to_string(ctx)?.to_std_string_escaped();
    let lid = args.get(2).and_then(|v| v.as_number()).unwrap_or(0.0) as usize;

    events::remove_js_listener(node_id, &event_type, lid);
    Ok(JsValue::undefined())
}

fn dom_dispatch_event(_this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let node_id = args.get(0).and_then(|v| v.as_number()).unwrap_or(0.0) as usize;
    // args[1] is the JS Event object (or event type string as shorthand)
    let event_obj = args.get(1).cloned().unwrap_or(JsValue::undefined());

    // Determine event type
    let event_type = if event_obj.is_string() {
        event_obj.to_string(ctx)?.to_std_string_escaped()
    } else if let Some(obj) = event_obj.as_object() {
        obj.get(boa_engine::JsString::from("type"), ctx)?
            .to_string(ctx)?
            .to_std_string_escaped()
    } else {
        return Ok(JsValue::from(false));
    };

    // Dispatch: invoke JS listeners for this node
    events::invoke_js_listeners(node_id, &event_type, &event_obj, ctx);

    // Walk up to parents if event bubbles (we check `bubbles` property)
    let bubbles = if let Some(obj) = event_obj.as_object() {
        obj.get(boa_engine::JsString::from("bubbles"), ctx)?
            .to_boolean()
    } else {
        false
    };

    if bubbles {
        let mut current_id = node_id;
        loop {
            let parent_id = get_node(current_id)
                .and_then(|n| n.read().unwrap().parent.as_ref().and_then(|w| w.upgrade()))
                .map(|p| Arc::as_ptr(&p) as usize);

            match parent_id {
                Some(pid) if pid != 0 => {
                    events::invoke_js_listeners(pid, &event_type, &event_obj, ctx);
                    current_id = pid;
                }
                _ => break,
            }
        }
    }

    Ok(JsValue::from(true))
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

    // DOM Tree Manipulation
    reg("____dom_get_document_id",    dom_get_document_id)?;
    reg("____dom_get_element_by_id",  dom_get_element_by_id)?;
    reg("____dom_query_selector",     dom_query_selector)?;
    reg("____dom_query_selector_all", dom_query_selector_all)?;
    reg("____dom_create_element",     dom_create_element)?;
    reg("____dom_append_child",       dom_append_child)?;
    reg("____dom_remove_child",       dom_remove_child)?;
    reg("____dom_get_parent_node",    dom_get_parent_node)?;
    reg("____dom_get_children",       dom_get_children)?;
    reg("____dom_get_text_content",   dom_get_text_content)?;
    reg("____dom_set_text_content",   dom_set_text_content)?;
    reg("____dom_get_inner_html",     dom_get_inner_html)?;
    reg("____dom_get_attribute",      dom_get_attribute)?;
    reg("____dom_set_attribute",      dom_set_attribute)?;
    reg("____dom_remove_attribute",   dom_remove_attribute)?;
    reg("____dom_has_attribute",      dom_has_attribute)?;
    reg("____dom_classlist_add",      dom_classlist_add)?;
    reg("____dom_classlist_remove",   dom_classlist_remove)?;
    reg("____dom_classlist_contains", dom_classlist_contains)?;
    reg("____dom_classlist_toggle",   dom_classlist_toggle)?;

    // Event API
    reg("____dom_add_event_listener",    dom_add_event_listener)?;
    reg("____dom_remove_event_listener", dom_remove_event_listener)?;
    reg("____dom_dispatch_event",        dom_dispatch_event)?;

    let js_classes = r#"
// ─── Event ────────────────────────────────────────────────────────────────────
class Event {
    constructor(type_, init) {
        this.type = String(type_);
        this.bubbles    = (init && init.bubbles)    ? Boolean(init.bubbles)    : false;
        this.cancelable = (init && init.cancelable) ? Boolean(init.cancelable) : false;
        this.defaultPrevented          = false;
        this.propagationStopped        = false;
        this.immediatePropagationStopped = false;
        this.target     = null;
        this.currentTarget = null;
    }
    preventDefault()             { if (this.cancelable) this.defaultPrevented = true; }
    stopPropagation()            { this.propagationStopped = true; }
    stopImmediatePropagation()   { this.propagationStopped = true; this.immediatePropagationStopped = true; }
}

// ─── EventTarget mixin ────────────────────────────────────────────────────────
// Shared by Node (and thus Element, Document).
class EventTarget {
    addEventListener(type_, callback, _options) {
        if (typeof callback !== 'function') return;
        // Store listener ID (returned by Rust) on the callback itself for later removal.
        var lid = ____dom_add_event_listener(this.__id, String(type_), callback);
        // Save for removeEventListener identity matching
        if (!this.__listenerMap) this.__listenerMap = [];
        this.__listenerMap.push({ type: type_, callback, lid });
    }
    removeEventListener(type_, callback) {
        if (!this.__listenerMap) return;
        for (var i = 0; i < this.__listenerMap.length; i++) {
            var entry = this.__listenerMap[i];
            if (entry.type === type_ && entry.callback === callback) {
                ____dom_remove_event_listener(this.__id, String(type_), entry.lid);
                this.__listenerMap.splice(i, 1);
                break;
            }
        }
    }
    dispatchEvent(event) {
        event.target = this;
        return ____dom_dispatch_event(this.__id, event);
    }
}

// ─── Node ─────────────────────────────────────────────────────────────────────
class Node extends EventTarget {
    constructor(id) {
        super();
        this.__id = id;
    }
    appendChild(child) {
        ____dom_append_child(this.__id, child.__id);
        return child;
    }
    removeChild(child) {
        ____dom_remove_child(this.__id, child.__id);
        return child;
    }
    get parentNode() {
        var pid = ____dom_get_parent_node(this.__id);
        return pid !== null ? new Node(pid) : null;
    }
    get childNodes() {
        var ids = ____dom_get_children(this.__id);
        return ids ? ids.map(id => new Node(id)) : [];
    }
    get textContent() {
        return ____dom_get_text_content(this.__id);
    }
    set textContent(val) {
        ____dom_set_text_content(this.__id, String(val));
    }
}

// ─── DOMTokenList ─────────────────────────────────────────────────────────────
class DOMTokenList {
    constructor(id) {
        this.__id = id;
    }
    add(...classes) {
        for (var c of classes) ____dom_classlist_add(this.__id, String(c));
    }
    remove(...classes) {
        for (var c of classes) ____dom_classlist_remove(this.__id, String(c));
    }
    contains(cls) {
        return ____dom_classlist_contains(this.__id, String(cls));
    }
    toggle(cls, force) {
        if (force !== undefined) {
            if (force) { ____dom_classlist_add(this.__id, String(cls)); return true; }
            else       { ____dom_classlist_remove(this.__id, String(cls)); return false; }
        }
        return ____dom_classlist_toggle(this.__id, String(cls));
    }
}

// ─── Element ──────────────────────────────────────────────────────────────────
class Element extends Node {
    constructor(id) { super(id); }

    getAttribute(name)        { return ____dom_get_attribute(this.__id, String(name)); }
    setAttribute(name, value) { ____dom_set_attribute(this.__id, String(name), String(value)); }
    removeAttribute(name)     { ____dom_remove_attribute(this.__id, String(name)); }
    hasAttribute(name)        { return ____dom_has_attribute(this.__id, String(name)); }

    get id()    { return this.getAttribute('id') || ''; }
    set id(val) { this.setAttribute('id', val); }

    get className()    { return this.getAttribute('class') || ''; }
    set className(val) { this.setAttribute('class', String(val)); }

    get classList() { return new DOMTokenList(this.__id); }

    get innerHTML() { return ____dom_get_inner_html(this.__id); }

    querySelector(sel) {
        var id = ____dom_query_selector(this.__id, String(sel));
        return id !== null ? new Element(id) : null;
    }
    querySelectorAll(sel) {
        var ids = ____dom_query_selector_all(this.__id, String(sel));
        return ids ? ids.map(id => new Element(id)) : [];
    }
    getElementsByTagName(tag) {
        return this.querySelectorAll(tag);
    }
    getElementsByClassName(cls) {
        return this.querySelectorAll('.' + cls);
    }
}

// ─── Document ─────────────────────────────────────────────────────────────────
class Document extends Node {
    constructor(id) { super(id); }

    getElementById(id) {
        var nodeId = ____dom_get_element_by_id(this.__id, String(id));
        return nodeId !== null ? new Element(nodeId) : null;
    }
    createElement(tag) {
        var nodeId = ____dom_create_element(String(tag));
        return new Element(nodeId);
    }
    querySelector(sel) {
        var id = ____dom_query_selector(this.__id, String(sel));
        return id !== null ? new Element(id) : null;
    }
    querySelectorAll(sel) {
        var ids = ____dom_query_selector_all(this.__id, String(sel));
        return ids ? ids.map(id => new Element(id)) : [];
    }
    createEvent(type_) { return new Event(type_); }
}

// Called by BrowserEngine after parsing to inject the root document.
function ____init_document(id) {
    globalThis.document = new Document(id);
}
    "#;

    ctx.eval(Source::from_bytes(js_classes))
        .map_err(|e| crate::runtime::map_boa_err(e))?;

    info!("DOM API JS bindings registered");
    Ok(())
}
