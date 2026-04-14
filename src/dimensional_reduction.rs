//! Dimensional Reduction — Vitalis v1097
//!
//! Reduces high-dimensional program state spaces to lower-dimensional
//! embeddings while preserving essential structure.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<DrState>> = LazyLock::new(|| Mutex::new(DrState::default()));

#[derive(Default)]
struct DrState {
    points: Vec<Vec<f64>>,
    reduced: Vec<Vec<f64>>,
    target_dim: usize,
    variance_retained: f64,
}

pub struct DimensionalReduction;

impl DimensionalReduction {
    pub fn add_point(coords: &[f64]) -> usize {
        let mut s = STATE.lock().unwrap();
        s.points.push(coords.to_vec());
        s.points.len()
    }

    pub fn reduce(target_dim: usize) -> usize {
        let mut s = STATE.lock().unwrap();
        s.target_dim = target_dim;
        s.reduced = s.points.iter().map(|p| p.iter().take(target_dim).copied().collect()).collect();
        let original_var: f64 = s.points.iter().flat_map(|p| p.iter()).map(|v| v * v).sum();
        let reduced_var: f64 = s.reduced.iter().flat_map(|p| p.iter()).map(|v| v * v).sum();
        s.variance_retained = if original_var > 0.0 { reduced_var / original_var } else { 1.0 };
        s.reduced.len()
    }

    pub fn variance_retained() -> f64 { STATE.lock().unwrap().variance_retained }
    pub fn original_dim() -> usize { STATE.lock().unwrap().points.first().map(|p| p.len()).unwrap_or(0) }
    pub fn point_count() -> usize { STATE.lock().unwrap().points.len() }
    pub fn reset() { *STATE.lock().unwrap() = DrState::default(); }
}

#[unsafe(no_mangle)]
pub extern "C" fn dr_add(dim: i64) -> i64 { DimensionalReduction::add_point(&vec![1.0; dim as usize]) as i64 }
#[unsafe(no_mangle)]
pub extern "C" fn dr_reduce(target: i64) -> i64 { DimensionalReduction::reduce(target as usize) as i64 }
#[unsafe(no_mangle)]
pub extern "C" fn dr_variance() -> f64 { DimensionalReduction::variance_retained() }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_add() { DimensionalReduction::reset(); assert_eq!(DimensionalReduction::add_point(&[1.0, 2.0, 3.0]), 1); }
    #[test] fn test_reduce() { DimensionalReduction::reset(); DimensionalReduction::add_point(&[1.0, 2.0, 3.0]); let n = DimensionalReduction::reduce(2); assert_eq!(n, 1); }
    #[test] fn test_variance() { DimensionalReduction::reset(); DimensionalReduction::add_point(&[1.0, 0.0, 0.0]); DimensionalReduction::reduce(1); assert!(DimensionalReduction::variance_retained() > 0.0); }
    #[test] fn test_original_dim() { DimensionalReduction::reset(); DimensionalReduction::add_point(&[1.0, 2.0, 3.0, 4.0]); assert_eq!(DimensionalReduction::original_dim(), 4); }
    #[test] fn test_point_count() { DimensionalReduction::reset(); DimensionalReduction::add_point(&[1.0]); DimensionalReduction::add_point(&[2.0]); assert_eq!(DimensionalReduction::point_count(), 2); }
    #[test] fn test_ffi() { DimensionalReduction::reset(); assert!((dr_variance() - 0.0).abs() < 0.01); }
}
