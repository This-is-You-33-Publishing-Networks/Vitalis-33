//! Cache — v373
//! LRU/LFU cache with TTL, hit/miss tracking, eviction policies, capacity management.

use std::collections::HashMap;

#[derive(Debug, Clone)]
struct CacheEntry {
    value: i64,
    frequency: u64,
    last_access: u64,
    inserted_at: u64,
    ttl: Option<u64>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum EvictionPolicy {
    Lru,
    Lfu,
    Fifo,
}

#[derive(Debug)]
pub struct Cache {
    entries: HashMap<i64, CacheEntry>,
    capacity: usize,
    hits: u64,
    misses: u64,
    evictions: u64,
    clock: u64,
    policy: EvictionPolicy,
}

impl Cache {
    pub fn new(capacity: usize) -> Self {
        Self {
            entries: HashMap::new(),
            capacity: capacity.max(1),
            hits: 0,
            misses: 0,
            evictions: 0,
            clock: 0,
            policy: EvictionPolicy::Lru,
        }
    }

    pub fn with_policy(capacity: usize, policy: EvictionPolicy) -> Self {
        let mut c = Self::new(capacity);
        c.policy = policy;
        c
    }

    pub fn put(&mut self, key: i64, value: i64) {
        self.clock += 1;
        if self.entries.contains_key(&key) {
            let entry = self.entries.get_mut(&key).unwrap();
            entry.value = value;
            entry.last_access = self.clock;
            entry.frequency += 1;
            return;
        }
        if self.entries.len() >= self.capacity {
            self.evict();
        }
        self.entries.insert(key, CacheEntry {
            value,
            frequency: 1,
            last_access: self.clock,
            inserted_at: self.clock,
            ttl: None,
        });
    }

    pub fn put_with_ttl(&mut self, key: i64, value: i64, ttl: u64) {
        self.clock += 1;
        if self.entries.len() >= self.capacity && !self.entries.contains_key(&key) {
            self.evict();
        }
        self.entries.insert(key, CacheEntry {
            value,
            frequency: 1,
            last_access: self.clock,
            inserted_at: self.clock,
            ttl: Some(self.clock + ttl),
        });
    }

    pub fn get(&mut self, key: i64) -> Option<i64> {
        self.clock += 1;
        self.expire_entries();
        if let Some(entry) = self.entries.get_mut(&key) {
            entry.last_access = self.clock;
            entry.frequency += 1;
            self.hits += 1;
            Some(entry.value)
        } else {
            self.misses += 1;
            None
        }
    }

    pub fn remove(&mut self, key: i64) -> bool {
        self.entries.remove(&key).is_some()
    }

    pub fn contains(&self, key: i64) -> bool {
        self.entries.contains_key(&key)
    }

    pub fn size(&self) -> usize {
        self.entries.len()
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.hits = 0;
        self.misses = 0;
        self.evictions = 0;
    }

    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 { 0.0 } else { self.hits as f64 / total as f64 }
    }

