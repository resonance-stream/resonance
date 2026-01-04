//! Mock Redis store for testing cache operations
//!
//! Provides a [`MockRedisStore`] that simulates Redis key-value operations
//! in-memory for testing without a real Redis server.
//!
//! # Lock Poisoning Recovery
//!
//! This implementation uses `unwrap_or_else(|e| e.into_inner())` when acquiring
//! locks to recover from poisoned locks. If a test panics while holding a lock,
//! subsequent tests can still access the store rather than failing with a
//! `PoisonError`. This prevents cascading test failures.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

/// Mock Redis client for testing cache operations
///
/// This struct provides an in-memory key-value store that mimics Redis
/// behavior for testing without requiring a real Redis server.
///
/// # Thread Safety
///
/// `MockRedisStore` uses `Arc<RwLock<...>>` internally, so it can be safely
/// cloned and shared across threads. All clones share the same underlying store.
///
/// # Example
///
/// ```rust
/// use resonance_test_utils::MockRedisStore;
///
/// let store = MockRedisStore::new();
/// store.setex("key1", 3600, "value1".to_string());
///
/// assert!(store.exists("key1"));
/// assert_eq!(store.get("key1"), Some("value1".to_string()));
/// ```
pub struct MockRedisStore {
    store: Arc<RwLock<HashMap<String, MockRedisEntry>>>,
}

/// Entry in the mock Redis store with expiration tracking
struct MockRedisEntry {
    value: String,
    expires_at: Option<Instant>,
}

