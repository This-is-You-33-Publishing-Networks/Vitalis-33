//! Living Mathematics — Vitalis v1267
//!
//! Programs as living mathematical structures that grow and prove themselves.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<LivingMathematicsState>> = LazyLock::new(|| Mutex::new(LivingMathematicsState::default()));

#[derive(Default)]
struct LivingMathematicsState {
    structures: Vec<(String, f64)>,
    growth_rate: f64,
    proofs: u64,
}

pub struct LivingMathematics;

impl LivingMathematics {
    pub fn add_structure(name: &str, complexity: f64) {
        let mut s = STATE.lock().unwrap();
        s.structures.push((name.to_string(), complexity));
    }
    pub fn grow(rate: f64) -> f64 {
        let mut s = STATE.lock().unwrap();
        s.growth_rate += rate;
        for item in s.structures.iter_mut() { item.1 *= 1.0 + rate; }
        s.growth_rate
    }
    pub fn prove() -> u64 {
        let mut s = STATE.lock().unwrap();
        s.proofs += 1;
        s.proofs
    }
    pub fn structure_count() -> usize {
        STATE.lock().unwrap().structures.len()
    }
    pub fn total_complexity() -> f64 {
        STATE.lock().unwrap().structures.iter().map(|x| x.1).sum()
    }
    pub fn reset() {
        *STATE.lock().unwrap() = LivingMathematicsState::default();
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn lm_grow(rate: f64) -> f64 { LivingMathematics::grow(rate) }
#[unsafe(no_mangle)]
pub extern "C" fn lm_prove() -> u64 { LivingMathematics::prove() }
#[unsafe(no_mangle)]
pub extern "C" fn lm_structure_count() -> usize { LivingMathematics::structure_count() }
#[unsafe(no_mangle)]
pub extern "C" fn lm_total_complexity() -> f64 { LivingMathematics::total_complexity() }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_add_structure() { LivingMathematics::reset(); LivingMathematics::add_structure("ring", 1.0); assert_eq!(LivingMathematics::structure_count(), 1); }
    #[test] fn test_grow() { LivingMathematics::reset(); LivingMathematics::add_structure("group", 2.0); let r = LivingMathematics::grow(0.5); assert!(r > 0.0); }
    #[test] fn test_prove() { LivingMathematics::reset(); let p = LivingMathematics::prove(); assert_eq!(p, 1); }
    #[test] fn test_multiple_proofs() { LivingMathematics::reset(); LivingMathematics::prove(); let p = LivingMathematics::prove(); assert_eq!(p, 2); }
    #[test] fn test_complexity() { LivingMathematics::reset(); LivingMathematics::add_structure("field", 3.0); assert!((LivingMathematics::total_complexity() - 3.0).abs() < 1e-9); }
    #[test] fn test_growth_accumulates() { LivingMathematics::reset(); LivingMathematics::grow(0.1); let r = LivingMathematics::grow(0.2); assert!((r - 0.3).abs() < 1e-9); }
    #[test] fn test_structure_grows() { LivingMathematics::reset(); LivingMathematics::add_structure("lattice", 1.0); LivingMathematics::grow(1.0); assert!(LivingMathematics::total_complexity() > 1.0); }
    #[test] fn test_reset() { LivingMathematics::reset(); LivingMathematics::add_structure("x", 1.0); LivingMathematics::reset(); assert_eq!(LivingMathematics::structure_count(), 0); }
}
