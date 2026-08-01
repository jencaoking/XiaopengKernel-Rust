//! Style Resolver

use crate::computed_style::ComputedStyle;
use xiaopeng_dom::NodePtr;
use xiaopeng_parser::StyleSheet;

pub struct StyleResolver<'a> {
    pub _stylesheet: &'a StyleSheet,
}

impl<'a> StyleResolver<'a> {
    pub fn new(stylesheet: &'a StyleSheet) -> Self {
        Self {
            _stylesheet: stylesheet,
        }
    }

    pub fn resolve_style(&self, _node: &NodePtr) -> ComputedStyle {
        ComputedStyle::default()
    }
}