impl MockRedisStore {
    /// Create a new mock Redis store
    pub fn new() -> Self {
        Self {
            store: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Set a key with expiration (SETEX equivalent)
    ///
    /// # Arguments
    ///
    /// * `key` - The key to set
    /// * `seconds` - TTL in seconds (0 or negative means no expiration)
    /// * `value` - The value to store
    pub fn setex(&self, key: &str, seconds: i64, value: String) {
        let expires_at = if seconds > 0 {
            Some(Instant::now() + Duration::from_secs(seconds as u64))
        } else {
            None
        };

        let mut store = self.store.write().unwrap_or_else(|e| e.into_inner());
        store.insert(key.to_string(), MockRedisEntry { value, expires_at });
    }

    /// Get a key value (GET equivalent)
    ///
    /// Returns `None` if the key doesn't exist or has expired.
    pub fn get(&self, key: &str) -> Option<String> {
        let store = self.store.read().unwrap_or_else(|e| e.into_inner());
        store.get(key).and_then(|entry| {
            if let Some(expires_at) = entry.expires_at {
                if Instant::now() > expires_at {
                    return None;
                }
            }
            Some(entry.value.clone())
        })
    }

    /// Delete a key (DEL equivalent)
    ///
    /// Returns `true` if the key existed and was deleted.
    pub fn del(&self, key: &str) -> bool {
        let mut store = self.store.write().unwrap_or_else(|e| e.into_inner());
        store.remove(key).is_some()
    }

    /// Check if a key exists (EXISTS equivalent)
    ///
    /// Returns `false` if the key doesn't exist or has expired.
    pub fn exists(&self, key: &str) -> bool {
        let store = self.store.read().unwrap_or_else(|e| e.into_inner());
        if let Some(entry) = store.get(key) {
            if let Some(expires_at) = entry.expires_at {
                return Instant::now() <= expires_at;
            }
            return true;
        }
        false
    }

    /// Get all keys matching a pattern (KEYS equivalent, simplified)
    ///
    /// Note: This is a simplified implementation that checks if the key
    /// contains the pattern (with wildcards removed). For more complex
    /// pattern matching, consider using a real Redis instance.
    pub fn keys(&self, pattern: &str) -> Vec<String> {
        let store = self.store.read().unwrap_or_else(|e| e.into_inner());
        let pattern = pattern.replace('*', "");
        store
            .keys()
            .filter(|k| k.contains(&pattern))
            .cloned()
            .collect()
    }

    /// Clear all keys (FLUSHALL equivalent)
    pub fn flush_all(&self) {
        let mut store = self.store.write().unwrap_or_else(|e| e.into_inner());
        store.clear();
    }

    /// Get the number of keys in the store
    ///
    /// Note: This includes expired keys that haven't been cleaned up yet.
    pub fn len(&self) -> usize {
        let store = self.store.read().unwrap_or_else(|e| e.into_inner());
        store.len()
    }

    /// Check if store is empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Set a key without expiration (SET equivalent)
    pub fn set(&self, key: &str, value: String) {
        self.setex(key, 0, value);
    }

    /// Get TTL remaining for a key in seconds (TTL equivalent)
    ///
    /// Returns:
    /// - `Some(seconds)` if the key exists and has an expiration
    /// - `Some(-1)` if the key exists but has no expiration
    /// - `None` if the key doesn't exist or has expired
    pub fn ttl(&self, key: &str) -> Option<i64> {
        let store = self.store.read().unwrap_or_else(|e| e.into_inner());
        store.get(key).and_then(|entry| {
            match entry.expires_at {
                Some(expires_at) => {
                    let now = Instant::now();
                    if now > expires_at {
                        None // Expired
                    } else {
                        Some((expires_at - now).as_secs() as i64)
                    }
                }
                None => Some(-1), // No expiration
            }
        })
    }
}

impl Default for MockRedisStore {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for MockRedisStore {
    fn clone(&self) -> Self {
        Self {
            store: self.store.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_redis_store_new() {
        let store = MockRedisStore::new();
        assert!(store.is_empty());
    }

    #[test]
    fn test_mock_redis_store_setex_and_get() {
        let store = MockRedisStore::new();
        store.setex("key1", 3600, "value1".to_string());

        assert!(store.exists("key1"));
        assert_eq!(store.get("key1"), Some("value1".to_string()));
        assert_eq!(store.len(), 1);
    }

    #[test]
    fn test_mock_redis_store_set() {
        let store = MockRedisStore::new();
        store.set("key1", "value1".to_string());

        assert!(store.exists("key1"));
        assert_eq!(store.get("key1"), Some("value1".to_string()));
        assert_eq!(store.ttl("key1"), Some(-1)); // No expiration
    }

    #[test]
    fn test_mock_redis_store_del() {
        let store = MockRedisStore::new();
        store.setex("key1", 3600, "value1".to_string());
        assert!(store.exists("key1"));

        assert!(store.del("key1"));
        assert!(!store.exists("key1"));
        assert_eq!(store.get("key1"), None);
    }

    #[test]
    fn test_mock_redis_store_del_nonexistent() {
        let store = MockRedisStore::new();
        assert!(!store.del("nonexistent"));
    }

    #[test]
    fn test_mock_redis_store_keys() {
        let store = MockRedisStore::new();
        store.setex("prefetch:user1:track1", 3600, "data1".to_string());
        store.setex("prefetch:user1:track2", 3600, "data2".to_string());
        store.setex("other:key", 3600, "data3".to_string());

        let keys = store.keys("prefetch:user1");
        assert_eq!(keys.len(), 2);
    }

    #[test]
    fn test_mock_redis_store_flush_all() {
        let store = MockRedisStore::new();
        store.setex("key1", 3600, "value1".to_string());
        store.setex("key2", 3600, "value2".to_string());
        assert_eq!(store.len(), 2);

        store.flush_all();
        assert!(store.is_empty());
    }

    #[test]
    fn test_mock_redis_store_clone() {
        let store = MockRedisStore::new();
        store.setex("key1", 3600, "value1".to_string());

        let store2 = store.clone();
        assert_eq!(store2.get("key1"), Some("value1".to_string()));

        // Changes in one should reflect in the other (shared Arc)
        store2.setex("key2", 3600, "value2".to_string());
        assert!(store.exists("key2"));
    }

    #[test]
    fn test_mock_redis_store_ttl() {
        let store = MockRedisStore::new();
        store.setex("key1", 3600, "value1".to_string());

        let ttl = store.ttl("key1").unwrap();
        assert!(ttl > 0 && ttl <= 3600);
    }

    #[test]
    fn test_mock_redis_store_ttl_nonexistent() {
        let store = MockRedisStore::new();
        assert_eq!(store.ttl("nonexistent"), None);
    }

    #[test]
    fn test_mock_redis_store_default() {
        let store = MockRedisStore::default();
        assert!(store.is_empty());
    }

    #[test]
    fn test_mock_redis_store_overwrite() {
        let store = MockRedisStore::new();
        store.setex("key1", 3600, "value1".to_string());
        store.setex("key1", 3600, "value2".to_string());

        assert_eq!(store.get("key1"), Some("value2".to_string()));
        assert_eq!(store.len(), 1);
    }
}
