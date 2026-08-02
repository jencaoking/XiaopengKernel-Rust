//! HTTP/1.1 transport via `hyper` + `tokio-rustls`.
//!
//! Supports plain HTTP and HTTPS. Uses HTTP/1.1 by default; ALPN negotiates
//! "http/1.1" for TLS connections.

use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::Request as HyperRequest;
use hyper_util::rt::TokioIo;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio_rustls::TlsConnector;
use tracing::{debug, info};
use url::Url;
use xiaopeng_common::{XiaopengError, XiaopengResult};

use crate::request::{Headers, HttpVersion, Request, Response};
use crate::tls::build_tls_config;
use crate::H1PoolType;

pub async fn send(req: &Request, pool: &H1PoolType) -> XiaopengResult<Response> {
    let url = Url::parse(&req.url).map_err(|e| XiaopengError::NetworkError {
        url: req.url.clone(),
        message: format!("invalid URL: {e}"),
    })?;

    let scheme  = url.scheme();
    let host    = url.host_str().unwrap_or("localhost");
    let port    = url.port_or_known_default().unwrap_or(80);
    let path_and_query = {
        let p = url.path();
        let q = url.query().map(|q| format!("?{q}")).unwrap_or_default();
        format!("{p}{q}")
    };
    let authority = format!("{host}:{port}");
    let path = if path_and_query.is_empty() { "/".to_string() } else { path_and_query };

    info!("HTTP/1.1 {} {scheme}://{authority}{path}", req.method);

    let method: hyper::Method = req.method.clone().into();
    let body_bytes = req.body.clone().unwrap_or_default();

    let host_key = format!("{}://{}", scheme, authority);

    let mut sender_opt = None;
    if let Some(mut s) = pool.lock().await.take(&host_key) {
        // If it's ready, we can reuse it. If not, we drop it.
        if s.ready().await.is_ok() {
            sender_opt = Some(s);
        }
    }

    let mut sender = match sender_opt {
        Some(s) => s,
        None => {
            let tcp = TcpStream::connect(&authority).await.map_err(|e| XiaopengError::NetworkError {
                url: req.url.clone(),
                message: format!("TCP connect failed: {e}"),
            })?;

            match scheme {
        "https" => {
            let mut tls_config = (*build_tls_config()).clone();
            // For HTTP/1.1 TLS, advertise only h1.
            tls_config.alpn_protocols = vec![b"http/1.1".to_vec()];
            let connector = TlsConnector::from(Arc::new(tls_config));
            let server_name = rustls::pki_types::ServerName::try_from(host.to_owned())
                .map_err(|e| XiaopengError::NetworkError {
                    url: req.url.clone(),
                    message: format!("invalid server name: {e}"),
                })?;
            let tls_stream = connector.connect(server_name, tcp).await.map_err(|e| {
                XiaopengError::NetworkError {
                    url: req.url.clone(),
                    message: format!("TLS handshake failed: {e}"),
                }
            })?;
            let io = TokioIo::new(tls_stream);
            let (sender, conn) =
                hyper::client::conn::http1::handshake(io).await.map_err(|e| {
                    XiaopengError::NetworkError {
                        url: req.url.clone(),
                        message: format!("HTTP/1.1 handshake failed: {e}"),
                    }
                })?;
            tokio::spawn(conn);
            sender
        }
        "http" => {
            let io = TokioIo::new(tcp);
            let (sender, conn) =
                hyper::client::conn::http1::handshake(io).await.map_err(|e| {
                    XiaopengError::NetworkError {
                        url: req.url.clone(),
                        message: format!("HTTP/1.1 handshake failed: {e}"),
                    }
                })?;
            tokio::spawn(conn);
            sender
        }
        _ => return Err(XiaopengError::NetworkError {
            url: req.url.clone(),
            message: format!("unsupported scheme: {scheme}"),
        }),
    }
    }
    };

    let res = send_h1_request(&mut sender, method, &authority, &path, &req.headers, body_bytes, &req.url, scheme == "https").await;
    
    // Attempt to put the connection back into the pool.
    // If it's still healthy, `ready()` will return Ok(()).
    if res.is_ok() && sender.ready().await.is_ok() {
        pool.lock().await.put(&host_key, sender);
    }
    
    res
}

async fn send_h1_request(
    sender: &mut hyper::client::conn::http1::SendRequest<Full<Bytes>>,
    method: hyper::Method,
    authority: &str,
    path: &str,
    headers: &Headers,
    body: Bytes,
    url: &str,
    _tls: bool,
) -> XiaopengResult<Response> {
    let mut hyper_req = HyperRequest::builder()
        .method(method)
        .uri(path)
        .header("host", authority)
        .header("user-agent", "XiaopengKernel/0.4")
        .header("accept", "*/*");

    for (k, v) in headers.iter() {
        hyper_req = hyper_req.header(k, v);
    }

    let hyper_req = hyper_req
        .body(Full::new(body))
        .map_err(|e| XiaopengError::NetworkError {
            url: url.to_string(),
            message: format!("build request: {e}"),
        })?;

    let hyper_resp = sender.send_request(hyper_req).await.map_err(|e| {
        XiaopengError::NetworkError {
            url: url.to_string(),
            message: format!("send request: {e}"),
        }
    })?;

    let status = hyper_resp.status().as_u16();
    let mut resp_headers = Headers::new();
    for (k, v) in hyper_resp.headers() {
        resp_headers.insert(k.as_str(), v.to_str().unwrap_or(""));
    }

    let body_bytes = hyper_resp
        .collect()
        .await
        .map_err(|e| XiaopengError::NetworkError {
            url: url.to_string(),
            message: format!("read body: {e}"),
        })?
        .to_bytes();

    debug!("HTTP/1.1 response: {status} ({} bytes)", body_bytes.len());
    Ok(Response {
        status,
        headers: resp_headers,
        body: body_bytes,
        version: HttpVersion::Http1_1,
    })
}
