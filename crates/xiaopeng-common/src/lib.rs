pub mod error;
pub mod logger;
pub mod types;

pub use error::{XiaopengError, XiaopengResult};
pub use logger::init_logging;
pub use types::{Color, Point, Rect, Size};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_formatting() {
        let err = XiaopengError::HtmlParseError("Unexpected EOF".into());
        assert_eq!(err.to_string(), "HTML parsing error: Unexpected EOF");
    }

    #[test]
    fn test_rect_contains() {
        let rect = Rect::new(0.0, 0.0, 100.0, 100.0);
        assert!(rect.contains(Point::new(50.0, 50.0)));
        assert!(!rect.contains(Point::new(150.0, 50.0)));
    }
}
