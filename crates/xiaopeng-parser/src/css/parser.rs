//! CSS Parser

#[derive(Debug, Clone)]
pub struct CssDeclaration {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone)]
pub struct CssRule {
    pub selectors: Vec<String>,
    pub declarations: Vec<CssDeclaration>,
}

#[derive(Debug, Clone, Default)]
pub struct StyleSheet {
    pub rules: Vec<CssRule>,
}
