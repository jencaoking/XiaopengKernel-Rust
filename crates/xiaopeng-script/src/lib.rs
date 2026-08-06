//! XiaopengKernel JavaScript Engine — powered by Boa

pub mod bindings;
pub mod runtime;

pub use bindings::console_log;
pub use runtime::JsRuntime;

use tracing::info;
use xiaopeng_common::XiaopengResult;

/// Convenience wrapper: create a one-shot runtime and evaluate `script_code`.
/// Returns the stringified JS result value.
pub fn eval_script(script_code: &str) -> XiaopengResult<String> {
    info!("eval_script: {} bytes", script_code.len());
    let mut rt = JsRuntime::new()?;
    rt.eval(script_code)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_eval_script_basic_arithmetic() {
        let result = eval_script("1 + 2").expect("Unwrap failed");
        assert_eq!(result, "3");
    }

    #[test]
    fn test_eval_script_console_log() {
        // console.log is registered; it should not panic
        let result = eval_script("console.log('hello boa'); 'done'").expect("Unwrap failed");
        assert_eq!(result, "done");
    }

    #[test]
    fn test_eval_script_string_ops() {
        let result = eval_script("'hello' + ' ' + 'world'").expect("Unwrap failed");
        assert_eq!(result, "hello world");
    }

    #[test]
    fn test_eval_script_function_def_and_call() {
        let result = eval_script("function add(a,b){ return a+b; } add(3,4)").expect("Unwrap failed");
        assert_eq!(result, "7");
    }

    #[test]
    fn test_eval_script_settimeout_stub() {
        // setTimeout must not panic; returns numeric ID (0)
        let result = eval_script("typeof setTimeout").expect("Unwrap failed");
        assert_eq!(result, "function");
    }

    #[test]
    fn test_eval_script_location_stub() {
        let result = eval_script("location.href").expect("Unwrap failed");
        assert_eq!(result, "about:blank");
    }

    #[test]
    fn test_persistent_runtime_state() {
        let mut rt = JsRuntime::new().expect("Unwrap failed");
        rt.eval("var x = 42;").expect("Unwrap failed");
        let result = rt.eval("x + 1").expect("Unwrap failed");
        assert_eq!(result, "43");
    }
}
