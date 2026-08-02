//! Thread-local JS event listener registry
//!
//! JsFunction is not Send, so we use thread_local storage. Since Boa always
//! runs on the same thread that owns the Context, this is safe.

use boa_engine::{Context, JsValue};
use boa_engine::object::builtins::JsFunction;
use std::cell::RefCell;
use std::collections::HashMap;
use tracing::warn;

thread_local! {
    /// node_id → event_type → Vec<(listener_id, JsFunction)>
    static JS_EVENT_LISTENERS: RefCell<HashMap<usize, HashMap<String, Vec<(usize, JsFunction)>>>> =
        RefCell::new(HashMap::new());

    /// Monotonically increasing listener ID counter.
    static LISTENER_ID_COUNTER: RefCell<usize> = RefCell::new(1);
}

fn next_listener_id() -> usize {
    LISTENER_ID_COUNTER.with(|c| {
        let id = *c.borrow();
        *c.borrow_mut() = id + 1;
        id
    })
}

/// Register a JS function as an event listener for `(node_id, event_type)`.
/// Returns a unique listener ID that can be used to remove it later.
pub fn add_js_listener(node_id: usize, event_type: String, func: JsFunction) -> usize {
    let lid = next_listener_id();
    JS_EVENT_LISTENERS.with(|m| {
        m.borrow_mut()
            .entry(node_id)
            .or_default()
            .entry(event_type)
            .or_default()
            .push((lid, func));
    });
    lid
}

/// Remove a listener by its listener_id for the given (node_id, event_type).
pub fn remove_js_listener(node_id: usize, event_type: &str, lid: usize) {
    JS_EVENT_LISTENERS.with(|m| {
        if let Some(types) = m.borrow_mut().get_mut(&node_id) {
            if let Some(listeners) = types.get_mut(event_type) {
                listeners.retain(|(id, _)| *id != lid);
            }
        }
    });
}

/// Remove all listeners for a given node (e.g. when node is GC'd from registry).
pub fn clear_node_listeners(node_id: usize) {
    JS_EVENT_LISTENERS.with(|m| {
        m.borrow_mut().remove(&node_id);
    });
}

/// Invoke all registered JS listeners for `(node_id, event_type)`.
/// `event_obj` is the JS Event object passed as the first argument to each callback.
pub fn invoke_js_listeners(node_id: usize, event_type: &str, event_obj: &JsValue, ctx: &mut Context) {
    // Snapshot the listener list to avoid re-entrant borrow while calling into Boa.
    let funcs: Vec<JsFunction> = JS_EVENT_LISTENERS.with(|m| {
        m.borrow()
            .get(&node_id)
            .and_then(|types| types.get(event_type))
            .map(|v| v.iter().map(|(_, f)| f.clone()).collect())
            .unwrap_or_default()
    });

    for func in funcs {
        if let Err(e) = func.call(&JsValue::undefined(), &[event_obj.clone()], ctx) {
            warn!("[DOM Event] listener threw: {e}");
        }
    }
}
