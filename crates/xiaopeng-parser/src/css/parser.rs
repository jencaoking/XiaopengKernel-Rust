//! CSS Parser

use super::tokenizer::{CssToken, CssTokenizer};

#[derive(Debug, Clone)]
pub struct CssDeclaration {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone)]
pub struct CssRule {
    pub selectors: Vec<String>,
    pub declarations: Vec<CssDeclaration>,
}

#[derive(Debug, Clone, Default)]
pub struct StyleSheet {
    pub rules: Vec<CssRule>,
}

pub struct CssParser<'a> {
    tokenizer: CssTokenizer<'a>,
    current_token: CssToken,
}

impl<'a> CssParser<'a> {
    pub fn new(input: &'a str) -> Self {
        let mut tokenizer = CssTokenizer::new(input);
        let current_token = tokenizer.next_token();
        Self { tokenizer, current_token }
    }

    fn advance(&mut self) {
        self.current_token = self.tokenizer.next_token();
    }

    fn skip_whitespace(&mut self) {
        while self.current_token == CssToken::Whitespace {
            self.advance();
        }
    }

    pub fn parse_stylesheet(&mut self) -> StyleSheet {
        let mut rules = Vec::new();
        loop {
            self.skip_whitespace();
            if self.current_token == CssToken::Eof {
                break;
            }
            if let Some(rule) = self.parse_rule() {
                rules.push(rule);
            } else {
                // simple error recovery: skip until next brace or EOF
                while !matches!(self.current_token, CssToken::RightBrace | CssToken::Eof) {
                    self.advance();
                }
                if self.current_token == CssToken::RightBrace {
                    self.advance();
                }
            }
        }
        StyleSheet { rules }
    }

    fn parse_rule(&mut self) -> Option<CssRule> {
        let mut selectors = Vec::new();
        let mut current_selector = String::new();

        loop {
            match &self.current_token {
                CssToken::Eof => return None,
                CssToken::LeftBrace => {
                    if !current_selector.trim().is_empty() {
                        selectors.push(current_selector.trim().to_string());
                    }
                    self.advance();
                    break;
                }
                CssToken::Delim(',') => {
                    if !current_selector.trim().is_empty() {
                        selectors.push(current_selector.trim().to_string());
                    }
                    current_selector.clear();
                    self.advance();
                }
                CssToken::Whitespace => {
                    current_selector.push(' ');
                    self.advance();
                }
                CssToken::Ident(id) => {
                    current_selector.push_str(id);
                    self.advance();
                }
                CssToken::Hash(hash) => {
                    current_selector.push('#');
                    current_selector.push_str(hash);
                    self.advance();
                }
                CssToken::Delim(c) => {
                    current_selector.push(*c);
                    self.advance();
                }
                CssToken::Colon => {
                    current_selector.push(':');
                    self.advance();
                }
                _ => {
                    // Fallback
                    self.advance();
                }
            }
        }

        let mut declarations = Vec::new();
        loop {
            self.skip_whitespace();
            match self.current_token {
                CssToken::RightBrace => {
                    self.advance();
                    break;
                }
                CssToken::Eof => break,
                _ => {
                    if let Some(decl) = self.parse_declaration() {
                        declarations.push(decl);
                    } else {
                        // skip until semicolon or right brace
                        while !matches!(self.current_token, CssToken::Semicolon | CssToken::RightBrace | CssToken::Eof) {
                            self.advance();
                        }
                        if self.current_token == CssToken::Semicolon {
                            self.advance();
                        }
                    }
                }
            }
        }

        Some(CssRule {
            selectors,
            declarations,
        })
    }

    fn parse_declaration(&mut self) -> Option<CssDeclaration> {
        let name = match &self.current_token {
            CssToken::Ident(id) => id.clone(),
            _ => return None,
        };
        self.advance();
        self.skip_whitespace();

        if self.current_token != CssToken::Colon {
            return None;
        }
        self.advance();
        self.skip_whitespace();

        let mut value = String::new();
        loop {
            match &self.current_token {
                CssToken::Semicolon => {
                    self.advance();
                    break;
                }
                CssToken::RightBrace | CssToken::Eof => {
                    break;
                }
                CssToken::Whitespace => {
                    value.push(' ');
                    self.advance();
                }
                CssToken::Ident(id) | CssToken::String(id) => {
                    value.push_str(id);
                    self.advance();
                }
                CssToken::Hash(hash) => {
                    value.push('#');
                    value.push_str(hash);
                    self.advance();
                }
                CssToken::Number(n) => {
                    value.push_str(&n.to_string());
                    self.advance();
                }
                CssToken::Percentage(p) => {
                    value.push_str(&format!("{}%", p));
                    self.advance();
                }
                CssToken::Dimension { value: v, unit } => {
                    value.push_str(&format!("{}{}", v, unit));
                    self.advance();
                }
                CssToken::Delim(c) => {
                    value.push(*c);
                    self.advance();
                }
                CssToken::Colon => {
                    value.push(':');
                    self.advance();
                }
                _ => {
                    self.advance();
                }
            }
        }

        Some(CssDeclaration {
            name,
            value: value.trim().to_string(),
        })
    }
}
