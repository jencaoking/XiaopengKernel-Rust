//! XiaopengKernel HTML & CSS Parser Module

use tracing::info;
use xiaopeng_common::XiaopengResult;
use xiaopeng_dom::Document;

pub fn parse_html(_input: &str) -> XiaopengResult<Document> {
    info!("Parsing HTML input");
    Ok(Document::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_html() {
        let res = parse_html("<html></html>");
        assert!(res.is_ok());
    }
}
