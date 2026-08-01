//! CSS Tokenizer

#[derive(Debug, Clone, PartialEq)]
pub enum CssToken {
    Ident(String),
    Hash(String),
    Delim(char),
    Number(f32),
    Percentage(f32),
    Dimension { value: f32, unit: String },
    String(String),
    Whitespace,
    Colon,
    Semicolon,
    LeftBrace,
    RightBrace,
    Eof,
}
