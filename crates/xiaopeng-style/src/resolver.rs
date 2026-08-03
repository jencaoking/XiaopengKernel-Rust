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

        // Apply normal declarations
        for (rule, _, _) in &matched_rules {
            for decl in &rule.declarations {
                if !decl.important {
                    self.apply_declaration(&mut computed, decl);
                }
            }
        }
        
        // Apply important declarations
        for (rule, _, _) in &matched_rules {
            for decl in &rule.declarations {
                if decl.important {
                    self.apply_declaration(&mut computed, decl);
                }
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

            if !self.matches_simple_selector(curr, part) {
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

    pub fn apply_declaration_pub(&self, style: &mut ComputedStyle, decl: &Declaration) {
        self.apply_declaration(style, decl);
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
                if let Some(c) = parse_color(&decl.value) {
                    style.color = c;
                }
            }
            "background-color" | "background" => {
                if let Some(c) = parse_color(&decl.value) {
                    style.background_color = c;
                }
            }
            "border-color" => {
                if let Some(c) = parse_color(&decl.value) {
                    style.border_color = c;
                }
            }
            "width" => {
                if let Some(v) = parse_length(&decl.value) { style.width = v; }
            }
            "height" => {
                if let Some(v) = parse_length(&decl.value) { style.height = v; }
            }
            "position" => {
                style.position = match decl.value.as_str() {
                    "absolute" => crate::computed_style::Position::Absolute,
                    "relative" => crate::computed_style::Position::Relative,
                    "fixed" => crate::computed_style::Position::Fixed,
                    "sticky" => crate::computed_style::Position::Sticky,
                    _ => crate::computed_style::Position::Static,
                };
            }
            "top" => {
                if let Some(v) = parse_length(&decl.value) { style.top = v; }
            }
            "bottom" => {
                if let Some(v) = parse_length(&decl.value) { style.bottom = v; }
            }
            "left" => {
                if let Some(v) = parse_length(&decl.value) { style.left = v; }
            }
            "right" => {
                if let Some(v) = parse_length(&decl.value) { style.right = v; }
            }
            "z-index" => {
                if let Ok(v) = decl.value.parse::<i32>() {
                    style.z_index = v;
                }
            }
            "font-size" => {
                // Font size should resolve to px
                if let Some(v) = parse_length(&decl.value) {
                    if let crate::computed_style::CssLength::Px(px) = v {
                        style.font_size = px;
                    }
                }
            }
            "margin" => {
                if let Some(v) = parse_length(&decl.value) {
                    style.margin_top = v;
                    style.margin_bottom = v;
                    style.margin_left = v;
                    style.margin_right = v;
                }
            }
            "margin-top" => { if let Some(v) = parse_length(&decl.value) { style.margin_top = v; } }
            "margin-bottom" => { if let Some(v) = parse_length(&decl.value) { style.margin_bottom = v; } }
            "margin-left" => { if let Some(v) = parse_length(&decl.value) { style.margin_left = v; } }
            "margin-right" => { if let Some(v) = parse_length(&decl.value) { style.margin_right = v; } }
            "padding" => {
                if let Some(v) = parse_length(&decl.value) {
                    style.padding_top = v;
                    style.padding_bottom = v;
                    style.padding_left = v;
                    style.padding_right = v;
                }
            }
            "padding-top" => { if let Some(v) = parse_length(&decl.value) { style.padding_top = v; } }
            "padding-bottom" => { if let Some(v) = parse_length(&decl.value) { style.padding_bottom = v; } }
            "padding-left" => { if let Some(v) = parse_length(&decl.value) { style.padding_left = v; } }
            "padding-right" => { if let Some(v) = parse_length(&decl.value) { style.padding_right = v; } }
            "border-width" => {
                if let Some(v) = parse_length(&decl.value) {
                    style.border_top_width = v;
                    style.border_bottom_width = v;
                    style.border_left_width = v;
                    style.border_right_width = v;
                }
            }
            "border" => {
                // Extremely simple border parsing: "1px solid red"
                let parts: Vec<&str> = decl.value.split_whitespace().collect();
                if let Some(w) = parts.iter().find_map(|p| parse_length(p)) {
                    style.border_top_width = w;
                    style.border_bottom_width = w;
                    style.border_left_width = w;
                    style.border_right_width = w;
                }
                if let Some(c) = parts.iter().find_map(|p| parse_color(p)) {
                    style.border_color = c;
                }
            }
            _ => {}
        }
    }
}

fn parse_length(val: &str) -> Option<crate::computed_style::CssLength> {
    use crate::computed_style::CssLength;
    if val == "auto" {
        Some(CssLength::Auto)
    } else if val == "inherit" {
        Some(CssLength::Inherit)
    } else if val == "0" {
        Some(CssLength::Px(0.0))
    } else if val.ends_with("px") {
        val.trim_end_matches("px").parse::<f32>().ok().map(CssLength::Px)
    } else if val.ends_with("%") {
        val.trim_end_matches("%").parse::<f32>().ok().map(CssLength::Percent)
    } else if val.ends_with("em") {
        val.trim_end_matches("em").parse::<f32>().ok().map(CssLength::Em)
    } else if val.ends_with("rem") {
        val.trim_end_matches("rem").parse::<f32>().ok().map(CssLength::Rem)
    } else if val.ends_with("vh") {
        val.trim_end_matches("vh").parse::<f32>().ok().map(CssLength::Vh)
    } else if val.ends_with("vw") {
        val.trim_end_matches("vw").parse::<f32>().ok().map(CssLength::Vw)
    } else {
        None
    }
}

fn parse_color(val: &str) -> Option<Color> {
    let val = val.trim();
    match val {
        "red" => Some(Color::rgb(255, 0, 0)),
        "blue" => Some(Color::rgb(0, 0, 255)),
        "green" => Some(Color::rgb(0, 255, 0)),
        "black" => Some(Color::rgb(0, 0, 0)),
        "white" => Some(Color::rgb(255, 255, 255)),
        "transparent" => Some(Color::rgba(0, 0, 0, 0)),
        _ if val.starts_with('#') => {
            let hex = &val[1..];
            if hex.len() == 6 || hex.len() == 8 {
                let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
                let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
                let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
                let a = if hex.len() == 8 { u8::from_str_radix(&hex[6..8], 16).ok()? } else { 255 };
                Some(Color::rgba(r, g, b, a))
            } else if hex.len() == 3 {
                let r = u8::from_str_radix(&hex[0..1], 16).ok()?;
                let g = u8::from_str_radix(&hex[1..2], 16).ok()?;
                let b = u8::from_str_radix(&hex[2..3], 16).ok()?;
                Some(Color::rgb(r * 17, g * 17, b * 17))
            } else {
                None
            }
        }
        _ if val.starts_with("rgb(") || val.starts_with("rgba(") => {
            let inner = val.split('(').nth(1)?.trim_end_matches(')');
            let parts: Vec<&str> = inner.split(',').map(|s| s.trim()).collect();
            if parts.len() >= 3 {
                let r = parts[0].parse::<u8>().ok()?;
                let g = parts[1].parse::<u8>().ok()?;
                let b = parts[2].parse::<u8>().ok()?;
                let a = if parts.len() == 4 { (parts[3].parse::<f32>().ok()? * 255.0) as u8 } else { 255 };
                Some(Color::rgba(r, g, b, a))
            } else {
                None
            }
        }
        _ => None,
    }
}
