//! XiaopengKernel Async Resource Loader & Network Module

pub mod cache;

pub use cache::ResourceCache;
use tracing::info;
use url::Url;
use xiaopeng_common::{XiaopengError, XiaopengResult};

pub async fn load_resource(raw_url: &str) -> XiaopengResult<String> {
    info!("Loading resource from URL: {}", raw_url);
    let parsed_url = Url::parse(raw_url)
        .map_err(|e| XiaopengError::NetworkError(format!("Invalid URL: {e}")))?;

    Ok(format!("Content loaded from {}", parsed_url))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_load_resource() {
        let res = load_resource("https://example.com").await;
        assert!(res.is_ok());
    }
}
