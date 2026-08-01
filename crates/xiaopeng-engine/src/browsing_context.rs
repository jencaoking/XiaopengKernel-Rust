//! Browsing Context & Frame management

use xiaopeng_dom::Document;

pub struct BrowsingContext {
    pub document: Option<Document>,
}

impl BrowsingContext {
    pub fn new() -> Self {
        Self { document: None }
    }
}

impl Default for BrowsingContext {
    fn default() -> Self {
        Self::new()
    }
}
