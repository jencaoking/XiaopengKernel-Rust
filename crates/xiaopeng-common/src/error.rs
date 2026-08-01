use thiserror::Error;

/// Central error enum for XiaopengKernel
#[derive(Error, Debug)]
pub enum XiaopengError {
    #[error("HTML parsing error: {0}")]
    HtmlParseError(String),

    #[error("CSS parsing error: {0}")]
    CssParseError(String),

    #[error("Style resolution error: {0}")]
    StyleError(String),

    #[error("Layout engine error: {0}")]
    LayoutError(String),

    #[error("Render engine error: {0}")]
    RenderError(String),

    #[error("Network / Loader error: {0}")]
    NetworkError(String),

    #[error("Script engine error: {0}")]
    ScriptError(String),

    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Generic error: {0}")]
    Generic(String),
}

/// Specialized Result type for XiaopengKernel operations
pub type XiaopengResult<T> = Result<T, XiaopengError>;
