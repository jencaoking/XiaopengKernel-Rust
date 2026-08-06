//! XiaopengKernel HTML Parser Module

pub mod html;

pub use html::{HtmlToken, HtmlTokenizer, HtmlTreeBuilder};
use tracing::{info, instrument};
use xiaopeng_common::XiaopengResult;
use xiaopeng_dom::Document;

#[instrument(skip(input), fields(input_len = input.len()))]
pub fn parse_html(input: &str) -> XiaopengResult<Document> {
    info!("Parsing HTML input");
    let mut tokenizer = HtmlTokenizer::new();
    tokenizer.push_chunk(input);
    tokenizer.end_of_file();
    let mut tree_builder = HtmlTreeBuilder::new();
    
    while let Ok(Some(token)) = tokenizer.next_token() {
        let is_eof = token == HtmlToken::Eof;
        tree_builder.process_token(token);
        if is_eof {
            break;
        }
    }
    
    Ok(tree_builder.document)
}



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_html() {
        let html_input = "<html><head><title>Test</title></head><body><div id='app'>Hello</div></body></html>";
        let res = parse_html(html_input);
        assert!(res.is_ok());
        let doc = res.expect("Unwrap failed");
        
        let root = doc.root.read().expect("Lock poisoned");
        assert_eq!(root.child_element_count(), 1, "Document should have 1 child element (<html>)");
        
        let html_node = root.first_element_child().expect("Unwrap failed");
        if let xiaopeng_dom::NodeData::Element(ref el) = html_node.read().expect("Lock poisoned").data {
            assert_eq!(el.tag_name, "html");
        } else {
            panic!("Expected html element");
        }
        
        assert_eq!(html_node.read().expect("Lock poisoned").child_element_count(), 2, "<html> should have <head> and <body>");
        
        let head_node = html_node.read().expect("Lock poisoned").first_element_child().expect("Unwrap failed");
        let body_node = html_node.read().expect("Lock poisoned").last_element_child().expect("Unwrap failed");
        
        if let xiaopeng_dom::NodeData::Element(ref el) = head_node.read().expect("Lock poisoned").data {
            assert_eq!(el.tag_name, "head");
        }
        
        if let xiaopeng_dom::NodeData::Element(ref el) = body_node.read().expect("Lock poisoned").data {
            assert_eq!(el.tag_name, "body");
        }
        
        let div_node = body_node.read().expect("Lock poisoned").first_element_child().expect("Unwrap failed");
        assert_eq!(
            div_node.read().expect("Lock poisoned").node_type(),
            xiaopeng_dom::NodeType::Element
        );
        let tag_name;
        let id_value;
        {
            let guard = div_node.read().expect("Lock poisoned");
            if let xiaopeng_dom::NodeData::Element(ref el) = guard.data {
                tag_name = el.tag_name.clone();
                id_value = el.id().cloned();
            } else {
                panic!("Expected div");
            }
        }
        
        assert_eq!(tag_name, "div");
        assert_eq!(id_value, Some("app".to_string()));
    }
}
