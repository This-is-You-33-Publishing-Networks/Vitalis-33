//! Collective Intelligence — Vitalis v990
//!
//! Multiple Vitalis instances collaborate: shared knowledge, distributed evolution,
//! consensus-based decision making across compiler instances.

use std::sync::{LazyLock, Mutex};
use std::collections::HashMap;

static STATE: LazyLock<Mutex<CollectiveState>> = LazyLock::new(|| Mutex::new(CollectiveState::default()));

#[derive(Default)]
struct CollectiveState {
    knowledge: HashMap<u32, Vec<String>>,
    total_shared: u64,
}

pub struct CollectiveIntelligence;

impl CollectiveIntelligence {
    pub fn share_knowledge(instance: u32, data: &str) {
        let mut s = STATE.lock().unwrap();
        s.knowledge.entry(instance).or_default().push(data.to_string());
        s.total_shared += 1;
    }

    pub fn consensus() -> String {
        let s = STATE.lock().unwrap();
        if s.knowledge.is_empty() { return "no_consensus".to_string(); }
        let all: Vec<&String> = s.knowledge.values().flat_map(|v| v.iter()).collect();
        if all.is_empty() { return "no_consensus".to_string(); }
        // Simple majority: pick most frequent
        let mut freq: HashMap<&str, usize> = HashMap::new();
        for item in &all { *freq.entry(item.as_str()).or_default() += 1; }
        freq.into_iter().max_by_key(|&(_, c)| c).map(|(k, _)| k.to_string()).unwrap_or_default()
    }

    pub fn instance_count() -> usize { STATE.lock().unwrap().knowledge.len() }
    pub fn total_shared() -> u64 { STATE.lock().unwrap().total_shared }
    pub fn reset() { *STATE.lock().unwrap() = CollectiveState::default(); }
}

#[unsafe(no_mangle)]
pub extern "C" fn ci_share(instance: i64, data_len: i64) -> i64 { let mut s = STATE.lock().unwrap(); s.knowledge.entry(instance as u32).or_default().push(format!("d{}", data_len)); s.total_shared += 1; 1 }
#[unsafe(no_mangle)]
pub extern "C" fn ci_consensus() -> i64 { CollectiveIntelligence::consensus().len() as i64 }
#[unsafe(no_mangle)]
pub extern "C" fn ci_instances() -> i64 { CollectiveIntelligence::instance_count() as i64 }
#[unsafe(no_mangle)]
pub extern "C" fn ci_total() -> i64 { CollectiveIntelligence::total_shared() as i64 }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_share() { CollectiveIntelligence::reset(); CollectiveIntelligence::share_knowledge(1, "opt_a"); assert_eq!(CollectiveIntelligence::total_shared(), 1); }
    #[test] fn test_consensus() { CollectiveIntelligence::reset(); CollectiveIntelligence::share_knowledge(1, "x"); CollectiveIntelligence::share_knowledge(2, "x"); assert_eq!(CollectiveIntelligence::consensus(), "x"); }
    #[test] fn test_instances() { CollectiveIntelligence::reset(); CollectiveIntelligence::share_knowledge(1, "a"); CollectiveIntelligence::share_knowledge(2, "b"); assert_eq!(CollectiveIntelligence::instance_count(), 2); }
    #[test] fn test_empty_consensus() { CollectiveIntelligence::reset(); assert_eq!(CollectiveIntelligence::consensus(), "no_consensus"); }
    #[test] fn test_ffi() { CollectiveIntelligence::reset(); assert_eq!(ci_total(), 0); }
}
