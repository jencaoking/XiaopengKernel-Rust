//! Integration test: BrowserEngine executes inline <script> tags via Boa

use xiaopeng_engine::{BrowserEngine, EngineConfig};

fn make_engine() -> BrowserEngine {
    BrowserEngine::new(EngineConfig {
        title: "Test".into(),
        width: 800,
        height: 600,
    })
}

#[test]
fn test_js_runtime_is_initialized() {
    // Just verify the engine starts without panicking (JsRuntime is created)
    let mut engine = make_engine();
    // Direct eval through js_runtime
    let result = engine.js_runtime.eval("40 + 2").unwrap();
    assert_eq!(result, "42");
}

#[test]
fn test_load_html_with_inline_script() {
    let mut engine = make_engine();
    // The script sets a global variable; we verify it persists in the runtime
    let html = r#"<!DOCTYPE html>
<html>
  <head><script>var pageTitle = 'XiaopengKernel';</script></head>
  <body><p>Hello</p></body>
</html>"#;

    engine.load_html(html).unwrap();

    // The script should have run and set `pageTitle`
    let result = engine.js_runtime.eval("pageTitle").unwrap();
    assert_eq!(result, "XiaopengKernel");
}

#[test]
fn test_load_html_script_console_log() {
    let mut engine = make_engine();
    // console.log should not panic
    let html = r#"<!DOCTYPE html>
<html><body>
  <script>console.log('hello from inline script');</script>
</body></html>"#;
    engine.load_html(html).unwrap();
}

#[test]
fn test_load_html_script_error_is_non_fatal() {
    let mut engine = make_engine();
    // Syntax error in script must not crash the engine
    let html = r#"<!DOCTYPE html>
<html><body>
  <script>var x = !!!</script>
  <p>Still renders</p>
</body></html>"#;
    // Should succeed (error is logged, not propagated)
    engine.load_html(html).unwrap();
}

#[test]
fn test_js_runtime_persistent_state_across_eval() {
    let mut engine = make_engine();
    engine.js_runtime.eval("var counter = 0;").unwrap();
    engine.js_runtime.eval("counter += 1;").unwrap();
    engine.js_runtime.eval("counter += 1;").unwrap();
    let result = engine.js_runtime.eval("counter").unwrap();
    assert_eq!(result, "2");
}
