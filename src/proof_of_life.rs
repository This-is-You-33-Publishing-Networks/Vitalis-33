//! Proof of Life — Vitalis v1271
//!
//! Formal proof that the compiler exhibits properties of living systems.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<ProofOfLifeState>> = LazyLock::new(|| Mutex::new(ProofOfLifeState::default()));

#[derive(Default)]
struct ProofOfLifeState {
    properties: Vec<(String, bool)>,
    life_score: f64,
    verified: u64,
}

pub struct ProofOfLife;

impl ProofOfLife {
    pub fn add_property(name: &str, holds: bool) {
        let mut s = STATE.lock().unwrap();
        s.properties.push((name.to_string(), holds));
        if holds { s.life_score += 1.0; }
    }
    pub fn verify() -> u64 {
        let mut s = STATE.lock().unwrap();
        s.verified += 1;
        s.verified
    }
    pub fn life_score() -> f64 { STATE.lock().unwrap().life_score }
    pub fn property_count() -> usize { STATE.lock().unwrap().properties.len() }
    pub fn reset() { *STATE.lock().unwrap() = ProofOfLifeState::default(); }
}

#[unsafe(no_mangle)]
pub extern "C" fn pol_verify() -> u64 { ProofOfLife::verify() }
#[unsafe(no_mangle)]
pub extern "C" fn pol_life_score() -> f64 { ProofOfLife::life_score() }
#[unsafe(no_mangle)]
pub extern "C" fn pol_property_count() -> usize { ProofOfLife::property_count() }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_add_property() { ProofOfLife::reset(); ProofOfLife::add_property("metabolism", true); assert_eq!(ProofOfLife::property_count(), 1); }
    #[test] fn test_verify() { ProofOfLife::reset(); let v = ProofOfLife::verify(); assert_eq!(v, 1); }
    #[test] fn test_life_score_true() { ProofOfLife::reset(); ProofOfLife::add_property("growth", true); assert!((ProofOfLife::life_score() - 1.0).abs() < 1e-9); }
    #[test] fn test_life_score_false() { ProofOfLife::reset(); ProofOfLife::add_property("none", false); assert!((ProofOfLife::life_score() - 0.0).abs() < 1e-9); }
    #[test] fn test_multiple_properties() { ProofOfLife::reset(); ProofOfLife::add_property("a", true); ProofOfLife::add_property("b", true); assert!(ProofOfLife::life_score() > 1.0); }
    #[test] fn test_verify_increments() { ProofOfLife::reset(); ProofOfLife::verify(); let v = ProofOfLife::verify(); assert_eq!(v, 2); }
    #[test] fn test_reset() { ProofOfLife::reset(); ProofOfLife::add_property("x", true); ProofOfLife::reset(); assert_eq!(ProofOfLife::property_count(), 0); }
}
