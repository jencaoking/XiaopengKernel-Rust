use xiaopeng_parser::html::tokenizer::{HtmlTokenizer, HtmlToken};

#[test]
fn test_tokenizer_empty_input() {
    let mut tokenizer = HtmlTokenizer::new("");
    let token = tokenizer.next_token().unwrap();
    assert_eq!(token, Some(HtmlToken::Eof));
}

#[test]
fn test_tokenizer_simple_text() {
    let mut tokenizer = HtmlTokenizer::new("Hello World");
    let mut tokens = Vec::new();
    while let Ok(Some(token)) = tokenizer.next_token() {
        tokens.push(token);
        if tokens.last() == Some(&HtmlToken::Eof) {
            break;
        }
    }
    
    // In our simplified Rust tokenizer, we emit char by char for Data state initially.
    // In a real tokenizer, contiguous text might be buffered, but right now we emit characters.
    // We expect 11 Character tokens + 1 Eof token.
    assert_eq!(tokens.len(), 12);
    assert_eq!(tokens[0], HtmlToken::Character('H'));
    assert_eq!(tokens[11], HtmlToken::Eof);
}

#[test]
fn test_tokenizer_simple_start_tag() {
    let mut tokenizer = HtmlTokenizer::new("<div>");
    let mut tokens = Vec::new();
    while let Ok(Some(token)) = tokenizer.next_token() {
        tokens.push(token);
        if tokens.last() == Some(&HtmlToken::Eof) {
            break;
        }
    }
    
    // Should emit: StartTag { name: "div" }, Eof
    assert_eq!(tokens.len(), 2);
    if let HtmlToken::StartTag { name, self_closing, attributes } = &tokens[0] {
        assert_eq!(name, "div");
        assert!(!self_closing);
        assert!(attributes.is_empty());
    } else {
        panic!("Expected StartTag");
    }
}
