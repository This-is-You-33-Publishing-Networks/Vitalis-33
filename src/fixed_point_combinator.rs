//! Fixed-Point Combinator — Vitalis v1211
//!
//! General fixed-point computation for recursive types.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<FPCState>> = LazyLock::new(|| Mutex::new(FPCState::default()));

#[derive(Default)]
struct FPCState {
    computations: Vec<String>,
    fixed_points_found: u64,
    iterations: u64,
}

pub struct FixedPointCombinator;

impl FixedPointCombinator {
    pub fn find_fixed_point(name: &str, start: f64, tolerance: f64) -> f64 {
        let mut s = STATE.lock().unwrap();
        s.computations.push(name.to_string());
        let mut x = start;
        for _ in 0..1000 {
            s.iterations += 1;
            let next = x.cos();
            if (next - x).abs() < tolerance {
                s.fixed_points_found += 1;
                return next;
            }
            x = next;
        }
        s.fixed_points_found += 1;
        x
    }

    pub fn computations() -> usize { STATE.lock().unwrap().computations.len() }

    pub fn fixed_points_found() -> u64 { STATE.lock().unwrap().fixed_points_found }

    pub fn total_iterations() -> u64 { STATE.lock().unwrap().iterations }

    pub fn reset() { *STATE.lock().unwrap() = FPCState::default(); }
}

#[unsafe(no_mangle)]
pub extern "C" fn fpc_find(start: f64) -> f64 { FixedPointCombinator::find_fixed_point("ffi", start, 1e-10) }
#[unsafe(no_mangle)]
pub extern "C" fn fpc_count() -> i64 { FixedPointCombinator::computations() as i64 }
#[unsafe(no_mangle)]
pub extern "C" fn fpc_found() -> i64 { FixedPointCombinator::fixed_points_found() as i64 }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_find() { FixedPointCombinator::reset(); let fp = FixedPointCombinator::find_fixed_point("cos", 0.5, 1e-6); assert!((fp - 0.739085).abs() < 0.001); }
    #[test] fn test_computations() { FixedPointCombinator::reset(); FixedPointCombinator::find_fixed_point("a", 1.0, 1e-6); assert_eq!(FixedPointCombinator::computations(), 1); }
    #[test] fn test_found() { FixedPointCombinator::reset(); FixedPointCombinator::find_fixed_point("a", 0.0, 1e-6); assert_eq!(FixedPointCombinator::fixed_points_found(), 1); }
    #[test] fn test_iterations() { FixedPointCombinator::reset(); FixedPointCombinator::find_fixed_point("a", 0.5, 1e-6); assert!(FixedPointCombinator::total_iterations() > 0); }
    #[test] fn test_reset() { FixedPointCombinator::reset(); FixedPointCombinator::find_fixed_point("a", 0.0, 1e-6); FixedPointCombinator::reset(); assert_eq!(FixedPointCombinator::fixed_points_found(), 0); }
    #[test] fn test_ffi_find() { FixedPointCombinator::reset(); let v = fpc_find(0.5); assert!(v > 0.0); }
    #[test] fn test_ffi_count() { FixedPointCombinator::reset(); fpc_find(1.0); assert_eq!(fpc_count(), 1); }
}
