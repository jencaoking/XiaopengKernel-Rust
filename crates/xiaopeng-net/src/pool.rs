//! Connection pooling for HTTP/1.1, HTTP/2, and HTTP/3.

use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant};

pub struct PoolEntry<T> {
    pub sender: T,
    pub last_used: Instant,
}

pub struct ConnectionPool<T> {
    max_per_host: usize,
    max_idle_time: Duration,
    connections: HashMap<String, VecDeque<PoolEntry<T>>>,
}

impl<T> ConnectionPool<T> {
    pub fn new(max_per_host: usize, max_idle_time: Duration) -> Self {
        Self {
            max_per_host,
            max_idle_time,
            connections: HashMap::new(),
        }
    }

    /// Retrieve a valid connection from the pool.
    pub fn take(&mut self, host: &str) -> Option<T> {
        let now = Instant::now();
        if let Some(queue) = self.connections.get_mut(host) {
            while let Some(entry) = queue.pop_back() {
                if now.duration_since(entry.last_used) <= self.max_idle_time {
                    return Some(entry.sender);
                }
            }
        }
        None
    }

    /// Add a connection back to the pool.
    pub fn put(&mut self, host: &str, sender: T) {
        let queue = self.connections.entry(host.to_string()).or_default();
        if queue.len() < self.max_per_host {
            queue.push_back(PoolEntry {
                sender,
                last_used: Instant::now(),
            });
        }
    }

    /// For multiplexed protocols (H2, H3), we can just peek and clone if available.
    /// This requires `T: Clone`.
    pub fn peek_clone(&mut self, host: &str) -> Option<T>
    where
        T: Clone,
    {
        let now = Instant::now();
        if let Some(queue) = self.connections.get_mut(host) {
            // Remove stale connections first
            queue.retain(|e| now.duration_since(e.last_used) <= self.max_idle_time);
            
            if let Some(entry) = queue.back_mut() {
                entry.last_used = now;
                return Some(entry.sender.clone());
            }
        }
        None
    }

    /// Evict a specific host's connections (e.g., if we detect the connection is dead).
    pub fn remove_host(&mut self, host: &str) {
        self.connections.remove(host);
    }
}
