//! Vacuum State — Vitalis v1107
//!
//! The minimal ground state of a program: the simplest equivalent form.
//! Reduces programs to their vacuum (minimal energy) configuration.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<VsState>> = LazyLock::new(|| Mutex::new(VsState::default()));

#[derive(Default)]
struct VsState {
    expressions: Vec<(String, f64)>,
    reductions: u64,
    vacuum_energy: f64,
}

pub struct VacuumState;

impl VacuumState {
    pub fn analyze(expr: &str, complexity: f64) -> f64 {
        let mut s = STATE.lock().unwrap();
        s.expressions.push((expr.to_string(), complexity));
        s.vacuum_energy = s.expressions.iter().map(|(_, c)| *c).fold(f64::INFINITY, f64::min);
        complexity
    }

    pub fn reduce_to_vacuum(expr: &str) -> f64 {
        let mut s = STATE.lock().unwrap();
        s.reductions += 1;
        if let Some(idx) = s.expressions.iter().position(|(e, _)| e == expr) {
            s.expressions[idx].1 *= 0.5;
            let result = s.expressions[idx].1;
            s.vacuum_energy = s.expressions.iter().map(|(_, c)| *c).fold(f64::INFINITY, f64::min);
            result
        } else { 0.0 }
    }

    pub fn vacuum_energy() -> f64 { STATE.lock().unwrap().vacuum_energy }
    pub fn expression_count() -> usize { STATE.lock().unwrap().expressions.len() }
    pub fn reductions() -> u64 { STATE.lock().unwrap().reductions }
    pub fn reset() { *STATE.lock().unwrap() = VsState::default(); }
}

#[unsafe(no_mangle)]
pub extern "C" fn vs_analyze(complexity: f64) -> f64 { VacuumState::analyze("ffi_expr", complexity) }
#[unsafe(no_mangle)]
pub extern "C" fn vs_reduce() -> f64 { VacuumState::reduce_to_vacuum("ffi_expr") }
#[unsafe(no_mangle)]
pub extern "C" fn vs_energy() -> f64 { VacuumState::vacuum_energy() }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_analyze() { VacuumState::reset(); VacuumState::analyze("e1", 5.0); assert!((VacuumState::vacuum_energy() - 5.0).abs() < 0.01); }
    #[test] fn test_min_vacuum() { VacuumState::reset(); VacuumState::analyze("a", 10.0); VacuumState::analyze("b", 2.0); assert!((VacuumState::vacuum_energy() - 2.0).abs() < 0.01); }
    #[test] fn test_reduce() { VacuumState::reset(); VacuumState::analyze("a", 10.0); let r = VacuumState::reduce_to_vacuum("a"); assert!(r < 10.0); }
    #[test] fn test_reduce_miss() { VacuumState::reset(); assert!((VacuumState::reduce_to_vacuum("nope") - 0.0).abs() < 0.01); }
    #[test] fn test_reductions() { VacuumState::reset(); VacuumState::reduce_to_vacuum("x"); assert_eq!(VacuumState::reductions(), 1); }
    #[test] fn test_expression_count() { VacuumState::reset(); VacuumState::analyze("a", 1.0); assert_eq!(VacuumState::expression_count(), 1); }
    #[test] fn test_ffi() { VacuumState::reset(); assert!((vs_energy() - 0.0).abs() < 0.01 || VacuumState::vacuum_energy().is_infinite()); }
}
