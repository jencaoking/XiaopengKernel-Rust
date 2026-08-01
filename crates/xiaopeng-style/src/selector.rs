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
pub enum AttributeOperator {
    Exists,         // [attr]
    Exact,          // [attr=value]
    Includes,       // [attr~=value]
    DashMatch,      // [attr|=value]
    Prefix,         // [attr^=value]
    Suffix,         // [attr$=value]
    Substring,      // [attr*=value]
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimpleSelector {
    pub selector_type: SelectorType,
    pub value: String, // Tag name, class name, ID, pseudo name
    
    // Attribute selector details
    pub attribute_name: Option<String>,
    pub attribute_value: Option<String>,
    pub attribute_operator: Option<AttributeOperator>,
}

impl SimpleSelector {
    pub fn new_basic(selector_type: SelectorType, value: String) -> Self {
        Self {
            selector_type,
            value,
            attribute_name: None,
            attribute_value: None,
            attribute_operator: None,
        }
    }

    pub fn new_attribute(name: String, op: AttributeOperator, val: Option<String>) -> Self {
        Self {
            selector_type: SelectorType::Attribute,
            value: String::new(),
            attribute_name: Some(name),
            attribute_value: val,
            attribute_operator: Some(op),
        }
    }
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
    pub combinators: Vec<Combinator>,
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
