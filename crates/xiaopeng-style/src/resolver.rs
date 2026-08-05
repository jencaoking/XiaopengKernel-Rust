//! Style Resolver

use crate::computed_style::{ComputedStyle, Display};
use crate::parser::{Declaration, StyleSheet};
use xiaopeng_common::Color;
use xiaopeng_dom::NodePtr;

pub struct StyleResolver<'a> {
    pub stylesheet: &'a StyleSheet,
}

impl<'a> StyleResolver<'a> {
    pub fn new(stylesheet: &'a StyleSheet) -> Self {
        Self { stylesheet }
    }

    pub fn resolve_style(
        &self, 
        node: &NodePtr,
        parent_style: Option<&ComputedStyle>,
        root_font_size: f32,
        viewport_width: f32,
        viewport_height: f32,
    ) -> ComputedStyle {
        let mut computed = ComputedStyle::default();
        if let Some(parent) = parent_style {
            // Inherit inheritable properties
            computed.color = parent.color;
            computed.font_size = parent.font_size;
            // (other inheritable properties can be added here)
        }
        let mut matched_rules = Vec::new();

        for (rule_idx, rule) in self.stylesheet.rules.iter().enumerate() {
            for selector in &rule.selectors {
                if crate::query::matches_selector(node, selector) {
                    matched_rules.push((rule, selector.specificity(), rule_idx));
                }
            }
        }

        matched_rules
    }

    pub fn get_matched_rules(&self, node: &NodePtr) -> Vec<(&'a crate::parser::Rule, u32, usize)> {
        let mut matched_rules = Vec::new();

        for (rule_idx, rule) in self.stylesheet.rules.iter().enumerate() {
            for selector in &rule.selectors {
                if crate::query::matches_selector(node, selector) {
                    matched_rules.push((rule, selector.specificity(), rule_idx));
                }
            }
        }

        matched_rules.sort_by(|a, b| {
            if a.1 == b.1 {
                a.2.cmp(&b.2)
            } else {
                a.1.cmp(&b.1)
            }
        });
        
        matched_rules
    }

    // Apply normal declarations
        for (rule, _, _) in &matched_rules {
            for decl in &rule.declarations {
                if !decl.important {
                    self.apply_declaration(&mut computed, decl, parent_style.map_or(16.0, |p| p.font_size), root_font_size, viewport_width, viewport_height);
                }
            }
        }
        
        // Apply important declarations
        for (rule, _, _) in &matched_rules {
            for decl in &rule.declarations {
                if decl.important {
                    self.apply_declaration(&mut computed, decl, parent_style.map_or(16.0, |p| p.font_size), root_font_size, viewport_width, viewport_height);
                }
            }
        }

