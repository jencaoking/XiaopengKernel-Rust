//! HTML Tree Builder Insertion Modes and DOM Construction

use tracing::{debug, trace};
use xiaopeng_dom::{Document, NodePtr};
use crate::html::tokenizer::HtmlToken;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InsertionMode {
    Initial,
    BeforeHtml,
    BeforeHead,
    InHead,
    InHeadNoscript,
    AfterHead,
    InBody,
    Text,
    InTable,
    InTableText,
    InCaption,
    InColumnGroup,
    InTableBody,
    InRow,
    InCell,
    InSelect,
    InSelectInTable,
    InTemplate,
    AfterBody,
    InFrameset,
    AfterFrameset,
    AfterAfterBody,
    AfterAfterFrameset,
    Plaintext,
}

#[allow(dead_code)]
pub struct HtmlTreeBuilder {
    pub document: Document,
    
    pub insertion_mode: InsertionMode,
    pub original_insertion_mode: InsertionMode,
    
    pub open_elements: Vec<NodePtr>,
    pub active_formatting_elements: Vec<NodePtr>,
    
    pub head_element: Option<NodePtr>,
    pub form_element: Option<NodePtr>,
    
    pub frameset_ok: bool,
    pub quirks_mode: bool,
    pub foster_parenting: bool,
}

impl HtmlTreeBuilder {
    pub fn new() -> Self {
        let document = Document::new();
        let open_elements = vec![document.root.clone()];
        Self {
            document,
            insertion_mode: InsertionMode::Initial,
            original_insertion_mode: InsertionMode::Initial,
            open_elements,
            active_formatting_elements: Vec::new(),
            head_element: None,
            form_element: None,
            frameset_ok: true,
            quirks_mode: false,
            foster_parenting: false,
        }
    }

    pub fn process_token(&mut self, token: HtmlToken) {
        trace!(?token, mode = ?self.insertion_mode, "Processing HTML token");
        
        match self.insertion_mode {
            InsertionMode::Initial => self.process_initial(&token),
            InsertionMode::BeforeHtml => self.process_before_html(&token),
            InsertionMode::BeforeHead => self.process_before_head(&token),
            InsertionMode::InHead => self.process_in_head(&token),
            InsertionMode::InHeadNoscript => self.process_in_head_noscript(&token),
            InsertionMode::AfterHead => self.process_after_head(&token),
            InsertionMode::InBody => self.process_in_body(&token),
            InsertionMode::Text => self.process_text(&token),
            InsertionMode::InTable => self.process_in_table(&token),
            InsertionMode::InTableText => self.process_in_table_text(&token),
            InsertionMode::InCaption => self.process_in_caption(&token),
            InsertionMode::InColumnGroup => self.process_in_column_group(&token),
            InsertionMode::InTableBody => self.process_in_table_body(&token),
            InsertionMode::InRow => self.process_in_row(&token),
            InsertionMode::InCell => self.process_in_cell(&token),
            InsertionMode::InSelect => self.process_in_select(&token),
            InsertionMode::InSelectInTable => self.process_in_select_in_table(&token),
            InsertionMode::InTemplate => self.process_in_template(&token),
            InsertionMode::AfterBody => self.process_after_body(&token),
            InsertionMode::InFrameset => self.process_in_frameset(&token),
            InsertionMode::AfterFrameset => self.process_after_frameset(&token),
            InsertionMode::AfterAfterBody => self.process_after_after_body(&token),
            InsertionMode::AfterAfterFrameset => self.process_after_after_frameset(&token),
            InsertionMode::Plaintext => self.process_plaintext(&token),
        }
    }

    // --- Insertion Mode Handlers (Stubs) ---

    fn process_initial(&mut self, token: &HtmlToken) {
        match token {
            HtmlToken::Character(c) if c.is_whitespace() => (),
            HtmlToken::Comment(_) => self.insert_comment(token),
            HtmlToken::Doctype { .. } => {
                self.insertion_mode = InsertionMode::BeforeHtml;
            }
            _ => {
                self.insertion_mode = InsertionMode::BeforeHtml;
                self.process_before_html(token);
            }
        }
    }

