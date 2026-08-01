//! CSS Selector and Specificity

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SimpleSelector {
    TagName(String),
    Id(String),
    Class(String),
    Universal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Specificity {
    pub a: u32, // ID selectors
    pub b: u32, // Class, attribute, pseudo-class
    pub c: u32, // Type selectors and pseudo-elements
}

#[derive(Debug, Clone)]
pub struct Selector {
    pub simple_selectors: Vec<SimpleSelector>,
}

impl Selector {
    pub fn specificity(&self) -> Specificity {
        let mut spec = Specificity { a: 0, b: 0, c: 0 };
        for sel in &self.simple_selectors {
            match sel {
                SimpleSelector::Id(_) => spec.a += 1,
                SimpleSelector::Class(_) => spec.b += 1,
                SimpleSelector::TagName(_) => spec.c += 1,
                SimpleSelector::Universal => {}
            }
        }
        spec
    }
}
