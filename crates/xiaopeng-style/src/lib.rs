//! XiaopengKernel CSS Style & Cascade Resolver Module

pub mod computed_style;
pub mod parser;
pub mod resolver;
pub mod selector;
pub mod styled_node;
pub mod query;

pub use computed_style::{ComputedStyle, Display};
pub use styled_node::StyledNode;
pub use resolver::StyleResolver;
pub use selector::{Selector, SimpleSelector, Specificity};
pub use query::{query_selector, query_selector_all};
use tracing::info;
use xiaopeng_common::XiaopengResult;

use xiaopeng_dom::NodePtr;

pub fn init_style() -> XiaopengResult<()> {
    info!("Style module initialized");
    Ok(())
}

pub fn resolve_style(
    node: &NodePtr,
    parent_style: Option<&ComputedStyle>,
    root_font_size: f32,
    viewport_width: f32,
    viewport_height: f32,
    stylesheet: &crate::parser::StyleSheet,
) -> ComputedStyle {
    use computed_style::{CssLength, Display};

    let mut style = ComputedStyle::default();
    if let Some(parent) = parent_style {
        style.color = parent.color;
        style.font_size = parent.font_size;
    }

    let n = node.read().expect("Lock poisoned");
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

    // --- 2. Evaluate CSS Cascading Rules using StyleResolver ---
    let resolver = StyleResolver::new(stylesheet);
    let mut resolved_style = resolver.resolve_style(node, parent_style, root_font_size, viewport_width, viewport_height);
    
    // Merge UA default styling with resolved cascading styling.
    // In a real browser, UA stylesheet is just another stylesheet with lower specificity.
    // Here we manually merge them for simplicity: if resolved_style has default values, use UA values.
    // But an easier way is to just apply the resolved_style OVER our UA style base we just created.
    
    // Since `StyleResolver::resolve_style` starts from `ComputedStyle::default()`, 
    // it overwrites things if they exist in the CSS. 
    // Wait, the correct way is to have the StyleResolver mutate our base `style`!
    // But `resolve_style` returns a new ComputedStyle. Let's just do an apply-all for now by using a modified method, 
    // or we can just apply matched rules manually here.
    
    // Let's refactor `resolve_style` to actually use `resolver.apply_matched_rules(&mut style, ...)` 
    // Actually `resolver.resolve_style` is already there. Let's just let it return the base style, and we copy overrides.
    // But wait, the easiest way is to re-implement the cascading loop here or in Resolver.
    // Let's just do:
    let matched_rules = resolver.get_matched_rules(node);
    for (rule, _, _) in &matched_rules {
        for decl in &rule.declarations {
            if !decl.important {
                resolver.apply_declaration_pub(&mut style, decl, parent_style.map_or(16.0, |p| p.font_size), root_font_size, viewport_width, viewport_height);
            }
        }
    }
    for (rule, _, _) in &matched_rules {
        for decl in &rule.declarations {
            if decl.important {
                resolver.apply_declaration_pub(&mut style, decl, parent_style.map_or(16.0, |p| p.font_size), root_font_size, viewport_width, viewport_height);
            }
        }
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
                resolver.apply_declaration_pub(&mut style, decl, parent_style.map_or(16.0, |p| p.font_size), root_font_size, viewport_width, viewport_height);
            }
        }
    }

    style.resolve_relative_units(style.font_size, root_font_size, viewport_width, viewport_height);
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
