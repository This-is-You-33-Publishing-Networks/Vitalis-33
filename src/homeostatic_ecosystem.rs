//! Homeostatic Ecosystem — Vitalis v1291
//!
//! Self-regulating ecosystem stable under perturbation.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<HomeostaticEcosystemState>> = LazyLock::new(|| Mutex::new(HomeostaticEcosystemState::default()));

#[derive(Default)]
struct HomeostaticEcosystemState {
    set_points: Vec<(String, f64)>,
    current_values: Vec<(String, f64)>,
    corrections: u64,
}

pub struct HomeostaticEcosystem;

impl HomeostaticEcosystem {
    pub fn set_point(name: &str, value: f64) {
        let mut s = STATE.lock().unwrap();
        s.set_points.push((name.to_string(), value));
        s.current_values.push((name.to_string(), value));
    }
    pub fn perturb(name: &str, delta: f64) {
        let mut s = STATE.lock().unwrap();
        for v in s.current_values.iter_mut() { if v.0 == name { v.1 += delta; } }
    }
    pub fn correct() -> u64 {
        let mut s = STATE.lock().unwrap();
        let set_points: Vec<(String, f64)> = s.set_points.clone();
        for (i, sp) in set_points.iter().enumerate() {
            if let Some(cv) = s.current_values.get_mut(i) {
                let diff = sp.1 - cv.1;
                cv.1 += diff * 0.5;
                if diff.abs() > 0.001 { s.corrections += 1; }
            }
        }
        s.corrections
    }
    pub fn corrections() -> u64 { STATE.lock().unwrap().corrections }
    pub fn reset() { *STATE.lock().unwrap() = HomeostaticEcosystemState::default(); }
}

#[unsafe(no_mangle)]
pub extern "C" fn heco_correct() -> u64 { HomeostaticEcosystem::correct() }
#[unsafe(no_mangle)]
pub extern "C" fn heco_corrections() -> u64 { HomeostaticEcosystem::corrections() }
#[unsafe(no_mangle)]
pub extern "C" fn heco_perturb() -> u64 { HomeostaticEcosystem::perturb("default", 1.0); HomeostaticEcosystem::corrections() }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_set_point() { HomeostaticEcosystem::reset(); HomeostaticEcosystem::set_point("temp", 37.0); assert_eq!(STATE.lock().unwrap().set_points.len(), 1); }
    #[test] fn test_perturb() { HomeostaticEcosystem::reset(); HomeostaticEcosystem::set_point("temp", 37.0); HomeostaticEcosystem::perturb("temp", 5.0); assert!((STATE.lock().unwrap().current_values[0].1 - 42.0).abs() < 1e-9); }
    #[test] fn test_correct() { HomeostaticEcosystem::reset(); HomeostaticEcosystem::set_point("temp", 37.0); HomeostaticEcosystem::perturb("temp", 5.0); let c = HomeostaticEcosystem::correct(); assert!(c > 0); }
    #[test] fn test_correction_moves_toward_set_point() { HomeostaticEcosystem::reset(); HomeostaticEcosystem::set_point("x", 10.0); HomeostaticEcosystem::perturb("x", 10.0); HomeostaticEcosystem::correct(); let val = STATE.lock().unwrap().current_values[0].1; assert!(val < 20.0); }
    #[test] fn test_no_correction_needed() { HomeostaticEcosystem::reset(); HomeostaticEcosystem::set_point("y", 5.0); let before = HomeostaticEcosystem::corrections(); HomeostaticEcosystem::correct(); assert_eq!(HomeostaticEcosystem::corrections(), before); }
    #[test] fn test_reset() { HomeostaticEcosystem::reset(); HomeostaticEcosystem::set_point("z", 1.0); HomeostaticEcosystem::reset(); assert_eq!(STATE.lock().unwrap().set_points.len(), 0); }
}
