//! Boa JavaScript Runtime — persistent context with DOM API bindings

use boa_engine::{Context, JsResult, JsString, JsValue, NativeFunction, Source};
use tracing::{info, warn};
use xiaopeng_common::{XiaopengError, XiaopengResult};

/// A persistent JavaScript runtime built on top of the Boa engine.
/// Holds a `Context` that maintains the global object, heap, and all JS state.
pub struct JsRuntime {
    pub context: Context,
}

impl JsRuntime {
    /// Create a new runtime and register all built-in Web API stubs.
    pub fn new() -> XiaopengResult<Self> {
        let mut context = Context::default();

        register_console(&mut context)?;
        register_timers(&mut context)?;
        register_location(&mut context)?;
        register_promise_integration(&mut context)?;
        crate::bindings::dom::register_dom_api(&mut context)?;

        info!("JsRuntime initialized with Boa engine");
        Ok(Self { context })
    }

    /// Evaluate a JavaScript source string and return the stringified result.
    pub fn eval(&mut self, code: &str) -> XiaopengResult<String> {
        info!("JsRuntime::eval — {} bytes", code.len());
        let result = self
            .context
            .eval(Source::from_bytes(code))
            .map_err(|e| XiaopengError::ScriptError {
                message: format!("{e}"),
            })?;

        let display = result
            .to_string(&mut self.context)
            .map(|s| s.to_std_string_escaped())
            .unwrap_or_else(|_| "undefined".into());

        Ok(display)
    }

    /// Evaluate code and return the raw `JsValue` for further Rust manipulation.
    pub fn eval_value(&mut self, code: &str) -> JsResult<JsValue> {
        self.context.eval(Source::from_bytes(code))
    }

    /// Expose a Rust callable to the JS global scope.
    /// `length` is the formal parameter count shown by `fn.length` in JS.
    pub fn register_global_fn(
        &mut self,
        name: &str,
        length: usize,
        f: NativeFunction,
    ) -> XiaopengResult<()> {
        let js_name = JsString::from(name);
        self.context
            .register_global_callable(js_name, length, f)
            .map_err(|e| XiaopengError::ScriptError {
                message: format!("register_global_fn({name}): {e}"),
            })?;
        Ok(())
    }

    /// Fire any timers whose deadline has passed, then drain microtasks.
    /// Call this once per event-loop tick.
    /// Returns (timers_fired, microtasks_drained).
    pub fn tick(&mut self) -> (usize, usize) {
        let timers = crate::bindings::timers::tick_timers(&mut self.context);
        // Always drain microtasks after a tick (timers may have resolved Promises).
        let micros = crate::bindings::timers::drain_microtasks(&mut self.context);
        (timers, micros)
    }

    /// Drain only the microtask queue (Promise .then callbacks etc.).
    pub fn drain_microtasks(&mut self) -> usize {
        crate::bindings::timers::drain_microtasks(&mut self.context)
    }

    /// Returns true if there are any pending timers or microtasks.
    pub fn has_pending_work(&self) -> bool {
        crate::bindings::timers::has_pending_work()
    }

