//! WHATWG HTML Tokenizer State Machine

use std::collections::VecDeque;
use tracing::trace;
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
    Cdata(String),
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

pub struct HtmlTokenizer {
    buffer: VecDeque<char>,
    eof: bool,
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

impl HtmlTokenizer {
    pub fn new() -> Self {
        Self {
            buffer: VecDeque::new(),
            eof: false,
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

    pub fn push_chunk(&mut self, chunk: &str) {
        self.buffer.extend(chunk.chars());
    }

    pub fn end_of_file(&mut self) {
        self.eof = true;
    }

    fn consume_next(&mut self) -> Result<Option<char>, ()> {
        if self.reconsume {
            self.reconsume = false;
            return Ok(self.current_char);
        }

        if let Some(c) = self.buffer.pop_front() {
            self.current_char = Some(c);
            self.position += 1;
            if c == '\n' {
                self.line += 1;
                self.column = 1;
            } else {
                self.column += 1;
            }
            Ok(Some(c))
        } else if self.eof {
            self.current_char = None;
            Ok(None)
        } else {
            Err(())
        }
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
            if let Some(HtmlToken::StartTag { attributes, .. }) = &mut self.current_token {
                attributes.push(attr);
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
            if let Some(HtmlToken::StartTag { attributes, .. }) = &mut self.current_token {
                attributes.push(attr);
            }
        }
        let token = self.current_token.take().unwrap();
        
        self.state = TokenizerState::Data;
        
        if let HtmlToken::StartTag { ref name, .. } = token {
            self.last_start_tag = name.clone();
            match name.as_str() {
                "title" | "textarea" => self.state = TokenizerState::Rcdata,
                "style" | "xmp" | "iframe" | "noembed" | "noframes" | "script" => self.state = TokenizerState::Rawtext,
                "plaintext" => self.state = TokenizerState::Plaintext,
                _ => {}
            }
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
            let c = match self.consume_next() {
                Ok(Some(ch)) => Some(ch),
                Ok(None) => None,
                Err(()) => return Ok(None), // starved
            };
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
                            return Ok(self.emit_current_token());
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
                        if let Some(&next_ch) = self.buffer.front() {
                            if next_ch == '-' {
                                self.consume_next().unwrap(); // consume the second '-'
                                self.current_token = Some(HtmlToken::Comment(String::new()));
                                self.state = TokenizerState::CommentStart;
                            } else {
                                self.current_token = Some(HtmlToken::Comment(String::new()));
                                self.reconsume_in(TokenizerState::BogusComment);
                            }
                        } else if self.eof {
                            self.current_token = Some(HtmlToken::Comment(String::new()));
                            self.reconsume_in(TokenizerState::BogusComment);
                        } else {
                            self.reconsume_in(TokenizerState::MarkupDeclarationOpen);
                            return Ok(None);
                        }
                    } else if ch.eq_ignore_ascii_case(&'d') {
                        if self.buffer.len() >= 6 {
                            let s: String = self.buffer.iter().take(6).collect();
                            if s.eq_ignore_ascii_case("octype") {
                                for _ in 0..6 { self.consume_next().unwrap(); }
                                self.current_token = Some(HtmlToken::Doctype {
                                    name: None,
                                    public_id: None,
                                    system_id: None,
                                    force_quirks: false,
                                });
                                self.state = TokenizerState::Doctype;
                            } else {
                                self.current_token = Some(HtmlToken::Comment(String::new()));
                                self.reconsume_in(TokenizerState::BogusComment);
                            }
                        } else if self.eof {
                            self.current_token = Some(HtmlToken::Comment(String::new()));
                            self.reconsume_in(TokenizerState::BogusComment);
                        } else {
                            self.reconsume_in(TokenizerState::MarkupDeclarationOpen);
                            return Ok(None);
                        }
                    } else if ch == '[' {
                        if self.buffer.len() >= 6 {
                            let s: String = self.buffer.iter().take(6).collect();
                            if s == "CDATA[" {
                                for _ in 0..6 { self.consume_next().unwrap(); }
                                self.current_token = Some(HtmlToken::Cdata(String::new()));
                                self.state = TokenizerState::CdataSection;
                            } else {
                                self.current_token = Some(HtmlToken::Comment(String::new()));
                                self.reconsume_in(TokenizerState::BogusComment);
                            }
                        } else if self.eof {
                            self.current_token = Some(HtmlToken::Comment(String::new()));
                            self.reconsume_in(TokenizerState::BogusComment);
                        } else {
                            self.reconsume_in(TokenizerState::MarkupDeclarationOpen);
                            return Ok(None);
                        }
                    } else {
                        self.current_token = Some(HtmlToken::Comment(String::new()));
                        self.reconsume_in(TokenizerState::BogusComment);
                    }
                }
                TokenizerState::CommentStartDash => {
                    // This state is now unused (merged into MarkupDeclarationOpen above),
                    // but keep it for safety: treat as bogus comment
                    self.current_token = Some(HtmlToken::Comment(String::new()));
                    self.reconsume_in(TokenizerState::BogusComment);
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
                    match ch {
                        '\t' | '\n' | '\x0C' | ' ' => self.state = TokenizerState::BeforeDoctypeName,
                        '>' => self.reconsume_in(TokenizerState::BeforeDoctypeName),
                        _ if eof => {
                            if let Some(HtmlToken::Doctype { ref mut force_quirks, .. }) = self.current_token {
                                *force_quirks = true;
                            }
                            return Ok(self.emit_current_token());
                        }
                        _ => self.reconsume_in(TokenizerState::BeforeDoctypeName),
                    }
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
                            if let Some(HtmlToken::Doctype { name: Some(ref mut n), .. }) = self.current_token {
                                n.push(ch.to_ascii_lowercase());
                            }
                        }
                    }
                }
                TokenizerState::AfterDoctypeName => {
                    match ch {
                        '\t' | '\n' | '\x0C' | ' ' => {}
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
                            let mut s = String::new();
                            s.push(ch);
                            for c in self.buffer.iter().take(5) {
                                s.push(*c);
                            }
                            if s.eq_ignore_ascii_case("PUBLIC") && s.len() == 6 {
                                for _ in 0..5 { self.consume_next().unwrap(); }
                                self.state = TokenizerState::AfterDoctypePublicKeyword;
                            } else if s.eq_ignore_ascii_case("SYSTEM") && s.len() == 6 {
                                for _ in 0..5 { self.consume_next().unwrap(); }
                                self.state = TokenizerState::AfterDoctypeSystemKeyword;
                            } else {
                                if let Some(HtmlToken::Doctype { ref mut force_quirks, .. }) = self.current_token {
                                    *force_quirks = true;
                                }
                                self.state = TokenizerState::BogusDoctype;
                            }
                        }
                    }
                }
                TokenizerState::AfterDoctypePublicKeyword => {
                    match ch {
                        '\t' | '\n' | '\x0C' | ' ' => self.state = TokenizerState::BeforeDoctypePublicIdentifier,
                        '"' => {
                            if let Some(HtmlToken::Doctype { ref mut public_id, .. }) = self.current_token {
                                *public_id = Some(String::new());
                            }
                            self.state = TokenizerState::DoctypePublicIdentifierDoubleQuoted;
                        }
                        '\'' => {
                            if let Some(HtmlToken::Doctype { ref mut public_id, .. }) = self.current_token {
                                *public_id = Some(String::new());
                            }
                            self.state = TokenizerState::DoctypePublicIdentifierSingleQuoted;
                        }
                        '>' => {
                            if let Some(HtmlToken::Doctype { ref mut force_quirks, .. }) = self.current_token { *force_quirks = true; }
                            self.state = TokenizerState::Data;
                            return Ok(self.emit_current_token());
                        }
                        _ if eof => {
                            if let Some(HtmlToken::Doctype { ref mut force_quirks, .. }) = self.current_token { *force_quirks = true; }
                            return Ok(self.emit_current_token());
                        }
                        _ => {
                            if let Some(HtmlToken::Doctype { ref mut force_quirks, .. }) = self.current_token { *force_quirks = true; }
                            self.reconsume_in(TokenizerState::BogusDoctype);
                        }
                    }
                }
                TokenizerState::BeforeDoctypePublicIdentifier => {
                    match ch {
                        '\t' | '\n' | '\x0C' | ' ' => {}
                        '"' => {
                            if let Some(HtmlToken::Doctype { ref mut public_id, .. }) = self.current_token {
                                *public_id = Some(String::new());
                            }
                            self.state = TokenizerState::DoctypePublicIdentifierDoubleQuoted;
                        }
                        '\'' => {
                            if let Some(HtmlToken::Doctype { ref mut public_id, .. }) = self.current_token {
                                *public_id = Some(String::new());
                            }
                            self.state = TokenizerState::DoctypePublicIdentifierSingleQuoted;
                        }
                        '>' => {
                            if let Some(HtmlToken::Doctype { ref mut force_quirks, .. }) = self.current_token { *force_quirks = true; }
                            self.state = TokenizerState::Data;
                            return Ok(self.emit_current_token());
                        }
                        _ if eof => {
                            if let Some(HtmlToken::Doctype { ref mut force_quirks, .. }) = self.current_token { *force_quirks = true; }
                            return Ok(self.emit_current_token());
                        }
                        _ => {
                            if let Some(HtmlToken::Doctype { ref mut force_quirks, .. }) = self.current_token { *force_quirks = true; }
                            self.reconsume_in(TokenizerState::BogusDoctype);
                        }
                    }
                }
                TokenizerState::DoctypePublicIdentifierDoubleQuoted => {
                    match ch {
                        '"' => self.state = TokenizerState::AfterDoctypePublicIdentifier,
                        '\0' => {
                            if let Some(HtmlToken::Doctype { public_id: Some(ref mut p), .. }) = self.current_token {
                                p.push('\u{FFFD}');
                            }
                        }
                        '>' => {
                            if let Some(HtmlToken::Doctype { ref mut force_quirks, .. }) = self.current_token { *force_quirks = true; }
                            self.state = TokenizerState::Data;
                            return Ok(self.emit_current_token());
                        }
                        _ if eof => {
                            if let Some(HtmlToken::Doctype { ref mut force_quirks, .. }) = self.current_token { *force_quirks = true; }
                            return Ok(self.emit_current_token());
                        }
                        _ => {
                            if let Some(HtmlToken::Doctype { public_id: Some(ref mut p), .. }) = self.current_token {
                                p.push(ch);
                            }
                        }
                    }
                }
                TokenizerState::DoctypePublicIdentifierSingleQuoted => {
                    match ch {
                        '\'' => self.state = TokenizerState::AfterDoctypePublicIdentifier,
                        '\0' => {
                            if let Some(HtmlToken::Doctype { public_id: Some(ref mut p), .. }) = self.current_token {
                                p.push('\u{FFFD}');
                            }
                        }
                        '>' => {
                            if let Some(HtmlToken::Doctype { ref mut force_quirks, .. }) = self.current_token { *force_quirks = true; }
                            self.state = TokenizerState::Data;
                            return Ok(self.emit_current_token());
                        }
                        _ if eof => {
                            if let Some(HtmlToken::Doctype { ref mut force_quirks, .. }) = self.current_token { *force_quirks = true; }
                            return Ok(self.emit_current_token());
                        }
                        _ => {
                            if let Some(HtmlToken::Doctype { public_id: Some(ref mut p), .. }) = self.current_token {
                                p.push(ch);
                            }
                        }
                    }
                }
                TokenizerState::AfterDoctypePublicIdentifier => {
                    match ch {
                        '\t' | '\n' | '\x0C' | ' ' => self.state = TokenizerState::BetweenDoctypePublicAndSystemIdentifiers,
                        '>' => {
                            self.state = TokenizerState::Data;
                            return Ok(self.emit_current_token());
                        }
                        '"' => {
                            if let Some(HtmlToken::Doctype { ref mut system_id, .. }) = self.current_token {
                                *system_id = Some(String::new());
                            }
                            self.state = TokenizerState::DoctypeSystemIdentifierDoubleQuoted;
                        }
                        '\'' => {
                            if let Some(HtmlToken::Doctype { ref mut system_id, .. }) = self.current_token {
                                *system_id = Some(String::new());
                            }
                            self.state = TokenizerState::DoctypeSystemIdentifierSingleQuoted;
                        }
                        _ if eof => {
                            if let Some(HtmlToken::Doctype { ref mut force_quirks, .. }) = self.current_token { *force_quirks = true; }
                            return Ok(self.emit_current_token());
                        }
                        _ => {
                            if let Some(HtmlToken::Doctype { ref mut force_quirks, .. }) = self.current_token { *force_quirks = true; }
                            self.reconsume_in(TokenizerState::BogusDoctype);
                        }
                    }
                }
                TokenizerState::BetweenDoctypePublicAndSystemIdentifiers => {
                    match ch {
                        '\t' | '\n' | '\x0C' | ' ' => {}
                        '>' => {
                            self.state = TokenizerState::Data;
                            return Ok(self.emit_current_token());
                        }
                        '"' => {
                            if let Some(HtmlToken::Doctype { ref mut system_id, .. }) = self.current_token {
                                *system_id = Some(String::new());
                            }
                            self.state = TokenizerState::DoctypeSystemIdentifierDoubleQuoted;
                        }
                        '\'' => {
                            if let Some(HtmlToken::Doctype { ref mut system_id, .. }) = self.current_token {
                                *system_id = Some(String::new());
                            }
                            self.state = TokenizerState::DoctypeSystemIdentifierSingleQuoted;
                        }
                        _ if eof => {
                            if let Some(HtmlToken::Doctype { ref mut force_quirks, .. }) = self.current_token { *force_quirks = true; }
                            return Ok(self.emit_current_token());
                        }
                        _ => {
                            if let Some(HtmlToken::Doctype { ref mut force_quirks, .. }) = self.current_token { *force_quirks = true; }
                            self.reconsume_in(TokenizerState::BogusDoctype);
                        }
                    }
                }
                TokenizerState::AfterDoctypeSystemKeyword => {
                    match ch {
                        '\t' | '\n' | '\x0C' | ' ' => self.state = TokenizerState::BeforeDoctypeSystemIdentifier,
                        '"' => {
                            if let Some(HtmlToken::Doctype { ref mut system_id, .. }) = self.current_token {
                                *system_id = Some(String::new());
                            }
                            self.state = TokenizerState::DoctypeSystemIdentifierDoubleQuoted;
                        }
                        '\'' => {
                            if let Some(HtmlToken::Doctype { ref mut system_id, .. }) = self.current_token {
                                *system_id = Some(String::new());
                            }
                            self.state = TokenizerState::DoctypeSystemIdentifierSingleQuoted;
                        }
                        '>' => {
                            if let Some(HtmlToken::Doctype { ref mut force_quirks, .. }) = self.current_token { *force_quirks = true; }
                            self.state = TokenizerState::Data;
                            return Ok(self.emit_current_token());
                        }
                        _ if eof => {
                            if let Some(HtmlToken::Doctype { ref mut force_quirks, .. }) = self.current_token { *force_quirks = true; }
                            return Ok(self.emit_current_token());
                        }
                        _ => {
                            if let Some(HtmlToken::Doctype { ref mut force_quirks, .. }) = self.current_token { *force_quirks = true; }
                            self.reconsume_in(TokenizerState::BogusDoctype);
                        }
                    }
                }
                TokenizerState::BeforeDoctypeSystemIdentifier => {
                    match ch {
                        '\t' | '\n' | '\x0C' | ' ' => {}
                        '"' => {
                            if let Some(HtmlToken::Doctype { ref mut system_id, .. }) = self.current_token {
                                *system_id = Some(String::new());
                            }
                            self.state = TokenizerState::DoctypeSystemIdentifierDoubleQuoted;
                        }
                        '\'' => {
                            if let Some(HtmlToken::Doctype { ref mut system_id, .. }) = self.current_token {
                                *system_id = Some(String::new());
                            }
                            self.state = TokenizerState::DoctypeSystemIdentifierSingleQuoted;
                        }
                        '>' => {
                            if let Some(HtmlToken::Doctype { ref mut force_quirks, .. }) = self.current_token { *force_quirks = true; }
                            self.state = TokenizerState::Data;
                            return Ok(self.emit_current_token());
                        }
                        _ if eof => {
                            if let Some(HtmlToken::Doctype { ref mut force_quirks, .. }) = self.current_token { *force_quirks = true; }
                            return Ok(self.emit_current_token());
                        }
                        _ => {
                            if let Some(HtmlToken::Doctype { ref mut force_quirks, .. }) = self.current_token { *force_quirks = true; }
                            self.reconsume_in(TokenizerState::BogusDoctype);
                        }
                    }
                }
                TokenizerState::DoctypeSystemIdentifierDoubleQuoted => {
                    match ch {
                        '"' => self.state = TokenizerState::AfterDoctypeSystemIdentifier,
                        '\0' => {
                            if let Some(HtmlToken::Doctype { system_id: Some(ref mut s), .. }) = self.current_token {
                                s.push('\u{FFFD}');
                            }
                        }
                        '>' => {
                            if let Some(HtmlToken::Doctype { ref mut force_quirks, .. }) = self.current_token { *force_quirks = true; }
                            self.state = TokenizerState::Data;
                            return Ok(self.emit_current_token());
                        }
                        _ if eof => {
                            if let Some(HtmlToken::Doctype { ref mut force_quirks, .. }) = self.current_token { *force_quirks = true; }
                            return Ok(self.emit_current_token());
                        }
                        _ => {
                            if let Some(HtmlToken::Doctype { system_id: Some(ref mut s), .. }) = self.current_token {
                                s.push(ch);
                            }
                        }
                    }
                }
                TokenizerState::DoctypeSystemIdentifierSingleQuoted => {
                    match ch {
                        '\'' => self.state = TokenizerState::AfterDoctypeSystemIdentifier,
                        '\0' => {
                            if let Some(HtmlToken::Doctype { system_id: Some(ref mut s), .. }) = self.current_token {
                                s.push('\u{FFFD}');
                            }
                        }
                        '>' => {
                            if let Some(HtmlToken::Doctype { ref mut force_quirks, .. }) = self.current_token { *force_quirks = true; }
                            self.state = TokenizerState::Data;
                            return Ok(self.emit_current_token());
                        }
                        _ if eof => {
                            if let Some(HtmlToken::Doctype { ref mut force_quirks, .. }) = self.current_token { *force_quirks = true; }
                            return Ok(self.emit_current_token());
                        }
                        _ => {
                            if let Some(HtmlToken::Doctype { system_id: Some(ref mut s), .. }) = self.current_token {
                                s.push(ch);
                            }
                        }
                    }
                }
                TokenizerState::AfterDoctypeSystemIdentifier => {
                    match ch {
                        '\t' | '\n' | '\x0C' | ' ' => {}
                        '>' => {
                            self.state = TokenizerState::Data;
                            return Ok(self.emit_current_token());
                        }
                        _ if eof => {
                            if let Some(HtmlToken::Doctype { ref mut force_quirks, .. }) = self.current_token { *force_quirks = true; }
                            return Ok(self.emit_current_token());
                        }
                        _ => {
                            self.reconsume_in(TokenizerState::BogusDoctype);
                        }
                    }
                }
                TokenizerState::BogusDoctype => {
                    match ch {
                        '>' => {
                            self.state = TokenizerState::Data;
                            return Ok(self.emit_current_token());
                        }
                        '\0' => {}
                        _ if eof => return Ok(self.emit_current_token()),
                        _ => {}
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
                TokenizerState::CdataSection => {
                    if eof {
                        return Ok(self.emit_current_token());
                    } else if ch == ']' {
                        self.state = TokenizerState::CdataSectionBracket;
                    } else {
                        if let Some(HtmlToken::Cdata(ref mut data)) = self.current_token {
                            data.push(ch);
                        }
                    }
                }
                TokenizerState::CdataSectionBracket => {
                    if ch == ']' {
                        self.state = TokenizerState::CdataSectionEnd;
                    } else {
                        if let Some(HtmlToken::Cdata(ref mut data)) = self.current_token {
                            data.push(']');
                            data.push(ch);
                        }
                        self.state = TokenizerState::CdataSection;
                    }
                }
                TokenizerState::CdataSectionEnd => {
                    if ch == '>' {
                        self.state = TokenizerState::Data;
                        return Ok(self.emit_current_token());
                    } else if ch == ']' {
                        if let Some(HtmlToken::Cdata(ref mut data)) = self.current_token {
                            data.push(']');
                        }
                    } else {
                        if let Some(HtmlToken::Cdata(ref mut data)) = self.current_token {
                            data.push(']');
                            data.push(']');
                            data.push(ch);
                        }
                        self.state = TokenizerState::CdataSection;
                    }
                }
            }
        }
    }
}
