//! Fixpoint Accelerator — Vitalis v1241
//!
//! Accelerates convergence via Aitken/Anderson methods.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<FxaState>> = LazyLock::new(|| Mutex::new(FxaState::default()));

#[derive(Default)]
struct FxaState {
    sequence: Vec<f64>,
    accelerated: Vec<f64>,
    speedup: f64,
}

pub struct FixpointAccelerator;

impl FixpointAccelerator {
    pub fn feed(value: f64) {
        let mut s = STATE.lock().unwrap();
        s.sequence.push(value);
        let n = s.sequence.len();
        if n >= 3 {
            let s0 = s.sequence[n - 3];
            let s1 = s.sequence[n - 2];
            let s2 = s.sequence[n - 1];
            let denom = s2 - 2.0 * s1 + s0;
            if denom.abs() > 1e-15 {
                let acc = s0 - (s1 - s0).powi(2) / denom;
                s.accelerated.push(acc);
                let orig_err = (s2 - s1).abs();
                let acc_err = (acc - s2).abs();
                if orig_err > 1e-15 {
                    s.speedup = orig_err / (acc_err + 1e-15);
                }
            }
        }
    }

    pub fn last_accelerated() -> Option<f64> {
        let s = STATE.lock().unwrap();
        s.accelerated.last().copied()
    }

    pub fn speedup() -> f64 { STATE.lock().unwrap().speedup }

    pub fn sequence_len() -> usize { STATE.lock().unwrap().sequence.len() }

    pub fn accelerated_len() -> usize { STATE.lock().unwrap().accelerated.len() }

    pub fn reset() { *STATE.lock().unwrap() = FxaState::default(); }
}

#[unsafe(no_mangle)]
pub extern "C" fn fxa_feed(val: f64) -> i64 { FixpointAccelerator::feed(val); FixpointAccelerator::sequence_len() as i64 }
#[unsafe(no_mangle)]
pub extern "C" fn fxa_speedup() -> f64 { FixpointAccelerator::speedup() }
#[unsafe(no_mangle)]
pub extern "C" fn fxa_acc_len() -> i64 { FixpointAccelerator::accelerated_len() as i64 }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_feed() { FixpointAccelerator::reset(); FixpointAccelerator::feed(1.0); assert_eq!(FixpointAccelerator::sequence_len(), 1); }
    #[test] fn test_acceleration() { FixpointAccelerator::reset(); FixpointAccelerator::feed(1.0); FixpointAccelerator::feed(0.5); FixpointAccelerator::feed(0.25); assert!(FixpointAccelerator::last_accelerated().is_some()); }
    #[test] fn test_speedup() { FixpointAccelerator::reset(); FixpointAccelerator::feed(2.0); FixpointAccelerator::feed(1.5); FixpointAccelerator::feed(1.25); assert!(FixpointAccelerator::speedup() > 0.0); }
    #[test] fn test_no_acc_early() { FixpointAccelerator::reset(); FixpointAccelerator::feed(1.0); assert!(FixpointAccelerator::last_accelerated().is_none()); }
    #[test] fn test_acc_len() { FixpointAccelerator::reset(); FixpointAccelerator::feed(3.0); FixpointAccelerator::feed(2.0); FixpointAccelerator::feed(1.5); assert_eq!(FixpointAccelerator::accelerated_len(), 1); }
    #[test] fn test_reset() { FixpointAccelerator::reset(); FixpointAccelerator::feed(1.0); FixpointAccelerator::reset(); assert_eq!(FixpointAccelerator::sequence_len(), 0); }
    #[test] fn test_ffi() { FixpointAccelerator::reset(); fxa_feed(1.0); fxa_feed(0.5); fxa_feed(0.25); assert!(fxa_acc_len() >= 1); }
}
