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
        
        // --- Simplified DOM Construction ---
        match &token {
            HtmlToken::StartTag { name, self_closing, attributes } => {
                let mut el_data = xiaopeng_dom::ElementData::new(name.clone());
                for attr in attributes {
                    el_data.set_attribute(attr.name.clone(), attr.value.clone());
                }
                
                let new_node = xiaopeng_dom::Node::new(xiaopeng_dom::NodeData::Element(el_data));
                
                if let Some(parent) = self.open_elements.last() {
                    xiaopeng_dom::Node::append_child(parent, &new_node);
                }
                
                if !self_closing && !Self::is_void_element(name) {
                    self.open_elements.push(new_node);
                }
            }
            HtmlToken::EndTag { name } => {
                // Find the matching tag from the bottom of the stack (reverse order)
                let mut pop_count = 0;
                let mut found = false;
                for node in self.open_elements.iter().rev() {
                    pop_count += 1;
                    if let xiaopeng_dom::NodeData::Element(ref el) = node.read().unwrap().data {
                        if el.tag_name == *name {
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
            HtmlToken::Character(c) => {
                if !c.is_whitespace() || self.insertion_mode == InsertionMode::InBody {
                    if let Some(parent) = self.open_elements.last() {
                        // Check if the last child is a text node, if so append, else create new
                        let last_child = parent.read().unwrap().last_child();
                        let mut appended = false;
                        if let Some(lc) = last_child {
                            let mut node = lc.write().unwrap();
                            if let xiaopeng_dom::NodeData::Text(ref mut t) = node.data {
                                t.push(*c);
                                appended = true;
                            }
                        }
                        if !appended {
                            let new_node = xiaopeng_dom::Node::new(xiaopeng_dom::NodeData::Text(c.to_string()));
                            xiaopeng_dom::Node::append_child(parent, &new_node);
                        }
                    }
                }
            }
            HtmlToken::Comment(data) => {
                let new_node = xiaopeng_dom::Node::new(xiaopeng_dom::NodeData::Comment(data.clone()));
                if let Some(parent) = self.open_elements.last() {
                    xiaopeng_dom::Node::append_child(parent, &new_node);
                }
            }
            _ => {}
        }
        
        // Mode transition simulation
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

    fn process_initial(&mut self, _token: &HtmlToken) {
        // Parse doctype, etc.
        self.insertion_mode = InsertionMode::BeforeHtml;
    }

    fn process_before_html(&mut self, _token: &HtmlToken) {
        self.insertion_mode = InsertionMode::BeforeHead;
    }

    fn process_before_head(&mut self, _token: &HtmlToken) {
        self.insertion_mode = InsertionMode::InHead;
    }

    fn process_in_head(&mut self, _token: &HtmlToken) {
        self.insertion_mode = InsertionMode::AfterHead;
    }

    fn process_in_head_noscript(&mut self, _token: &HtmlToken) {
        self.insertion_mode = InsertionMode::InHead;
    }

    fn process_after_head(&mut self, _token: &HtmlToken) {
        self.insertion_mode = InsertionMode::InBody;
    }

    fn process_in_body(&mut self, _token: &HtmlToken) {
        // Handles most elements
    }

    fn process_text(&mut self, _token: &HtmlToken) {}
    fn process_in_table(&mut self, _token: &HtmlToken) {}
    fn process_in_table_text(&mut self, _token: &HtmlToken) {}
    fn process_in_caption(&mut self, _token: &HtmlToken) {}
    fn process_in_column_group(&mut self, _token: &HtmlToken) {}
    fn process_in_table_body(&mut self, _token: &HtmlToken) {}
    fn process_in_row(&mut self, _token: &HtmlToken) {}
    fn process_in_cell(&mut self, _token: &HtmlToken) {}
    fn process_in_select(&mut self, _token: &HtmlToken) {}
    fn process_in_select_in_table(&mut self, _token: &HtmlToken) {}
    fn process_in_template(&mut self, _token: &HtmlToken) {}
    fn process_after_body(&mut self, _token: &HtmlToken) {}
    fn process_in_frameset(&mut self, _token: &HtmlToken) {}
    fn process_after_frameset(&mut self, _token: &HtmlToken) {}
    fn process_after_after_body(&mut self, _token: &HtmlToken) {}
    fn process_after_after_frameset(&mut self, _token: &HtmlToken) {}
    fn process_plaintext(&mut self, _token: &HtmlToken) {}

    // --- DOM Construction Helpers (Stubs) ---

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
