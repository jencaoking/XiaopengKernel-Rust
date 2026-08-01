use xiaopeng_style::parser::CssParser;
use xiaopeng_style::selector::{Combinator, SelectorType};

#[test]
fn test_css_parser_simple_rule() {
    let css = "div { width: 100px; height: 50%; }";
    let mut parser = CssParser::new(css);
    let sheet = parser.parse();

    assert_eq!(sheet.rules.len(), 1);
    let rule = &sheet.rules[0];
    assert_eq!(rule.selectors.len(), 1);
    assert_eq!(rule.declarations.len(), 2);

    assert_eq!(rule.declarations[0].property, "width");
    assert!(rule.declarations[0].value.contains("100"));
    assert!(rule.declarations[0].value.contains("px"));

    assert_eq!(rule.declarations[1].property, "height");
    assert!(rule.declarations[1].value.contains("50"));
    assert!(rule.declarations[1].value.contains("%"));
}

#[test]
fn test_css_parser_multiple_selectors() {
    let css = "h1, h2, h3 { font-weight: bold; }";
    let mut parser = CssParser::new(css);
    let sheet = parser.parse();

    assert_eq!(sheet.rules.len(), 1);
    assert_eq!(sheet.rules[0].selectors.len(), 3);
}

#[test]
fn test_css_parser_class_selector() {
    let css = ".container { margin: 0; }";
    let mut parser = CssParser::new(css);
    let sheet = parser.parse();

    assert_eq!(sheet.rules.len(), 1);
    let rule = &sheet.rules[0];
    assert_eq!(rule.selectors.len(), 1);
    
    let sel = &rule.selectors[0];
    assert_eq!(sel.parts.len(), 1);
    assert_eq!(sel.parts[0].selector_type, SelectorType::Class);
    assert_eq!(sel.parts[0].value, "container");
}

#[test]
fn test_css_parser_id_selector() {
    let css = "#main { padding: 10px; }";
    let mut parser = CssParser::new(css);
    let sheet = parser.parse();

    assert_eq!(sheet.rules.len(), 1);
    let rule = &sheet.rules[0];
    assert_eq!(rule.selectors.len(), 1);
    
    let sel = &rule.selectors[0];
    assert_eq!(sel.parts.len(), 1);
    assert_eq!(sel.parts[0].selector_type, SelectorType::Id);
    assert_eq!(sel.parts[0].value, "main");
}

#[test]
fn test_css_parser_complex_selector() {
    let css = "div.content > p { color: blue; }";
    let mut parser = CssParser::new(css);
    let sheet = parser.parse();

    assert_eq!(sheet.rules.len(), 1);
    let rule = &sheet.rules[0];
    assert_eq!(rule.selectors.len(), 1);
    
    let sel = &rule.selectors[0];
    // Expected: Tag(div) -> None -> Class(content) -> Child -> Tag(p)
    assert_eq!(sel.parts.len(), 3);
    assert_eq!(sel.combinators.len(), 2);

    assert_eq!(sel.parts[0].selector_type, SelectorType::Tag);
    assert_eq!(sel.parts[0].value, "div");

    assert_eq!(sel.combinators[0], Combinator::None);

    assert_eq!(sel.parts[1].selector_type, SelectorType::Class);
    assert_eq!(sel.parts[1].value, "content");

    assert_eq!(sel.combinators[1], Combinator::Child);

    assert_eq!(sel.parts[2].selector_type, SelectorType::Tag);
    assert_eq!(sel.parts[2].value, "p");
}
