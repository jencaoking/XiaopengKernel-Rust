//! Integration tests: setTimeout / setInterval / clearTimeout / Promise / queueMicrotask

use xiaopeng_script::JsRuntime;


fn rt() -> JsRuntime {
    JsRuntime::new().expect("JsRuntime::new")
}

// ─── setTimeout ────────────────────────────────────────────────────────────────

#[test]
fn test_set_timeout_fires_after_delay() {
    let mut rt = rt();
    // Schedule a 0 ms callback; it should fire on the first tick() call.
    rt.eval("var fired = false; setTimeout(function(){ fired = true; }, 0);").unwrap();
    assert_eq!(rt.eval("fired").unwrap(), "false", "not yet fired before tick");

    // tick() drives the timers.
    rt.tick();

    let result = rt.eval("fired").unwrap();
    assert_eq!(result, "true", "should have fired after tick()");
}

#[test]
fn test_set_timeout_receives_extra_args() {
    let mut rt = rt();
    rt.eval("var got = 0; setTimeout(function(a, b){ got = a + b; }, 0, 3, 7);").unwrap();
    rt.tick();
    let result = rt.eval("got").unwrap();
    assert_eq!(result, "10");
}

#[test]
fn test_clear_timeout_prevents_firing() {
    let mut rt = rt();
    rt.eval("var fired = false; var id = setTimeout(function(){ fired = true; }, 0);").unwrap();
    rt.eval("clearTimeout(id);").unwrap();
    rt.tick();
    let result = rt.eval("fired").unwrap();
    assert_eq!(result, "false", "cleared timer must not fire");
}

#[test]
fn test_set_timeout_ordering() {
    let mut rt = rt();
    rt.eval(r#"
        var log = [];
        setTimeout(function(){ log.push('a'); }, 0);
        setTimeout(function(){ log.push('b'); }, 0);
        setTimeout(function(){ log.push('c'); }, 0);
    "#).unwrap();
    // Run enough ticks to drain all 0ms timers.
    for _ in 0..5 { rt.tick(); }
    let result = rt.eval("log.join(',')").unwrap();
    assert_eq!(result, "a,b,c");
}

// ─── setInterval ───────────────────────────────────────────────────────────────

#[test]
fn test_set_interval_fires_multiple_times() {
    let mut rt = rt();
    rt.eval("var count = 0; setInterval(function(){ count++; }, 0);").unwrap();
    // Each tick should fire the interval once (since delay = 0 ms).
    for _ in 0..3 { rt.tick(); }
    let result = rt.eval("count").unwrap();
    // Should have fired at least 3 times.
    let n: i64 = result.parse().unwrap();
    assert!(n >= 3, "interval should fire each tick, got {n}");
}

#[test]
fn test_clear_interval_stops_repeating() {
    let mut rt = rt();
    rt.eval("var count = 0; var id = setInterval(function(){ count++; }, 0);").unwrap();
    rt.tick(); // fires once → count = 1
    rt.eval("clearInterval(id);").unwrap();
    rt.tick(); // should NOT fire anymore
    rt.tick();
    let result = rt.eval("count").unwrap();
    assert_eq!(result, "1", "interval must stop after clearInterval");
}

// ─── queueMicrotask ────────────────────────────────────────────────────────────

#[test]
fn test_queue_microtask_runs_before_next_macrotask() {
    let mut rt = rt();
    rt.eval(r#"
        var log = [];
        // Schedule a macrotask
        setTimeout(function(){ log.push('macro'); }, 0);
        // Queue a microtask directly — should run before the timer callback
        queueMicrotask(function(){ log.push('micro'); });
    "#).unwrap();

    // Drain microtasks first (simulates end-of-current-script checkpoint)
    rt.drain_microtasks();
    // Now run the timer
    rt.tick();

    let result = rt.eval("log.join(',')").unwrap();
    assert_eq!(result, "micro,macro");
}

// ─── Promise ───────────────────────────────────────────────────────────────────

#[test]
fn test_promise_then_callback_runs() {
    let mut rt = rt();
    rt.eval(r#"
        var result = 'none';
        Promise.resolve(42).then(function(v) { result = v; });
    "#).unwrap();

    // The .then callback is a microtask — drain them.
    rt.drain_microtasks();

    let result = rt.eval("result").unwrap();
    assert_eq!(result, "42");
}

#[test]
fn test_promise_chain() {
    let mut rt = rt();
    rt.eval(r#"
        var output = [];
        Promise.resolve(1)
            .then(function(v) { output.push(v); return v + 1; })
            .then(function(v) { output.push(v); return v + 1; })
            .then(function(v) { output.push(v); });
    "#).unwrap();

    // Three chained microtask ticks.
    for _ in 0..5 { rt.drain_microtasks(); }

    let result = rt.eval("output.join(',')").unwrap();
    assert_eq!(result, "1,2,3");
}

#[test]
fn test_promise_catch() {
    let mut rt = rt();
    rt.eval(r#"
        var caught = 'none';
        Promise.reject('boom').catch(function(e) { caught = e; });
    "#).unwrap();
    for _ in 0..3 { rt.drain_microtasks(); }
    let result = rt.eval("caught").unwrap();
    assert_eq!(result, "boom");
}

#[test]
fn test_promise_after_timeout() {
    let mut rt = rt();
    rt.eval(r#"
        var done = false;
        setTimeout(function() {
            Promise.resolve('async').then(function(v) { done = (v === 'async'); });
        }, 0);
    "#).unwrap();

    rt.tick();              // fires setTimeout → queues Promise microtask
    rt.drain_microtasks();  // runs Promise .then

    let result = rt.eval("done").unwrap();
    assert_eq!(result, "true");
}
