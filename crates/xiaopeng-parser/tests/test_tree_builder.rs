use xiaopeng_parser::html::tree_builder::{HtmlTreeBuilder, InsertionMode};
use xiaopeng_parser::html::tokenizer::HtmlToken;

#[test]
fn test_tree_builder_initial_mode() {
    let mut builder = HtmlTreeBuilder::new();
    assert_eq!(builder.insertion_mode, InsertionMode::Initial);
    
    // Send a mock Doctype token to trigger transition
    let token = HtmlToken::Doctype {
        name: Some("html".into()),
        public_id: None,
        system_id: None,
        force_quirks: false,
    };
    
    builder.process_token(token);
    assert_eq!(builder.insertion_mode, InsertionMode::BeforeHtml);
}

#[test]
fn test_tree_builder_flow() {
    let mut builder = HtmlTreeBuilder::new();
    
    // Simulate flow: Initial -> BeforeHtml -> BeforeHead -> InHead -> AfterHead -> InBody
    let transitions = vec![
        HtmlToken::Doctype { name: Some("html".into()), public_id: None, system_id: None, force_quirks: false },
        HtmlToken::StartTag { name: "html".into(), self_closing: false, attributes: vec![] }, // To BeforeHead
        HtmlToken::StartTag { name: "head".into(), self_closing: false, attributes: vec![] }, // To InHead
        HtmlToken::EndTag { name: "head".into() }, // To AfterHead
        HtmlToken::StartTag { name: "body".into(), self_closing: false, attributes: vec![] }, // To InBody
    ];

    for token in transitions {
        builder.process_token(token);
    }
    
    assert_eq!(builder.insertion_mode, InsertionMode::InBody);
    
    // Verify tree state
    let doc = builder.document;
    let root = doc.root.read().unwrap();
    let html = root.first_element_child().unwrap();
    assert_eq!(html.read().unwrap().child_element_count(), 2);
    
    let head = html.read().unwrap().first_element_child().unwrap();
    let head_tag;
    {
        let guard = head.read().unwrap();
        if let xiaopeng_dom::NodeData::Element(ref el) = guard.data {
            head_tag = el.tag_name.clone();
        } else { panic!("Expected head"); }
    }
    assert_eq!(head_tag, "head");
    
    let body = html.read().unwrap().last_element_child().unwrap();
    let body_tag;
    {
        let guard = body.read().unwrap();
        if let xiaopeng_dom::NodeData::Element(ref el) = guard.data {
            body_tag = el.tag_name.clone();
        } else { panic!("Expected body"); }
    }
    assert_eq!(body_tag, "body");
}
