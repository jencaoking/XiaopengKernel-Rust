//! CSS Parser

use crate::selector::{Combinator, Selector, SelectorType, SimpleSelector, AttributeOperator};

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
            if c.is_alphanumeric() || c == '-' || c == '_' || c > '\x7F' {
                result.push(self.consume_char().unwrap());
            } else if c == '\\' {
                self.consume_char(); // consume '\'
                if let Some(escaped) = self.consume_char() {
                    result.push(escaped);
                }
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
                    parts.push(SimpleSelector::new_basic(SelectorType::Class, class_name));
                }
                Some('#') => {
                    self.consume_char();
                    let id_name = self.consume_ident();
                    parts.push(SimpleSelector::new_basic(SelectorType::Id, id_name));
                }
                Some('*') => {
                    self.consume_char();
                    parts.push(SimpleSelector::new_basic(SelectorType::Universal, "*".into()));
                }
                Some(':') => {
                    self.consume_char();
                    if self.next_char() == Some(':') {
                        self.consume_char();
                        let pseudo_elem = self.consume_ident();
                        parts.push(SimpleSelector::new_basic(SelectorType::PseudoElement, pseudo_elem));
                    } else {
                        let pseudo_class = self.consume_ident();
                        // Handle functional pseudo-classes like :nth-child(...)
                        let mut value = pseudo_class;
                        if self.next_char() == Some('(') {
                            self.consume_char();
                            value.push('(');
                            while let Some(c) = self.next_char() {
                                value.push(self.consume_char().unwrap());
                                if c == ')' { break; }
                            }
                        }
                        parts.push(SimpleSelector::new_basic(SelectorType::PseudoClass, value));
                    }
                }
                Some('[') => {
                    self.consume_char();
                    self.consume_whitespace();
                    let attr_name = self.consume_ident();
                    self.consume_whitespace();
                    
                    let mut op = AttributeOperator::Exists;
                    let mut attr_val = None;
                    
                    if let Some(c) = self.next_char() {
                        if c != ']' {
                            let mut op_str = String::new();
                            op_str.push(self.consume_char().unwrap());
                            if self.next_char() == Some('=') {
                                op_str.push(self.consume_char().unwrap());
                            }
                            
                            op = match op_str.as_str() {
                                "=" => AttributeOperator::Exact,
                                "~=" => AttributeOperator::Includes,
                                "|=" => AttributeOperator::DashMatch,
                                "^=" => AttributeOperator::Prefix,
                                "$=" => AttributeOperator::Suffix,
                                "*=" => AttributeOperator::Substring,
                                _ => AttributeOperator::Exists, // Fallback
                            };
                            
                            self.consume_whitespace();
                            let mut val = String::new();
                            
                            // Check for quote
                            let quote = self.next_char();
                            if quote == Some('"') || quote == Some('\'') {
                                let q = self.consume_char().unwrap();
                                while let Some(vc) = self.next_char() {
                                    if vc == q {
                                        self.consume_char();
                                        break;
                                    }
                                    val.push(self.consume_char().unwrap());
                                }
                            } else {
                                while let Some(vc) = self.next_char() {
                                    if vc == ']' || vc.is_whitespace() {
                                        break;
                                    }
                                    val.push(self.consume_char().unwrap());
                                }
                            }
                            attr_val = Some(val);
                        }
                    }
                    
                    self.consume_whitespace();
                    if self.next_char() == Some(']') {
                        self.consume_char();
                    }
                    parts.push(SimpleSelector::new_attribute(attr_name, op, attr_val));
                }
                Some(c) if c.is_alphabetic() || c == '_' || c == '-' => {
                    let tag_name = self.consume_ident();
                    parts.push(SimpleSelector::new_basic(SelectorType::Tag, tag_name));
                }
                _ => {
                    // Consume the unknown character to prevent infinite loop
                    self.consume_char();
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
                    
                    let mut value = value.trim().to_string();
                    let mut important = false;
                    let val_lower = value.to_lowercase();
                    if val_lower.ends_with("!important") {
                        value = value[..value.len() - 10].trim().to_string();
                        important = true;
                    }
                    
                    declarations.push(Declaration {
                        property,
                        value,
                        important,
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
