//! Flexbox Layout Algorithm via Taffy
use crate::layout_box::LayoutBox;
use taffy::prelude::*;
use xiaopeng_style::computed_style::Display as KernelDisplay;

pub fn layout_flex(node: &mut LayoutBox, offset_x: f32, offset_y: f32) {
    let mut taffy = TaffyTree::new();
    
    // 1. Build Taffy Tree from our LayoutBox tree
    let root_node = build_taffy_tree(&mut taffy, node);

    // 2. Compute Layout
    let available_space = Size { width: AvailableSpace::Definite(1024.0), height: AvailableSpace::MaxContent };
    taffy.compute_layout(root_node, available_space).unwrap();

    // 3. Sync layout results back to LayoutBox
    sync_taffy_layout(&taffy, root_node, node, offset_x, offset_y);
}

fn build_taffy_tree(taffy: &mut TaffyTree, lbox: &LayoutBox) -> NodeId {
    let mut style = Style::DEFAULT;
    
    style.display = match lbox.style.display {
        KernelDisplay::Flex => Display::Flex,
        KernelDisplay::Grid => Display::Grid,
        KernelDisplay::None => Display::None,
        _ => Display::Block,
    };
    
    // Map position
    style.position = match lbox.style.position {
        xiaopeng_style::computed_style::Position::Absolute => Position::Absolute,
        _ => Position::Relative, // Relative in Taffy behaves like static flow if no offsets, but allows offsets.
    };

    if let Some(top) = lbox.style.top.to_px(0.0) {
        style.inset.top = LengthPercentageAuto::length(top);
    }
    if let Some(bottom) = lbox.style.bottom.to_px(0.0) {
        style.inset.bottom = LengthPercentageAuto::length(bottom);
    }
    if let Some(left) = lbox.style.left.to_px(0.0) {
        style.inset.left = LengthPercentageAuto::length(left);
    }
    if let Some(right) = lbox.style.right.to_px(0.0) {
        style.inset.right = LengthPercentageAuto::length(right);
    }
    
    if let Some(w) = lbox.style.width.to_px(0.0) {
        style.size.width = Dimension::length(w);
    }
    
    if let Some(h) = lbox.style.height.to_px(0.0) {
        style.size.height = Dimension::length(h);
    }
    
    // Map Flexbox properties
    style.flex_direction = match lbox.style.flex_direction {
        xiaopeng_style::computed_style::FlexDirection::Row => FlexDirection::Row,
        xiaopeng_style::computed_style::FlexDirection::RowReverse => FlexDirection::RowReverse,
        xiaopeng_style::computed_style::FlexDirection::Column => FlexDirection::Column,
        xiaopeng_style::computed_style::FlexDirection::ColumnReverse => FlexDirection::ColumnReverse,
    };

    style.flex_wrap = match lbox.style.flex_wrap {
        xiaopeng_style::computed_style::FlexWrap::Nowrap => FlexWrap::NoWrap,
        xiaopeng_style::computed_style::FlexWrap::Wrap => FlexWrap::Wrap,
        xiaopeng_style::computed_style::FlexWrap::WrapReverse => FlexWrap::WrapReverse,
    };

    style.justify_content = Some(match lbox.style.justify_content {
        xiaopeng_style::computed_style::JustifyContent::FlexStart => JustifyContent::FLEX_START,
        xiaopeng_style::computed_style::JustifyContent::FlexEnd => JustifyContent::FLEX_END,
        xiaopeng_style::computed_style::JustifyContent::Center => JustifyContent::CENTER,
        xiaopeng_style::computed_style::JustifyContent::SpaceBetween => JustifyContent::SPACE_BETWEEN,
        xiaopeng_style::computed_style::JustifyContent::SpaceAround => JustifyContent::SPACE_AROUND,
        xiaopeng_style::computed_style::JustifyContent::SpaceEvenly => JustifyContent::SPACE_EVENLY,
    });

    style.align_items = Some(match lbox.style.align_items {
        xiaopeng_style::computed_style::AlignItems::Stretch => AlignItems::STRETCH,
        xiaopeng_style::computed_style::AlignItems::FlexStart => AlignItems::FLEX_START,
        xiaopeng_style::computed_style::AlignItems::FlexEnd => AlignItems::FLEX_END,
        xiaopeng_style::computed_style::AlignItems::Center => AlignItems::CENTER,
        xiaopeng_style::computed_style::AlignItems::Baseline => AlignItems::BASELINE,
    });
    
    style.flex_grow = lbox.style.flex_grow;
    style.flex_shrink = lbox.style.flex_shrink;
    
    if let Some(basis) = lbox.style.flex_basis.to_px(0.0) {
        style.flex_basis = Dimension::length(basis);
    } else if lbox.style.flex_basis == xiaopeng_style::computed_style::CssLength::Auto {
        style.flex_basis = Dimension::auto();
    }
    
    // Recursively build children
    let mut child_nodes = Vec::new();
    for child in &lbox.children {
        child_nodes.push(build_taffy_tree(taffy, child));
    }
    
    taffy.new_with_children(style, &child_nodes).unwrap()
}

fn sync_taffy_layout(taffy: &TaffyTree, node_id: NodeId, lbox: &mut LayoutBox, offset_x: f32, offset_y: f32) {
    let layout = taffy.layout(node_id).unwrap();
    
    // Copy computed geometry back
    lbox.dimensions.content.x = offset_x + layout.location.x;
    lbox.dimensions.content.y = offset_y + layout.location.y;
    lbox.dimensions.content.width = layout.size.width;
    lbox.dimensions.content.height = layout.size.height;
    
    // Recursively sync children
    let child_ids = taffy.children(node_id).unwrap();
    for (i, child) in lbox.children.iter_mut().enumerate() {
        sync_taffy_layout(taffy, child_ids[i], child, lbox.dimensions.content.x, lbox.dimensions.content.y);
    }
}