    /// Run the event loop until all timers and microtasks are exhausted,
    /// or until `max_ticks` ticks have been run (to avoid infinite loops).
    pub fn run_event_loop(&mut self, max_ticks: usize) {
        let mut ticks = 0;
        while self.has_pending_work() && ticks < max_ticks {
            self.tick();
            ticks += 1;
            // Small sleep to avoid a hot loop when all timers are in the future.
            if !crate::bindings::timers::has_pending_work() { break; }
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
    }
}

impl Default for JsRuntime {
    fn default() -> Self {
        Self::new().expect("Failed to create JsRuntime")
    }
}

pub(crate) fn map_boa_err(e: boa_engine::JsError) -> XiaopengError {
    XiaopengError::ScriptError { message: format!("{e}") }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Register a callable (constructable + callable) on the JS global object.
fn reg_callable(ctx: &mut Context, name: &str, length: usize, f: NativeFunction) -> XiaopengResult<()> {
    ctx.register_global_callable(JsString::from(name), length, f)
        .map_err(|e| XiaopengError::ScriptError { message: format!("{e}") })
}

/// Register a builtin callable (callable only, not constructable).
fn reg_builtin(ctx: &mut Context, name: &str, length: usize, f: NativeFunction) -> XiaopengResult<()> {
    ctx.register_global_builtin_callable(JsString::from(name), length, f)
        .map_err(|e| XiaopengError::ScriptError { message: format!("{e}") })
}

// ---------------------------------------------------------------------------
// console.*
// ---------------------------------------------------------------------------

fn register_console(ctx: &mut Context) -> XiaopengResult<()> {
    // Register a private backing function used by the JS shim
    reg_builtin(ctx, "____console_log", 1, NativeFunction::from_fn_ptr(js_console_log))?;

    // Build `console` object with the standard methods
    let init = r#"
        var console = (function() {
            return {
                log:   function() { ____console_log(Array.prototype.join.call(arguments, ' ')); },
                warn:  function() { ____console_log('[WARN] '  + Array.prototype.join.call(arguments, ' ')); },
                error: function() { ____console_log('[ERROR] ' + Array.prototype.join.call(arguments, ' ')); },
                info:  function() { ____console_log('[INFO] '  + Array.prototype.join.call(arguments, ' ')); },
                debug: function() { ____console_log('[DEBUG] ' + Array.prototype.join.call(arguments, ' ')); },
            };
        })();
    "#;

    ctx.eval(Source::from_bytes(init))
        .map_err(|e| XiaopengError::ScriptError { message: format!("{e}") })?;

    info!("JS console API registered");
    Ok(())
}

fn js_console_log(_this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let msg = args
        .first()
        .cloned()
        .unwrap_or(JsValue::undefined())
        .to_string(ctx)?
        .to_std_string_escaped();
    info!("[JS Console] {}", msg);
    Ok(JsValue::undefined())
}

// ---------------------------------------------------------------------------
// setTimeout / clearTimeout / setInterval / clearInterval — real implementation
// ---------------------------------------------------------------------------

fn register_timers(ctx: &mut Context) -> XiaopengResult<()> {
    // Real native hooks
    reg_callable(ctx, "____setTimeout_native",     2, NativeFunction::from_fn_ptr(js_set_timeout))?;
    reg_callable(ctx, "____setInterval_native",    2, NativeFunction::from_fn_ptr(js_set_interval))?;
    reg_callable(ctx, "clearTimeout",              1, NativeFunction::from_fn_ptr(js_clear_timer))?;
    reg_callable(ctx, "clearInterval",             1, NativeFunction::from_fn_ptr(js_clear_timer))?;
    reg_builtin(ctx,  "____enqueue_microtask",     1, NativeFunction::from_fn_ptr(js_enqueue_microtask))?;

    // JS wrappers: coerce delay, default to 0 ms; forward extra arguments.
    let js_wrap = r#"
        function setTimeout(fn, delay) {
            var extra = Array.prototype.slice.call(arguments, 2);
            return ____setTimeout_native.apply(null, [fn, (delay >>> 0) || 0].concat(extra));
        }
        function setInterval(fn, delay) {
            var extra = Array.prototype.slice.call(arguments, 2);
            return ____setInterval_native.apply(null, [fn, (delay >>> 0) || 0].concat(extra));
        }
    "#;
    ctx.eval(Source::from_bytes(js_wrap))
        .map_err(|e| XiaopengError::ScriptError { message: format!("{e}") })?;

    info!("JS timer API registered");
    Ok(())
}

fn js_set_timeout(_this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    use crate::bindings::timers;
    use boa_engine::object::builtins::JsFunction;

    let func = args.get(0).and_then(|v| {
        if v.is_callable() { v.as_object().and_then(|o| JsFunction::from_object(o.clone())) }
        else { None }
    });
    let delay_ms = args.get(1).and_then(|v| v.as_number()).unwrap_or(0.0) as u64;

    if let Some(f) = func {
        // Collect any extra arguments to forward to the callback
        let extra: Vec<JsValue> = args.get(2..).unwrap_or(&[]).to_vec();
        let id = timers::set_timeout(f, extra, delay_ms);
        return Ok(JsValue::from(id));
    }
    Ok(JsValue::from(0_u32))
}

fn js_set_interval(_this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    use crate::bindings::timers;
    use boa_engine::object::builtins::JsFunction;

    let func = args.get(0).and_then(|v| {
        if v.is_callable() { v.as_object().and_then(|o| JsFunction::from_object(o.clone())) }
        else { None }
    });
    let interval_ms = args.get(1).and_then(|v| v.as_number()).unwrap_or(0.0) as u64;

    if let Some(f) = func {
        let extra: Vec<JsValue> = args.get(2..).unwrap_or(&[]).to_vec();
        let id = timers::set_interval(f, extra, interval_ms);
        return Ok(JsValue::from(id));
    }
    Ok(JsValue::from(0_u32))
}

fn js_clear_timer(_this: &JsValue, args: &[JsValue], _ctx: &mut Context) -> JsResult<JsValue> {
    use crate::bindings::timers;
    if let Some(id) = args.first().and_then(|v| v.as_number()) {
        timers::clear_timer(id as u32);
    }
    Ok(JsValue::undefined())
}

fn js_enqueue_microtask(_this: &JsValue, args: &[JsValue], _ctx: &mut Context) -> JsResult<JsValue> {
    use crate::bindings::timers;
    use boa_engine::object::builtins::JsFunction;
    if let Some(func) = args.first().and_then(|v| {
        if v.is_callable() { v.as_object().and_then(|o| JsFunction::from_object(o.clone())) }
        else { None }
    }) {
        timers::enqueue_microtask(func);
    }
    Ok(JsValue::undefined())
}

// ---------------------------------------------------------------------------
// window / location stubs
// ---------------------------------------------------------------------------

fn register_location(ctx: &mut Context) -> XiaopengResult<()> {
    let init = r#"
        var location = {
            href:     'about:blank',
            protocol: 'about:',
            host:     '',
            hostname: '',
            port:     '',
            pathname: 'blank',
            search:   '',
            hash:     '',
            reload:   function() {},
            assign:   function(url) { this.href = String(url); },
            replace:  function(url) { this.href = String(url); },
            toString: function() { return this.href; },
        };
        var window = {};
        window.location = location;
    "#;

    ctx.eval(Source::from_bytes(init))
        .map_err(|e| XiaopengError::ScriptError { message: format!("{e}") })?;

    info!("JS location/window stubs registered");
    Ok(())
}

// ---------------------------------------------------------------------------
// Promise / microtask integration
// ---------------------------------------------------------------------------

fn register_promise_integration(ctx: &mut Context) -> XiaopengResult<()> {
    // Boa 0.20 has built-in Promise support. We hook into queueMicrotask so any
    // Promise .then()/.catch()/.finally() callback flows through our microtask queue.
    // We also expose `queueMicrotask` as a Web API.
    //
    // The trick: override the internal microtask scheduler by providing a
    // `queueMicrotask` global and patching Promise.resolve().then() chains to use it.
    // Boa 0.20 schedules its own microtasks; we expose our own queue for userland code.

    let init = r#"
        // queueMicrotask Web API
        function queueMicrotask(fn) {
            if (typeof fn !== 'function') return;
            ____enqueue_microtask(fn);
        }

        // Minimal Promise polyfill integration:
        // If native Promise is available (Boa 0.20 provides it), wrap it so
        // .then() callbacks go through queueMicrotask. Otherwise, provide a
        // lightweight polyfill.
        (function() {
            if (typeof Promise !== 'undefined') {
                // Native Promise exists in Boa 0.20. Patch .then to forward
                // callbacks through our queue so they're visible to tick().
                var NativePromise = Promise;
                var origThen = NativePromise.prototype.then;
                NativePromise.prototype.then = function(onFulfilled, onRejected) {
                    var wrapped = onFulfilled;
                    if (typeof onFulfilled === 'function') {
                        wrapped = function(v) { return onFulfilled(v); };
                    }
                    return origThen.call(this, wrapped, onRejected);
                };
                return;
            }

            // Lightweight Promise polyfill for environments without native Promise.
            function XPromise(executor) {
                this._state = 'pending';
                this._value = undefined;
                this._handlers = [];
                var self = this;

                function resolve(value) {
                    if (self._state !== 'pending') return;
                    self._state = 'fulfilled';
                    self._value = value;
                    self._handlers.forEach(function(h) { self._invokeHandler(h); });
                }
                function reject(reason) {
                    if (self._state !== 'pending') return;
                    self._state = 'rejected';
                    self._value = reason;
                    self._handlers.forEach(function(h) { self._invokeHandler(h); });
                }
                try { executor(resolve, reject); }
                catch(e) { reject(e); }
            }

            XPromise.prototype._invokeHandler = function(handler) {
                var self = this;
                queueMicrotask(function() {
                    var fn = self._state === 'fulfilled' ? handler.onFulfilled : handler.onRejected;
                    if (typeof fn !== 'function') {
                        var next = self._state === 'fulfilled' ? handler.resolve : handler.reject;
                        next(self._value);
                        return;
                    }
                    try { handler.resolve(fn(self._value)); }
                    catch(e) { handler.reject(e); }
                });
            };

            XPromise.prototype.then = function(onFulfilled, onRejected) {
                var self = this;
                return new XPromise(function(resolve, reject) {
                    var h = { onFulfilled: onFulfilled, onRejected: onRejected, resolve: resolve, reject: reject };
                    if (self._state === 'pending') {
                        self._handlers.push(h);
                    } else {
                        self._invokeHandler(h);
                    }
                });
            };

            XPromise.prototype.catch = function(onRejected) { return this.then(null, onRejected); };
            XPromise.prototype.finally = function(fn) {
                return this.then(
                    function(v)  { fn(); return v; },
                    function(e)  { fn(); throw e; }
                );
            };

            XPromise.resolve = function(v) {
                return new XPromise(function(res) { res(v); });
            };
            XPromise.reject = function(r) {
                return new XPromise(function(_, rej) { rej(r); });
            };
            XPromise.all = function(promises) {
                return new XPromise(function(resolve, reject) {
                    var results = [], count = promises.length;
                    if (count === 0) { resolve([]); return; }
                    promises.forEach(function(p, i) {
                        XPromise.resolve(p).then(function(v) {
                            results[i] = v;
                            if (--count === 0) resolve(results);
                        }, reject);
                    });
                });
            };

            globalThis.Promise = XPromise;
        })();
    "#;

    ctx.eval(Source::from_bytes(init))
        .map_err(|e| XiaopengError::ScriptError { message: format!("{e}") })?;

    info!("JS Promise / microtask integration registered");
    Ok(())
}
