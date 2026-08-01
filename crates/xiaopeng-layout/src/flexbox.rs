//! Flexbox Layout Algorithm via Taffy
use crate::layout_box::LayoutBox;
use taffy::prelude::*;
use xiaopeng_style::computed_style::Display as KernelDisplay;

pub fn layout_flex(node: &mut LayoutBox) {
    let mut taffy = TaffyTree::new();
    
    // 1. Build Taffy Tree from our LayoutBox tree
    let root_node = build_taffy_tree(&mut taffy, node);

    // 2. Compute Layout
    let available_space = Size { width: AvailableSpace::Definite(1024.0), height: AvailableSpace::MaxContent };
    taffy.compute_layout(root_node, available_space).unwrap();

    // 3. Sync layout results back to LayoutBox
    sync_taffy_layout(&taffy, root_node, node);
}

fn build_taffy_tree(taffy: &mut TaffyTree, lbox: &LayoutBox) -> NodeId {
    let mut style = Style::DEFAULT;
    
    style.display = match lbox.style.display {
        KernelDisplay::Flex => Display::Flex,
        KernelDisplay::Grid => Display::Grid,
        KernelDisplay::None => Display::None,
        _ => Display::Block,
    };
    
    if let Some(w) = lbox.style.width {
        style.size.width = Dimension::length(w);
    }
    
    if let Some(h) = lbox.style.height {
        style.size.height = Dimension::length(h);
    }
    
    // Recursively build children
    let mut child_nodes = Vec::new();
    for child in &lbox.children {
        child_nodes.push(build_taffy_tree(taffy, child));
    }
    
    taffy.new_with_children(style, &child_nodes).unwrap()
}

fn sync_taffy_layout(taffy: &TaffyTree, node_id: NodeId, lbox: &mut LayoutBox) {
    let layout = taffy.layout(node_id).unwrap();
    
    // Copy computed geometry back
    lbox.dimensions.content.x = layout.location.x;
    lbox.dimensions.content.y = layout.location.y;
    lbox.dimensions.content.width = layout.size.width;
    lbox.dimensions.content.height = layout.size.height;
    
    // Recursively sync children
    let child_ids = taffy.children(node_id).unwrap();
    for (i, child) in lbox.children.iter_mut().enumerate() {
        sync_taffy_layout(taffy, child_ids[i], child);
    }
}
