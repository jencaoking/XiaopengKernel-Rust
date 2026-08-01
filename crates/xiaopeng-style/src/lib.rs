//! XiaopengKernel CSS Style & Cascade Resolver Module

pub mod computed_style;
pub mod parser;
pub mod resolver;
pub mod selector;

pub use computed_style::{ComputedStyle, Display};
pub use resolver::StyleResolver;
pub use selector::{Selector, SimpleSelector, Specificity};
use tracing::info;
use xiaopeng_common::XiaopengResult;

pub fn init_style() -> XiaopengResult<()> {
    info!("Style module initialized");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_specificity() {
        let sel = Selector {
            parts: vec![
                selector::SimpleSelector { selector_type: selector::SelectorType::Id, value: "header".into() },
                selector::SimpleSelector { selector_type: selector::SelectorType::Class, value: "btn".into() },
                selector::SimpleSelector { selector_type: selector::SelectorType::Tag, value: "button".into() },
            ],
            combinators: vec![selector::Combinator::None, selector::Combinator::None],
        };
        let spec = sel.specificity();
        assert_eq!(spec, Specificity { a: 1, b: 1, c: 1 });
    }
}
