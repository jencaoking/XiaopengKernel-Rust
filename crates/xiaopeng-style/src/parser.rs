//! CSS Parser

use crate::selector::{Combinator, Selector, SelectorType, SimpleSelector};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Declaration {
    pub property: String,
    pub value: String,
    pub important: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rule {
    pub selectors: Vec<Selector>,
    pub declarations: Vec<Declaration>,
}

#[derive(Debug, Clone, Default)]
pub struct StyleSheet {
    pub rules: Vec<Rule>,
}

pub struct CssParser<'a> {
    input: &'a str,
    pos: usize,
}

impl<'a> CssParser<'a> {
    pub fn new(input: &'a str) -> Self {
        Self { input, pos: 0 }
    }

    fn next_char(&self) -> Option<char> {
        self.input[self.pos..].chars().next()
    }

    fn consume_char(&mut self) -> Option<char> {
        let mut iter = self.input[self.pos..].chars();
        match iter.next() {
            Some(c) => {
                self.pos += c.len_utf8();
                Some(c)
            }
            None => None,
        }
    }

    fn consume_whitespace(&mut self) {
        while let Some(c) = self.next_char() {
            if c.is_whitespace() {
                self.consume_char();
            } else {
                break;
            }
        }
    }

    fn consume_ident(&mut self) -> String {
        let mut result = String::new();
        while let Some(c) = self.next_char() {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                result.push(self.consume_char().unwrap());
            } else {
                break;
            }
        }
        result
    }

    pub fn parse(&mut self) -> StyleSheet {
        let mut rules = Vec::new();
        while self.pos < self.input.len() {
            self.consume_whitespace();
            if self.pos >= self.input.len() {
                break;
            }
            if let Some(rule) = self.parse_rule() {
                rules.push(rule);
            }
        }
        StyleSheet { rules }
    }

    fn parse_rule(&mut self) -> Option<Rule> {
        let selectors = self.parse_selectors();
        if selectors.is_empty() {
            return None;
        }

        let declarations = self.parse_declarations();
        Some(Rule {
            selectors,
            declarations,
        })
    }

    fn parse_selectors(&mut self) -> Vec<Selector> {
        let mut selectors = Vec::new();
        loop {
            self.consume_whitespace();
            if self.next_char() == Some('{') || self.pos >= self.input.len() {
                break;
            }
            if let Some(sel) = self.parse_selector() {
                selectors.push(sel);
            }
            self.consume_whitespace();
            if self.next_char() == Some(',') {
                self.consume_char();
            }
        }
        selectors
    }

    fn parse_selector(&mut self) -> Option<Selector> {
        let mut parts = Vec::new();
        let mut combinators = Vec::new();

        self.consume_whitespace();
        let mut parsing_part = true;
        
        while parsing_part {
            self.consume_whitespace();
            
            // Check for combinators
            if let Some(c) = self.next_char() {
                if c == ',' || c == '{' {
                    break;
                }
                
                // If it's not the first part, we might need a combinator
                if !parts.is_empty() {
                    match c {
                        '>' => { self.consume_char(); combinators.push(Combinator::Child); self.consume_whitespace(); },
                        '+' => { self.consume_char(); combinators.push(Combinator::NextSibling); self.consume_whitespace(); },
                        '~' => { self.consume_char(); combinators.push(Combinator::SubsequentSibling); self.consume_whitespace(); },
                        '.' | '#' | ':' | '[' => {
                            combinators.push(Combinator::None);
                        }
                        c if c.is_alphanumeric() => {
                            // Implied descendant combinator if space separated
                            if let Some(last_char) = self.input[..self.pos].chars().last() {
                                if last_char.is_whitespace() {
                                    combinators.push(Combinator::Descendant);
                                } else {
                                    combinators.push(Combinator::None);
                                }
                            }
                        }
                        _ => {}
                    }
                }
            } else {
                break;
            }

            match self.next_char() {
                Some('.') => {
                    self.consume_char();
                    let class_name = self.consume_ident();
                    parts.push(SimpleSelector { selector_type: SelectorType::Class, value: class_name });
                }
                Some('#') => {
                    self.consume_char();
                    let id_name = self.consume_ident();
                    parts.push(SimpleSelector { selector_type: SelectorType::Id, value: id_name });
                }
                Some('*') => {
                    self.consume_char();
                    parts.push(SimpleSelector { selector_type: SelectorType::Universal, value: "*".into() });
                }
                Some(c) if c.is_alphabetic() => {
                    let tag_name = self.consume_ident();
                    parts.push(SimpleSelector { selector_type: SelectorType::Tag, value: tag_name });
                }
                _ => {
                    parsing_part = false;
                }
            }
        }

        if parts.is_empty() {
            None
        } else {
            // Fix up combinators count (should be parts.len() - 1)
            while combinators.len() >= parts.len() {
                combinators.pop();
            }
            Some(Selector { parts, combinators })
        }
    }

    fn parse_declarations(&mut self) -> Vec<Declaration> {
        let mut declarations = Vec::new();
        self.consume_whitespace();
        if self.next_char() == Some('{') {
            self.consume_char(); // Consume '{'
            
            loop {
                self.consume_whitespace();
                if self.next_char() == Some('}') || self.pos >= self.input.len() {
                    break;
                }
                
                let property = self.consume_ident();
                self.consume_whitespace();
                
                if self.next_char() == Some(':') {
                    self.consume_char();
                    self.consume_whitespace();
                    
                    let mut value = String::new();
                    while let Some(c) = self.next_char() {
                        if c == ';' || c == '}' {
                            break;
                        }
                        value.push(self.consume_char().unwrap());
                    }
                    
                    declarations.push(Declaration {
                        property,
                        value: value.trim().to_string(),
                        important: false, // Stub
                    });
                    
                    if self.next_char() == Some(';') {
                        self.consume_char();
                    }
                } else {
                    // Recover from error
                    while let Some(c) = self.next_char() {
                        if c == ';' || c == '}' {
                            if c == ';' { self.consume_char(); }
                            break;
                        }
                        self.consume_char();
                    }
                }
            }
            
            if self.next_char() == Some('}') {
                self.consume_char();
            }
        }
        declarations
    }
}
