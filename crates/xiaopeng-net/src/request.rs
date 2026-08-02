//! Shared HTTP request/response types for xiaopeng-net.

use std::collections::HashMap;
use bytes::Bytes;

// ---------------------------------------------------------------------------
// Method
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Method {
    Get,
    Post,
    Put,
    Delete,
    Head,
    Options,
    Patch,
    Trace,
    Connect,
    Custom(String),
}

impl std::fmt::Display for Method {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Method::Get     => write!(f, "GET"),
            Method::Post    => write!(f, "POST"),
            Method::Put     => write!(f, "PUT"),
            Method::Delete  => write!(f, "DELETE"),
            Method::Head    => write!(f, "HEAD"),
            Method::Options => write!(f, "OPTIONS"),
            Method::Patch   => write!(f, "PATCH"),
            Method::Trace   => write!(f, "TRACE"),
            Method::Connect => write!(f, "CONNECT"),
            Method::Custom(s) => write!(f, "{s}"),
        }
    }
}

impl From<Method> for hyper::Method {
    fn from(m: Method) -> Self {
        match m {
            Method::Get     => hyper::Method::GET,
            Method::Post    => hyper::Method::POST,
            Method::Put     => hyper::Method::PUT,
            Method::Delete  => hyper::Method::DELETE,
            Method::Head    => hyper::Method::HEAD,
            Method::Options => hyper::Method::OPTIONS,
            Method::Patch   => hyper::Method::PATCH,
            Method::Trace   => hyper::Method::TRACE,
            Method::Connect => hyper::Method::CONNECT,
            Method::Custom(s) => hyper::Method::from_bytes(s.as_bytes())
                .unwrap_or(hyper::Method::GET),
        }
    }
}

// ---------------------------------------------------------------------------
// Headers
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
pub struct Headers(pub HashMap<String, String>);

impl Headers {
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    pub fn insert(&mut self, name: impl Into<String>, value: impl Into<String>) {
        self.0.insert(name.into().to_lowercase(), value.into());
    }

    pub fn get(&self, name: &str) -> Option<&str> {
        self.0.get(&name.to_lowercase()).map(|s| s.as_str())
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.0.iter().map(|(k, v)| (k.as_str(), v.as_str()))
    }
}

// ---------------------------------------------------------------------------
// RequestMode
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RequestMode {
    SameOrigin,
    NoCors,
    Cors,
    Navigate,
}

impl Default for RequestMode {
    fn default() -> Self {
        RequestMode::NoCors
    }
}

// ---------------------------------------------------------------------------
// Request
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct Request {
    pub method:  Method,
    pub url:     String,
    pub headers: Headers,
    pub body:    Option<Bytes>,
    pub initiator_origin: Option<String>,
    pub mode: RequestMode,
}

impl Request {
    pub fn get(url: impl Into<String>) -> Self {
        Self {
            method:  Method::Get,
            url:     url.into(),
            headers: Headers::new(),
            body:    None,
            initiator_origin: None,
            mode: RequestMode::Navigate,
        }
    }

    pub fn post(url: impl Into<String>, body: impl Into<Bytes>) -> Self {
        Self {
            method:  Method::Post,
            url:     url.into(),
            headers: Headers::new(),
            body:    Some(body.into()),
            initiator_origin: None,
            mode: RequestMode::Navigate,
        }
    }

    pub fn with_header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(name, value);
        self
    }
}

// ---------------------------------------------------------------------------
// Response
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct Response {
    pub status:  u16,
    pub headers: Headers,
    pub body:    Bytes,
    /// Protocol version used for this response.
    pub version: HttpVersion,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpVersion {
    Http1_1,
    Http2,
    Http3,
}

impl std::fmt::Display for HttpVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HttpVersion::Http1_1 => write!(f, "HTTP/1.1"),
            HttpVersion::Http2   => write!(f, "HTTP/2"),
            HttpVersion::Http3   => write!(f, "HTTP/3"),
        }
    }
}

impl Response {
    pub fn ok(&self) -> bool { self.status >= 200 && self.status < 300 }
    pub fn redirect(&self) -> bool { self.status >= 300 && self.status < 400 }
    pub fn body_text(&self) -> std::borrow::Cow<'_, str> {
        String::from_utf8_lossy(&self.body)
    }
}
