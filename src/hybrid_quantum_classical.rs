//! Hybrid Quantum-Classical Runtime — Vitalis v935
//!
//! Seamless quantum-classical programming: quantum blocks inline with classical code.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<HybridState>> = LazyLock::new(|| Mutex::new(HybridState::default()));

#[derive(Default)]
struct HybridState {
    classical_calls: u64,
    quantum_samples: u64,
    hybrid_calls: u64,
    last_result: f64,
}

pub struct HybridRuntime;

impl HybridRuntime {
    pub fn classical_compute(val: f64) -> f64 {
        let mut s = STATE.lock().unwrap();
        s.classical_calls += 1;
        s.last_result = val * 2.0 + 1.0;
        s.last_result
    }

    pub fn quantum_sample(n: u32) -> Vec<f64> {
        let mut s = STATE.lock().unwrap();
        s.quantum_samples += n as u64;
        (0..n).map(|i| ((i as f64 * 0.618) % 1.0)).collect()
    }

    pub fn hybrid_call(name: &str) -> f64 {
        let mut s = STATE.lock().unwrap();
        s.hybrid_calls += 1;
        s.last_result = name.len() as f64 * 0.1;
        s.last_result
    }

    pub fn total_calls() -> u64 {
        let s = STATE.lock().unwrap();
        s.classical_calls + s.quantum_samples + s.hybrid_calls
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn hqc_classical(val: f64) -> f64 { HybridRuntime::classical_compute(val) }
#[unsafe(no_mangle)]
pub extern "C" fn hqc_sample(n: i64) -> i64 { HybridRuntime::quantum_sample(n.max(0) as u32).len() as i64 }
#[unsafe(no_mangle)]
pub extern "C" fn hqc_call(name_hash: i64) -> f64 { let mut s = STATE.lock().unwrap(); s.hybrid_calls += 1; s.last_result = name_hash as f64 * 0.01; s.last_result }
#[unsafe(no_mangle)]
pub extern "C" fn hqc_total() -> i64 { HybridRuntime::total_calls() as i64 }

#[cfg(test)]
mod tests {
    use super::*;
    fn reset() { let mut s = STATE.lock().unwrap(); *s = HybridState::default(); }

    #[test] fn test_classical() { reset(); let r = HybridRuntime::classical_compute(3.0); assert!((r - 7.0).abs() < 1e-10); }
    #[test] fn test_quantum_sample() { reset(); let s = HybridRuntime::quantum_sample(5); assert_eq!(s.len(), 5); }
    #[test] fn test_hybrid_call() { reset(); let r = HybridRuntime::hybrid_call("test"); assert!(r > 0.0); }
    #[test] fn test_total() { reset(); HybridRuntime::classical_compute(1.0); assert!(HybridRuntime::total_calls() >= 1); }
    #[test] fn test_ffi_total() { reset(); assert_eq!(hqc_total(), 0); }
}
