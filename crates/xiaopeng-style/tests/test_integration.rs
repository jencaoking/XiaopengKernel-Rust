use xiaopeng_common::Color;
use xiaopeng_dom::Node;
use xiaopeng_parser::parse_html;
use xiaopeng_style::parser::CssParser;
use xiaopeng_style::resolver::StyleResolver;

#[test]
fn test_end_to_end_pipeline() {
    // 1. Parse HTML into DOM
    let html = r#"
        <html>
            <head><title>Test Engine</title></head>
            <body>
                <div id="container" class="layout-box">
                    <p class="text-bold">Hello World</p>
                    <span id="special">Special Text</span>
                    <div id="hidden-box">Invisible</div>
                </div>
            </body>
        </html>
    "#;
    let document = parse_html(html).expect("HTML parsing failed");

    // 2. Parse CSS into StyleSheet
    let css = r#"
        body { font-size: 16px; color: black; }
        .layout-box { width: 500px; height: 300px; padding: 10px; }
        div p { color: blue; }
        #special { color: red; font-weight: bold; }
        .text-bold { font-weight: bold; }
        #hidden-box { display: none; }
    "#;
    let mut css_parser = CssParser::new(css);
    let stylesheet = css_parser.parse();

    // 3. Initialize Style Resolver
    let resolver = StyleResolver::new(&stylesheet);

    // 4. Resolve styles for specific nodes
    let root = document.root;
    
    // Find our nodes using DOM APIs
    let container_node = Node::get_element_by_id(&root, "container").expect("Container not found");
    let special_node = Node::get_element_by_id(&root, "special").expect("Special not found");
    let p_nodes = Node::get_elements_by_tag_name(&root, "p");
    assert_eq!(p_nodes.len(), 1);
    let p_node = &p_nodes[0];

    // Check Container Style (Class selector)
    let container_style = resolver.resolve_style(&container_node, None, 16.0, 800.0, 600.0);
    assert_eq!(container_style.width, xiaopeng_style::computed_style::CssLength::Px(500.0));
    assert_eq!(container_style.height, xiaopeng_style::computed_style::CssLength::Px(300.0));

    // Check P Style (Descendant selector & Class selector)
    let p_style = resolver.resolve_style(p_node, None, 16.0, 800.0, 600.0);
    assert_eq!(p_style.color, Color { r: 0, g: 0, b: 255, a: 255 }); // Blue from 'div p'

    // Check Special Style (ID selector trumps all)
    let special_style = resolver.resolve_style(&special_node, None, 16.0, 800.0, 600.0);
    assert_eq!(special_style.color, Color { r: 255, g: 0, b: 0, a: 255 }); // Red from '#special'

    // 5. Build StyledTree and verify display: none is omitted
    let styled_tree = xiaopeng_style::StyledNode::build(&root, &resolver, None, 16.0, 800.0, 600.0).expect("Root should not be display: none");
    // Find container in the styled tree robustly
    fn find_styled_node<'a>(node: &'a xiaopeng_style::StyledNode, id: &str) -> Option<&'a xiaopeng_style::StyledNode> {
        let n = node.node.read().expect("Lock poisoned");
        if let xiaopeng_dom::NodeData::Element(ref el) = n.data {
            if el.id().map(|s| s.as_str()) == Some(id) {
                return Some(node);
            }
        }
        for child in &node.children {
            if let Some(found) = find_styled_node(child, id) {
                return Some(found);
            }
        }
        None
    }

    let container_styled = find_styled_node(&styled_tree, "container").expect("container should be in styled tree");
    
    assert_eq!(
        xiaopeng_dom::Node::to_html(&container_styled.node).contains("layout-box"),
        true
    );
    
    // Container should have 2 element children (p, span), and some text nodes.
    // However, #hidden-box should NOT be in the styled tree!
    let has_hidden = container_styled.children.iter().any(|c| {
        let node_ref = c.node.read().expect("Lock poisoned");
        if let xiaopeng_dom::NodeData::Element(ref el) = node_ref.data {
            el.id() == Some(&"hidden-box".to_string())
        } else {
            false
        }
    });
    
    assert_eq!(has_hidden, false, "Elements with display: none should be excluded from StyledNode tree");
}
