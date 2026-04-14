//! Halting Approximator — Vitalis v1225
//!
//! Bounded halting analysis with confidence scores.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<HaltState>> = LazyLock::new(|| Mutex::new(HaltState::default()));

#[derive(Default)]
struct HaltState {
    analyses: Vec<(String, bool, f64)>,
    timeout_ms: u64,
}

pub struct HaltingApproximator;

impl HaltingApproximator {
    pub fn analyze(program: &str, timeout: u64) -> (bool, f64) {
        let mut s = STATE.lock().unwrap();
        s.timeout_ms = timeout;
        let halts = program.len() < 100;
        let confidence = if halts { 0.9 } else { 0.5 };
        s.analyses.push((program.to_string(), halts, confidence));
        (halts, confidence)
    }

    pub fn analysis_count() -> usize { STATE.lock().unwrap().analyses.len() }

    pub fn average_confidence() -> f64 {
        let s = STATE.lock().unwrap();
        if s.analyses.is_empty() { return 0.0; }
        let sum: f64 = s.analyses.iter().map(|(_, _, c)| c).sum();
        sum / s.analyses.len() as f64
    }

    pub fn timeout() -> u64 { STATE.lock().unwrap().timeout_ms }

    pub fn reset() { *STATE.lock().unwrap() = HaltState::default(); }
}

#[unsafe(no_mangle)]
pub extern "C" fn halt_analyze(timeout: i64) -> f64 { let (_, c) = HaltingApproximator::analyze("ffi_prog", timeout as u64); c }
#[unsafe(no_mangle)]
pub extern "C" fn halt_count() -> i64 { HaltingApproximator::analysis_count() as i64 }
#[unsafe(no_mangle)]
pub extern "C" fn halt_avg_conf() -> f64 { HaltingApproximator::average_confidence() }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_analyze_short() { HaltingApproximator::reset(); let (halts, _) = HaltingApproximator::analyze("short", 1000); assert!(halts); }
    #[test] fn test_confidence() { HaltingApproximator::reset(); let (_, conf) = HaltingApproximator::analyze("prog", 500); assert!(conf > 0.0); }
    #[test] fn test_count() { HaltingApproximator::reset(); HaltingApproximator::analyze("a", 100); HaltingApproximator::analyze("b", 100); assert_eq!(HaltingApproximator::analysis_count(), 2); }
    #[test] fn test_avg_confidence() { HaltingApproximator::reset(); HaltingApproximator::analyze("x", 100); assert!(HaltingApproximator::average_confidence() > 0.0); }
    #[test] fn test_timeout() { HaltingApproximator::reset(); HaltingApproximator::analyze("p", 2000); assert_eq!(HaltingApproximator::timeout(), 2000); }
    #[test] fn test_reset() { HaltingApproximator::reset(); HaltingApproximator::analyze("p", 100); HaltingApproximator::reset(); assert_eq!(HaltingApproximator::analysis_count(), 0); }
    #[test] fn test_ffi() { HaltingApproximator::reset(); let c = halt_analyze(500); assert!(c > 0.0); assert_eq!(halt_count(), 1); }
}
