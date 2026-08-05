//! XiaopengKernel Network Module
//!
//! Provides a unified `fetch()` API that automatically negotiates the best
//! available HTTP protocol:
//!
//! - **HTTP/3** if the server advertises QUIC via `Alt-Svc`.
//! - **HTTP/2** for HTTPS connections where ALPN negotiates h2.
//! - **HTTP/1.1** as universal fallback.
//!
//! All transports support TLS via `rustls` with system root certificates
//! (falling back to Mozilla's webpki-roots bundle).

pub mod cache;
pub mod http1;
pub mod http2;
pub mod pool;
pub mod request;
pub mod tls;

pub use cache::ResourceCache;
pub use request::{Headers, HttpVersion, Method, Request, Response, StreamResponse};

use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{info, warn};
use xiaopeng_common::{XiaopengError, XiaopengResult};

// ---------------------------------------------------------------------------
// NetClient — stateful HTTP client with integrated LRU cache
// ---------------------------------------------------------------------------

/// Protocol preference for a `NetClient` instance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtocolHint {
    /// Always use HTTP/1.1.
    Http1,
    /// Always use HTTP/2 (HTTPS only).
    Http2,
    /// Always use HTTP/3 (HTTPS only, QUIC).
    Http3,
    /// Automatically pick the best protocol (default).
    Auto,
}

use crate::pool::ConnectionPool;
use http_body_util::Full;
use bytes::Bytes;
use hyper::client::conn::http1::SendRequest as H1Send;
use hyper::client::conn::http2::SendRequest as H2Send;
use std::time::Duration;

pub type H1PoolType = Arc<Mutex<ConnectionPool<H1Send<Full<Bytes>>>>>;
pub type H2PoolType = Arc<Mutex<ConnectionPool<H2Send<Full<Bytes>>>>>;

/// A browser-style HTTP client that:
/// - Caches responses with `Cache-Control: max-age`.
/// - Respects `Alt-Svc` headers to upgrade to HTTP/3 on subsequent requests.
/// - Supports redirect following (up to 10 hops).
pub struct NetClient {
    cache: Arc<Mutex<ResourceCache>>,
    protocol_hint: ProtocolHint,
    max_redirects: usize,
    
    // Connection Pools
    h1_pool: H1PoolType,
    h2_pool: H2PoolType,
}

impl NetClient {
    pub fn new() -> Self {
        let max_conns = 6;
        let idle_time = Duration::from_secs(60);
        
        Self {
            cache: Arc::new(Mutex::new(ResourceCache::new(256))),
            protocol_hint: ProtocolHint::Auto,
            max_redirects: 10,
            h1_pool: Arc::new(Mutex::new(ConnectionPool::new(max_conns, idle_time))),
            h2_pool: Arc::new(Mutex::new(ConnectionPool::new(max_conns, idle_time))),
        }
    }

    pub fn with_protocol(mut self, hint: ProtocolHint) -> Self {
        self.protocol_hint = hint;
        self
    }

    pub fn with_max_redirects(mut self, n: usize) -> Self {
        self.max_redirects = n;
        self
    }

