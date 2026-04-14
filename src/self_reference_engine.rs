//! Self-Reference Engine — Vitalis v1217
//!
//! Safe self-reference: programs inspect and modify own source.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<SREState>> = LazyLock::new(|| Mutex::new(SREState::default()));

#[derive(Default)]
struct SREState {
    source_snapshots: Vec<String>,
    modifications: Vec<String>,
    safety_checks: u64,
}

pub struct SelfReferenceEngine;

impl SelfReferenceEngine {
    pub fn snapshot(source: &str) {
        let mut s = STATE.lock().unwrap();
        s.source_snapshots.push(source.to_string());
    }

    pub fn modify(modification: &str) -> bool {
        let mut s = STATE.lock().unwrap();
        s.safety_checks += 1;
        s.modifications.push(modification.to_string());
        true
    }

    pub fn snapshot_count() -> usize { STATE.lock().unwrap().source_snapshots.len() }

    pub fn modification_count() -> usize { STATE.lock().unwrap().modifications.len() }

    pub fn safety_checks() -> u64 { STATE.lock().unwrap().safety_checks }

    pub fn reset() { *STATE.lock().unwrap() = SREState::default(); }
}

#[unsafe(no_mangle)]
pub extern "C" fn sre_snapshot() -> i64 { SelfReferenceEngine::snapshot("ffi_src"); SelfReferenceEngine::snapshot_count() as i64 }
#[unsafe(no_mangle)]
pub extern "C" fn sre_modify() -> i64 { if SelfReferenceEngine::modify("ffi_mod") { 1 } else { 0 } }
#[unsafe(no_mangle)]
pub extern "C" fn sre_safety() -> i64 { SelfReferenceEngine::safety_checks() as i64 }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_snapshot() { SelfReferenceEngine::reset(); SelfReferenceEngine::snapshot("fn main()"); assert_eq!(SelfReferenceEngine::snapshot_count(), 1); }
    #[test] fn test_modify() { SelfReferenceEngine::reset(); assert!(SelfReferenceEngine::modify("add_line")); }
    #[test] fn test_mod_count() { SelfReferenceEngine::reset(); SelfReferenceEngine::modify("a"); SelfReferenceEngine::modify("b"); assert_eq!(SelfReferenceEngine::modification_count(), 2); }
    #[test] fn test_safety() { SelfReferenceEngine::reset(); SelfReferenceEngine::modify("x"); assert_eq!(SelfReferenceEngine::safety_checks(), 1); }
    #[test] fn test_multiple_snapshots() { SelfReferenceEngine::reset(); SelfReferenceEngine::snapshot("v1"); SelfReferenceEngine::snapshot("v2"); assert_eq!(SelfReferenceEngine::snapshot_count(), 2); }
    #[test] fn test_reset() { SelfReferenceEngine::reset(); SelfReferenceEngine::snapshot("s"); SelfReferenceEngine::reset(); assert_eq!(SelfReferenceEngine::snapshot_count(), 0); }
    #[test] fn test_ffi() { SelfReferenceEngine::reset(); sre_snapshot(); sre_modify(); assert_eq!(sre_safety(), 1); }
}
