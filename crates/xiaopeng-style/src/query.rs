use std::sync::Arc;
use xiaopeng_dom::{NodeData, NodePtr};
use crate::selector::{Combinator, Selector, SelectorType, SimpleSelector};
use crate::parser::CssParser;

pub fn matches_simple_selector(node: &NodePtr, part: &SimpleSelector) -> bool {
    let n = node.read().unwrap();
    match &n.data {
        NodeData::Element(el) => match part.selector_type {
            SelectorType::Tag => el.tag_name == part.value,
            SelectorType::Id => el.id().map(|s| s.as_str()) == Some(&part.value),
            SelectorType::Class => el.classes().contains(&part.value.as_str()),
            SelectorType::Universal => true,
            _ => false, // Attributes and pseudo-classes unimplemented in matching stub
        },
        _ => false,
    }
}

pub fn matches_selector(node: &NodePtr, selector: &Selector) -> bool {
    if selector.parts.is_empty() {
        return false;
    }

    // Match from right to left (rightmost part must match current node)
    let mut current_node = Some(Arc::clone(node));
    let mut part_idx = selector.parts.len() as isize - 1;

    while part_idx >= 0 {
        let part = &selector.parts[part_idx as usize];
        let Some(ref curr) = current_node else { return false; };

        if !matches_simple_selector(curr, part) {
            // If it's a descendant combinator, we can ascend the tree looking for a match
            if part_idx < selector.parts.len() as isize - 1 {
                let comb = selector.combinators[part_idx as usize];
                if comb == Combinator::Descendant {
                    let parent = {
                        let n = curr.read().unwrap();
                        n.parent.as_ref().and_then(|w| w.upgrade())
                    };
                    current_node = parent;
                    continue;
                }
            }
            return false;
        }

        if part_idx > 0 {
            let comb = selector.combinators[(part_idx - 1) as usize];
            match comb {
                Combinator::None => {
                    // In valid AST, multiple parts with `None` combinator (like div.class)
                    // apply to the same element. They are usually merged or we just check them all.
                    // Here we just keep `current_node` the same for the next iteration.
                }
                Combinator::Descendant | Combinator::Child => {
                    let parent = {
                        let n = curr.read().unwrap();
                        n.parent.as_ref().and_then(|w| w.upgrade())
                    };
                    current_node = parent;
                }
                Combinator::NextSibling | Combinator::SubsequentSibling => {
                    // Simplification for stubs
                    return false;
                }
            }
        }
        part_idx -= 1;
    }
    true
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
        let is_element = matches!(node.read().unwrap().data, NodeData::Element(_));
        if is_element {
            for sel in selectors {
                if matches_selector(node, sel) {
                    results.push(Arc::clone(node));
                    break;
                }
            }
        }
        
        let children = node.read().unwrap().children.clone();
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
        let is_element = matches!(node.read().unwrap().data, NodeData::Element(_));
        if is_element {
            for sel in selectors {
                if matches_selector(node, sel) {
                    return Some(Arc::clone(node));
                }
            }
        }
        
        let children = node.read().unwrap().children.clone();
        for child in children {
            if let Some(found) = traverse(&child, selectors) {
                return Some(found);
            }
        }
        None
    }
    
    traverse(root, &selectors)
}
