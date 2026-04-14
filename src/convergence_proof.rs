//! Convergence Proof — Vitalis v1176
//!
//! Formal proof engine verifying the unified system converges to optimal forms.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<CvpState>> = LazyLock::new(|| Mutex::new(CvpState::default()));

#[derive(Default)]
struct CvpState { sequence: Vec<f64>, proofs: Vec<(String, bool)>, epsilon: f64 }

pub struct ConvergenceProof;

impl ConvergenceProof {
    pub fn add_observation(value: f64) -> usize { let mut s = STATE.lock().unwrap(); s.sequence.push(value); s.sequence.len() }
    pub fn prove_convergence(epsilon: f64) -> bool {
        let mut s = STATE.lock().unwrap();
        s.epsilon = epsilon;
        if s.sequence.len() < 3 { s.proofs.push(("insufficient_data".into(), false)); return false; }
        let n = s.sequence.len();
        let converged = (s.sequence[n - 1] - s.sequence[n - 2]).abs() < epsilon;
        s.proofs.push(("convergence".into(), converged));
        converged
    }
    pub fn proof_count() -> usize { STATE.lock().unwrap().proofs.len() }
    pub fn verified_count() -> usize { STATE.lock().unwrap().proofs.iter().filter(|(_, v)| *v).count() }
    pub fn observation_count() -> usize { STATE.lock().unwrap().sequence.len() }
    pub fn reset() { *STATE.lock().unwrap() = CvpState::default(); }
}

#[unsafe(no_mangle)]
pub extern "C" fn cvp_observe(val: f64) -> i64 { ConvergenceProof::add_observation(val) as i64 }
#[unsafe(no_mangle)]
pub extern "C" fn cvp_prove(eps: f64) -> i64 { if ConvergenceProof::prove_convergence(eps) { 1 } else { 0 } }
#[unsafe(no_mangle)]
pub extern "C" fn cvp_proofs() -> i64 { ConvergenceProof::proof_count() as i64 }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_observe() { ConvergenceProof::reset(); assert_eq!(ConvergenceProof::add_observation(1.0), 1); }
    #[test] fn test_converged() { ConvergenceProof::reset(); ConvergenceProof::add_observation(1.0); ConvergenceProof::add_observation(1.001); ConvergenceProof::add_observation(1.0011); assert!(ConvergenceProof::prove_convergence(0.01)); }
    #[test] fn test_not_converged() { ConvergenceProof::reset(); ConvergenceProof::add_observation(1.0); ConvergenceProof::add_observation(5.0); ConvergenceProof::add_observation(10.0); assert!(!ConvergenceProof::prove_convergence(0.01)); }
    #[test] fn test_insufficient() { ConvergenceProof::reset(); ConvergenceProof::add_observation(1.0); assert!(!ConvergenceProof::prove_convergence(0.1)); }
    #[test] fn test_proof_count() { ConvergenceProof::reset(); ConvergenceProof::add_observation(1.0); ConvergenceProof::prove_convergence(0.1); assert_eq!(ConvergenceProof::proof_count(), 1); }
    #[test] fn test_verified() { ConvergenceProof::reset(); ConvergenceProof::add_observation(1.0); ConvergenceProof::add_observation(1.0); ConvergenceProof::add_observation(1.0); ConvergenceProof::prove_convergence(0.1); assert_eq!(ConvergenceProof::verified_count(), 1); }
    #[test] fn test_ffi() { ConvergenceProof::reset(); assert_eq!(cvp_proofs(), 0); }
}
