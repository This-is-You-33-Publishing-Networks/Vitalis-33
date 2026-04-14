//! Infinite Regress Resolver — Vitalis v1054
//!
//! Resolves infinite self-reference chains in meta-compilation
//! using fixed-point semantics and bounded approximation.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<IrrState>> = LazyLock::new(|| Mutex::new(IrrState::default()));

#[derive(Default)]
struct IrrState {
    chains: Vec<(String, u32)>,
    resolutions: Vec<(String, f64)>,
    max_depth: u32,
    fixed_points_found: u64,
}

pub struct InfiniteRegressResolver;

impl InfiniteRegressResolver {
    pub fn detect_regress(name: &str, depth: u32) -> bool {
        let mut s = STATE.lock().unwrap();
        s.chains.push((name.to_string(), depth));
        if depth > s.max_depth { s.max_depth = depth; }
        depth > 10
    }

    pub fn resolve_fixpoint(name: &str, max_iterations: u32) -> f64 {
        let mut s = STATE.lock().unwrap();
        let mut value = 0.5_f64;
        for _ in 0..max_iterations {
            let next = (value + 1.0 / (value + 1.0)) / 2.0;
            if (next - value).abs() < 1e-10 { break; }
            value = next;
        }
        s.fixed_points_found += 1;
        s.resolutions.push((name.to_string(), value));
        value
    }

    pub fn approximate_bounded(depth_limit: u32) -> f64 {
        let s = STATE.lock().unwrap();
        let total_depth: u32 = s.chains.iter().map(|(_, d)| d).sum();
        (total_depth as f64 / (depth_limit as f64).max(1.0)).min(1.0)
    }

    pub fn fixed_points_found() -> u64 { STATE.lock().unwrap().fixed_points_found }
    pub fn max_depth() -> u32 { STATE.lock().unwrap().max_depth }
    pub fn chain_count() -> usize { STATE.lock().unwrap().chains.len() }
    pub fn reset() { *STATE.lock().unwrap() = IrrState::default(); }
}

#[unsafe(no_mangle)]
pub extern "C" fn irr_detect(depth: i64) -> i64 { if InfiniteRegressResolver::detect_regress("ffi", depth as u32) { 1 } else { 0 } }
#[unsafe(no_mangle)]
pub extern "C" fn irr_resolve(iters: i64) -> f64 { InfiniteRegressResolver::resolve_fixpoint("ffi", iters as u32) }
#[unsafe(no_mangle)]
pub extern "C" fn irr_fixpoints() -> i64 { InfiniteRegressResolver::fixed_points_found() as i64 }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_no_regress() { InfiniteRegressResolver::reset(); assert!(!InfiniteRegressResolver::detect_regress("a", 5)); }
    #[test] fn test_regress() { InfiniteRegressResolver::reset(); assert!(InfiniteRegressResolver::detect_regress("a", 20)); }
    #[test] fn test_fixpoint() { InfiniteRegressResolver::reset(); let v = InfiniteRegressResolver::resolve_fixpoint("f", 1000); assert!(v > 0.0 && v < 1.0); }
    #[test] fn test_fixpoint_convergence() { InfiniteRegressResolver::reset(); let v1 = InfiniteRegressResolver::resolve_fixpoint("a", 100); let v2 = InfiniteRegressResolver::resolve_fixpoint("b", 1000); assert!((v1 - v2).abs() < 0.01); }
    #[test] fn test_bounded() { InfiniteRegressResolver::reset(); InfiniteRegressResolver::detect_regress("a", 5); let b = InfiniteRegressResolver::approximate_bounded(10); assert!(b >= 0.0 && b <= 1.0); }
    #[test] fn test_ffi() { InfiniteRegressResolver::reset(); assert_eq!(irr_fixpoints(), 0); }
}
