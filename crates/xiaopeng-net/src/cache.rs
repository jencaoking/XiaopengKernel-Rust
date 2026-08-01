//! LRU Resource Cache

use std::collections::HashMap;

pub struct ResourceCache {
    entries: HashMap<String, Vec<u8>>,
}

impl ResourceCache {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    pub fn get(&self, key: &str) -> Option<&Vec<u8>> {
        self.entries.get(key)
    }

    pub fn insert(&mut self, key: impl Into<String>, value: Vec<u8>) {
        self.entries.insert(key.into(), value);
    }
}

impl Default for ResourceCache {
    fn default() -> Self {
        Self::new()
    }
}
