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

pub struct CssTokenizer<'a> {
    input: &'a str,
    pos: usize,
}

impl<'a> CssTokenizer<'a> {
    pub fn new(input: &'a str) -> Self {
        Self { input, pos: 0 }
    }

    pub fn next_token(&mut self) -> CssToken {
        self.skip_comments();
        
        if self.pos >= self.input.len() {
            return CssToken::Eof;
        }

        let chars: Vec<char> = self.input[self.pos..].chars().collect();
        let c = chars[0];

        if c.is_whitespace() {
            self.pos += c.len_utf8();
            while self.pos < self.input.len() && self.input[self.pos..].chars().next().unwrap().is_whitespace() {
                self.pos += self.input[self.pos..].chars().next().unwrap().len_utf8();
            }
            return CssToken::Whitespace;
        }

        match c {
            ':' => { self.pos += 1; CssToken::Colon }
            ';' => { self.pos += 1; CssToken::Semicolon }
            '{' => { self.pos += 1; CssToken::LeftBrace }
            '}' => { self.pos += 1; CssToken::RightBrace }
            '#' => {
                self.pos += 1;
                let hash = self.consume_ident();
                CssToken::Hash(hash)
            }
            '"' | '\'' => {
                self.pos += 1;
                let mut string = String::new();
                let mut iter = self.input[self.pos..].chars();
                while let Some(ch) = iter.next() {
                    self.pos += ch.len_utf8();
                    if ch == c {
                        break;
                    }
                    string.push(ch);
                }
                CssToken::String(string)
            }
            _ if c.is_ascii_digit() || (c == '.' && chars.get(1).map_or(false, |n| n.is_ascii_digit())) => {
                self.consume_numeric()
            }
            _ if is_ident_start(c) => {
                CssToken::Ident(self.consume_ident())
            }
            _ => {
                self.pos += c.len_utf8();
                CssToken::Delim(c)
            }
        }
    }

    fn skip_comments(&mut self) {
        while self.input[self.pos..].starts_with("/*") {
            if let Some(end) = self.input[self.pos..].find("*/") {
                self.pos += end + 2;
            } else {
                self.pos = self.input.len();
            }
        }
    }

    fn consume_ident(&mut self) -> String {
        let mut ident = String::new();
        let mut iter = self.input[self.pos..].chars();
        while let Some(c) = iter.next() {
            if is_ident_char(c) {
                ident.push(c);
                self.pos += c.len_utf8();
            } else {
                break;
            }
        }
        ident
    }

    fn consume_numeric(&mut self) -> CssToken {
        let mut num_str = String::new();
        let mut iter = self.input[self.pos..].chars();
        while let Some(c) = iter.next() {
            if c.is_ascii_digit() || c == '.' || c == '+' || c == '-' || c == 'e' || c == 'E' {
                num_str.push(c);
                self.pos += c.len_utf8();
            } else {
                break;
            }
        }
        let value = num_str.parse().unwrap_or(0.0);

        if self.pos < self.input.len() && self.input[self.pos..].starts_with('%') {
            self.pos += 1;
            CssToken::Percentage(value)
        } else if self.pos < self.input.len() && is_ident_start(self.input[self.pos..].chars().next().unwrap()) {
            let unit = self.consume_ident();
            CssToken::Dimension { value, unit }
        } else {
            CssToken::Number(value)
        }
    }
}

fn is_ident_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_' || c == '-'
}

fn is_ident_char(c: char) -> bool {
    is_ident_start(c) || c.is_ascii_digit()
}
