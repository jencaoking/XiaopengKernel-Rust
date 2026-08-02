use xiaopeng_common::Color;
use xiaopeng_dom::{ElementData, Node, NodeData};
use xiaopeng_style::computed_style::Display;
use xiaopeng_style::parser::CssParser;
use xiaopeng_style::resolver::StyleResolver;

#[test]
fn test_style_resolver_simple_tag() {
    let div = Node::new(NodeData::Element(ElementData::new("div".into())));

    let css = "div { color: red; }";
    let mut parser = CssParser::new(css);
    let sheet = parser.parse();

    let resolver = StyleResolver::new(&sheet);
    let style = resolver.resolve_style(&div);

    assert_eq!(style.color, Color { r: 255, g: 0, b: 0, a: 255 });
}

#[test]
fn test_style_resolver_class() {
    let mut div_data = ElementData::new("div".into());
    div_data.add_class("test");
    let div = Node::new(NodeData::Element(div_data));

    let css = ".test { width: 100px; }";
    let mut parser = CssParser::new(css);
    let sheet = parser.parse();

    let resolver = StyleResolver::new(&sheet);
    let style = resolver.resolve_style(&div);

    assert_eq!(style.width, xiaopeng_style::computed_style::CssLength::Px(100.0));
}

#[test]
fn test_style_resolver_descendant() {
    let mut p_data = ElementData::new("div".into());
    p_data.set_attribute("id".into(), "p".into());
    let div = Node::new(NodeData::Element(p_data));

    let mut span_data = ElementData::new("span".into());
    span_data.set_attribute("id".into(), "c".into());
    let span = Node::new(NodeData::Element(span_data));

    Node::append_child(&div, &span);

    let css = "div span { display: block; }";
    let mut parser = CssParser::new(css);
    let sheet = parser.parse();

    let resolver = StyleResolver::new(&sheet);
    let style = resolver.resolve_style(&span);

    assert_eq!(style.display, Display::Block);
}

#[test]
fn test_style_resolver_specificity() {
    let mut div_data = ElementData::new("div".into());
    div_data.set_attribute("id".into(), "myid".into());
    div_data.add_class("myclass");
    let div = Node::new(NodeData::Element(div_data));

    let css = ".myclass { color: blue; } #myid { color: red; }";
    let mut parser = CssParser::new(css);
    let sheet = parser.parse();

    let resolver = StyleResolver::new(&sheet);
    let style = resolver.resolve_style(&div);

    // ID should win over Class
    assert_eq!(style.color, Color { r: 255, g: 0, b: 0, a: 255 });
}

#[test]
fn test_style_resolver_order() {
    let div = Node::new(NodeData::Element(ElementData::new("div".into())));

    let css = "div { color: red; } div { color: blue; }";
    let mut parser = CssParser::new(css);
    let sheet = parser.parse();

    let resolver = StyleResolver::new(&sheet);
    let style = resolver.resolve_style(&div);

    // Last rule should win due to identical specificity
    assert_eq!(style.color, Color { r: 0, g: 0, b: 255, a: 255 });
}
