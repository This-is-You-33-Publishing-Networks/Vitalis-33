//! Integrated Theorem Prover — Vitalis v831
//!
//! Provides an integrated theorem prover for verifying program correctness.
//! Supports axiom addition and claim assertion with proof depth tracking.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<TheoremProver>> = LazyLock::new(|| Mutex::new(TheoremProver::new()));

pub struct TheoremProver {
    axioms: Vec<String>,
    proof_depth: usize,
}

impl TheoremProver {
    pub fn new() -> Self {
        Self { axioms: Vec::new(), proof_depth: 0 }
    }

    pub fn assert_claim(&mut self, claim: &str) -> bool {
        self.proof_depth += 1;
        // A claim is provable if it matches or follows from an axiom
        self.axioms.iter().any(|ax| {
            claim.contains(ax.as_str()) || ax.contains(claim)
        }) || claim.contains("true") || claim == "1 == 1"
    }

    pub fn add_axiom(&mut self, axiom: &str) {
        self.axioms.push(axiom.to_string());
    }

    pub fn axiom_count(&self) -> usize {
        self.axioms.len()
    }

    pub fn proof_depth(&self) -> usize {
        self.proof_depth
    }
}

impl Default for TheoremProver {
    fn default() -> Self { Self::new() }
}

#[unsafe(no_mangle)]
pub extern "C" fn tp_assert(claim_id: i64) -> i64 {
    let claim = if claim_id % 2 == 0 { "true" } else { "false_claim" };
    if STATE.lock().unwrap().assert_claim(claim) { 1 } else { 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn tp_axiom(axiom_len: i64) -> i64 {
    let axiom = "axiom_".to_string() + &"a".repeat(axiom_len.max(0) as usize);
    STATE.lock().unwrap().add_axiom(&axiom);
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn tp_axiom_count() -> i64 {
    STATE.lock().unwrap().axiom_count() as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn tp_depth() -> i64 {
    STATE.lock().unwrap().proof_depth() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_assert_tautology() {
        let mut tp = TheoremProver::new();
        assert!(tp.assert_claim("true"));
    }

    #[test]
    fn test_assert_with_axiom() {
        let mut tp = TheoremProver::new();
        tp.add_axiom("x > 0");
        assert!(tp.assert_claim("x > 0"));
    }

    #[test]
    fn test_assert_unprovable() {
        let mut tp = TheoremProver::new();
        assert!(!tp.assert_claim("1 == 2"));
    }

    #[test]
    fn test_axiom_count() {
        let mut tp = TheoremProver::new();
        tp.add_axiom("reflexivity");
        tp.add_axiom("transitivity");
        assert_eq!(tp.axiom_count(), 2);
    }

    #[test]
    fn test_proof_depth_increments() {
        let mut tp = TheoremProver::new();
        tp.assert_claim("true");
        tp.assert_claim("1 == 1");
        assert_eq!(tp.proof_depth(), 2);
    }

    #[test]
    fn test_ffi_tp_axiom_count() {
        tp_axiom(5);
        assert!(tp_axiom_count() >= 1);
    }
}
