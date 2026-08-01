//! WHATWG HTML Tokenizer State Machine

use std::str::Chars;
use tracing::{debug, trace};
use xiaopeng_common::XiaopengResult;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attribute {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HtmlToken {
    Doctype {
        name: Option<String>,
        public_id: Option<String>,
        system_id: Option<String>,
        force_quirks: bool,
    },
    StartTag {
        name: String,
        self_closing: bool,
        attributes: Vec<Attribute>,
    },
    EndTag {
        name: String,
    },
    Character(char),
    Comment(String),
    Eof,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenizerState {
    Data,
    TagOpen,
    EndTagOpen,
    TagName,
    BeforeAttributeName,
    AttributeName,
    AfterAttributeName,
    BeforeAttributeValue,
    AttributeValueDoubleQuoted,
    AttributeValueSingleQuoted,
    AttributeValueUnquoted,
    AfterAttributeValue,
    SelfClosingStartTag,
    BogusComment,
    MarkupDeclarationOpen,
    CommentStart,
    CommentStartDash,
    Comment,
    CommentEndDash,
    CommentEnd,
    CommentEndBang,
    Doctype,
    BeforeDoctypeName,
    DoctypeName,
    AfterDoctypeName,
    AfterDoctypePublicKeyword,
    BeforeDoctypePublicIdentifier,
    DoctypePublicIdentifierDoubleQuoted,
    DoctypePublicIdentifierSingleQuoted,
    AfterDoctypePublicIdentifier,
    BetweenDoctypePublicAndSystemIdentifiers,
    AfterDoctypeSystemKeyword,
    BeforeDoctypeSystemIdentifier,
    DoctypeSystemIdentifierDoubleQuoted,
    DoctypeSystemIdentifierSingleQuoted,
    AfterDoctypeSystemIdentifier,
    BogusDoctype,
    CdataSection,
    CdataSectionBracket,
    CdataSectionEnd,
    Rcdata,
    RcdataLessThanSign,
    RcdataEndTagOpen,
    RcdataEndTagName,
    Rawtext,
    RawtextLessThanSign,
    RawtextEndTagOpen,
    RawtextEndTagName,
    Plaintext,
}

pub struct HtmlTokenizer<'a> {
    input: Chars<'a>,
    current_char: Option<char>,
    reconsume: bool,
    pub state: TokenizerState,
    
    // Position tracking
    position: usize,
    line: usize,
    column: usize,

    // Token building state
    current_token: Option<HtmlToken>,
    #[allow(dead_code)]
    current_attribute: Option<Attribute>,
    
    // For parsing end tags in RCDATA/RAWTEXT
    last_start_tag: String,
    #[allow(dead_code)]
    temp_buffer: String,
}

impl<'a> HtmlTokenizer<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            input: input.chars(),
            current_char: None,
            reconsume: false,
            state: TokenizerState::Data,
            position: 0,
            line: 1,
            column: 1,
            current_token: None,
            current_attribute: None,
            last_start_tag: String::new(),
            temp_buffer: String::new(),
        }
    }

    fn consume_next(&mut self) -> Option<char> {
        if self.reconsume {
            self.reconsume = false;
            return self.current_char;
        }

        self.current_char = self.input.next();
        if let Some(c) = self.current_char {
            self.position += 1;
            if c == '\n' {
                self.line += 1;
                self.column = 1;
            } else {
                self.column += 1;
            }
        }
        self.current_char
    }

    fn reconsume_in(&mut self, state: TokenizerState) {
        self.reconsume = true;
        self.state = state;
    }

    fn emit(&mut self, token: HtmlToken) -> Option<HtmlToken> {
        trace!(?token, "Emitting HTML token");
        Some(token)
    }

    fn create_start_tag(&mut self) {
        self.current_token = Some(HtmlToken::StartTag {
            name: String::new(),
            self_closing: false,
            attributes: Vec::new(),
        });
    }

    fn create_end_tag(&mut self) {
        self.current_token = Some(HtmlToken::EndTag {
            name: String::new(),
        });
    }

    fn push_new_attribute(&mut self) {
        if let Some(attr) = self.current_attribute.take() {
            match &mut self.current_token {
                Some(HtmlToken::StartTag { attributes, .. }) => {
                    attributes.push(attr);
                }
                _ => {}
            }
        }
        self.current_attribute = Some(Attribute {
            name: String::new(),
            value: String::new(),
        });
    }

    fn append_to_attribute_name(&mut self, c: char) {
        if let Some(ref mut attr) = self.current_attribute {
            attr.name.push(c.to_ascii_lowercase());
        }
    }

    fn append_to_attribute_value(&mut self, c: char) {
        if let Some(ref mut attr) = self.current_attribute {
            attr.value.push(c);
        }
    }
    
    fn emit_current_token(&mut self) -> Option<HtmlToken> {
        if let Some(attr) = self.current_attribute.take() {
            match &mut self.current_token {
                Some(HtmlToken::StartTag { attributes, .. }) => {
                    attributes.push(attr);
                }
                _ => {}
            }
        }
        let token = self.current_token.take().unwrap();
        if let HtmlToken::StartTag { ref name, .. } = token {
            self.last_start_tag = name.clone();
        }
        self.emit(token)
    }

    fn append_to_tag_name(&mut self, c: char) {
        match &mut self.current_token {
            Some(HtmlToken::StartTag { name, .. }) | Some(HtmlToken::EndTag { name }) => {
                name.push(c.to_ascii_lowercase());
            }
            _ => {}
        }
    }

    pub fn next_token(&mut self) -> XiaopengResult<Option<HtmlToken>> {
        loop {
            let c = self.consume_next();
            let eof = c.is_none();
            let ch = c.unwrap_or('\0');

            match self.state {
                TokenizerState::Data => {
                    if eof {
                        return Ok(self.emit(HtmlToken::Eof));
                    }
                    match ch {
                        '<' => self.state = TokenizerState::TagOpen,
                        '\0' => {
                            // Parse error, emit replacement character
                            return Ok(self.emit(HtmlToken::Character('\u{FFFD}')));
                        }
                        _ => return Ok(self.emit(HtmlToken::Character(ch))),
                    }
                }
                TokenizerState::TagOpen => {
                    if eof {
                        // Parse error
                        self.reconsume_in(TokenizerState::Data);
                        return Ok(self.emit(HtmlToken::Character('<')));
                    }
                    match ch {
                        '!' => self.state = TokenizerState::MarkupDeclarationOpen,
                        '/' => self.state = TokenizerState::EndTagOpen,
                        'a'..='z' | 'A'..='Z' => {
                            self.create_start_tag();
                            self.reconsume_in(TokenizerState::TagName);
                        }
                        '?' => {
                            // Parse error, bogus comment
                            self.current_token = Some(HtmlToken::Comment(String::new()));
                            self.reconsume_in(TokenizerState::BogusComment);
                        }
                        _ => {
                            // Parse error
                            self.reconsume_in(TokenizerState::Data);
                            return Ok(self.emit(HtmlToken::Character('<')));
                        }
                    }
                }
                TokenizerState::TagName => {
                    if eof {
                        // Parse error
                        return Ok(self.emit(HtmlToken::Eof));
                    }
                    match ch {
                        '\t' | '\n' | '\x0C' | ' ' => self.state = TokenizerState::BeforeAttributeName,
                        '/' => self.state = TokenizerState::SelfClosingStartTag,
                        '>' => {
                            self.state = TokenizerState::Data;
                            let token = self.current_token.take().unwrap();
                            if let HtmlToken::StartTag { ref name, .. } = token {
                                self.last_start_tag = name.clone();
                            }
                            return Ok(self.emit(token));
                        }
                        '\0' => {
                            // Parse error
                            self.append_to_tag_name('\u{FFFD}');
                        }
                        _ => self.append_to_tag_name(ch),
                    }
                }
                TokenizerState::EndTagOpen => {
                    if eof {
                        self.reconsume_in(TokenizerState::Data);
                        return Ok(self.emit(HtmlToken::Character('<')));
                    }
                    match ch {
                        'a'..='z' | 'A'..='Z' => {
                            self.create_end_tag();
                            self.reconsume_in(TokenizerState::TagName);
                        }
                        '>' => {
                            // Parse error
                            self.state = TokenizerState::Data;
                        }
                        _ => {
                            // Bogus comment
                            self.current_token = Some(HtmlToken::Comment(String::new()));
                            self.reconsume_in(TokenizerState::BogusComment);
                        }
                    }
                }
                TokenizerState::BeforeAttributeName => {
                    match ch {
                        '\t' | '\n' | '\x0C' | ' ' => {}
                        '/' | '>' => self.reconsume_in(TokenizerState::AfterAttributeName),
                        '\0' if eof => self.reconsume_in(TokenizerState::AfterAttributeName),
                        '=' => {
                            self.push_new_attribute();
                            self.append_to_attribute_name(ch);
                            self.state = TokenizerState::AttributeName;
                        }
                        _ => {
                            self.push_new_attribute();
                            self.reconsume_in(TokenizerState::AttributeName);
                        }
                    }
                }
                TokenizerState::AttributeName => {
                    match ch {
                        '\t' | '\n' | '\x0C' | ' ' | '/' | '>' => {
                            self.reconsume_in(TokenizerState::AfterAttributeName);
                        }
                        '\0' if eof => {
                            self.reconsume_in(TokenizerState::AfterAttributeName);
                        }
                        '=' => self.state = TokenizerState::BeforeAttributeValue,
                        '\0' => self.append_to_attribute_name('\u{FFFD}'),
                        _ => self.append_to_attribute_name(ch),
                    }
                }
                TokenizerState::AfterAttributeName => {
                    match ch {
                        '\t' | '\n' | '\x0C' | ' ' => {}
                        '/' => self.state = TokenizerState::SelfClosingStartTag,
                        '=' => self.state = TokenizerState::BeforeAttributeValue,
                        '>' => {
                            self.state = TokenizerState::Data;
                            return Ok(self.emit_current_token());
                        }
                        _ if eof => {
                            self.reconsume_in(TokenizerState::Data);
                        }
                        _ => {
                            self.push_new_attribute();
                            self.reconsume_in(TokenizerState::AttributeName);
                        }
                    }
                }
                TokenizerState::BeforeAttributeValue => {
                    match ch {
                        '\t' | '\n' | '\x0C' | ' ' => {}
                        '"' => self.state = TokenizerState::AttributeValueDoubleQuoted,
                        '\'' => self.state = TokenizerState::AttributeValueSingleQuoted,
                        '>' => {
                            self.state = TokenizerState::Data;
                            return Ok(self.emit_current_token());
                        }
                        _ => self.reconsume_in(TokenizerState::AttributeValueUnquoted),
                    }
                }
                TokenizerState::AttributeValueDoubleQuoted => {
                    match ch {
                        '"' => self.state = TokenizerState::AfterAttributeValue,
                        '\0' => self.append_to_attribute_value('\u{FFFD}'),
                        _ if eof => return Ok(self.emit_current_token()), // EOF parse error
                        _ => self.append_to_attribute_value(ch),
                    }
                }
                TokenizerState::AttributeValueSingleQuoted => {
                    match ch {
                        '\'' => self.state = TokenizerState::AfterAttributeValue,
                        '\0' => self.append_to_attribute_value('\u{FFFD}'),
                        _ if eof => return Ok(self.emit_current_token()),
                        _ => self.append_to_attribute_value(ch),
                    }
                }
                TokenizerState::AttributeValueUnquoted => {
                    match ch {
                        '\t' | '\n' | '\x0C' | ' ' => self.state = TokenizerState::BeforeAttributeName,
                        '>' => {
                            self.state = TokenizerState::Data;
                            return Ok(self.emit_current_token());
                        }
                        '\0' => self.append_to_attribute_value('\u{FFFD}'),
                        _ if eof => return Ok(self.emit_current_token()),
                        _ => self.append_to_attribute_value(ch),
                    }
                }
                TokenizerState::AfterAttributeValue => {
                    match ch {
                        '\t' | '\n' | '\x0C' | ' ' => self.state = TokenizerState::BeforeAttributeName,
                        '/' => self.state = TokenizerState::SelfClosingStartTag,
                        '>' => {
                            self.state = TokenizerState::Data;
                            return Ok(self.emit_current_token());
                        }
                        _ if eof => return Ok(self.emit_current_token()),
                        _ => self.reconsume_in(TokenizerState::BeforeAttributeName),
                    }
                }
                TokenizerState::SelfClosingStartTag => {
                    match ch {
                        '>' => {
                            if let Some(HtmlToken::StartTag { ref mut self_closing, .. }) = self.current_token {
                                *self_closing = true;
                            }
                            self.state = TokenizerState::Data;
                            return Ok(self.emit_current_token());
                        }
                        _ if eof => return Ok(self.emit_current_token()),
                        _ => self.reconsume_in(TokenizerState::BeforeAttributeName),
                    }
                }
                TokenizerState::BogusComment => {
                    match ch {
                        '>' => {
                            self.state = TokenizerState::Data;
                            let token = self.current_token.take().unwrap();
                            return Ok(self.emit(token));
                        }
                        '\0' => {
                            if let Some(HtmlToken::Comment(ref mut data)) = self.current_token {
                                data.push('\u{FFFD}');
                            }
                        }
                        _ if eof => {
                            let token = self.current_token.take().unwrap();
                            return Ok(self.emit(token));
                        }
                        _ => {
                            if let Some(HtmlToken::Comment(ref mut data)) = self.current_token {
                                data.push(ch);
                            }
                        }
                    }
                }
                TokenizerState::MarkupDeclarationOpen => {
                    if ch == '-' {
                        self.state = TokenizerState::CommentStartDash;
                    } else if ch.to_ascii_uppercase() == 'D' {
                        self.reconsume_in(TokenizerState::Doctype);
                    } else {
                        self.current_token = Some(HtmlToken::Comment(String::new()));
                        self.reconsume_in(TokenizerState::BogusComment);
                    }
                }
                TokenizerState::CommentStartDash => {
                    if ch == '-' {
                        self.current_token = Some(HtmlToken::Comment(String::new()));
                        self.state = TokenizerState::CommentStart;
                    } else {
                        self.current_token = Some(HtmlToken::Comment(String::new()));
                        self.reconsume_in(TokenizerState::BogusComment);
                    }
                }
                TokenizerState::CommentStart => {
                    if ch == '-' {
                        self.state = TokenizerState::CommentEndDash;
                    } else if ch == '>' {
                        self.state = TokenizerState::Data;
                        return Ok(self.emit_current_token());
                    } else if eof {
                        self.reconsume_in(TokenizerState::Data);
                    } else {
                        self.reconsume_in(TokenizerState::Comment);
                    }
                }
                TokenizerState::Comment => {
                    if ch == '-' {
                        self.state = TokenizerState::CommentEndDash;
                    } else if eof {
                        return Ok(self.emit_current_token());
                    } else {
                        if let Some(HtmlToken::Comment(ref mut data)) = self.current_token {
                            data.push(ch);
                        }
                    }
                }
                TokenizerState::CommentEndDash => {
                    if ch == '-' {
                        self.state = TokenizerState::CommentEnd;
                    } else if eof {
                        return Ok(self.emit_current_token());
                    } else {
                        if let Some(HtmlToken::Comment(ref mut data)) = self.current_token {
                            data.push('-');
                        }
                        self.reconsume_in(TokenizerState::Comment);
                    }
                }
                TokenizerState::CommentEnd => {
                    if ch == '>' {
                        self.state = TokenizerState::Data;
                        return Ok(self.emit_current_token());
                    } else if ch == '!' {
                        self.state = TokenizerState::CommentEndBang;
                    } else if ch == '-' {
                        if let Some(HtmlToken::Comment(ref mut data)) = self.current_token {
                            data.push('-');
                        }
                    } else if eof {
                        return Ok(self.emit_current_token());
                    } else {
                        if let Some(HtmlToken::Comment(ref mut data)) = self.current_token {
                            data.push('-');
                            data.push('-');
                        }
                        self.reconsume_in(TokenizerState::Comment);
                    }
                }
                TokenizerState::CommentEndBang => {
                    if ch == '-' {
                        if let Some(HtmlToken::Comment(ref mut data)) = self.current_token {
                            data.push('-');
                            data.push('-');
                            data.push('!');
                        }
                        self.state = TokenizerState::CommentEndDash;
                    } else if ch == '>' {
                        self.state = TokenizerState::Data;
                        return Ok(self.emit_current_token());
                    } else if eof {
                        return Ok(self.emit_current_token());
                    } else {
                        if let Some(HtmlToken::Comment(ref mut data)) = self.current_token {
                            data.push('-');
                            data.push('-');
                            data.push('!');
                        }
                        self.reconsume_in(TokenizerState::Comment);
                    }
                }
                TokenizerState::Doctype => {
                    self.current_token = Some(HtmlToken::Doctype {
                        name: None,
                        public_id: None,
                        system_id: None,
                        force_quirks: false,
                    });
                    self.state = TokenizerState::BeforeDoctypeName;
                }
                TokenizerState::BeforeDoctypeName => {
                    match ch {
                        '\t' | '\n' | '\x0C' | ' ' => {}
                        '>' => {
                            self.state = TokenizerState::Data;
                            if let Some(HtmlToken::Doctype { ref mut force_quirks, .. }) = self.current_token {
                                *force_quirks = true;
                            }
                            return Ok(self.emit_current_token());
                        }
                        _ if eof => {
                            if let Some(HtmlToken::Doctype { ref mut force_quirks, .. }) = self.current_token {
                                *force_quirks = true;
                            }
                            return Ok(self.emit_current_token());
                        }
                        _ => {
                            if let Some(HtmlToken::Doctype { ref mut name, .. }) = self.current_token {
                                *name = Some(String::new());
                            }
                            self.reconsume_in(TokenizerState::DoctypeName);
                        }
                    }
                }
                TokenizerState::DoctypeName => {
                    match ch {
                        '\t' | '\n' | '\x0C' | ' ' => self.state = TokenizerState::AfterDoctypeName,
                        '>' => {
                            self.state = TokenizerState::Data;
                            return Ok(self.emit_current_token());
                        }
                        _ if eof => {
                            if let Some(HtmlToken::Doctype { ref mut force_quirks, .. }) = self.current_token {
                                *force_quirks = true;
                            }
                            return Ok(self.emit_current_token());
                        }
                        _ => {
                            if let Some(HtmlToken::Doctype { ref mut name, .. }) = self.current_token {
                                if let Some(n) = name {
                                    n.push(ch.to_ascii_lowercase());
                                }
                            }
                        }
                    }
                }
                TokenizerState::AfterDoctypeName | TokenizerState::BogusDoctype => {
                    match ch {
                        '>' => {
                            self.state = TokenizerState::Data;
                            return Ok(self.emit_current_token());
                        }
                        _ if eof => return Ok(self.emit_current_token()),
                        _ => self.state = TokenizerState::BogusDoctype,
                    }
                }
                TokenizerState::Rcdata => {
                    if eof { return Ok(self.emit(HtmlToken::Eof)); }
                    match ch {
                        '<' => self.state = TokenizerState::RcdataLessThanSign,
                        '\0' => return Ok(self.emit(HtmlToken::Character('\u{FFFD}'))),
                        _ => return Ok(self.emit(HtmlToken::Character(ch))),
                    }
                }
                TokenizerState::RcdataLessThanSign => {
                    if ch == '/' {
                        self.temp_buffer.clear();
                        self.state = TokenizerState::RcdataEndTagOpen;
                    } else {
                        self.reconsume_in(TokenizerState::Rcdata);
                        return Ok(self.emit(HtmlToken::Character('<')));
                    }
                }
                TokenizerState::RcdataEndTagOpen => {
                    if ch.is_ascii_alphabetic() {
                        self.create_end_tag();
                        self.reconsume_in(TokenizerState::RcdataEndTagName);
                    } else {
                        self.reconsume_in(TokenizerState::Rcdata);
                        return Ok(self.emit(HtmlToken::Character('<')));
                    }
                }
                TokenizerState::RcdataEndTagName => {
                    let mut is_match = false;
                    if let Some(HtmlToken::EndTag { ref name }) = self.current_token {
                        is_match = name.eq_ignore_ascii_case(&self.last_start_tag);
                    }
                    if ch.is_ascii_whitespace() && is_match {
                        self.state = TokenizerState::BeforeAttributeName;
                    } else if ch == '/' && is_match {
                        self.state = TokenizerState::SelfClosingStartTag;
                    } else if ch == '>' && is_match {
                        self.state = TokenizerState::Data;
                        return Ok(self.emit_current_token());
                    } else if ch.is_ascii_alphabetic() {
                        if let Some(HtmlToken::EndTag { ref mut name }) = self.current_token {
                            name.push(ch.to_ascii_lowercase());
                            self.temp_buffer.push(ch);
                        }
                    } else {
                        self.state = TokenizerState::Rcdata;
                        return Ok(self.emit(HtmlToken::Character('<')));
                    }
                }
                TokenizerState::Rawtext => {
                    if eof { return Ok(self.emit(HtmlToken::Eof)); }
                    match ch {
                        '<' => self.state = TokenizerState::RawtextLessThanSign,
                        '\0' => return Ok(self.emit(HtmlToken::Character('\u{FFFD}'))),
                        _ => return Ok(self.emit(HtmlToken::Character(ch))),
                    }
                }
                TokenizerState::RawtextLessThanSign => {
                    if ch == '/' {
                        self.temp_buffer.clear();
                        self.state = TokenizerState::RawtextEndTagOpen;
                    } else {
                        self.reconsume_in(TokenizerState::Rawtext);
                        return Ok(self.emit(HtmlToken::Character('<')));
                    }
                }
                TokenizerState::RawtextEndTagOpen => {
                    if ch.is_ascii_alphabetic() {
                        self.create_end_tag();
                        self.reconsume_in(TokenizerState::RawtextEndTagName);
                    } else {
                        self.reconsume_in(TokenizerState::Rawtext);
                        return Ok(self.emit(HtmlToken::Character('<')));
                    }
                }
                TokenizerState::RawtextEndTagName => {
                    let mut is_match = false;
                    if let Some(HtmlToken::EndTag { ref name }) = self.current_token {
                        is_match = name.eq_ignore_ascii_case(&self.last_start_tag);
                    }
                    if ch.is_ascii_whitespace() && is_match {
                        self.state = TokenizerState::BeforeAttributeName;
                    } else if ch == '/' && is_match {
                        self.state = TokenizerState::SelfClosingStartTag;
                    } else if ch == '>' && is_match {
                        self.state = TokenizerState::Data;
                        return Ok(self.emit_current_token());
                    } else if ch.is_ascii_alphabetic() {
                        if let Some(HtmlToken::EndTag { ref mut name }) = self.current_token {
                            name.push(ch.to_ascii_lowercase());
                        }
                    } else {
                        self.state = TokenizerState::Rawtext;
                        return Ok(self.emit(HtmlToken::Character('<')));
                    }
                }
                TokenizerState::Plaintext => {
                    if eof { return Ok(self.emit(HtmlToken::Eof)); }
                    match ch {
                        '\0' => return Ok(self.emit(HtmlToken::Character('\u{FFFD}'))),
                        _ => return Ok(self.emit(HtmlToken::Character(ch))),
                    }
                }
                _ => {
                    if eof {
                        return Ok(self.emit(HtmlToken::Eof));
                    }
                    self.state = TokenizerState::Data;
                }
            }
        }
    }
}