    fn process_before_html(&mut self, token: &HtmlToken) {
        match token {
            HtmlToken::Character(c) if c.is_whitespace() => (),
            HtmlToken::Comment(_) => self.insert_comment(token),
            HtmlToken::StartTag { name, .. } if name == "html" => {
                self.insert_element_with_token(token);
                self.insertion_mode = InsertionMode::BeforeHead;
            }
            _ => {
                self.insert_html_element();
                self.insertion_mode = InsertionMode::BeforeHead;
                self.process_before_head(token);
            }
        }
    }

    fn process_before_head(&mut self, token: &HtmlToken) {
        match token {
            HtmlToken::Character(c) if c.is_whitespace() => (),
            HtmlToken::Comment(_) => self.insert_comment(token),
            HtmlToken::StartTag { name, .. } if name == "head" => {
                self.insert_element_with_token(token);
                self.head_element = self.open_elements.last().cloned();
                self.insertion_mode = InsertionMode::InHead;
            }
            _ => {
                self.insert_head_element();
                self.insertion_mode = InsertionMode::InHead;
                self.process_in_head(token);
            }
        }
    }

    fn process_in_head(&mut self, token: &HtmlToken) {
        match token {
            HtmlToken::Character(c) if c.is_whitespace() => self.insert_character(*c),
            HtmlToken::Comment(_) => self.insert_comment(token),
            HtmlToken::StartTag { name, .. } if matches!(name.as_str(), "base" | "basefont" | "bgsound" | "link" | "meta") => {
                self.insert_element_with_token(token);
                self.open_elements.pop(); // Pop immediately because they are void
            }
            HtmlToken::StartTag { name, .. } if matches!(name.as_str(), "title" | "style" | "script" | "noscript") => {
                self.insert_element_with_token(token);
                self.original_insertion_mode = self.insertion_mode;
                self.insertion_mode = InsertionMode::Text;
            }
            HtmlToken::EndTag { name } if name == "head" => {
                self.open_elements.pop();
                self.insertion_mode = InsertionMode::AfterHead;
            }
            HtmlToken::EndTag { name } if matches!(name.as_str(), "title" | "style" | "script" | "noscript") => {
                self.open_elements.pop();
            }
            _ => {
                self.open_elements.pop();
                self.insertion_mode = InsertionMode::AfterHead;
                self.process_after_head(token);
            }
        }
    }

    fn process_in_head_noscript(&mut self, _token: &HtmlToken) {
        self.insertion_mode = InsertionMode::InHead;
    }

    fn process_after_head(&mut self, token: &HtmlToken) {
        match token {
            HtmlToken::Character(c) if c.is_whitespace() => self.insert_character(*c),
            HtmlToken::Comment(_) => self.insert_comment(token),
            HtmlToken::StartTag { name, .. } if name == "body" => {
                self.insert_element_with_token(token);
                self.frameset_ok = false;
                self.insertion_mode = InsertionMode::InBody;
            }
            _ => {
                self.insert_body_element();
                self.insertion_mode = InsertionMode::InBody;
                self.process_in_body(token);
            }
        }
    }

    fn process_in_body(&mut self, token: &HtmlToken) {
        match token {
            HtmlToken::Character(c) => self.insert_character(*c),
            HtmlToken::Comment(_) => self.insert_comment(token),
            HtmlToken::StartTag { name, .. } if name == "table" => {
                self.insert_element_with_token(token);
                self.insertion_mode = InsertionMode::InTable;
            }
            HtmlToken::StartTag { name, .. } if name == "select" => {
                self.insert_element_with_token(token);
                self.insertion_mode = InsertionMode::InSelect;
            }
            HtmlToken::StartTag { name, .. } if name == "template" => {
                self.insert_element_with_token(token);
                self.insertion_mode = InsertionMode::InTemplate;
            }
            HtmlToken::StartTag { name, .. } if name == "frameset" => {
                self.insert_element_with_token(token);
                self.insertion_mode = InsertionMode::InFrameset;
            }
            HtmlToken::StartTag { name, self_closing, .. } => {
                self.insert_element_with_token(token);
                if matches!(name.as_str(), "script" | "style" | "textarea" | "title" | "xmp" | "iframe" | "noembed" | "noframes" | "noscript") {
                    self.original_insertion_mode = self.insertion_mode;
                    self.insertion_mode = InsertionMode::Text;
                } else if *self_closing || Self::is_void_element(name) {
                    self.open_elements.pop();
                }
            }
            HtmlToken::EndTag { name } => {
                self.pop_until_element(name);
            }
            _ => {}
        }
    }