        computed.resolve_relative_units(computed.font_size, root_font_size, viewport_width, viewport_height);
        computed
    }



    pub fn apply_declaration_pub(&self, style: &mut ComputedStyle, decl: &Declaration, parent_font_size: f32, root_font_size: f32, viewport_width: f32, viewport_height: f32) {
        self.apply_declaration(style, decl, parent_font_size, root_font_size, viewport_width, viewport_height);
    }

    fn apply_declaration(
        &self, 
        style: &mut ComputedStyle, 
        decl: &Declaration,
        parent_font_size: f32,
        root_font_size: f32,
        viewport_width: f32,
        viewport_height: f32,
    ) {
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
                if let Some(v) = parse_length(&decl.value) {
                    match v {
                        crate::computed_style::CssLength::Px(px) => style.font_size = px,
                        crate::computed_style::CssLength::Em(em) => style.font_size = em * parent_font_size,
                        crate::computed_style::CssLength::Rem(rem) => style.font_size = rem * root_font_size,
                        crate::computed_style::CssLength::Vh(vh) => style.font_size = vh * viewport_height / 100.0,
                        crate::computed_style::CssLength::Vw(vw) => style.font_size = vw * viewport_width / 100.0,
                        crate::computed_style::CssLength::Percent(p) => style.font_size = p * parent_font_size / 100.0,
                        _ => {}
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
            "flex-direction" => {
                use crate::computed_style::FlexDirection;
                style.flex_direction = match decl.value.as_str() {
                    "row-reverse" => FlexDirection::RowReverse,
                    "column" => FlexDirection::Column,
                    "column-reverse" => FlexDirection::ColumnReverse,
                    _ => FlexDirection::Row,
                };
            }
            "flex-wrap" => {
                use crate::computed_style::FlexWrap;
                style.flex_wrap = match decl.value.as_str() {
                    "wrap" => FlexWrap::Wrap,
                    "wrap-reverse" => FlexWrap::WrapReverse,
                    _ => FlexWrap::Nowrap,
                };
            }
            "justify-content" => {
                use crate::computed_style::JustifyContent;
                style.justify_content = match decl.value.as_str() {
                    "flex-end" => JustifyContent::FlexEnd,
                    "center" => JustifyContent::Center,
                    "space-between" => JustifyContent::SpaceBetween,
                    "space-around" => JustifyContent::SpaceAround,
                    "space-evenly" => JustifyContent::SpaceEvenly,
                    _ => JustifyContent::FlexStart,
                };
            }
            "align-items" => {
                use crate::computed_style::AlignItems;
                style.align_items = match decl.value.as_str() {
                    "flex-start" => AlignItems::FlexStart,
                    "flex-end" => AlignItems::FlexEnd,
                    "center" => AlignItems::Center,
                    "baseline" => AlignItems::Baseline,
                    _ => AlignItems::Stretch,
                };
            }
            "flex-grow" => {
                if let Ok(v) = decl.value.parse::<f32>() {
                    style.flex_grow = v;
                }
            }
            "flex-shrink" => {
                if let Ok(v) = decl.value.parse::<f32>() {
                    style.flex_shrink = v;
                }
            }
            "flex-basis" => {
                if let Some(v) = parse_length(&decl.value) {
                    style.flex_basis = v;
                }
            }
            "flex" => {
                // Shorthand for flex: flex-grow flex-shrink flex-basis
                // For simplicity, just handle a single number as flex-grow: e.g. `flex: 1`
                let parts: Vec<&str> = decl.value.split_whitespace().collect();
                if let Some(p1) = parts.get(0) {
                    if let Ok(v) = p1.parse::<f32>() {
                        style.flex_grow = v;
                        style.flex_shrink = 1.0;
                        style.flex_basis = crate::computed_style::CssLength::Px(0.0);
                    }
                }
            }
            "grid-template-columns" => {
                style.grid_template_columns = parse_grid_tracks(&decl.value);
            }
            "grid-template-rows" => {
                style.grid_template_rows = parse_grid_tracks(&decl.value);
            }
            "grid-column-start" => { if let Ok(v) = decl.value.parse::<i32>() { style.grid_column_start = Some(v); } }
            "grid-column-end" => { if let Ok(v) = decl.value.parse::<i32>() { style.grid_column_end = Some(v); } }
            "grid-row-start" => { if let Ok(v) = decl.value.parse::<i32>() { style.grid_row_start = Some(v); } }
            "grid-row-end" => { if let Ok(v) = decl.value.parse::<i32>() { style.grid_row_end = Some(v); } }
            "grid-column" => {
                let parts: Vec<&str> = decl.value.split('/').collect();
                if parts.len() > 0 { if let Ok(v) = parts[0].trim().parse::<i32>() { style.grid_column_start = Some(v); } }
                if parts.len() > 1 { if let Ok(v) = parts[1].trim().parse::<i32>() { style.grid_column_end = Some(v); } }
            }
            "grid-row" => {
                let parts: Vec<&str> = decl.value.split('/').collect();
                if parts.len() > 0 { if let Ok(v) = parts[0].trim().parse::<i32>() { style.grid_row_start = Some(v); } }
                if parts.len() > 1 { if let Ok(v) = parts[1].trim().parse::<i32>() { style.grid_row_end = Some(v); } }
            }
            "gap" => {
                if let Some(v) = parse_length(&decl.value) { style.gap = v; }
            }
            _ => {}
        }
    }
}

fn parse_grid_tracks(val: &str) -> Vec<crate::computed_style::GridTrackSize> {
    use crate::computed_style::GridTrackSize;
    let mut tracks = Vec::new();
    for part in val.split_whitespace() {
        if part == "auto" {
            tracks.push(GridTrackSize::Auto);
        } else if part.ends_with("fr") {
            if let Ok(v) = part.trim_end_matches("fr").parse::<f32>() {
                tracks.push(GridTrackSize::Fraction(v));
            }
        } else if let Some(len) = parse_length(part) {
            tracks.push(GridTrackSize::Length(len));
        }
    }
    tracks
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
