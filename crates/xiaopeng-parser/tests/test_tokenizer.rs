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

#[test]
fn test_tokenizer_attributes() {
    let html = "<div id=\"test\" class='foo' disabled>";
    let mut tokenizer = HtmlTokenizer::new(html);

    let token = tokenizer.next_token().unwrap().unwrap();
    match token {
        HtmlToken::StartTag { name, self_closing, attributes } => {
            assert_eq!(name, "div");
            assert_eq!(self_closing, false);
            assert_eq!(attributes.len(), 3);
            
            assert_eq!(attributes[0].name, "id");
            assert_eq!(attributes[0].value, "test");
            
            assert_eq!(attributes[1].name, "class");
            assert_eq!(attributes[1].value, "foo");
            
            assert_eq!(attributes[2].name, "disabled");
            assert_eq!(attributes[2].value, "");
        }
        _ => panic!("Expected start tag div"),
    }
}

#[test]
fn test_tokenizer_rawtext_script() {
    let input = "<script>var a = b < c;</script>";
    let mut tokenizer = HtmlTokenizer::new(input);

    let token1 = tokenizer.next_token().unwrap().unwrap();
    if let HtmlToken::StartTag { name, self_closing, .. } = token1 {
        assert_eq!(name, "script");
        assert!(!self_closing);
    } else {
        panic!("Expected start tag");
    }

    // Now, the '< c;' shouldn't trigger a tag!
    // It should be emitted as characters
    let mut script_content = String::new();
    loop {
        let t = tokenizer.next_token().unwrap().unwrap();
        match t {
            HtmlToken::Character(c) => script_content.push(c),
            HtmlToken::EndTag { name } => {
                assert_eq!(name, "script");
                break;
            }
            HtmlToken::Eof => panic!("Unexpected EOF"),
            _ => panic!("Unexpected token inside rawtext: {:?}", t),
        }
    }
    
    assert_eq!(script_content, "var a = b < c;");
}
