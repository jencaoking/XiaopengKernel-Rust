//! CSS Selector and Specificity

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SelectorType {
    Tag,
    Id,
    Class,
    Universal,
    Attribute,
    PseudoClass,
    PseudoElement,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Combinator {
    None,
    Descendant,       // ' '
    Child,            // '>'
    NextSibling,      // '+'
    SubsequentSibling // '~'
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimpleSelector {
    pub selector_type: SelectorType,
    pub value: String,
    // Note: Attribute/Pseudo details omitted for brevity matching C++ phase
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Specificity {
    pub a: u32, // ID selectors
    pub b: u32, // Class, attribute, pseudo-class
    pub c: u32, // Type selectors and pseudo-elements
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Selector {
    pub parts: Vec<SimpleSelector>,
    pub combinators: Vec<Combinator>, // length = parts.len() - 1
}

impl Selector {
    pub fn specificity(&self) -> Specificity {
        let mut spec = Specificity { a: 0, b: 0, c: 0 };
        for part in &self.parts {
            match part.selector_type {
                SelectorType::Id => spec.a += 1,
                SelectorType::Class | SelectorType::Attribute | SelectorType::PseudoClass => spec.b += 1,
                SelectorType::Tag | SelectorType::PseudoElement => spec.c += 1,
                SelectorType::Universal => {}
            }
        }
        spec
    }
}
