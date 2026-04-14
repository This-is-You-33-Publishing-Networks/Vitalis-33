//! Version Consciousness — Vitalis v1297
//!
//! Awareness of own evolutionary history and trajectory.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<VersionConsciousnessState>> = LazyLock::new(|| Mutex::new(VersionConsciousnessState::default()));

#[derive(Default)]
struct VersionConsciousnessState {
    history: Vec<(u64, String)>,
    trajectory: f64,
    awareness_level: f64,
}

pub struct VersionConsciousness;

impl VersionConsciousness {
    pub fn record_event(epoch: u64, desc: &str) {
        let mut s = STATE.lock().unwrap();
        s.history.push((epoch, desc.to_string()));
        s.awareness_level = (s.history.len() as f64).ln().max(0.0);
    }
    pub fn set_trajectory(t: f64) { STATE.lock().unwrap().trajectory = t; }
    pub fn awareness_level() -> f64 { STATE.lock().unwrap().awareness_level }
    pub fn trajectory() -> f64 { STATE.lock().unwrap().trajectory }
    pub fn history_len() -> usize { STATE.lock().unwrap().history.len() }
    pub fn reset() { *STATE.lock().unwrap() = VersionConsciousnessState::default(); }
}

#[unsafe(no_mangle)]
pub extern "C" fn vc_awareness_level() -> f64 { VersionConsciousness::awareness_level() }
#[unsafe(no_mangle)]
pub extern "C" fn vc_trajectory() -> f64 { VersionConsciousness::trajectory() }
#[unsafe(no_mangle)]
pub extern "C" fn vc_history_len() -> usize { VersionConsciousness::history_len() }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_record_event() { VersionConsciousness::reset(); VersionConsciousness::record_event(1, "birth"); assert_eq!(VersionConsciousness::history_len(), 1); }
    #[test] fn test_awareness_grows() { VersionConsciousness::reset(); for i in 0..10 { VersionConsciousness::record_event(i, "e"); } assert!(VersionConsciousness::awareness_level() > 0.0); }
    #[test] fn test_set_trajectory() { VersionConsciousness::reset(); VersionConsciousness::set_trajectory(0.95); assert!((VersionConsciousness::trajectory() - 0.95).abs() < 1e-9); }
    #[test] fn test_initial_awareness() { VersionConsciousness::reset(); assert!((VersionConsciousness::awareness_level() - 0.0).abs() < 1e-9); }
    #[test] fn test_history_accumulates() { VersionConsciousness::reset(); VersionConsciousness::record_event(1, "a"); VersionConsciousness::record_event(2, "b"); assert_eq!(VersionConsciousness::history_len(), 2); }
    #[test] fn test_reset() { VersionConsciousness::reset(); VersionConsciousness::record_event(1, "x"); VersionConsciousness::reset(); assert_eq!(VersionConsciousness::history_len(), 0); }
}
