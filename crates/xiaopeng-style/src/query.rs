use std::sync::Arc;
use xiaopeng_dom::{NodeData, NodePtr};
use crate::selector::{Combinator, Selector, SelectorType, SimpleSelector};
use crate::parser::CssParser;

pub fn matches_simple_selector(node: &NodePtr, part: &SimpleSelector) -> bool {
    let n = node.read().expect("Lock poisoned");
    match &n.data {
        NodeData::Element(el) => match part.selector_type {
            SelectorType::Tag => el.tag_name.eq_ignore_ascii_case(&part.value) || part.value == "*",
            SelectorType::Id => el.id().map(|s| s.as_str()) == Some(&part.value),
            SelectorType::Class => el.classes().contains(&part.value.as_str()),
            SelectorType::Universal => true,
            SelectorType::Attribute => {
                let name = part.attribute_name.as_ref().expect("Unwrap failed");
                if !el.has_attribute(name) { return false; }
                let op = part.attribute_operator.as_ref().expect("Unwrap failed");
                if *op == crate::selector::AttributeOperator::Exists { return true; }
                let val = part.attribute_value.as_ref().expect("Unwrap failed");
                let actual = el.get_attribute(name).expect("Unwrap failed");
                match op {
                    crate::selector::AttributeOperator::Exact => actual == val,
                    crate::selector::AttributeOperator::Includes => actual.split_whitespace().any(|s| s == val),
                    crate::selector::AttributeOperator::DashMatch => actual == val || actual.starts_with(&format!("{}-", val)),
                    crate::selector::AttributeOperator::Prefix => actual.starts_with(val),
                    crate::selector::AttributeOperator::Suffix => actual.ends_with(val),
                    crate::selector::AttributeOperator::Substring => actual.contains(val),
                    _ => false,
                }
            }
            SelectorType::PseudoClass => {
                match part.value.as_str() {
                    "first-child" => {
                        if let Some(p) = n.parent.as_ref().and_then(|w| w.upgrade()) {
                            let p_node = p.read().expect("Lock poisoned");
                            p_node.children.iter().find(|c| matches!(c.read().expect("Lock poisoned").data, NodeData::Element(_)))
                                .map_or(false, |first| NodePtr::ptr_eq(first, node))
                        } else { false }
                    },
                    "last-child" => {
                        if let Some(p) = n.parent.as_ref().and_then(|w| w.upgrade()) {
                            let p_node = p.read().expect("Lock poisoned");
                            p_node.children.iter().rev().find(|c| matches!(c.read().expect("Lock poisoned").data, NodeData::Element(_)))
                                .map_or(false, |last| NodePtr::ptr_eq(last, node))
                        } else { false }
                    },
                    "empty" => n.children.iter().all(|c| {
                        let cn = c.read().expect("Lock poisoned");
                        match &cn.data {
                            NodeData::Element(_) => false,
                            NodeData::Text(t) => t.trim().is_empty(),
                            _ => true,
                        }
                    }),
                    "root" => el.tag_name.eq_ignore_ascii_case("html"),
                    _ => false,
                }
            },
            _ => false,
        },
        _ => false,
    }
}

pub fn matches_selector(node: &NodePtr, selector: &Selector) -> bool {
    if selector.parts.is_empty() { return false; }

    fn get_parent(node: &NodePtr) -> Option<NodePtr> {
        node.read().expect("Lock poisoned").parent.as_ref().and_then(|w| w.upgrade())
    }

    fn get_prev_sibling(node: &NodePtr) -> Option<NodePtr> {
        let p = get_parent(node)?;
        let p_node = p.read().expect("Lock poisoned");
        let mut prev = None;
        for child in &p_node.children {
            if !matches!(child.read().expect("Lock poisoned").data, NodeData::Element(_)) { continue; }
            if NodePtr::ptr_eq(child, node) { return prev; }
            prev = Some(NodePtr::clone_ptr(child));
        }
        None
    }

    fn match_parts(curr: &NodePtr, selector: &Selector, part_idx: isize) -> bool {
        if part_idx < 0 { return true; }
        
        let part = &selector.parts[part_idx as usize];
        
        if part_idx > 0 {
            let comb = selector.combinators[(part_idx - 1) as usize];
            if comb == Combinator::None {
                if !matches_simple_selector(curr, part) { return false; }
                return match_parts(curr, selector, part_idx - 1);
            }
        }
        
        if !matches_simple_selector(curr, part) { return false; }
        if part_idx == 0 { return true; }
        
        let comb = selector.combinators[(part_idx - 1) as usize];
        match comb {
            Combinator::None => unreachable!(),
            Combinator::Child => {
                if let Some(p) = get_parent(curr) {
                    match_parts(&p, selector, part_idx - 1)
                } else { false }
            }
            Combinator::Descendant => {
                let mut p = get_parent(curr);
                while let Some(parent) = p {
                    if match_parts(&parent, selector, part_idx - 1) { return true; }
                    p = get_parent(&parent);
                }
                false
            }
            Combinator::NextSibling => {
                if let Some(prev) = get_prev_sibling(curr) {
                    match_parts(&prev, selector, part_idx - 1)
                } else { false }
            }
            Combinator::SubsequentSibling => {
                let mut prev = get_prev_sibling(curr);
                while let Some(ps) = prev {
                    if match_parts(&ps, selector, part_idx - 1) { return true; }
                    prev = get_prev_sibling(&ps);
                }
                false
            }
        }
    }

    match_parts(node, selector, selector.parts.len() as isize - 1)
}

pub fn query_selector_all(root: &NodePtr, selector_str: &str) -> Vec<NodePtr> {
    let mut parser = CssParser::new(selector_str);
    let selectors = parser.parse_selectors();
    if selectors.is_empty() {
        return Vec::new();
    }
    
    let mut results = Vec::new();
    
    fn traverse(node: &NodePtr, selectors: &[Selector], results: &mut Vec<NodePtr>) {
        // If node is an Element, test against selectors
        let is_element = matches!(node.read().expect("Lock poisoned").data, NodeData::Element(_));
        if is_element {
            for sel in selectors {
                if matches_selector(node, sel) {
                    results.push(NodePtr::clone_ptr(node));
                    break;
                }
            }
        }
        
        let children = node.read().expect("Lock poisoned").children.clone();
        for child in children {
            traverse(&child, selectors, results);
        }
    }
    
    traverse(root, &selectors, &mut results);
    results
}

pub fn query_selector(root: &NodePtr, selector_str: &str) -> Option<NodePtr> {
    let mut parser = CssParser::new(selector_str);
    let selectors = parser.parse_selectors();
    if selectors.is_empty() {
        return None;
    }
    
    fn traverse(node: &NodePtr, selectors: &[Selector]) -> Option<NodePtr> {
        let is_element = matches!(node.read().expect("Lock poisoned").data, NodeData::Element(_));
        if is_element {
            for sel in selectors {
                if matches_selector(node, sel) {
                    return Some(NodePtr::clone_ptr(node));
                }
            }
        }
        
        let children = node.read().expect("Lock poisoned").children.clone();
        for child in children {
            if let Some(found) = traverse(&child, selectors) {
                return Some(found);
            }
        }
        None
    }
    
    traverse(root, &selectors)
}
