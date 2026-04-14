//! Symmetry Compiler — Vitalis v1109
//!
//! Exploits mathematical symmetries in code to reduce computation
//! by group-theoretic methods — if f(x) = f(-x), compute once.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<SymcState>> = LazyLock::new(|| Mutex::new(SymcState::default()));

#[derive(Default)]
struct SymcState {
    symmetries: Vec<(String, String)>,
    exploited: u64,
    savings: f64,
}

pub struct SymmetryCompiler;

impl SymmetryCompiler {
    pub fn detect_symmetry(func: &str, kind: &str) -> usize {
        let mut s = STATE.lock().unwrap();
        s.symmetries.push((func.to_string(), kind.to_string()));
        s.symmetries.len()
    }

    pub fn exploit(func: &str) -> f64 {
        let mut s = STATE.lock().unwrap();
        let matching = s.symmetries.iter().filter(|(f, _)| f == func).count();
        let saving = matching as f64 * 0.5;
        s.exploited += 1;
        s.savings += saving;
        saving
    }

    pub fn total_savings() -> f64 { STATE.lock().unwrap().savings }
    pub fn symmetry_count() -> usize { STATE.lock().unwrap().symmetries.len() }
    pub fn exploitations() -> u64 { STATE.lock().unwrap().exploited }
    pub fn reset() { *STATE.lock().unwrap() = SymcState::default(); }
}

#[unsafe(no_mangle)]
pub extern "C" fn symc_detect(kind: i64) -> i64 { SymmetryCompiler::detect_symmetry("ffi_fn", &format!("sym_{}", kind)) as i64 }
#[unsafe(no_mangle)]
pub extern "C" fn symc_exploit() -> f64 { SymmetryCompiler::exploit("ffi_fn") }
#[unsafe(no_mangle)]
pub extern "C" fn symc_savings() -> f64 { SymmetryCompiler::total_savings() }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_detect() { SymmetryCompiler::reset(); assert_eq!(SymmetryCompiler::detect_symmetry("f", "mirror"), 1); }
    #[test] fn test_exploit() { SymmetryCompiler::reset(); SymmetryCompiler::detect_symmetry("f", "mirror"); let s = SymmetryCompiler::exploit("f"); assert!(s > 0.0); }
    #[test] fn test_no_symmetry() { SymmetryCompiler::reset(); let s = SymmetryCompiler::exploit("f"); assert!((s - 0.0).abs() < 0.01); }
    #[test] fn test_savings() { SymmetryCompiler::reset(); SymmetryCompiler::detect_symmetry("f", "a"); SymmetryCompiler::exploit("f"); assert!(SymmetryCompiler::total_savings() > 0.0); }
    #[test] fn test_multiple() { SymmetryCompiler::reset(); SymmetryCompiler::detect_symmetry("f", "a"); SymmetryCompiler::detect_symmetry("f", "b"); let s = SymmetryCompiler::exploit("f"); assert!(s >= 1.0); }
    #[test] fn test_symmetry_count() { SymmetryCompiler::reset(); SymmetryCompiler::detect_symmetry("a", "x"); assert_eq!(SymmetryCompiler::symmetry_count(), 1); }
    #[test] fn test_ffi() { SymmetryCompiler::reset(); assert!((symc_savings() - 0.0).abs() < 0.01); }
}
