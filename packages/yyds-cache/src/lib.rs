use yyds_types::{DsValue, DsError};
use lru::LruCache;
use std::num::NonZeroUsize;
use tokio::sync::Mutex;
use std::sync::Arc;

/// Cache entry
#[derive(Debug, Clone)]
struct CacheEntry {
    value: DsValue,
    // Add metadata like TTL if needed
}

/// Cache manager for hot data management (Blaze/Nova levels)
pub struct CacheManager {
    // We use a Mutex-wrapped LRU cache. 
    // In a high-concurrency scenario, we might use multiple shards.
    cache: Arc<Mutex<LruCache<String, CacheEntry>>>,
}

impl CacheManager {
    pub fn new(capacity: usize) -> Self {
        let cap = NonZeroUsize::new(capacity).expect("capacity must be non-zero");
        Self {
            cache: Arc::new(Mutex::new(LruCache::new(cap))),
        }
    }

    /// Put a value into the cache
    pub async fn put(&self, key: String, value: DsValue) {
        let mut cache = self.cache.lock().await;
        cache.put(key, CacheEntry { value });
    }

    /// Get a value from the cache
    pub async fn get(&self, key: &str) -> Option<DsValue> {
        let mut cache = self.cache.lock().await;
        cache.get(key).map(|entry| entry.value.clone())
    }

    /// Invalidate a key
    pub async fn invalidate(&self, key: &str) {
        let mut cache = self.cache.lock().await;
        cache.pop(key);
    }

    /// Clear the cache
    pub async fn clear(&self) {
        let mut cache = self.cache.lock().await;
        cache.clear();
    }
}
