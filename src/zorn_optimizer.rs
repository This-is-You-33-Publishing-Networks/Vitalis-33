//! Zorn Optimizer — Vitalis v1235
//!
//! Zorn's lemma for maximal optimization chains.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<ZornState>> = LazyLock::new(|| Mutex::new(ZornState::default()));

#[derive(Default)]
struct ZornState {
    chains: Vec<Vec<f64>>,
    maximal_elements: Vec<f64>,
    applications: u64,
}

pub struct ZornOptimizer;

impl ZornOptimizer {
    pub fn add_chain(chain: Vec<f64>) -> f64 {
        let mut s = STATE.lock().unwrap();
        let max = chain.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        s.maximal_elements.push(max);
        s.chains.push(chain);
        s.applications += 1;
        max
    }

    pub fn chain_count() -> usize { STATE.lock().unwrap().chains.len() }

    pub fn global_maximum() -> f64 {
        let s = STATE.lock().unwrap();
        s.maximal_elements.iter().cloned().fold(f64::NEG_INFINITY, f64::max)
    }

    pub fn applications() -> u64 { STATE.lock().unwrap().applications }

    pub fn reset() { *STATE.lock().unwrap() = ZornState::default(); }
}

#[unsafe(no_mangle)]
pub extern "C" fn zorn_add() -> f64 { ZornOptimizer::add_chain(vec![1.0, 2.0, 3.0]) }
#[unsafe(no_mangle)]
pub extern "C" fn zorn_max() -> f64 { ZornOptimizer::global_maximum() }
#[unsafe(no_mangle)]
pub extern "C" fn zorn_apps() -> i64 { ZornOptimizer::applications() as i64 }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_add_chain() { ZornOptimizer::reset(); let m = ZornOptimizer::add_chain(vec![1.0, 5.0, 3.0]); assert!((m - 5.0).abs() < f64::EPSILON); }
    #[test] fn test_chain_count() { ZornOptimizer::reset(); ZornOptimizer::add_chain(vec![1.0]); assert_eq!(ZornOptimizer::chain_count(), 1); }
    #[test] fn test_global_max() { ZornOptimizer::reset(); ZornOptimizer::add_chain(vec![2.0]); ZornOptimizer::add_chain(vec![9.0]); assert!((ZornOptimizer::global_maximum() - 9.0).abs() < f64::EPSILON); }
    #[test] fn test_applications() { ZornOptimizer::reset(); ZornOptimizer::add_chain(vec![1.0]); ZornOptimizer::add_chain(vec![2.0]); assert_eq!(ZornOptimizer::applications(), 2); }
    #[test] fn test_reset() { ZornOptimizer::reset(); ZornOptimizer::add_chain(vec![1.0]); ZornOptimizer::reset(); assert_eq!(ZornOptimizer::chain_count(), 0); }
    #[test] fn test_ffi() { ZornOptimizer::reset(); let m = zorn_add(); assert!((m - 3.0).abs() < f64::EPSILON); assert_eq!(zorn_apps(), 1); }
}
