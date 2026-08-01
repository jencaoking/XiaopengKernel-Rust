pub mod error;
pub mod logger;

pub use error::{XiaopengError, XiaopengResult};
pub use logger::init_logging;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_formatting() {
        let err = XiaopengError::HtmlParseError("Unexpected EOF".into());
        assert_eq!(err.to_string(), "HTML parsing error: Unexpected EOF");
    }
}
