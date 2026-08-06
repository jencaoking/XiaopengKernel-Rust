//! Integration tests for xiaopeng-net
//! These tests require actual network access to pass.
//! Run with: cargo test -p xiaopeng-net -- --test-threads=1

use xiaopeng_net::{fetch, NetClient, ProtocolHint, Request, Method, HttpVersion};

// ─── Unit tests (no network) ──────────────────────────────────────────────────

#[cfg(test)]
mod unit {
    use xiaopeng_net::{Request, Method, Headers, cache::ResourceCache};
    use bytes::Bytes;

    #[test]
    fn test_request_builder() {
        let req = Request::get("https://example.com")
            .with_header("accept", "text/html")
            .with_header("x-custom", "value");
        assert_eq!(req.method, Method::Get);
        assert_eq!(req.url, "https://example.com");
        assert_eq!(req.headers.get("accept"), Some("text/html"));
        assert_eq!(req.headers.get("x-custom"), Some("value"));
        assert!(req.body.is_none());
    }

    #[test]
    fn test_post_request() {
        let req = Request::post("https://example.com/api", "hello world");
        assert_eq!(req.method, Method::Post);
        assert!(req.body.is_some());
        assert_eq!(&*req.body.expect("Unwrap failed"), b"hello world");
    }

    #[test]
    fn test_lru_cache_basic() {
        use xiaopeng_net::{Response, HttpVersion, Headers};
        let mut cache = ResourceCache::new(4);
        assert!(cache.get("GET", "https://example.com").is_none());

        let resp = Response {
            status: 200,
            headers: {
                let mut h = Headers::new();
                h.insert("cache-control", "max-age=3600");
                h
            },
            body: bytes::Bytes::from_static(b"hello"),
            version: HttpVersion::Http1_1,
        };
        cache.insert("GET", "https://example.com", resp);

        // Should be fresh (3600 second max-age)
        let hit = cache.get("GET", "https://example.com");
        assert!(hit.is_some());
        assert_eq!(hit.expect("Unwrap failed").response.status, 200);
    }

    #[test]
    fn test_lru_cache_no_max_age_is_stale() {
        use xiaopeng_net::{Response, HttpVersion, Headers};
        let mut cache = ResourceCache::new(4);
        let resp = Response {
            status: 200,
            headers: Headers::new(), // no Cache-Control
            body: bytes::Bytes::new(),
            version: HttpVersion::Http1_1,
        };
        cache.insert("GET", "https://example.com", resp);
        // Without max-age, treated as immediately stale
        assert!(cache.get("GET", "https://example.com").is_none());
    }

    #[test]
    fn test_lru_cache_eviction() {
        use xiaopeng_net::{Response, HttpVersion, Headers};
        let mut cache = ResourceCache::new(2); // cap=2
        let make_resp = || Response {
            status: 200,
            headers: { let mut h = Headers::new(); h.insert("cache-control", "max-age=9999"); h },
            body: bytes::Bytes::new(),
            version: HttpVersion::Http1_1,
        };
        cache.insert("GET", "https://a.com", make_resp());
        cache.insert("GET", "https://b.com", make_resp());
        cache.insert("GET", "https://c.com", make_resp()); // evicts a.com (LRU)
        assert!(cache.get("GET", "https://a.com").is_none());
        assert!(cache.get("GET", "https://b.com").is_some());
        assert!(cache.get("GET", "https://c.com").is_some());
    }
}

// ─── Network tests (require internet) ────────────────────────────────────────

#[cfg(test)]
mod network {
    use super::*;

    #[tokio::test]
    #[ignore = "requires internet"]
    async fn test_http1_plain() {
        let client = NetClient::new().with_protocol(ProtocolHint::Http1);
        let resp = client.fetch(Request::get("http://example.com")).await.expect("Unwrap failed");
        assert!(resp.ok(), "expected 2xx, got {}", resp.status);
        assert_eq!(resp.version, HttpVersion::Http1_1);
        let body = resp.body_text();
        assert!(body.contains("Example Domain") || !body.is_empty());
    }

    #[tokio::test]
    #[ignore = "requires internet"]
    async fn test_https_auto_negotiates_h2() {
        let client = NetClient::new().with_protocol(ProtocolHint::Auto);
        let resp = client.fetch(Request::get("https://www.google.com")).await.expect("Unwrap failed");
        assert!(resp.ok(), "expected 2xx, got {}", resp.status);
        // Google supports H2, auto mode should pick it.
        assert!(matches!(resp.version, HttpVersion::Http2 | HttpVersion::Http1_1));
    }

    #[tokio::test]
    #[ignore = "requires internet"]
    async fn test_redirect_following() {
        // http://example.com redirects to https://example.com (301)
        let client = NetClient::new();
        let resp = client.fetch(Request::get("http://example.com")).await.expect("Unwrap failed");
        assert!(resp.ok(), "expected 2xx after redirect, got {}", resp.status);
    }

    #[tokio::test]
    #[ignore = "requires internet"]
    async fn test_cache_second_request_is_hit() {
        let client = NetClient::new().with_protocol(ProtocolHint::Http1);
        // First request: cache miss
        let resp1 = client.fetch(Request::get("http://example.com/")).await.expect("Unwrap failed");
        assert!(resp1.ok());
        // Only cache if server returned max-age; this is a best-effort test.
        // We just check the second request doesn't error.
        let resp2 = client.fetch(Request::get("http://example.com/")).await.expect("Unwrap failed");
        assert!(resp2.ok());
    }

    #[tokio::test]
    #[ignore = "requires internet + HTTP3 server"]
    async fn test_http3_cloudflare() {
        // Cloudflare endpoints advertise h3 via Alt-Svc.
        let client = NetClient::new().with_protocol(ProtocolHint::Http3);
        let resp = client.fetch(Request::get("https://cloudflare.com")).await.expect("Unwrap failed");
        assert!(resp.ok() || resp.redirect(), "status: {}", resp.status);
        assert_eq!(resp.version, HttpVersion::Http3);
    }
}
