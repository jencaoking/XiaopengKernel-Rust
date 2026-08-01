//! Style Resolver

use crate::computed_style::{ComputedStyle, Display};
use crate::parser::{Declaration, StyleSheet};
use crate::selector::{Combinator, Selector, SelectorType, SimpleSelector};
use std::sync::Arc;
use xiaopeng_common::Color;
use xiaopeng_dom::{NodeData, NodePtr};

pub struct StyleResolver<'a> {
    pub stylesheet: &'a StyleSheet,
}

impl<'a> StyleResolver<'a> {
    pub fn new(stylesheet: &'a StyleSheet) -> Self {
        Self { stylesheet }
    }

    pub fn resolve_style(&self, node: &NodePtr) -> ComputedStyle {
        let mut computed = ComputedStyle::default();
        let mut matched_rules = Vec::new();

        for (rule_idx, rule) in self.stylesheet.rules.iter().enumerate() {
            for selector in &rule.selectors {
                if self.matches_selector(node, selector) {
                    matched_rules.push((rule, selector.specificity(), rule_idx));
                }
            }
        }

        // Sort rules by specificity, then by source order
        matched_rules.sort_by(|a, b| {
            if a.1 == b.1 {
                a.2.cmp(&b.2)
            } else {
                a.1.cmp(&b.1)
            }
        });

        // Apply declarations
        for (rule, _, _) in matched_rules {
            for decl in &rule.declarations {
                self.apply_declaration(&mut computed, decl);
            }
        }

        computed
    }

    fn matches_selector(&self, node: &NodePtr, selector: &Selector) -> bool {
        if selector.parts.is_empty() {
            return false;
        }

        // Match from right to left (rightmost part must match current node)
        let mut current_node = Some(Arc::clone(node));
        let mut part_idx = selector.parts.len() as isize - 1;

        while part_idx >= 0 {
            let part = &selector.parts[part_idx as usize];
            let Some(ref curr) = current_node else { return false; };

            if !self.matches_simple_selector(&curr, part) {
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

    fn matches_simple_selector(&self, node: &NodePtr, part: &SimpleSelector) -> bool {
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

    fn apply_declaration(&self, style: &mut ComputedStyle, decl: &Declaration) {
        match decl.property.as_str() {
            "display" => {
                style.display = match decl.value.as_str() {
                    "none" => Display::None,
                    "inline" => Display::Inline,
                    "flex" => Display::Flex,
                    "grid" => Display::Grid,
                    _ => Display::Block,
                };
            }
            "color" => {
                if decl.value == "red" {
                    style.color = Color { r: 255, g: 0, b: 0, a: 255 };
                } else if decl.value == "blue" {
                    style.color = Color { r: 0, g: 0, b: 255, a: 255 };
                }
            }
            "width" => {
                if decl.value.ends_with("px") {
                    if let Ok(v) = decl.value.trim_end_matches("px").parse::<f32>() {
                        style.width = Some(v);
                    }
                }
            }
            "height" => {
                if decl.value.ends_with("px") {
                    if let Ok(v) = decl.value.trim_end_matches("px").parse::<f32>() {
                        style.height = Some(v);
                    }
                }
            }
            _ => {}
        }
    }
}
