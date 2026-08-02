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
// setTimeout / clearTimeout / setInterval / clearInterval stubs
// ---------------------------------------------------------------------------

fn register_timers(ctx: &mut Context) -> XiaopengResult<()> {
    reg_callable(ctx, "setTimeout",    2, NativeFunction::from_fn_ptr(js_set_timeout))?;
    reg_callable(ctx, "clearTimeout",  1, NativeFunction::from_fn_ptr(js_noop))?;
    reg_callable(ctx, "setInterval",   2, NativeFunction::from_fn_ptr(js_set_timeout))?;
    reg_callable(ctx, "clearInterval", 1, NativeFunction::from_fn_ptr(js_noop))?;
    info!("JS timer stubs registered");
    Ok(())
}

fn js_set_timeout(_this: &JsValue, _args: &[JsValue], _ctx: &mut Context) -> JsResult<JsValue> {
    // Real implementation: push callback onto EventLoop macrotask queue.
    // Stub: return timer ID 0.
    warn!("setTimeout/setInterval called — callbacks not yet driven by EventLoop");
    Ok(JsValue::from(0_i32))
}

fn js_noop(_this: &JsValue, _args: &[JsValue], _ctx: &mut Context) -> JsResult<JsValue> {
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
