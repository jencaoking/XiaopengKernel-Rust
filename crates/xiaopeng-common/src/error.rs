use anyhow::Context;
use thiserror::Error;

/// Unified domain-specific error type for XiaopengKernel
#[derive(Error, Debug)]
pub enum XiaopengError {
    #[error("HTML parsing error at line {line}, col {col}: {message}")]
    HtmlParseError {
        line: usize,
        col: usize,
        message: String,
    },

    #[error("CSS parsing error in '{location}': {message}")]
    CssParseError { location: String, message: String },

    #[error("Style resolution error: {message}")]
    StyleError { message: String },

    #[error("Layout engine error in {component}: {message}")]
    LayoutError { component: String, message: String },

    #[error("Render engine error [{backend}]: {message}")]
    RenderError { backend: String, message: String },

    #[error("Network / Loader error for '{url}': {message}")]
    NetworkError { url: String, message: String },

    #[error("Script execution error at line {line:?}: {message}")]
    ScriptError {
        line: Option<usize>,
        message: String,
    },

    #[error("URL parse error: {0}")]
    UrlParseError(#[from] url::ParseError),

    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Uncategorized internal error: {0}")]
    Other(#[from] anyhow::Error),
}

/// Specialized Result alias using domain XiaopengError
pub type XiaopengResult<T> = Result<T, XiaopengError>;

/// Extension helper to conveniently attach anyhow context to XiaopengResult
pub trait XiaopengResultExt<T> {
    fn context<C>(self, context: C) -> anyhow::Result<T>
    where
        C: std::fmt::Display + Send + Sync + 'static;

    fn with_context<C, F>(self, f: F) -> anyhow::Result<T>
    where
        C: std::fmt::Display + Send + Sync + 'static,
        F: FnOnce() -> C;
}

impl<T> XiaopengResultExt<T> for XiaopengResult<T> {
    fn context<C>(self, context: C) -> anyhow::Result<T>
    where
        C: std::fmt::Display + Send + Sync + 'static,
    {
        self.map_err(anyhow::Error::from).context(context)
    }

    fn with_context<C, F>(self, f: F) -> anyhow::Result<T>
    where
        C: std::fmt::Display + Send + Sync + 'static,
        F: FnOnce() -> C,
    {
        self.map_err(anyhow::Error::from).with_context(f)
    }
}