    fn process_text(&mut self, token: &HtmlToken) {
        match token {
            HtmlToken::Character(c) => self.insert_character(*c),
            HtmlToken::EndTag { .. } => {
                self.open_elements.pop();
                self.insertion_mode = self.original_insertion_mode;
            }
            HtmlToken::Eof => {
                self.open_elements.pop();
                self.insertion_mode = self.original_insertion_mode;
            }
            _ => {}
        }
    }
    fn process_in_table(&mut self, token: &HtmlToken) {
        match token {
            HtmlToken::Character(_) => self.insert_character(match token { HtmlToken::Character(c) => *c, _ => unreachable!() }),
            HtmlToken::Comment(_) => self.insert_comment(token),
            HtmlToken::StartTag { name, .. } if name == "caption" => {
                self.insert_element_with_token(token);
                self.insertion_mode = InsertionMode::InCaption;
            }
            HtmlToken::StartTag { name, .. } if name == "colgroup" => {
                self.insert_element_with_token(token);
                self.insertion_mode = InsertionMode::InColumnGroup;
            }
            HtmlToken::StartTag { name, .. } if matches!(name.as_str(), "tbody" | "thead" | "tfoot") => {
                self.insert_element_with_token(token);
                self.insertion_mode = InsertionMode::InTableBody;
            }
            HtmlToken::StartTag { name, .. } if matches!(name.as_str(), "td" | "th" | "tr") => {
                let synthetic = HtmlToken::StartTag { name: "tbody".into(), self_closing: false, attributes: vec![] };
                self.process_in_table(&synthetic);
                self.process_in_table_body(token);
            }
            HtmlToken::EndTag { name } if name == "table" => {
                self.pop_until_element("table");
                self.insertion_mode = InsertionMode::InBody;
            }
            _ => self.process_in_body(token),
        }
    }
    fn process_in_table_text(&mut self, _token: &HtmlToken) {}
    fn process_in_caption(&mut self, token: &HtmlToken) {
        match token {
            HtmlToken::EndTag { name } if name == "caption" => {
                self.pop_until_element("caption");
                self.insertion_mode = InsertionMode::InTable;
            }
            _ => self.process_in_body(token),
        }
    }
    fn process_in_column_group(&mut self, token: &HtmlToken) {
        match token {
            HtmlToken::Character(c) if c.is_whitespace() => self.insert_character(*c),
            HtmlToken::StartTag { name, .. } if name == "col" => {
                self.insert_element_with_token(token);
                self.open_elements.pop();
            }
            HtmlToken::EndTag { name } if name == "colgroup" => {
                self.pop_until_element("colgroup");
                self.insertion_mode = InsertionMode::InTable;
            }
            _ => {
                self.pop_until_element("colgroup");
                self.insertion_mode = InsertionMode::InTable;
                self.process_in_table(token);
            }
        }
    }
    fn process_in_table_body(&mut self, token: &HtmlToken) {
        match token {
            HtmlToken::StartTag { name, .. } if name == "tr" => {
                self.insert_element_with_token(token);
                self.insertion_mode = InsertionMode::InRow;
            }
            HtmlToken::StartTag { name, .. } if matches!(name.as_str(), "th" | "td") => {
                let synthetic = HtmlToken::StartTag { name: "tr".into(), self_closing: false, attributes: vec![] };
                self.process_in_table_body(&synthetic);
                self.process_in_row(token);
            }
            HtmlToken::EndTag { name } if matches!(name.as_str(), "tbody" | "thead" | "tfoot") => {
                self.pop_until_element(name);
                self.insertion_mode = InsertionMode::InTable;
            }
            HtmlToken::EndTag { name } if name == "table" => {
                self.insertion_mode = InsertionMode::InTable;
                self.process_in_table(token);
            }
            _ => self.process_in_table(token),
        }
    }
    fn process_in_row(&mut self, token: &HtmlToken) {
        match token {
            HtmlToken::StartTag { name, .. } if matches!(name.as_str(), "th" | "td") => {
                self.insert_element_with_token(token);
                self.insertion_mode = InsertionMode::InCell;
            }
            HtmlToken::EndTag { name } if name == "tr" => {
                self.pop_until_element("tr");
                self.insertion_mode = InsertionMode::InTableBody;
            }
            HtmlToken::EndTag { name } if name == "table" => {
                self.pop_until_element("tr");
                self.insertion_mode = InsertionMode::InTableBody;
                self.process_in_table_body(token);
            }
            _ => self.process_in_table(token),
        }
    }
    fn process_in_cell(&mut self, token: &HtmlToken) {
        match token {
            HtmlToken::EndTag { name } if matches!(name.as_str(), "td" | "th") => {
                self.pop_until_element(name);
                self.insertion_mode = InsertionMode::InRow;
            }
            HtmlToken::EndTag { name } if matches!(name.as_str(), "tr" | "tbody" | "thead" | "tfoot" | "table") => {
                self.insertion_mode = InsertionMode::InRow;
                self.process_in_row(token);
            }
            _ => self.process_in_body(token),
        }
    }
    fn process_in_select(&mut self, token: &HtmlToken) {
        match token {
            HtmlToken::Character(c) => self.insert_character(*c),
            HtmlToken::StartTag { name, .. } if name == "option" => self.insert_element_with_token(token),
            HtmlToken::StartTag { name, .. } if name == "optgroup" => self.insert_element_with_token(token),
            HtmlToken::EndTag { name } if name == "option" || name == "optgroup" => self.pop_until_element(name),
            HtmlToken::EndTag { name } if name == "select" => {
                self.pop_until_element("select");
                self.insertion_mode = InsertionMode::InBody;
            }
            _ => {}
        }
    }
    fn process_in_select_in_table(&mut self, token: &HtmlToken) {
        match token {
            HtmlToken::EndTag { name } if matches!(name.as_str(), "caption" | "table" | "tbody" | "tfoot" | "thead" | "tr" | "td" | "th") => {
                self.pop_until_element("select");
                self.insertion_mode = InsertionMode::InBody;
                self.process_in_body(token);
            }
            _ => self.process_in_select(token),
        }
    }
    fn process_in_template(&mut self, token: &HtmlToken) {
        match token {
            HtmlToken::EndTag { name } if name == "template" => {
                self.pop_until_element("template");
                self.insertion_mode = InsertionMode::InBody;
            }
            _ => self.process_in_body(token),
        }
    }
    fn process_after_body(&mut self, token: &HtmlToken) {
        match token {
            HtmlToken::Character(c) if c.is_whitespace() => self.insert_character(*c),
            HtmlToken::Comment(_) => self.insert_comment(token),
            HtmlToken::EndTag { name } if name == "html" => self.insertion_mode = InsertionMode::AfterAfterBody,
            HtmlToken::Eof => (),
            _ => {
                self.insertion_mode = InsertionMode::InBody;
                self.process_in_body(token);
            }
        }
    }
    fn process_in_frameset(&mut self, token: &HtmlToken) {
        match token {
            HtmlToken::Character(c) if c.is_whitespace() => self.insert_character(*c),
            HtmlToken::Comment(_) => self.insert_comment(token),
            HtmlToken::StartTag { name, .. } if name == "frameset" => self.insert_element_with_token(token),
            HtmlToken::StartTag { name, .. } if name == "frame" => {
                self.insert_element_with_token(token);
                self.open_elements.pop();
            }
            HtmlToken::EndTag { name } if name == "frameset" => {
                self.pop_until_element("frameset");
                self.insertion_mode = InsertionMode::AfterFrameset;
            }
            _ => {}
        }
    }
    fn process_after_frameset(&mut self, token: &HtmlToken) {
        match token {
            HtmlToken::Character(c) if c.is_whitespace() => self.insert_character(*c),
            HtmlToken::EndTag { name } if name == "html" => self.insertion_mode = InsertionMode::AfterAfterFrameset,
            _ => {}
        }
    }
    fn process_after_after_body(&mut self, _token: &HtmlToken) {}
    fn process_after_after_frameset(&mut self, _token: &HtmlToken) {}
    fn process_plaintext(&mut self, token: &HtmlToken) {
        if let HtmlToken::Character(c) = token {
            self.insert_character(*c);
        }
    }

