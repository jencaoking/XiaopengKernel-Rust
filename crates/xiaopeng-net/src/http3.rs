//! HTTP/3 transport via `h3` + `h3-quinn` + `quinn` (QUIC).
//!
//! HTTP/3 runs exclusively over QUIC (UDP). TLS 1.3 is mandatory.
//! ALPN is set to "h3".

use bytes::{Buf, Bytes};
use h3_quinn::quinn;
use std::sync::Arc;
use tracing::{debug, info};
use url::Url;
use xiaopeng_common::{XiaopengError, XiaopengResult};

use crate::request::{Headers, HttpVersion, Request, Response};
use crate::tls::build_tls_config;
use crate::H3PoolType;

pub async fn send(req: &Request, pool: &H3PoolType) -> XiaopengResult<Response> {
    let url = Url::parse(&req.url).map_err(|e| XiaopengError::NetworkError {
        url: req.url.clone(),
        message: format!("invalid URL: {e}"),
    })?;

    if url.scheme() != "https" {
        return Err(XiaopengError::NetworkError {
            url: req.url.clone(),
            message: "HTTP/3 requires HTTPS".into(),
        });
    }

    let host = url.host_str().unwrap_or("localhost").to_owned();
    let port = url.port_or_known_default().unwrap_or(443);
    let path_and_query = {
        let p = url.path();
        let q = url.query().map(|q| format!("?{q}")).unwrap_or_default();
        if p.is_empty() { format!("/{q}") } else { format!("{p}{q}") }
    };
    let authority = format!("{host}:{port}");

    info!("HTTP/3 https://{authority}{path_and_query}");

    let host_key = format!("https://{}", authority);

    let mut sender_opt = None;
    if let Some(s) = pool.lock().await.peek_clone(&host_key) {
        sender_opt = Some(s);
    }

    let mut send_request = match sender_opt {
        Some(s) => s,
        None => {
            // Build QUIC client config (TLS 1.3 + ALPN "h3")
            let mut tls_config = (*build_tls_config()).clone();
            tls_config.alpn_protocols = vec![b"h3".to_vec()];

            let quic_client_config = quinn::crypto::rustls::QuicClientConfig::try_from(tls_config)
                .map_err(|e| XiaopengError::NetworkError {
                    url: req.url.clone(),
                    message: format!("QUIC TLS config: {e}"),
                })?;

            let client_config = quinn::ClientConfig::new(Arc::new(quic_client_config));

            // Bind to any local port.
            let mut endpoint = quinn::Endpoint::client("0.0.0.0:0".parse().unwrap())
                .map_err(|e| XiaopengError::NetworkError {
                    url: req.url.clone(),
                    message: format!("QUIC endpoint: {e}"),
                })?;
            endpoint.set_default_client_config(client_config);

            // Resolve the remote address.
            let addr = tokio::net::lookup_host(&authority)
                .await
                .map_err(|e| XiaopengError::NetworkError {
                    url: req.url.clone(),
                    message: format!("DNS lookup failed: {e}"),
                })?
                .find(|a| a.is_ipv4())
                .ok_or_else(|| XiaopengError::NetworkError {
                    url: req.url.clone(),
                    message: "no IPv4 address found".into(),
                })?;

            // QUIC connect
            let quic_conn = endpoint.connect(addr, &host)
                .map_err(|e| XiaopengError::NetworkError {
                    url: req.url.clone(),
                    message: format!("QUIC connect: {e}"),
                })?
                .await
                .map_err(|e| XiaopengError::NetworkError {
                    url: req.url.clone(),
                    message: format!("QUIC handshake: {e}"),
                })?;

            // Build H3 connection
            let h3_conn = h3_quinn::Connection::new(quic_conn);
            let (mut h3_driver, send_request) = h3::client::new(h3_conn)
                .await
                .map_err(|e| XiaopengError::NetworkError {
                    url: req.url.clone(),
                    message: format!("H3 connection: {e}"),
                })?;

            // Drive the connection in the background.
            tokio::spawn(async move {
                let _ = std::future::poll_fn(|cx| h3_driver.poll_close(cx)).await;
            });
            
            pool.lock().await.put(&host_key, send_request.clone());
            
            send_request
        }
    };

    // Build the HTTP/3 request
    let method_str = req.method.to_string();
    let uri = format!("https://{authority}{path_and_query}")
        .parse::<http::Uri>()
        .map_err(|e| XiaopengError::NetworkError {
            url: req.url.clone(),
            message: format!("URI parse: {e}"),
        })?;

    let mut h3_req = http::Request::builder()
        .method(method_str.as_str())
        .uri(uri)
        .header("user-agent", "XiaopengKernel/0.4")
        .header("accept", "*/*");

    for (k, v) in req.headers.iter() {
        h3_req = h3_req.header(k, v);
    }

    let body_bytes = req.body.clone().unwrap_or_default();

    let h3_req = h3_req
        .body(())
        .map_err(|e| XiaopengError::NetworkError {
            url: req.url.clone(),
            message: format!("build H3 request: {e}"),
        })?;

    // Send request
    let mut req_stream = send_request.send_request(h3_req).await.map_err(|e| {
        XiaopengError::NetworkError {
            url: req.url.clone(),
            message: format!("H3 send request: {e}"),
        }
    })?;

    // Send body if any
    if !body_bytes.is_empty() {
        req_stream.send_data(body_bytes.clone()).await.map_err(|e| {
            XiaopengError::NetworkError {
                url: req.url.clone(),
                message: format!("H3 send body: {e}"),
            }
        })?;
    }
    req_stream.finish().await.map_err(|e| XiaopengError::NetworkError {
        url: req.url.clone(),
        message: format!("H3 finish: {e}"),
    })?;

    // Receive response headers
    let hyper_resp = req_stream.recv_response().await.map_err(|e| {
        XiaopengError::NetworkError {
            url: req.url.clone(),
            message: format!("H3 recv response: {e}"),
        }
    })?;

    let status = hyper_resp.status().as_u16();
    let mut resp_headers = Headers::new();
    for (k, v) in hyper_resp.headers() {
        resp_headers.insert(k.as_str(), v.to_str().unwrap_or(""));
    }

    // Collect response body chunks
    let mut body_buf = Vec::new();
    while let Some(mut chunk) = req_stream.recv_data().await.map_err(|e| {
        XiaopengError::NetworkError {
            url: req.url.clone(),
            message: format!("H3 recv data: {e}"),
        }
    })? {
        body_buf.extend_from_slice(&chunk.copy_to_bytes(chunk.remaining()));
    }

    let body = Bytes::from(body_buf);
    debug!("HTTP/3 response: {status} ({} bytes)", body.len());



    Ok(Response {
        status,
        headers: resp_headers,
        body,
        version: HttpVersion::Http3,
    })
}
