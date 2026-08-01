pub mod error;
pub mod logger;
pub mod types;

pub use error::{XiaopengError, XiaopengResult, XiaopengResultExt};
pub use logger::init_logging;
pub use types::{Color, Point, Rect, Size};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_formatting() {
        let err = XiaopengError::HtmlParseError {
            line: 10,
            col: 5,
            message: "Unexpected EOF".into(),
        };
        assert_eq!(
            err.to_string(),
            "HTML parsing error at line 10, col 5: Unexpected EOF"
        );
    }

    #[test]
    fn test_anyhow_context_integration() {
        let res: XiaopengResult<()> = Err(XiaopengError::NetworkError {
            url: "https://example.com".into(),
            message: "Connection timed out".into(),
        });
        let anyhow_res = res.context("Failed to fetch homepage resource");
        assert!(anyhow_res.is_err());
        let err_msg = format!("{:#}", anyhow_res.unwrap_err());
        assert!(err_msg.contains("Failed to fetch homepage resource"));
        assert!(err_msg.contains("Connection timed out"));
    }
}
