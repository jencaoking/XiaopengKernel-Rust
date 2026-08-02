//! HTTP/2 transport via `hyper` (h2 feature) + `tokio-rustls`.
//!
//! HTTP/2 requires TLS (h2 via ALPN). Plain-text h2c is not commonly used
//! by browsers, so we only implement HTTPS here. Falls back gracefully.

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

pub async fn send(req: &Request) -> XiaopengResult<Response> {
    let url = Url::parse(&req.url).map_err(|e| XiaopengError::NetworkError {
        url: req.url.clone(),
        message: format!("invalid URL: {e}"),
    })?;

    let host   = url.host_str().unwrap_or("localhost");
    let port   = url.port_or_known_default().unwrap_or(443);
    let path_and_query = {
        let p = url.path();
        let q = url.query().map(|q| format!("?{q}")).unwrap_or_default();
        format!("{p}{q}")
    };
    let authority  = format!("{host}:{port}");
    let path = if path_and_query.is_empty() { "/".to_string() } else { path_and_query };

    if url.scheme() != "https" {
        return Err(XiaopengError::NetworkError {
            url: req.url.clone(),
            message: "HTTP/2 requires HTTPS; plain h2c not supported".into(),
        });
    }

    info!("HTTP/2 https://{authority}{path}");

    // TLS with ALPN "h2"
    let mut tls_config = (*build_tls_config()).clone();
    tls_config.alpn_protocols = vec![b"h2".to_vec()];

    let connector = TlsConnector::from(Arc::new(tls_config));
    let tcp = TcpStream::connect(&authority).await.map_err(|e| XiaopengError::NetworkError {
        url: req.url.clone(),
        message: format!("TCP connect: {e}"),
    })?;

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
    let (mut sender, conn) = hyper::client::conn::http2::handshake(
        hyper_util::rt::TokioExecutor::new(),
        io,
    )
    .await
    .map_err(|e| XiaopengError::NetworkError {
        url: req.url.clone(),
        message: format!("HTTP/2 handshake: {e}"),
    })?;

    tokio::spawn(conn);

    let method: hyper::Method = req.method.clone().into();
    let body_bytes = req.body.clone().unwrap_or_default();

    let mut hyper_req = HyperRequest::builder()
        .method(method)
        .uri(format!("https://{authority}{path}"))
        .header("user-agent", "XiaopengKernel/0.4")
        .header("accept", "*/*");

    for (k, v) in req.headers.iter() {
        hyper_req = hyper_req.header(k, v);
    }

    let hyper_req = hyper_req
        .body(Full::new(body_bytes))
        .map_err(|e| XiaopengError::NetworkError {
            url: req.url.clone(),
            message: format!("build request: {e}"),
        })?;

    let hyper_resp = sender.send_request(hyper_req).await.map_err(|e| {
        XiaopengError::NetworkError {
            url: req.url.clone(),
            message: format!("send request: {e}"),
        }
    })?;

    let status = hyper_resp.status().as_u16();
    let mut resp_headers = Headers::new();
    for (k, v) in hyper_resp.headers() {
        resp_headers.insert(k.as_str(), v.to_str().unwrap_or(""));
    }

    let body = hyper_resp
        .collect()
        .await
        .map_err(|e| XiaopengError::NetworkError {
            url: req.url.clone(),
            message: format!("read body: {e}"),
        })?
        .to_bytes();

    debug!("HTTP/2 response: {status} ({} bytes)", body.len());
    Ok(Response {
        status,
        headers: resp_headers,
        body,
        version: HttpVersion::Http2,
    })
}
