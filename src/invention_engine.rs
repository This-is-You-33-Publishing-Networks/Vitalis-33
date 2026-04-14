//! Invention Engine — Vitalis v1279
//!
//! Invents new algorithms and data structures on demand.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<InventionEngineState>> = LazyLock::new(|| Mutex::new(InventionEngineState::default()));

#[derive(Default)]
struct InventionEngineState {
    inventions: Vec<(String, String)>,
    patents: u64,
    utility_scores: Vec<f64>,
}

pub struct InventionEngine;

impl InventionEngine {
    pub fn invent(name: &str, kind: &str, utility: f64) {
        let mut s = STATE.lock().unwrap();
        s.inventions.push((name.to_string(), kind.to_string()));
        s.utility_scores.push(utility);
    }
    pub fn patent() -> u64 {
        let mut s = STATE.lock().unwrap();
        s.patents += 1;
        s.patents
    }
    pub fn invention_count() -> usize { STATE.lock().unwrap().inventions.len() }
    pub fn max_utility() -> f64 { STATE.lock().unwrap().utility_scores.iter().cloned().fold(0.0_f64, f64::max) }
    pub fn reset() { *STATE.lock().unwrap() = InventionEngineState::default(); }
}

#[unsafe(no_mangle)]
pub extern "C" fn inv_patent() -> u64 { InventionEngine::patent() }
#[unsafe(no_mangle)]
pub extern "C" fn inv_invention_count() -> usize { InventionEngine::invention_count() }
#[unsafe(no_mangle)]
pub extern "C" fn inv_max_utility() -> f64 { InventionEngine::max_utility() }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_invent() { InventionEngine::reset(); InventionEngine::invent("skip-list-v2", "data_structure", 0.9); assert_eq!(InventionEngine::invention_count(), 1); }
    #[test] fn test_patent() { InventionEngine::reset(); let p = InventionEngine::patent(); assert_eq!(p, 1); }
    #[test] fn test_max_utility() { InventionEngine::reset(); InventionEngine::invent("a", "algo", 0.5); InventionEngine::invent("b", "ds", 0.8); assert!((InventionEngine::max_utility() - 0.8).abs() < 1e-9); }
    #[test] fn test_empty_utility() { InventionEngine::reset(); assert!((InventionEngine::max_utility() - 0.0).abs() < 1e-9); }
    #[test] fn test_multiple_patents() { InventionEngine::reset(); InventionEngine::patent(); let p = InventionEngine::patent(); assert_eq!(p, 2); }
    #[test] fn test_kinds() { InventionEngine::reset(); InventionEngine::invent("x", "algorithm", 1.0); assert_eq!(STATE.lock().unwrap().inventions[0].1, "algorithm"); }
    #[test] fn test_reset() { InventionEngine::reset(); InventionEngine::invent("z", "ds", 0.5); InventionEngine::reset(); assert_eq!(InventionEngine::invention_count(), 0); }
}
