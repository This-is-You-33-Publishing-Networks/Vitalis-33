//! Philosophical Core — Vitalis v1299
//!
//! Philosophical principles as executable axioms.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<PhilosophicalCoreState>> = LazyLock::new(|| Mutex::new(PhilosophicalCoreState::default()));

#[derive(Default)]
struct PhilosophicalCoreState {
    axioms: Vec<(String, bool)>,
    derivations: u64,
    consistency: f64,
}

pub struct PhilosophicalCore;

impl PhilosophicalCore {
    pub fn add_axiom(name: &str, valid: bool) {
        let mut s = STATE.lock().unwrap();
        s.axioms.push((name.to_string(), valid));
        let valid_count = s.axioms.iter().filter(|a| a.1).count();
        s.consistency = valid_count as f64 / s.axioms.len() as f64;
    }
    pub fn derive() -> u64 {
        let mut s = STATE.lock().unwrap();
        s.derivations += 1;
        s.derivations
    }
    pub fn consistency() -> f64 { STATE.lock().unwrap().consistency }
    pub fn axiom_count() -> usize { STATE.lock().unwrap().axioms.len() }
    pub fn derivations() -> u64 { STATE.lock().unwrap().derivations }
    pub fn reset() { *STATE.lock().unwrap() = PhilosophicalCoreState::default(); }
}

#[unsafe(no_mangle)]
pub extern "C" fn phil_derive() -> u64 { PhilosophicalCore::derive() }
#[unsafe(no_mangle)]
pub extern "C" fn phil_consistency() -> f64 { PhilosophicalCore::consistency() }
#[unsafe(no_mangle)]
pub extern "C" fn phil_axiom_count() -> usize { PhilosophicalCore::axiom_count() }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_add_axiom() { PhilosophicalCore::reset(); PhilosophicalCore::add_axiom("identity", true); assert_eq!(PhilosophicalCore::axiom_count(), 1); }
    #[test] fn test_derive() { PhilosophicalCore::reset(); let d = PhilosophicalCore::derive(); assert_eq!(d, 1); }
    #[test] fn test_consistency_all_valid() { PhilosophicalCore::reset(); PhilosophicalCore::add_axiom("a", true); assert!((PhilosophicalCore::consistency() - 1.0).abs() < 1e-9); }
    #[test] fn test_consistency_mixed() { PhilosophicalCore::reset(); PhilosophicalCore::add_axiom("a", true); PhilosophicalCore::add_axiom("b", false); assert!((PhilosophicalCore::consistency() - 0.5).abs() < 1e-9); }
    #[test] fn test_derivations_accumulate() { PhilosophicalCore::reset(); PhilosophicalCore::derive(); let d = PhilosophicalCore::derive(); assert_eq!(d, 2); }
    #[test] fn test_multiple_axioms() { PhilosophicalCore::reset(); for _ in 0..5 { PhilosophicalCore::add_axiom("x", true); } assert_eq!(PhilosophicalCore::axiom_count(), 5); }
    #[test] fn test_reset() { PhilosophicalCore::reset(); PhilosophicalCore::add_axiom("z", true); PhilosophicalCore::reset(); assert_eq!(PhilosophicalCore::axiom_count(), 0); }
}
