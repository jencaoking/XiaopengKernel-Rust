use std::sync::{Arc, Mutex};
use xiaopeng_dom::{ElementData, Node, NodeData, Event, EventPhase, EventListener};

#[test]
fn test_event_dispatch_phases() {
    let document = Node::new(NodeData::Document);
    
    let mut div_data = ElementData::new("div".into());
    div_data.set_attribute("id".into(), "parent".into());
    let parent = Node::new(NodeData::Element(div_data));
    
    let mut btn_data = ElementData::new("button".into());
    btn_data.set_attribute("id".into(), "child".into());
    let child = Node::new(NodeData::Element(btn_data));

    Node::append_child(&document, &parent);
    Node::append_child(&parent, &child);

    let event_order = Arc::new(Mutex::new(Vec::new()));

    // 1. Parent Capture Phase
    let order_clone = event_order.clone();
    Node::add_event_listener(
        &parent,
        "click",
        Arc::new(move |event: &mut Event| {
            assert_eq!(event.phase, EventPhase::CapturingPhase);
            order_clone.lock().expect("Unwrap failed").push("parent_capture");
        }),
        true, // use_capture
    );

    // 2. Child Target Phase (Listener 1)
    let order_clone2 = event_order.clone();
    Node::add_event_listener(
        &child,
        "click",
        Arc::new(move |event: &mut Event| {
            assert_eq!(event.phase, EventPhase::AtTarget);
            order_clone2.lock().expect("Unwrap failed").push("child_target_1");
        }),
        false,
    );

    // 3. Child Target Phase (Listener 2 - captures, but at target it runs in registration order)
    let order_clone3 = event_order.clone();
    Node::add_event_listener(
        &child,
        "click",
        Arc::new(move |event: &mut Event| {
            assert_eq!(event.phase, EventPhase::AtTarget);
            order_clone3.lock().expect("Unwrap failed").push("child_target_2");
        }),
        true,
    );

    // 4. Parent Bubble Phase
    let order_clone4 = event_order.clone();
    Node::add_event_listener(
        &parent,
        "click",
        Arc::new(move |event: &mut Event| {
            assert_eq!(event.phase, EventPhase::BubblingPhase);
            order_clone4.lock().expect("Unwrap failed").push("parent_bubble");
        }),
        false,
    );

    // Dispatch
    let mut event = Event::new("click", true, true);
    Node::dispatch_event(&child, &mut event);

    let order = event_order.lock().expect("Unwrap failed");
    assert_eq!(
        *order,
        vec!["parent_capture", "child_target_1", "child_target_2", "parent_bubble"]
    );
}

#[test]
fn test_event_stop_propagation() {
    let parent = Node::new(NodeData::Element(ElementData::new("div".into())));
    let child = Node::new(NodeData::Element(ElementData::new("button".into())));
    Node::append_child(&parent, &child);

    let event_order = Arc::new(Mutex::new(Vec::new()));

    let order_clone = event_order.clone();
    Node::add_event_listener(
        &parent,
        "click",
        Arc::new(move |_: &mut Event| {
            order_clone.lock().expect("Unwrap failed").push("parent_capture");
        }),
        true,
    );

    let order_clone2 = event_order.clone();
    Node::add_event_listener(
        &child,
        "click",
        Arc::new(move |event: &mut Event| {
            order_clone2.lock().expect("Unwrap failed").push("child_target");
            event.stop_propagation();
        }),
        false,
    );

    let order_clone3 = event_order.clone();
    Node::add_event_listener(
        &parent,
        "click",
        Arc::new(move |_: &mut Event| {
            order_clone3.lock().expect("Unwrap failed").push("parent_bubble");
        }),
        false,
    );

    let mut event = Event::new("click", true, true);
    Node::dispatch_event(&child, &mut event);

    let order = event_order.lock().expect("Unwrap failed");
    // Parent bubble should NOT fire because propagation was stopped at child.
    assert_eq!(
        *order,
        vec!["parent_capture", "child_target"]
    );
}

#[test]
fn test_event_prevent_default() {
    let node = Node::new(NodeData::Element(ElementData::new("a".into())));
    
    Node::add_event_listener(
        &node,
        "click",
        Arc::new(|event: &mut Event| {
            event.prevent_default();
        }),
        false,
    );

    let mut event = Event::new("click", true, true);
    let allowed = Node::dispatch_event(&node, &mut event);
    
    assert_eq!(allowed, false);
    assert_eq!(event.default_prevented, true);
}
