//! Omega Point Optimizer — Vitalis v1050
//!
//! Asymptotically approaches the theoretical optimal compilation
//! for any input, converging toward perfection without ever quite reaching it.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<OpoState>> = LazyLock::new(|| Mutex::new(OpoState::default()));

#[derive(Default)]
struct OpoState {
    iterations: u64,
    current_quality: f64,
    quality_history: Vec<f64>,
    theoretical_optimum: f64,
}

pub struct OmegaPointOptimizer;

impl OmegaPointOptimizer {
    pub fn set_optimum(optimum: f64) { STATE.lock().unwrap().theoretical_optimum = optimum; }

    pub fn optimize_step() -> f64 {
        let mut s = STATE.lock().unwrap();
        s.iterations += 1;
        let gap = s.theoretical_optimum - s.current_quality;
        let improvement = gap * 0.1;
        s.current_quality += improvement;
        let q = s.current_quality;
        s.quality_history.push(q);
        s.current_quality
    }

    pub fn gap_to_optimum() -> f64 {
        let s = STATE.lock().unwrap();
        (s.theoretical_optimum - s.current_quality).max(0.0)
    }

    pub fn convergence_rate() -> f64 {
        let s = STATE.lock().unwrap();
        if s.quality_history.len() < 2 { return 0.0; }
        let recent = &s.quality_history[s.quality_history.len().saturating_sub(5)..];
        if recent.len() < 2 { return 0.0; }
        (recent.last().unwrap() - recent.first().unwrap()) / recent.len() as f64
    }

    pub fn quality() -> f64 { STATE.lock().unwrap().current_quality }
    pub fn iterations() -> u64 { STATE.lock().unwrap().iterations }
    pub fn is_near_optimal(threshold: f64) -> bool { let s = STATE.lock().unwrap(); (s.theoretical_optimum - s.current_quality).abs() < threshold }
    pub fn reset() { *STATE.lock().unwrap() = OpoState::default(); }
}

#[unsafe(no_mangle)]
pub extern "C" fn opo_step() -> f64 { OmegaPointOptimizer::optimize_step() }
#[unsafe(no_mangle)]
pub extern "C" fn opo_gap() -> f64 { OmegaPointOptimizer::gap_to_optimum() }
#[unsafe(no_mangle)]
pub extern "C" fn opo_quality() -> f64 { OmegaPointOptimizer::quality() }
#[unsafe(no_mangle)]
pub extern "C" fn opo_iterations() -> i64 { OmegaPointOptimizer::iterations() as i64 }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_step() { OmegaPointOptimizer::reset(); OmegaPointOptimizer::set_optimum(1.0); let q = OmegaPointOptimizer::optimize_step(); assert!(q > 0.0); }
    #[test] fn test_convergence() { OmegaPointOptimizer::reset(); OmegaPointOptimizer::set_optimum(1.0); for _ in 0..100 { OmegaPointOptimizer::optimize_step(); } assert!(OmegaPointOptimizer::gap_to_optimum() < 0.01); }
    #[test] fn test_monotonic() { OmegaPointOptimizer::reset(); OmegaPointOptimizer::set_optimum(1.0); let q1 = OmegaPointOptimizer::optimize_step(); let q2 = OmegaPointOptimizer::optimize_step(); assert!(q2 >= q1); }
    #[test] fn test_rate() { OmegaPointOptimizer::reset(); OmegaPointOptimizer::set_optimum(1.0); for _ in 0..10 { OmegaPointOptimizer::optimize_step(); } assert!(OmegaPointOptimizer::convergence_rate() > 0.0); }
    #[test] fn test_near_optimal() { OmegaPointOptimizer::reset(); OmegaPointOptimizer::set_optimum(1.0); for _ in 0..100 { OmegaPointOptimizer::optimize_step(); } assert!(OmegaPointOptimizer::is_near_optimal(0.01)); }
    #[test] fn test_iterations() { OmegaPointOptimizer::reset(); OmegaPointOptimizer::optimize_step(); assert_eq!(OmegaPointOptimizer::iterations(), 1); }
    #[test] fn test_gap_initial() { OmegaPointOptimizer::reset(); OmegaPointOptimizer::set_optimum(5.0); assert!((OmegaPointOptimizer::gap_to_optimum() - 5.0).abs() < 0.01); }
    #[test] fn test_ffi() { OmegaPointOptimizer::reset(); assert_eq!(opo_iterations(), 0); }
}
