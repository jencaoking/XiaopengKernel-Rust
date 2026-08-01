//! XiaopengKernel CSS Style & Cascade Resolver Module

pub mod computed_style;
pub mod parser;
pub mod resolver;
pub mod selector;
pub mod styled_node;

pub use computed_style::{ComputedStyle, Display};
pub use styled_node::StyledNode;
pub use resolver::StyleResolver;
pub use selector::{Selector, SimpleSelector, Specificity};
use tracing::info;
use xiaopeng_common::XiaopengResult;

use xiaopeng_dom::NodePtr;

pub fn init_style() -> XiaopengResult<()> {
    info!("Style module initialized");
    Ok(())
}

pub fn resolve_style(node: &NodePtr) -> ComputedStyle {
    // Basic stub: in a real engine, this requires access to the parsed stylesheets.
    // For now, we return default style, but we might set display to None for elements like <head>.
    let mut style = ComputedStyle::default();
    
    let n = node.read().unwrap();
    if let xiaopeng_dom::NodeData::Element(ref el) = n.data {
        if el.tag_name == "head" || el.tag_name == "style" || el.tag_name == "script" || el.tag_name == "title" {
            style.display = Display::None;
        }
    }
    
    style
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_specificity() {
        let sel = Selector {
            parts: vec![
                selector::SimpleSelector::new_basic(selector::SelectorType::Id, "header".into()),
                selector::SimpleSelector::new_basic(selector::SelectorType::Class, "btn".into()),
                selector::SimpleSelector::new_basic(selector::SelectorType::Tag, "button".into()),
            ],
            combinators: vec![selector::Combinator::None, selector::Combinator::None],
        };
        let spec = sel.specificity();
        assert_eq!(spec, Specificity { a: 1, b: 1, c: 1 });
    }
}
