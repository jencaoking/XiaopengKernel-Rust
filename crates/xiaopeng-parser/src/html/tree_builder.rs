//! HTML Tree Builder Insertion Modes Stubs

use xiaopeng_dom::Document;

pub struct HtmlTreeBuilder {
    pub document: Document,
}

impl HtmlTreeBuilder {
    pub fn new() -> Self {
        Self {
            document: Document::new(),
        }
    }
}

impl Default for HtmlTreeBuilder {
    fn default() -> Self {
        Self::new()
    }
}