    // --- DOM Construction Helpers ---

    pub fn insert_element_with_token(&mut self, token: &HtmlToken) {
        if let HtmlToken::StartTag { name, attributes, .. } = token {
            let mut el_data = xiaopeng_dom::ElementData::new(name.clone());
            for attr in attributes {
                el_data.set_attribute(attr.name.clone(), attr.value.clone());
            }
            
            let new_node = xiaopeng_dom::Node::new(xiaopeng_dom::NodeData::Element(el_data));
            
            if let Some(parent) = self.open_elements.last() {
                xiaopeng_dom::Node::append_child(parent, &new_node);
            }
            
            self.open_elements.push(new_node);
        }
    }

    pub fn insert_html_element(&mut self) {
        let el_data = xiaopeng_dom::ElementData::new("html".into());
        let new_node = xiaopeng_dom::Node::new(xiaopeng_dom::NodeData::Element(el_data));
        if let Some(parent) = self.open_elements.last() {
            xiaopeng_dom::Node::append_child(parent, &new_node);
        }
        self.open_elements.push(new_node);
    }
    
    pub fn insert_head_element(&mut self) {
        let el_data = xiaopeng_dom::ElementData::new("head".into());
        let new_node = xiaopeng_dom::Node::new(xiaopeng_dom::NodeData::Element(el_data));
        if let Some(parent) = self.open_elements.last() {
            xiaopeng_dom::Node::append_child(parent, &new_node);
        }
        self.head_element = Some(new_node.clone());
        self.open_elements.push(new_node);
    }

