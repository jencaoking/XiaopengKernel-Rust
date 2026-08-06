//! LRU HTTP response cache.

use lru::LruCache;
use std::num::NonZeroUsize;
use crate::request::Response;

/// Cached entry keyed by `method + url`.
#[derive(Clone)]
pub struct CachedResponse {
    pub response: Response,
    /// When this entry was inserted (seconds since UNIX epoch).
    pub inserted_at: u64,
    /// max-age in seconds, if provided by Cache-Control header.
    pub max_age: Option<u64>,
    /// Whether no-cache or must-revalidate is present.
    pub must_revalidate: bool,
    /// ETag header value, if any.
    pub etag: Option<String>,
    /// Last-Modified header value, if any.
    pub last_modified: Option<String>,
}

impl CachedResponse {
    pub fn is_fresh(&self) -> bool {
        if self.must_revalidate {
            return false;
        }
        if let Some(max_age) = self.max_age {
            let now = now_secs();
            now < self.inserted_at + max_age
        } else {
            // No expiry info → treat as stale (always re-validate).
            false
        }
    }
}

fn now_secs() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// In-memory LRU cache for HTTP responses.
pub struct ResourceCache {
    inner: LruCache<String, CachedResponse>,
}

impl ResourceCache {
    pub fn new(capacity: usize) -> Self {
        let cap = NonZeroUsize::new(capacity).unwrap_or(NonZeroUsize::new(128).expect("Unwrap failed"));
        Self { inner: LruCache::new(cap) }
    }

    fn key(method: &str, url: &str) -> String {
        format!("{method}:{url}")
    }

    pub fn get(&mut self, method: &str, url: &str) -> Option<&CachedResponse> {
        let entry = self.get_entry(method, url)?;
        if entry.is_fresh() { Some(entry) } else { None }
    }

    pub fn get_entry(&mut self, method: &str, url: &str) -> Option<&CachedResponse> {
        self.inner.get(&Self::key(method, url))
    }

    pub fn insert(&mut self, method: &str, url: &str, response: Response) {
        let mut max_age = None;
        let mut no_store = false;
        let mut must_revalidate = false;

        if let Some(cc) = response.headers.get("cache-control") {
            for part in cc.split(',') {
                let part = part.trim().to_lowercase();
                if part == "no-store" {
                    no_store = true;
                } else if part == "no-cache" || part == "must-revalidate" {
                    must_revalidate = true;
                } else if let Some(s) = part.strip_prefix("max-age=") {
                    max_age = s.parse::<u64>().ok();
                }
            }
        }

        if no_store {
            return;
        }

        let etag = response.headers.get("etag").map(|s| s.to_string());
        let last_modified = response.headers.get("last-modified").map(|s| s.to_string());

        let entry = CachedResponse {
            response,
            inserted_at: now_secs(),
            max_age,
            must_revalidate,
            etag,
            last_modified,
        };
        self.inner.put(Self::key(method, url), entry);
    }

    pub fn invalidate(&mut self, method: &str, url: &str) {
        self.inner.pop(&Self::key(method, url));
    }

    pub fn clear(&mut self) {
        self.inner.clear();
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }
}

impl Default for ResourceCache {
    fn default() -> Self {
        Self::new(128)
    }
}
