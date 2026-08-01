//! WHATWG HTML Tokenizer State Machine Stubs

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HtmlToken {
    Doctype { name: Option<String> },
    StartTag { name: String, self_closing: bool },
    EndTag { name: String },
    Character(char),
    Comment(String),
    Eof,
}

pub struct HtmlTokenizer<'a> {
    _input: &'a str,
}

impl<'a> HtmlTokenizer<'a> {
    pub fn new(input: &'a str) -> Self {
        Self { _input: input }
    }
}