    pub fn insert_body_element(&mut self) {
        let el_data = xiaopeng_dom::ElementData::new("body".into());
        let new_node = xiaopeng_dom::Node::new(xiaopeng_dom::NodeData::Element(el_data));
        if let Some(parent) = self.open_elements.last() {
            xiaopeng_dom::Node::append_child(parent, &new_node);
        }
        self.open_elements.push(new_node);
    }

    pub fn insert_character(&mut self, c: char) {
        if let Some(parent) = self.open_elements.last() {
            let last_child = parent.read().unwrap().last_child();
            let mut appended = false;
            if let Some(lc) = last_child {
                let mut node = lc.write().unwrap();
                if let xiaopeng_dom::NodeData::Text(ref mut t) = node.data {
                    t.push(c);
                    appended = true;
                }
            }
            if !appended {
                let new_node = xiaopeng_dom::Node::new(xiaopeng_dom::NodeData::Text(c.to_string()));
                xiaopeng_dom::Node::append_child(parent, &new_node);
            }
        }
    }

    pub fn insert_comment(&mut self, token: &HtmlToken) {
        if let HtmlToken::Comment(data) = token {
            let new_node = xiaopeng_dom::Node::new(xiaopeng_dom::NodeData::Comment(data.clone()));
            if let Some(parent) = self.open_elements.last() {
                xiaopeng_dom::Node::append_child(parent, &new_node);
            }
        }
    }
    
    pub fn pop_until_element(&mut self, name: &str) {
        let mut pop_count = 0;
        let mut found = false;
        for node in self.open_elements.iter().rev() {
            pop_count += 1;
            if let xiaopeng_dom::NodeData::Element(ref el) = node.read().unwrap().data {
                if el.tag_name == name {
                    found = true;
                    break;
                }
            }
        }
        if found {
            for _ in 0..pop_count {
                self.open_elements.pop();
            }
        }
    }

    pub fn insert_element(&mut self, _tag_name: &str) {
        debug!("Inserting element: {}", _tag_name);
    }
    
    fn is_void_element(name: &str) -> bool {
        matches!(
            name,
            "area" | "base" | "br" | "col" | "embed" | "hr" | "img" | "input" | "link" | "meta" | "param" | "source" | "track" | "wbr"
        )
    }
}

impl Default for HtmlTreeBuilder {
    fn default() -> Self {
        Self::new()
    }
}
