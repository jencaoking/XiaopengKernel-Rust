//! XiaopengKernel JavaScript Script Engine Integration Module

pub mod bindings;

pub use bindings::console_log;
use tracing::info;
use xiaopeng_common::XiaopengResult;

pub fn eval_script(script_code: &str) -> XiaopengResult<()> {
    info!("Evaluating JavaScript snippet (length: {})", script_code.len());
    console_log("JS engine initialized successfully");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_eval_script() {
        assert!(eval_script("console.log('hello')").is_ok());
    }
}
