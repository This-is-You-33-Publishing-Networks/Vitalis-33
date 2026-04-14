//! Collective Memory — Vitalis v1035
//!
//! Shared knowledge base across all compilation sessions,
//! persisting learned patterns and optimization strategies.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<CmemState>> = LazyLock::new(|| Mutex::new(CmemState::default()));

#[derive(Default)]
struct CmemState {
    memories: Vec<(String, String, u64)>,
    access_count: u64,
    decay_cycles: u64,
}

pub struct CollectiveMemory;

impl CollectiveMemory {
    pub fn remember(key: &str, value: &str) -> usize {
        let mut s = STATE.lock().unwrap();
        if let Some(m) = s.memories.iter_mut().find(|(k, _, _)| k == key) {
            m.1 = value.to_string();
            m.2 += 1;
        } else {
            s.memories.push((key.to_string(), value.to_string(), 1));
        }
        s.memories.len()
    }

    pub fn recall(key: &str) -> Option<String> {
        let mut s = STATE.lock().unwrap();
        s.access_count += 1;
        s.memories.iter_mut().find(|(k, _, _)| k == key).map(|(_, v, c)| { *c += 1; v.clone() })
    }

    pub fn decay() -> usize {
        let mut s = STATE.lock().unwrap();
        s.decay_cycles += 1;
        let before = s.memories.len();
        for m in s.memories.iter_mut() { m.2 = m.2.saturating_sub(1); }
        s.memories.retain(|(_, _, c)| *c > 0);
        before - s.memories.len()
    }

    pub fn strongest_memory() -> Option<String> {
        let s = STATE.lock().unwrap();
        s.memories.iter().max_by_key(|(_, _, c)| *c).map(|(k, _, _)| k.clone())
    }

    pub fn size() -> usize { STATE.lock().unwrap().memories.len() }
    pub fn total_accesses() -> u64 { STATE.lock().unwrap().access_count }
    pub fn reset() { *STATE.lock().unwrap() = CmemState::default(); }
}

#[unsafe(no_mangle)]
pub extern "C" fn cmem_remember(key: i64) -> i64 { CollectiveMemory::remember(&format!("k{}", key), "val") as i64 }
#[unsafe(no_mangle)]
pub extern "C" fn cmem_recall(key: i64) -> i64 { if CollectiveMemory::recall(&format!("k{}", key)).is_some() { 1 } else { 0 } }
#[unsafe(no_mangle)]
pub extern "C" fn cmem_size() -> i64 { CollectiveMemory::size() as i64 }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_remember() { CollectiveMemory::reset(); assert_eq!(CollectiveMemory::remember("k1", "v1"), 1); }
    #[test] fn test_recall() { CollectiveMemory::reset(); CollectiveMemory::remember("k1", "v1"); assert_eq!(CollectiveMemory::recall("k1"), Some("v1".to_string())); }
    #[test] fn test_recall_miss() { CollectiveMemory::reset(); assert!(CollectiveMemory::recall("nope").is_none()); }
    #[test] fn test_decay() { CollectiveMemory::reset(); CollectiveMemory::remember("k1", "v1"); CollectiveMemory::recall("k1"); let forgotten = CollectiveMemory::decay(); assert_eq!(forgotten, 0); let forgotten2 = CollectiveMemory::decay(); assert_eq!(forgotten2, 1); }
    #[test] fn test_strongest() { CollectiveMemory::reset(); CollectiveMemory::remember("a", "x"); CollectiveMemory::remember("b", "y"); CollectiveMemory::recall("b"); assert_eq!(CollectiveMemory::strongest_memory(), Some("b".to_string())); }
    #[test] fn test_size() { CollectiveMemory::reset(); CollectiveMemory::remember("a", "x"); assert_eq!(CollectiveMemory::size(), 1); }
    #[test] fn test_ffi() { CollectiveMemory::reset(); assert_eq!(cmem_size(), 0); }
}
