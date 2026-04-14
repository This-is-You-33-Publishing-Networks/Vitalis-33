//! Digital Physics — Vitalis v1067
//!
//! Programs as physical systems with conservation laws, symmetries,
//! and invariants. Computation obeys physics-like constraints.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<DphyState>> = LazyLock::new(|| Mutex::new(DphyState::default()));

#[derive(Default)]
struct DphyState {
    conserved_quantities: Vec<(String, f64)>,
    symmetries: Vec<String>,
    violations: u64,
    total_energy: f64,
}

pub struct DigitalPhysics;

impl DigitalPhysics {
    pub fn conserve(quantity: &str, value: f64) -> f64 {
        let mut s = STATE.lock().unwrap();
        s.total_energy += value;
        s.conserved_quantities.push((quantity.to_string(), value));
        s.total_energy
    }

    pub fn check_conservation() -> bool {
        let s = STATE.lock().unwrap();
        let sum: f64 = s.conserved_quantities.iter().map(|(_, v)| v).sum();
        (sum - s.total_energy).abs() < 1e-10
    }

    pub fn add_symmetry(name: &str) -> usize {
        let mut s = STATE.lock().unwrap();
        s.symmetries.push(name.to_string());
        s.symmetries.len()
    }

    pub fn report_violation() -> u64 {
        let mut s = STATE.lock().unwrap();
        s.violations += 1;
        s.violations
    }

    pub fn symmetry_count() -> usize { STATE.lock().unwrap().symmetries.len() }
    pub fn total_energy() -> f64 { STATE.lock().unwrap().total_energy }
    pub fn violations() -> u64 { STATE.lock().unwrap().violations }
    pub fn reset() { *STATE.lock().unwrap() = DphyState::default(); }
}

#[unsafe(no_mangle)]
pub extern "C" fn dphy_conserve(val: f64) -> f64 { DigitalPhysics::conserve("energy", val) }
#[unsafe(no_mangle)]
pub extern "C" fn dphy_check() -> i64 { if DigitalPhysics::check_conservation() { 1 } else { 0 } }
#[unsafe(no_mangle)]
pub extern "C" fn dphy_symmetries() -> i64 { DigitalPhysics::symmetry_count() as i64 }
#[unsafe(no_mangle)]
pub extern "C" fn dphy_energy() -> f64 { DigitalPhysics::total_energy() }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_conserve() { DigitalPhysics::reset(); let e = DigitalPhysics::conserve("mass", 5.0); assert!((e - 5.0).abs() < 0.01); }
    #[test] fn test_conservation_holds() { DigitalPhysics::reset(); DigitalPhysics::conserve("a", 3.0); DigitalPhysics::conserve("b", 2.0); assert!(DigitalPhysics::check_conservation()); }
    #[test] fn test_symmetry() { DigitalPhysics::reset(); assert_eq!(DigitalPhysics::add_symmetry("rotation"), 1); }
    #[test] fn test_violation() { DigitalPhysics::reset(); assert_eq!(DigitalPhysics::report_violation(), 1); }
    #[test] fn test_energy() { DigitalPhysics::reset(); DigitalPhysics::conserve("x", 10.0); assert!((DigitalPhysics::total_energy() - 10.0).abs() < 0.01); }
    #[test] fn test_symmetry_count() { DigitalPhysics::reset(); DigitalPhysics::add_symmetry("a"); DigitalPhysics::add_symmetry("b"); assert_eq!(DigitalPhysics::symmetry_count(), 2); }
    #[test] fn test_violations_count() { DigitalPhysics::reset(); DigitalPhysics::report_violation(); DigitalPhysics::report_violation(); assert_eq!(DigitalPhysics::violations(), 2); }
    #[test] fn test_ffi() { DigitalPhysics::reset(); assert_eq!(dphy_symmetries(), 0); }
}
