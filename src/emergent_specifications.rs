//! Emergent Specifications — Vitalis v996
//!
//! Specifications emerge from runtime behavior rather than being written.
//! Observes execution patterns and infers invariants.

use std::sync::{LazyLock, Mutex};
use std::collections::HashMap;

static STATE: LazyLock<Mutex<EsState>> = LazyLock::new(|| Mutex::new(EsState::default()));

#[derive(Default)]
struct EsState {
    observations: HashMap<String, Vec<i64>>,
    specs: Vec<String>,
}

pub struct EmergentSpec;

impl EmergentSpec {
    pub fn observe_behavior(call: &str, result: i64) {
        STATE.lock().unwrap().observations.entry(call.to_string()).or_default().push(result);
    }

    pub fn infer_spec() -> Vec<String> {
        let mut s = STATE.lock().unwrap();
        let mut specs = Vec::new();
        for (call, results) in &s.observations {
            if results.iter().all(|&r| r >= 0) { specs.push(format!("{}: returns >= 0", call)); }
            if results.len() > 1 { let min = results.iter().min().unwrap(); let max = results.iter().max().unwrap(); specs.push(format!("{}: range [{}, {}]", call, min, max)); }
        }
        s.specs = specs.clone();
        specs
    }

    pub fn spec_count() -> usize { STATE.lock().unwrap().specs.len() }
    pub fn confidence() -> f64 { let s = STATE.lock().unwrap(); let total: usize = s.observations.values().map(|v| v.len()).sum(); if total < 5 { 0.3 } else if total < 20 { 0.6 } else { 0.9 } }
    pub fn reset() { *STATE.lock().unwrap() = EsState::default(); }
}

#[unsafe(no_mangle)]
pub extern "C" fn es_observe(call_hash: i64, result: i64) -> i64 { STATE.lock().unwrap().observations.entry(format!("fn_{}", call_hash)).or_default().push(result); 1 }
#[unsafe(no_mangle)]
pub extern "C" fn es_infer_count() -> i64 { EmergentSpec::infer_spec().len() as i64 }
#[unsafe(no_mangle)]
pub extern "C" fn es_spec_count() -> i64 { EmergentSpec::spec_count() as i64 }
#[unsafe(no_mangle)]
pub extern "C" fn es_confidence() -> f64 { EmergentSpec::confidence() }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_observe() { EmergentSpec::reset(); EmergentSpec::observe_behavior("add", 5); assert_eq!(STATE.lock().unwrap().observations.len(), 1); }
    #[test] fn test_infer() { EmergentSpec::reset(); EmergentSpec::observe_behavior("f", 1); EmergentSpec::observe_behavior("f", 2); let specs = EmergentSpec::infer_spec(); assert!(!specs.is_empty()); }
    #[test] fn test_confidence() { EmergentSpec::reset(); assert!(EmergentSpec::confidence() > 0.0); }
    #[test] fn test_spec_count() { EmergentSpec::reset(); EmergentSpec::infer_spec(); assert_eq!(EmergentSpec::spec_count(), 0); }
    #[test] fn test_ffi() { EmergentSpec::reset(); assert_eq!(es_spec_count(), 0); }
}
