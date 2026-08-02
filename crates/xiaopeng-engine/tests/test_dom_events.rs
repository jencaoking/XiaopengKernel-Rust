//! Integration tests: DOM event binding (addEventListener / dispatchEvent)

use xiaopeng_engine::{BrowserEngine, EngineConfig};

fn make_engine() -> BrowserEngine {
    BrowserEngine::new(EngineConfig {
        title: "Event Test".into(),
        width: 800,
        height: 600,
    })
}

/// JS can register a listener and then manually dispatch an event on the same node.
/// The listener should mutate a global var so we can read it back.
#[test]
fn test_add_event_listener_and_dispatch() {
    let mut engine = make_engine();
    let html = r#"<!DOCTYPE html>
<html><body>
  <button id="btn">Click me</button>
  <script>
    var clicked = false;
    var btn = document.getElementById('btn');
    btn.addEventListener('click', function(e) {
        clicked = true;
    });
    // Dispatch a synthetic click event
    var evt = new Event('click');
    btn.dispatchEvent(evt);
  </script>
</body></html>"#;

    engine.load_html(html).unwrap();

    let result = engine.js_runtime.eval("clicked").unwrap();
    assert_eq!(result, "true", "listener should have fired and set clicked=true");
}

/// Multiple listeners on the same event type should all be called.
#[test]
fn test_multiple_listeners_all_called() {
    let mut engine = make_engine();
    let html = r#"<!DOCTYPE html>
<html><body>
  <div id="target"></div>
  <script>
    var count = 0;
    var el = document.getElementById('target');
    el.addEventListener('custom', function() { count += 1; });
    el.addEventListener('custom', function() { count += 10; });
    el.addEventListener('custom', function() { count += 100; });
    el.dispatchEvent(new Event('custom'));
  </script>
</body></html>"#;

    engine.load_html(html).unwrap();

    let result = engine.js_runtime.eval("count").unwrap();
    assert_eq!(result, "111");
}

/// Event object is passed to the listener and properties are readable.
#[test]
fn test_event_object_passed_to_listener() {
    let mut engine = make_engine();
    let html = r#"<!DOCTYPE html>
<html><body>
  <div id="el"></div>
  <script>
    var capturedType = '';
    var el = document.getElementById('el');
    el.addEventListener('mousedown', function(e) {
        capturedType = e.type;
    });
    el.dispatchEvent(new Event('mousedown'));
  </script>
</body></html>"#;

    engine.load_html(html).unwrap();

    let result = engine.js_runtime.eval("capturedType").unwrap();
    assert_eq!(result, "mousedown");
}

/// removeEventListener should prevent the listener from being called.
#[test]
fn test_remove_event_listener() {
    let mut engine = make_engine();
    let html = r#"<!DOCTYPE html>
<html><body>
  <div id="el"></div>
  <script>
    var count = 0;
    var el = document.getElementById('el');
    var handler = function() { count += 1; };
    el.addEventListener('click', handler);
    el.dispatchEvent(new Event('click'));   // fires once → count = 1
    el.removeEventListener('click', handler);
    el.dispatchEvent(new Event('click'));   // should NOT fire → count stays 1
  </script>
</body></html>"#;

    engine.load_html(html).unwrap();

    let result = engine.js_runtime.eval("count").unwrap();
    assert_eq!(result, "1", "listener should only fire once before being removed");
}

/// Event with bubbles=true should propagate up to parent nodes.
#[test]
fn test_event_bubbling() {
    let mut engine = make_engine();
    let html = r#"<!DOCTYPE html>
<html><body>
  <div id="parent">
    <span id="child"></span>
  </div>
  <script>
    var log = [];
    var parent = document.getElementById('parent');
    var child  = document.getElementById('child');

    parent.addEventListener('custom', function() { log.push('parent'); });
    child.addEventListener('custom',  function() { log.push('child');  });

    // Dispatch bubbling event on child — should reach parent too
    child.dispatchEvent(new Event('custom', { bubbles: true }));
  </script>
</body></html>"#;

    engine.load_html(html).unwrap();

    let result = engine.js_runtime.eval("log.join(',')").unwrap();
    // child fires first (at-target), then bubbles to parent
    assert!(result.contains("child"), "child listener must fire");
    assert!(result.contains("parent"), "parent listener must fire due to bubbling");
}
