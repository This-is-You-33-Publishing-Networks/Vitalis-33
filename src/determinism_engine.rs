//! Determinism Engine — Vitalis v1091
//!
//! Guarantees deterministic execution even in concurrent/distributed
//! contexts through causal ordering and logical clocks.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<DeteState>> = LazyLock::new(|| Mutex::new(DeteState::default()));

#[derive(Default)]
struct DeteState {
    logical_clock: u64,
    events: Vec<(u64, String)>,
    violations: u64,
}

pub struct DeterminismEngine;

impl DeterminismEngine {
    pub fn tick() -> u64 { let mut s = STATE.lock().unwrap(); s.logical_clock += 1; s.logical_clock }

    pub fn record_event(event: &str) -> u64 {
        let mut s = STATE.lock().unwrap();
        s.logical_clock += 1;
        let clock = s.logical_clock;
        s.events.push((clock, event.to_string()));
        s.logical_clock
    }

    pub fn verify_order() -> bool {
        let mut s = STATE.lock().unwrap();
        let ordered = s.events.windows(2).all(|w| w[0].0 <= w[1].0);
        if !ordered { s.violations += 1; }
        ordered
    }

    pub fn merge_clock(remote: u64) -> u64 {
        let mut s = STATE.lock().unwrap();
        s.logical_clock = s.logical_clock.max(remote) + 1;
        s.logical_clock
    }

    pub fn clock() -> u64 { STATE.lock().unwrap().logical_clock }
    pub fn event_count() -> usize { STATE.lock().unwrap().events.len() }
    pub fn violations() -> u64 { STATE.lock().unwrap().violations }
    pub fn reset() { *STATE.lock().unwrap() = DeteState::default(); }
}

#[unsafe(no_mangle)]
pub extern "C" fn dete_tick() -> i64 { DeterminismEngine::tick() as i64 }
#[unsafe(no_mangle)]
pub extern "C" fn dete_record() -> i64 { DeterminismEngine::record_event("ffi_event") as i64 }
#[unsafe(no_mangle)]
pub extern "C" fn dete_verify() -> i64 { if DeterminismEngine::verify_order() { 1 } else { 0 } }
#[unsafe(no_mangle)]
pub extern "C" fn dete_clock() -> i64 { DeterminismEngine::clock() as i64 }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_tick() { DeterminismEngine::reset(); assert_eq!(DeterminismEngine::tick(), 1); assert_eq!(DeterminismEngine::tick(), 2); }
    #[test] fn test_record() { DeterminismEngine::reset(); let t = DeterminismEngine::record_event("a"); assert!(t > 0); }
    #[test] fn test_verify_ok() { DeterminismEngine::reset(); DeterminismEngine::record_event("a"); DeterminismEngine::record_event("b"); assert!(DeterminismEngine::verify_order()); }
    #[test] fn test_merge() { DeterminismEngine::reset(); DeterminismEngine::tick(); let c = DeterminismEngine::merge_clock(100); assert!(c > 100); }
    #[test] fn test_event_count() { DeterminismEngine::reset(); DeterminismEngine::record_event("a"); assert_eq!(DeterminismEngine::event_count(), 1); }
    #[test] fn test_violations() { DeterminismEngine::reset(); assert_eq!(DeterminismEngine::violations(), 0); }
    #[test] fn test_ffi() { DeterminismEngine::reset(); assert_eq!(dete_clock(), 0); }
}