    /// Perform an HTTP request with caching, protocol negotiation, and redirect following.
    pub async fn fetch(&self, mut req: Request) -> XiaopengResult<Response> {
        self.check_security_policy(&mut req)?;
        let mut current_req = req.clone();
        for hop in 0..=self.max_redirects {
            let method_str = current_req.method.to_string();
            let mut stale_cached_response = None;

            // 1. Check cache (only for GET/HEAD).
            if matches!(current_req.method, Method::Get | Method::Head) {
                let mut cache = self.cache.lock().await;
                if let Some(entry) = cache.get_entry(&method_str, &current_req.url) {
                    if entry.is_fresh() {
                        info!("Cache HIT (fresh): {} {}", method_str, current_req.url);
                        return Ok(entry.response.clone());
                    } else {
                        // Stale. Save it to validate.
                        stale_cached_response = Some(entry.clone());
                    }
                }
            }

            // If we have a stale response, set validation headers
            if let Some(ref stale) = stale_cached_response {
                if let Some(etag) = &stale.etag {
                    current_req.headers.insert("if-none-match", etag);
                } else if let Some(last_modified) = &stale.last_modified {
                    current_req.headers.insert("if-modified-since", last_modified);
                }
            }

            let resp = self.send_one(&current_req).await?;

            // 1.5. Validate CORS response if needed
            self.validate_cors_response(&current_req, &resp)?;

            // Check if 304 Not Modified
            if resp.status == 304 {
                if let Some(mut stale) = stale_cached_response {
                    info!("Cache HIT (304 Not Modified): {} {}", method_str, current_req.url);
                    // Update headers (like a new Date, new Cache-Control)
                    for (k, v) in resp.headers.iter() {
                        stale.response.headers.insert(k, v);
                    }
                    
                    let mut cache = self.cache.lock().await;
                    cache.insert(&method_str, &current_req.url, stale.response.clone());
                    return Ok(stale.response);
                }
            }

            // Cache the response if it's a success.
            if resp.ok() && matches!(current_req.method, Method::Get | Method::Head) {
                let mut cache = self.cache.lock().await;
                cache.insert(&method_str, &current_req.url, resp.clone());
            }

            // HTTP/3 Alt-Svc tracking removed (experimental stack dropped)

            // Handle redirects.
            if resp.redirect() {
                if hop == self.max_redirects {
                    return Err(XiaopengError::NetworkError {
                        url: current_req.url.clone(),
                        message: format!("too many redirects (>{} hops)", self.max_redirects),
                    });
                }
                let location = resp.headers.get("location").unwrap_or("").to_owned();
                if location.is_empty() {
                    return Err(XiaopengError::NetworkError {
                        url: current_req.url.clone(),
                        message: "redirect with no Location header".into(),
                    });
                }
                info!("Redirect {} → {location}", resp.status);
                // Resolve relative URLs.
                let next_url = if location.starts_with("http") {
                    location
                } else {
                    let base = url::Url::parse(&current_req.url)
                        .map_err(|e| XiaopengError::NetworkError {
                            url: current_req.url.clone(),
                            message: format!("base URL parse: {e}"),
                        })?;
                    base.join(&location)
                        .map_err(|e| XiaopengError::NetworkError {
                            url: current_req.url.clone(),
                            message: format!("resolve redirect: {e}"),
                        })?
                        .to_string()
                };
                // 301/302/303 → downgrade to GET.
                let method = if [301, 302, 303].contains(&resp.status) {
                    Method::Get
                } else {
                    current_req.method.clone()
                };
                current_req = Request {
                    method,
                    url: next_url,
                    headers: current_req.headers.clone(),
                    body: if resp.status == 303 { None } else { current_req.body.clone() },
                    initiator_origin: current_req.initiator_origin.clone(),
                    mode: current_req.mode.clone(),
                };
                self.check_security_policy(&mut current_req)?;
                continue;
            }

            return Ok(resp);
        }

        unreachable!()
    }

    fn check_security_policy(&self, req: &mut Request) -> XiaopengResult<()> {
        use crate::request::RequestMode;
        if let Some(ref init_origin) = req.initiator_origin {
            let init_is_https = init_origin.starts_with("https://");
            let target_is_https = req.url.starts_with("https://");
            
            // Mixed Content
            if init_is_https && !target_is_https {
                return Err(XiaopengError::NetworkError {
                    url: req.url.clone(),
                    message: "Mixed Content: HTTPS origin blocked from loading HTTP resource".into(),
                });
            }

            let is_cross_origin = {
                if let (Ok(t_url), Ok(i_url)) = (url::Url::parse(&req.url), url::Url::parse(init_origin)) {
                    let t_origin = format!("{}://{}:{}", t_url.scheme(), t_url.host_str().unwrap_or(""), t_url.port_or_known_default().unwrap_or(80));
                    let i_origin = format!("{}://{}:{}", i_url.scheme(), i_url.host_str().unwrap_or(""), i_url.port_or_known_default().unwrap_or(80));
                    t_origin != i_origin
                } else {
                    false
                }
            };

            if is_cross_origin {
                match req.mode {
                    RequestMode::SameOrigin => {
                        return Err(XiaopengError::NetworkError {
                            url: req.url.clone(),
                            message: "Same-Origin Policy: cross-origin request blocked".into(),
                        });
                    }
                    RequestMode::Cors => {
                        req.headers.insert("origin", init_origin.clone());
                    }
                    _ => {} // NoCors, Navigate are allowed
                }
            }
        }
        Ok(())
    }

