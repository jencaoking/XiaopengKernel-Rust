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
    AfterAttributeValueQuoted,
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

    #[allow(dead_code)]
    fn create_end_tag(&mut self) {
        self.current_token = Some(HtmlToken::EndTag {
            name: String::new(),
        });
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
                // ... Stubs for other states (they can be filled iteratively)
                _ => {
                    // Fallback stub for unimplemented states
                    if eof {
                        return Ok(self.emit(HtmlToken::Eof));
                    }
                    debug!("Unimplemented tokenizer state: {:?}", self.state);
                    self.state = TokenizerState::Data;
                }
            }
        }
    }
}
