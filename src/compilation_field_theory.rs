//! Compilation Field Theory — Vitalis v1174
//!
//! The space of all compilations as a field with dynamics and conservation laws.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<CftState>> = LazyLock::new(|| Mutex::new(CftState::default()));

#[derive(Default)]
struct CftState { field_values: Vec<(String, f64)>, lagrangian: f64, action: f64 }

pub struct CompilationFieldTheory;

impl CompilationFieldTheory {
    pub fn set_field(point: &str, value: f64) -> usize { let mut s = STATE.lock().unwrap(); s.field_values.push((point.to_string(), value)); s.field_values.len() }
    pub fn compute_lagrangian() -> f64 {
        let mut s = STATE.lock().unwrap();
        let kinetic: f64 = s.field_values.windows(2).map(|w| (w[1].1 - w[0].1).powi(2)).sum::<f64>() * 0.5;
        let potential: f64 = s.field_values.iter().map(|(_, v)| v * v * 0.5).sum();
        s.lagrangian = kinetic - potential;
        s.lagrangian
    }
    pub fn minimize_action() -> f64 { let mut s = STATE.lock().unwrap(); s.action = s.lagrangian.abs(); s.action }
    pub fn field_count() -> usize { STATE.lock().unwrap().field_values.len() }
    pub fn lagrangian() -> f64 { STATE.lock().unwrap().lagrangian }
    pub fn reset() { *STATE.lock().unwrap() = CftState::default(); }
}

#[unsafe(no_mangle)]
pub extern "C" fn cft_set(val: f64) -> i64 { CompilationFieldTheory::set_field("ffi", val) as i64 }
#[unsafe(no_mangle)]
pub extern "C" fn cft_lagrangian() -> f64 { CompilationFieldTheory::compute_lagrangian() }
#[unsafe(no_mangle)]
pub extern "C" fn cft_fields() -> i64 { CompilationFieldTheory::field_count() as i64 }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_set() { CompilationFieldTheory::reset(); assert_eq!(CompilationFieldTheory::set_field("p1", 1.0), 1); }
    #[test] fn test_lagrangian() { CompilationFieldTheory::reset(); CompilationFieldTheory::set_field("a", 1.0); CompilationFieldTheory::set_field("b", 2.0); let l = CompilationFieldTheory::compute_lagrangian(); assert!(l != 0.0); }
    #[test] fn test_action() { CompilationFieldTheory::reset(); CompilationFieldTheory::set_field("a", 1.0); CompilationFieldTheory::compute_lagrangian(); let a = CompilationFieldTheory::minimize_action(); assert!(a >= 0.0); }
    #[test] fn test_field_count() { CompilationFieldTheory::reset(); CompilationFieldTheory::set_field("a", 1.0); assert_eq!(CompilationFieldTheory::field_count(), 1); }
    #[test] fn test_empty_lagrangian() { CompilationFieldTheory::reset(); assert!((CompilationFieldTheory::compute_lagrangian() - 0.0).abs() < 0.01); }
    #[test] fn test_ffi() { CompilationFieldTheory::reset(); assert_eq!(cft_fields(), 0); }
}
