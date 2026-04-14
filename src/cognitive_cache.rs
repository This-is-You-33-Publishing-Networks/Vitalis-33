//! Cognitive Cache — Vitalis v1025
//!
//! Remembers and recalls compilation insights across sessions and projects.
//! A persistent memory of optimization strategies and their outcomes.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<CcacheState>> = LazyLock::new(|| Mutex::new(CcacheState::default()));

#[derive(Default)]
struct CcacheState {
    entries: Vec<(String, String, f64)>,
    hits: u64,
    misses: u64,
}

pub struct CognitiveCache;

impl CognitiveCache {
    pub fn store(key: &str, insight: &str, relevance: f64) -> usize {
        let mut s = STATE.lock().unwrap();
        if let Some(e) = s.entries.iter_mut().find(|(k, _, _)| k == key) {
            e.1 = insight.to_string();
            e.2 = relevance;
        } else {
            s.entries.push((key.to_string(), insight.to_string(), relevance));
        }
        s.entries.len()
    }

    pub fn recall(key: &str) -> Option<String> {
        let mut s = STATE.lock().unwrap();
        let found = s.entries.iter().find(|(k, _, _)| k == key).map(|e| e.1.clone());
        if found.is_some() {
            s.hits += 1;
        } else {
            s.misses += 1;
        }
        found
    }

    pub fn hit_rate() -> f64 {
        let s = STATE.lock().unwrap();
        let total = s.hits + s.misses;
        if total == 0 { 0.0 } else { s.hits as f64 / total as f64 }
    }

    pub fn evict_below(threshold: f64) -> usize {
        let mut s = STATE.lock().unwrap();
        let before = s.entries.len();
        s.entries.retain(|(_, _, r)| *r >= threshold);
        before - s.entries.len()
    }

    pub fn size() -> usize { STATE.lock().unwrap().entries.len() }
    pub fn reset() { *STATE.lock().unwrap() = CcacheState::default(); }
}

#[unsafe(no_mangle)]
pub extern "C" fn ccache_store(key: i64, relevance: f64) -> i64 { CognitiveCache::store(&format!("k{}", key), "insight", relevance) as i64 }
#[unsafe(no_mangle)]
pub extern "C" fn ccache_recall(key: i64) -> i64 { if CognitiveCache::recall(&format!("k{}", key)).is_some() { 1 } else { 0 } }
#[unsafe(no_mangle)]
pub extern "C" fn ccache_hit_rate() -> f64 { CognitiveCache::hit_rate() }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_store() { CognitiveCache::reset(); assert_eq!(CognitiveCache::store("k1", "opt", 0.9), 1); }
    #[test] fn test_recall_hit() { CognitiveCache::reset(); CognitiveCache::store("k1", "opt", 0.9); assert_eq!(CognitiveCache::recall("k1"), Some("opt".to_string())); }
    #[test] fn test_recall_miss() { CognitiveCache::reset(); assert!(CognitiveCache::recall("nope").is_none()); }
    #[test] fn test_hit_rate() { CognitiveCache::reset(); CognitiveCache::store("k1", "x", 1.0); CognitiveCache::recall("k1"); CognitiveCache::recall("nope"); assert!((CognitiveCache::hit_rate() - 0.5).abs() < 0.01); }
    #[test] fn test_evict() { CognitiveCache::reset(); CognitiveCache::store("a", "x", 0.1); CognitiveCache::store("b", "y", 0.9); assert_eq!(CognitiveCache::evict_below(0.5), 1); assert_eq!(CognitiveCache::size(), 1); }
    #[test] fn test_update() { CognitiveCache::reset(); CognitiveCache::store("k", "old", 0.5); CognitiveCache::store("k", "new", 0.9); assert_eq!(CognitiveCache::recall("k"), Some("new".to_string())); }
    #[test] fn test_ffi() { CognitiveCache::reset(); assert!((ccache_hit_rate() - 0.0).abs() < 0.01); }
}
