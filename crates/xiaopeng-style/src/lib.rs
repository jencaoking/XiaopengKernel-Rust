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
    use computed_style::{CssLength, Display};

    let mut style = ComputedStyle::default();

    let n = node.read().unwrap();
    let el = match &n.data {
        xiaopeng_dom::NodeData::Element(el) => el,
        _ => return style,
    };

    // --- 1. UA Stylesheet defaults ---
    match el.tag_name.as_str() {
        "head" | "style" | "script" | "title" | "meta" | "link" => {
            style.display = Display::None;
            return style;
        }
        "html" | "body" | "div" | "section" | "article" | "main"
        | "header" | "footer" | "nav" | "aside" | "figure"
        | "p" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6"
        | "ul" | "ol" | "li" | "table" | "form" | "fieldset"
        | "blockquote" | "pre" | "details" | "summary" => {
            style.display = Display::Block;
        }
        "span" | "a" | "em" | "strong" | "b" | "i" | "u" | "s"
        | "small" | "code" | "kbd" | "mark" | "abbr" | "cite"
        | "sub" | "sup" | "label" | "button" | "input" | "select"
        | "textarea" | "img" | "br" | "time" | "data" => {
            style.display = Display::Inline;
        }
        _ => {
            style.display = Display::Block;
        }
    }

    // UA font-size for headings
    match el.tag_name.as_str() {
        "h1" => { style.font_size = 32.0; style.margin_top = CssLength::Px(21.44); style.margin_bottom = CssLength::Px(21.44); }
        "h2" => { style.font_size = 24.0; style.margin_top = CssLength::Px(19.92); style.margin_bottom = CssLength::Px(19.92); }
        "h3" => { style.font_size = 18.72; style.margin_top = CssLength::Px(18.72); style.margin_bottom = CssLength::Px(18.72); }
        "h4" => { style.font_size = 16.0; style.margin_top = CssLength::Px(21.28); style.margin_bottom = CssLength::Px(21.28); }
        "h5" => { style.font_size = 13.28; }
        "h6" => { style.font_size = 10.72; }
        "p"  => { style.margin_top = CssLength::Px(16.0); style.margin_bottom = CssLength::Px(16.0); }
        "body" => {
            style.margin_top = CssLength::Px(8.0);
            style.margin_bottom = CssLength::Px(8.0);
            style.margin_left = CssLength::Px(8.0);
            style.margin_right = CssLength::Px(8.0);
        }
        _ => {}
    }

    // --- 2. Parse inline style="" attribute ---
    let inline_css = el.attributes.get_named_item("style").map(|a| a.value.clone());
    drop(n); // release read lock before calling parser

    if let Some(css_text) = inline_css {
        // Wrap in a dummy selector rule to reuse existing declaration parser
        let wrapped = format!("__inline__ {{ {} }}", css_text);
        let sheet = parser::CssParser::new(&wrapped).parse();
        if let Some(rule) = sheet.rules.first() {
            let resolver = StyleResolver::new(&sheet);
            for decl in &rule.declarations {
                resolver.apply_declaration_pub(&mut style, decl);
            }
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
