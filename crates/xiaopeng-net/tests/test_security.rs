use xiaopeng_net::{NetClient, Request};
use xiaopeng_net::request::RequestMode;
use xiaopeng_common::error::XiaopengError;

fn is_network_error_with(res: Result<xiaopeng_net::Response, XiaopengError>, msg: &str) -> bool {
    match res {
        Err(XiaopengError::NetworkError { message, .. }) => message.contains(msg),
        _ => false,
    }
}

#[tokio::test]
async fn test_mixed_content_block() {
    let client = NetClient::new();
    let mut req = Request::get("http://example.com");
    req.initiator_origin = Some("https://example.com".into());
    req.mode = RequestMode::Cors;

    let res = client.fetch(req).await;
    assert!(is_network_error_with(res, "Mixed Content"));
}

#[tokio::test]
async fn test_same_origin_policy_block() {
    let client = NetClient::new();
    let mut req = Request::get("https://b.com/api");
    req.initiator_origin = Some("https://a.com".into());
    req.mode = RequestMode::SameOrigin;

    let res = client.fetch(req).await;
    assert!(is_network_error_with(res, "Same-Origin Policy"));
}

#[tokio::test]
async fn test_same_origin_policy_allow() {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let client = NetClient::new();
    let mut req = Request::get("https://a.com:443/api");
    req.initiator_origin = Some("https://a.com".into());
    req.mode = RequestMode::SameOrigin;

    // This won't throw SOP error, though it might fail due to network if a.com is not mocked
    let res = client.fetch(req).await;
    assert!(!is_network_error_with(res, "Same-Origin Policy"));
}

#[tokio::test]
async fn test_cors_missing_acao() {
    let _ = rustls::crypto::ring::default_provider().install_default();
    // If a request is cross-origin with Cors mode and the server doesn't provide Access-Control-Allow-Origin, it fails.
    let client = NetClient::new();
    let mut req = Request::get("https://example.com"); // example.com does not set CORS headers normally
    req.initiator_origin = Some("https://a.com".into());
    req.mode = RequestMode::Cors;

    let res = client.fetch(req).await;
    assert!(is_network_error_with(res, "CORS error: Missing or invalid Access-Control-Allow-Origin"));
}