    pub fn eviction_count(&self) -> u64 {
        self.evictions
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    fn evict(&mut self) {
        let victim = match self.policy {
            EvictionPolicy::Lru => {
                self.entries.iter()
                    .min_by_key(|(_, e)| e.last_access)
                    .map(|(k, _)| *k)
            }
            EvictionPolicy::Lfu => {
                self.entries.iter()
                    .min_by_key(|(_, e)| (e.frequency, e.last_access))
                    .map(|(k, _)| *k)
            }
            EvictionPolicy::Fifo => {
                self.entries.iter()
                    .min_by_key(|(_, e)| e.inserted_at)
                    .map(|(k, _)| *k)
            }
        };
        if let Some(key) = victim {
            self.entries.remove(&key);
            self.evictions += 1;
        }
    }

    fn expire_entries(&mut self) {
        let clock = self.clock;
        self.entries.retain(|_, e| {
            e.ttl.map_or(true, |ttl| clock <= ttl)
        });
    }
}

use std::sync::Mutex;
use std::sync::LazyLock;
static CACHE: LazyLock<Mutex<Cache>> = LazyLock::new(|| Mutex::new(Cache::new(1024)));

#[unsafe(no_mangle)]
pub extern "C" fn slang_cache_create(capacity: i64) -> i64 {
    *CACHE.lock().unwrap() = Cache::new(capacity.max(1) as usize);
    1
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_cache_put(key: i64, value: i64) -> i64 {
    CACHE.lock().unwrap().put(key, value);
    1
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_cache_get(key: i64) -> i64 {
    CACHE.lock().unwrap().get(key).unwrap_or(-1)
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_cache_remove(key: i64) -> i64 {
    if CACHE.lock().unwrap().remove(key) { 1 } else { 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_cache_contains(key: i64) -> i64 {
    if CACHE.lock().unwrap().contains(key) { 1 } else { 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_cache_size() -> i64 {
    CACHE.lock().unwrap().size() as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_cache_clear() -> i64 {
    CACHE.lock().unwrap().clear();
    1
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_cache_hit_rate() -> f64 {
    CACHE.lock().unwrap().hit_rate()
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_cache_eviction_count() -> i64 {
    CACHE.lock().unwrap().eviction_count() as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_cache_capacity() -> i64 {
    CACHE.lock().unwrap().capacity() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_cache() {
        let cache = Cache::new(10);
        assert_eq!(cache.size(), 0);
        assert_eq!(cache.capacity(), 10);
    }

    #[test]
    fn test_put_get() {
        let mut cache = Cache::new(10);
        cache.put(1, 100);
        assert_eq!(cache.get(1), Some(100));
    }

    #[test]
    fn test_get_miss() {
        let mut cache = Cache::new(10);
        assert_eq!(cache.get(999), None);
        assert_eq!(cache.misses, 1);
    }

    #[test]
    fn test_overwrite() {
        let mut cache = Cache::new(10);
        cache.put(1, 100);
        cache.put(1, 200);
        assert_eq!(cache.get(1), Some(200));
    }

    #[test]
    fn test_remove() {
        let mut cache = Cache::new(10);
        cache.put(1, 100);
        assert!(cache.remove(1));
        assert!(!cache.contains(1));
    }

    #[test]
    fn test_remove_nonexistent() {
        let mut cache = Cache::new(10);
        assert!(!cache.remove(999));
    }

    #[test]
    fn test_contains() {
        let mut cache = Cache::new(10);
        cache.put(5, 50);
        assert!(cache.contains(5));
        assert!(!cache.contains(6));
    }

    #[test]
    fn test_lru_eviction() {
        let mut cache = Cache::new(3);
        cache.put(1, 10);
        cache.put(2, 20);
        cache.put(3, 30);
        cache.get(1); // touch 1
        cache.put(4, 40); // should evict 2 (LRU)
        assert!(cache.contains(1));
        assert!(!cache.contains(2));
        assert!(cache.contains(3));
        assert!(cache.contains(4));
    }

    #[test]
    fn test_lfu_eviction() {
        let mut cache = Cache::with_policy(3, EvictionPolicy::Lfu);
        cache.put(1, 10);
        cache.put(2, 20);
        cache.put(3, 30);
        cache.get(1); // freq 1 → 2
        cache.get(1); // freq 2 → 3
        cache.get(2); // freq 1 → 2
        cache.put(4, 40); // should evict 3 (lowest freq)
        assert!(cache.contains(1));
        assert!(cache.contains(2));
        assert!(!cache.contains(3));
        assert!(cache.contains(4));
    }

    #[test]
    fn test_fifo_eviction() {
        let mut cache = Cache::with_policy(3, EvictionPolicy::Fifo);
        cache.put(1, 10);
        cache.put(2, 20);
        cache.put(3, 30);
        cache.put(4, 40); // evicts 1 (inserted first)
        assert!(!cache.contains(1));
        assert!(cache.contains(4));
    }

    #[test]
    fn test_hit_rate() {
        let mut cache = Cache::new(10);
        cache.put(1, 10);
        cache.get(1); // hit
        cache.get(2); // miss
        let rate = cache.hit_rate();
        assert!((rate - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_hit_rate_no_access() {
        let cache = Cache::new(10);
        assert_eq!(cache.hit_rate(), 0.0);
    }

    #[test]
    fn test_eviction_count() {
        let mut cache = Cache::new(2);
        cache.put(1, 1);
        cache.put(2, 2);
        cache.put(3, 3); // evict
        cache.put(4, 4); // evict
        assert_eq!(cache.eviction_count(), 2);
    }

    #[test]
    fn test_clear() {
        let mut cache = Cache::new(10);
        cache.put(1, 1);
        cache.put(2, 2);
        cache.clear();
        assert_eq!(cache.size(), 0);
        assert_eq!(cache.hits, 0);
        assert_eq!(cache.misses, 0);
    }

    #[test]
    fn test_ttl_expiry() {
        let mut cache = Cache::new(10);
        cache.put_with_ttl(1, 100, 5);
        assert_eq!(cache.get(1), Some(100));
        // Advance clock past TTL
        cache.clock = 100;
        assert_eq!(cache.get(1), None);
    }

    #[test]
    fn test_capacity_one() {
        let mut cache = Cache::new(1);
        cache.put(1, 10);
        cache.put(2, 20);
        assert_eq!(cache.size(), 1);
        assert!(cache.contains(2));
        assert!(!cache.contains(1));
    }

    #[test]
    fn test_size_tracking() {
        let mut cache = Cache::new(100);
        for i in 0..50 {
            cache.put(i, i * 10);
        }
        assert_eq!(cache.size(), 50);
    }

    #[test]
    fn test_frequency_update() {
        let mut cache = Cache::new(10);
        cache.put(1, 10);
        cache.get(1);
        cache.get(1);
        cache.get(1);
        assert_eq!(cache.entries[&1].frequency, 4); // 1 initial + 3 gets
    }

    #[test]
    fn test_multiple_operations() {
        let mut cache = Cache::new(5);
        for i in 0..10 {
            cache.put(i, i * 100);
        }
        assert_eq!(cache.size(), 5);
        assert_eq!(cache.eviction_count(), 5);
    }

    #[test]
    fn test_zero_capacity_normalized() {
        let cache = Cache::new(0);
        assert_eq!(cache.capacity(), 1);
    }
}
