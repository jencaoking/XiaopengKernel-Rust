use xiaopeng_parser::html::tokenizer::{HtmlTokenizer, HtmlToken};

fn create_tokenizer(input: &str) -> HtmlTokenizer {
    let mut t = HtmlTokenizer::new();
    t.push_chunk(input);
    t.end_of_file();
    t
}

#[test]
fn test_tokenizer_empty_input() {
    let mut tokenizer = create_tokenizer("");
    let token = tokenizer.next_token().expect("Unwrap failed");
    assert_eq!(token, Some(HtmlToken::Eof));
}

#[test]
fn test_tokenizer_simple_text() {
    let mut tokenizer = create_tokenizer("Hello World");
    let mut tokens = Vec::new();
    while let Ok(Some(token)) = tokenizer.next_token() {
        tokens.push(token);
        if tokens.last() == Some(&HtmlToken::Eof) {
            break;
        }
    }
    
    // Contiguous characters of the same type (whitespace/non-whitespace) are merged.
    // We expect 3 Character tokens ("Hello", " ", "World") + 1 Eof token.
    assert_eq!(tokens.len(), 4);
    assert_eq!(tokens[0], HtmlToken::Character("Hello".to_string()));
    assert_eq!(tokens[1], HtmlToken::Character(" ".to_string()));
    assert_eq!(tokens[2], HtmlToken::Character("World".to_string()));
    assert_eq!(tokens[3], HtmlToken::Eof);
}

#[test]
fn test_tokenizer_simple_start_tag() {
    let mut tokenizer = create_tokenizer("<div>");
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
    let mut tokenizer = create_tokenizer(html);

    let token = tokenizer.next_token().expect("Unwrap failed").expect("Unwrap failed");
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
    let mut tokenizer = create_tokenizer(input);

    let token1 = tokenizer.next_token().expect("Unwrap failed").expect("Unwrap failed");
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
        let t = tokenizer.next_token().expect("Unwrap failed").expect("Unwrap failed");
        match t {
            HtmlToken::Character(c) => script_content.push_str(&c),
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
