use xiaopeng_script::JsRuntime;
use xiaopeng_script::bindings::dom::expose_node;
use xiaopeng_dom::{Node, NodeData};

#[test]
fn test_dom_node_creation() {
    let mut runtime = JsRuntime::new().unwrap();
    let root = Node::new(NodeData::Document);
    let root_id = expose_node(root);
    runtime.eval(&format!("____init_document({});", root_id)).unwrap();

    let script = r#"
        var div = document.createElement("div");
        div.id = "my-div";
        div.className = "container";
        document.appendChild(div);
        
        var span = document.createElement("span");
        span.textContent = "Hello JS DOM";
        div.appendChild(span);
        
        var found = document.getElementById("my-div");
        found !== null && found.className === "container" && found.childNodes.length === 1
    "#;

    let res = runtime.eval(script).unwrap();
    assert_eq!(res, "true");
}

#[test]
fn test_dom_class_list() {
    let mut runtime = JsRuntime::new().unwrap();
    let root = Node::new(NodeData::Document);
    let root_id = expose_node(root);
    runtime.eval(&format!("____init_document({});", root_id)).unwrap();

    let script = r#"
        var div = document.createElement("div");
        div.classList.add("btn", "btn-primary");
        var hasBtn = div.classList.contains("btn");
        div.classList.remove("btn");
        var hasBtnAfter = div.classList.contains("btn");
        div.classList.toggle("active");
        
        hasBtn === true && hasBtnAfter === false && div.classList.contains("active") === true
    "#;

    let res = runtime.eval(script).unwrap();
    assert_eq!(res, "true");
}

#[test]
fn test_dom_events() {
    let mut runtime = JsRuntime::new().unwrap();
    let root = Node::new(NodeData::Document);
    let root_id = expose_node(root);
    runtime.eval(&format!("____init_document({});", root_id)).unwrap();

    let script = r#"
        var div = document.createElement("div");
        var clicked = 0;
        var lastTarget = null;
        
        div.addEventListener("click", function(e) {
            clicked++;
            lastTarget = e.target;
        });
        
        var evt = new Event("click");
        div.dispatchEvent(evt);
        
        clicked === 1 && lastTarget === div
    "#;

    let res = runtime.eval(script).unwrap();
    assert_eq!(res, "true");
}

#[test]
fn test_event_bubbling() {
    let mut runtime = JsRuntime::new().unwrap();
    let root = Node::new(NodeData::Document);
    let root_id = expose_node(root);
    runtime.eval(&format!("____init_document({});", root_id)).unwrap();

    let script = r#"
        var parent = document.createElement("div");
        var child = document.createElement("button");
        parent.appendChild(child);
        document.appendChild(parent);
        
        var parentClicked = 0;
        var childClicked = 0;
        
        parent.addEventListener("click", function(e) {
            parentClicked++;
        });
        
        child.addEventListener("click", function(e) {
            childClicked++;
        });
        
        var evt = new Event("click", { bubbles: true });
        child.dispatchEvent(evt);
        
        parentClicked === 1 && childClicked === 1
    "#;

    let res = runtime.eval(script).unwrap();
    assert_eq!(res, "true");
}
