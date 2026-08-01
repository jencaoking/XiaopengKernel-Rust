//! XiaopengKernel HTML & CSS Parser Module

pub mod css;
pub mod html;

pub use css::{CssRule, StyleSheet};
pub use html::{HtmlToken, HtmlTokenizer, HtmlTreeBuilder};
use tracing::info;
use xiaopeng_common::XiaopengResult;
use xiaopeng_dom::Document;

pub fn parse_html(_input: &str) -> XiaopengResult<Document> {
    info!("Parsing HTML input");
    let tree_builder = HtmlTreeBuilder::new();
    Ok(tree_builder.document)
}

pub fn parse_css(_input: &str) -> XiaopengResult<StyleSheet> {
    info!("Parsing CSS input");
    Ok(StyleSheet::default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_html() {
        let res = parse_html("<html></html>");
        assert!(res.is_ok());
    }

    #[test]
    fn test_parse_css() {
        let res = parse_css("body { color: red; }");
        assert!(res.is_ok());
    }
}