    fn validate_cors_response(&self, req: &Request, resp: &Response) -> XiaopengResult<()> {
        use crate::request::RequestMode;
        if req.mode == RequestMode::Cors {
            if let Some(ref init_origin) = req.initiator_origin {
                let is_cross_origin = {
                    if let (Ok(t_url), Ok(i_url)) = (url::Url::parse(&req.url), url::Url::parse(init_origin)) {
                        let t_origin = format!("{}://{}:{}", t_url.scheme(), t_url.host_str().unwrap_or(""), t_url.port_or_known_default().unwrap_or(80));
                        let i_origin = format!("{}://{}:{}", i_url.scheme(), i_url.host_str().unwrap_or(""), i_url.port_or_known_default().unwrap_or(80));
                        t_origin != i_origin
                    } else {
                        false
                    }
                };

                if is_cross_origin {
                    let acao = resp.headers.get("access-control-allow-origin").unwrap_or("");
                    if acao != "*" && acao != init_origin {
                        return Err(XiaopengError::NetworkError {
                            url: req.url.clone(),
                            message: format!("CORS error: Missing or invalid Access-Control-Allow-Origin: '{}'", acao),
                        });
                    }
                }
            }
        }
        Ok(())
    }

    /// Send a single request, choosing the transport layer automatically.
    async fn send_one(&self, req: &Request) -> XiaopengResult<Response> {
        // H3 is removed. Only H1 and H2 remain.

        match self.protocol_hint {
            ProtocolHint::Http1 => http1::send(req, &self.h1_pool).await,
            ProtocolHint::Http2 => http2::send(req, &self.h2_pool).await,
            ProtocolHint::Http3 => {
                // Already tried H3 above and it failed. Try H2 as fallback.
                match http2::send(req, &self.h2_pool).await {
                    Ok(r) => Ok(r),
                    Err(_) => http1::send(req, &self.h1_pool).await,
                }
            }
            ProtocolHint::Auto => {
                // For HTTPS, try H2 first; fall back to H1.
                if req.url.starts_with("https://") {
                    match http2::send(req, &self.h2_pool).await {
                        Ok(r) => Ok(r),
                        Err(_) => http1::send(req, &self.h1_pool).await,
                    }
                } else {
                    http1::send(req, &self.h1_pool).await
                }
            }
        }
    }
    
    pub async fn fetch_stream(&self, mut req: Request) -> XiaopengResult<StreamResponse> {
        self.check_security_policy(&mut req)?;

        match self.protocol_hint {
            ProtocolHint::Http1 => http1::send_stream(&req, &self.h1_pool).await,
            ProtocolHint::Http2 => http2::send_stream(&req, &self.h2_pool).await,
            ProtocolHint::Http3 => {
                match http2::send_stream(&req, &self.h2_pool).await {
                    Ok(r) => Ok(r),
                    Err(_) => http1::send_stream(&req, &self.h1_pool).await,
                }
            }
            ProtocolHint::Auto => {
                if req.url.starts_with("https://") {
                    match http2::send_stream(&req, &self.h2_pool).await {
                        Ok(r) => Ok(r),
                        Err(_) => http1::send_stream(&req, &self.h1_pool).await,
                    }
                } else {
                    http1::send_stream(&req, &self.h1_pool).await
                }
            }
        }
    }

    /// Invalidate the cache entry for a given method + URL.
    pub async fn invalidate_cache(&self, method: &str, url: &str) {
        self.cache.lock().await.invalidate(method, url);
    }

    /// Clear the entire cache.
    pub async fn clear_cache(&self) {
        self.cache.lock().await.clear();
    }
}

impl Default for NetClient {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Convenience top-level `fetch()` function (uses a one-shot client)
// ---------------------------------------------------------------------------

/// Fetch a URL, automatically negotiating the best available HTTP protocol.
/// Creates a transient `NetClient` (no shared cache). For persistent
/// connections and caching, use `NetClient` directly.
pub async fn fetch(url: &str) -> XiaopengResult<Response> {
    NetClient::new()
        .fetch(Request::get(url))
        .await
}

pub async fn fetch_stream(url: &str) -> XiaopengResult<StreamResponse> {
    NetClient::new()
        .fetch_stream(Request::get(url))
        .await
}

/// Legacy compatibility alias (returns response body as String).
pub async fn load_resource(raw_url: &str) -> XiaopengResult<String> {
    let resp = fetch(raw_url).await?;
    if !resp.ok() {
        return Err(XiaopengError::NetworkError {
            url: raw_url.to_string(),
            message: format!("HTTP error {}", resp.status),
        });
    }
    Ok(resp.body_text().into_owned())
}
