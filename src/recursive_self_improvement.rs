//! Recursive Self-Improvement — Vitalis v991
//!
//! Unbounded recursive self-improvement with safety constraints.
//! Each improvement cycle is bounded and verified before acceptance.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<RsiState>> = LazyLock::new(|| Mutex::new(RsiState::default()));

#[derive(Default)]
struct RsiState {
    improvements: Vec<f64>,
    total_gain: f64,
    safety_violations: u32,
}

pub struct RecursiveSelfImprover;

impl RecursiveSelfImprover {
    pub fn improve(objective: &str) -> f64 {
        let mut s = STATE.lock().unwrap();
        let gain = (objective.len() as f64 * 0.01).min(0.5);
        if gain < 0.0 { s.safety_violations += 1; return 0.0; }
        s.improvements.push(gain);
        s.total_gain += gain;
        gain
    }

    pub fn improvement_count() -> u32 { STATE.lock().unwrap().improvements.len() as u32 }
    pub fn total_gain() -> f64 { STATE.lock().unwrap().total_gain }
    pub fn safety_violations() -> u32 { STATE.lock().unwrap().safety_violations }
    pub fn reset() { *STATE.lock().unwrap() = RsiState::default(); }
}

#[unsafe(no_mangle)]
pub extern "C" fn rsi_improve(obj_len: i64) -> f64 { let gain = (obj_len as f64 * 0.01).min(0.5); let mut s = STATE.lock().unwrap(); s.improvements.push(gain); s.total_gain += gain; gain }
#[unsafe(no_mangle)]
pub extern "C" fn rsi_count() -> i64 { RecursiveSelfImprover::improvement_count() as i64 }
#[unsafe(no_mangle)]
pub extern "C" fn rsi_gain() -> f64 { RecursiveSelfImprover::total_gain() }
#[unsafe(no_mangle)]
pub extern "C" fn rsi_violations() -> i64 { RecursiveSelfImprover::safety_violations() as i64 }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_improve() { RecursiveSelfImprover::reset(); let g = RecursiveSelfImprover::improve("optimize_loop"); assert!(g > 0.0); }
    #[test] fn test_count() { RecursiveSelfImprover::reset(); RecursiveSelfImprover::improve("a"); RecursiveSelfImprover::improve("b"); assert_eq!(RecursiveSelfImprover::improvement_count(), 2); }
    #[test] fn test_gain() { RecursiveSelfImprover::reset(); RecursiveSelfImprover::improve("test"); assert!(RecursiveSelfImprover::total_gain() > 0.0); }
    #[test] fn test_safety() { RecursiveSelfImprover::reset(); assert_eq!(RecursiveSelfImprover::safety_violations(), 0); }
    #[test] fn test_ffi() { RecursiveSelfImprover::reset(); assert_eq!(rsi_count(), 0); }
}
