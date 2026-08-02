//! LRU HTTP response cache.

use lru::LruCache;
use std::num::NonZeroUsize;
use bytes::Bytes;
use crate::request::{Response, HttpVersion};

/// Cached entry keyed by `method + url`.
#[derive(Clone)]
pub struct CachedResponse {
    pub response: Response,
    /// When this entry was inserted (seconds since UNIX epoch).
    pub inserted_at: u64,
    /// max-age in seconds, if provided by Cache-Control header.
    pub max_age: Option<u64>,
}

impl CachedResponse {
    pub fn is_fresh(&self) -> bool {
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
        let cap = NonZeroUsize::new(capacity).unwrap_or(NonZeroUsize::new(128).unwrap());
        Self { inner: LruCache::new(cap) }
    }

    fn key(method: &str, url: &str) -> String {
        format!("{method}:{url}")
    }

    pub fn get(&mut self, method: &str, url: &str) -> Option<&CachedResponse> {
        let k = Self::key(method, url);
        let entry = self.inner.get(&k)?;
        if entry.is_fresh() { Some(entry) } else { None }
    }

    pub fn insert(&mut self, method: &str, url: &str, response: Response) {
        // Parse max-age from Cache-Control header.
        let max_age = response.headers.get("cache-control").and_then(|v| {
            v.split(',').find_map(|part| {
                let part = part.trim();
                if let Some(s) = part.strip_prefix("max-age=") {
                    s.parse::<u64>().ok()
                } else {
                    None
                }
            })
        });
        let entry = CachedResponse {
            response,
            inserted_at: now_secs(),
            max_age,
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
