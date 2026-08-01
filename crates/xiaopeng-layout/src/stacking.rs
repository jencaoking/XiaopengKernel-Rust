//! Stacking Contexts and Z-Index Sorting

use crate::layout_box::LayoutBox;

/// A Stacking Context defines the rendering order of elements.
/// It forms a tree independent of the DOM/Layout tree.
#[derive(Debug)]
pub struct StackingContext<'a> {
    pub layout_box: &'a LayoutBox,
    // Elements with negative z-index
    pub negative_z: Vec<StackingContext<'a>>,
    // Elements that don't establish a new stacking context, or have z-index: 0 / auto
    pub zero_z: Vec<&'a LayoutBox>,
    // Elements with positive z-index
    pub positive_z: Vec<StackingContext<'a>>,
}

impl<'a> StackingContext<'a> {
    pub fn new(layout_box: &'a LayoutBox) -> Self {
        Self {
            layout_box,
            negative_z: Vec::new(),
            zero_z: Vec::new(),
            positive_z: Vec::new(),
        }
    }

    /// Builds a stacking context tree starting from the root layout box.
    pub fn build(root: &'a LayoutBox) -> Self {
        let mut context = StackingContext::new(root);
        Self::collect_children(root, &mut context);
        context.sort();
        context
    }

    fn collect_children(current_box: &'a LayoutBox, current_context: &mut StackingContext<'a>) {
        for child in &current_box.children {
            let z = child.style.z_index;
            
            // In a full browser, opacity, transform, etc. also trigger new contexts.
            // For now, if z-index is non-zero, it forms a new stacking context.
            if z != 0 {
                let mut child_context = StackingContext::new(child);
                Self::collect_children(child, &mut child_context);
                
                if z < 0 {
                    current_context.negative_z.push(child_context);
                } else {
                    current_context.positive_z.push(child_context);
                }
            } else {
                // Same context
                current_context.zero_z.push(child);
                Self::collect_children(child, current_context);
            }
        }
    }

    /// Sorts the negative and positive vectors by z-index
    fn sort(&mut self) {
        self.negative_z.sort_by_key(|c| c.layout_box.style.z_index);
        self.positive_z.sort_by_key(|c| c.layout_box.style.z_index);

        for child in &mut self.negative_z {
            child.sort();
        }
        for child in &mut self.positive_z {
            child.sort();
        }
    }

    /// Flattens the stacking context into a linear painting order (Display List)
    pub fn flatten(&self) -> Vec<&'a LayoutBox> {
        let mut list = Vec::new();
        
        // 1. Background and borders of the element forming the context (represented by the box itself)
        list.push(self.layout_box);

        // 2. Child stacking contexts with negative z-index
        for child in &self.negative_z {
            list.extend(child.flatten());
        }

        // 3. In-flow, non-positioned, block-level elements & zero/auto z-index
        list.extend(self.zero_z.iter().copied());

        // 4. Child stacking contexts with positive z-index
        for child in &self.positive_z {
            list.extend(child.flatten());
        }

        list
    }
}
