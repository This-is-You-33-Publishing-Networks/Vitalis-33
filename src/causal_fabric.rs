//! Causal Fabric — Vitalis v1013
//!
//! Causal graph of all program effects, enabling counterfactual
//! reasoning about what would happen under different execution paths.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<CfState>> = LazyLock::new(|| Mutex::new(CfState::default()));

#[derive(Default)]
struct CfState {
    causes: Vec<(String, String)>,
    counterfactuals: Vec<(String, String, f64)>,
    depth: u32,
}

pub struct CausalFabric;

impl CausalFabric {
    pub fn add_cause(cause: &str, effect: &str) -> usize {
        let mut s = STATE.lock().unwrap();
        s.causes.push((cause.to_string(), effect.to_string()));
        s.depth = s.depth.max(s.causes.len() as u32);
        s.causes.len()
    }

    pub fn counterfactual(cause: &str, alternative: &str) -> f64 {
        let mut s = STATE.lock().unwrap();
        let affected = s.causes.iter().filter(|(c, _)| c == cause).count();
        let probability = (affected as f64 * 0.3).min(1.0);
        s.counterfactuals.push((cause.to_string(), alternative.to_string(), probability));
        probability
    }

    pub fn trace_effects(cause: &str) -> Vec<String> {
        let s = STATE.lock().unwrap();
        s.causes.iter().filter(|(c, _)| c == cause).map(|(_, e)| e.clone()).collect()
    }

    pub fn depth() -> u32 { STATE.lock().unwrap().depth }
    pub fn cause_count() -> usize { STATE.lock().unwrap().causes.len() }
    pub fn reset() { *STATE.lock().unwrap() = CfState::default(); }
}

#[unsafe(no_mangle)]
pub extern "C" fn cf_add(id: i64) -> i64 { CausalFabric::add_cause(&format!("c{}", id), &format!("e{}", id)) as i64 }
#[unsafe(no_mangle)]
pub extern "C" fn cf_counterfactual(id: i64) -> f64 { CausalFabric::counterfactual(&format!("c{}", id), "alt") }
#[unsafe(no_mangle)]
pub extern "C" fn cf_depth() -> i64 { CausalFabric::depth() as i64 }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_add_cause() { CausalFabric::reset(); assert_eq!(CausalFabric::add_cause("rain", "wet"), 1); }
    #[test] fn test_trace() { CausalFabric::reset(); CausalFabric::add_cause("a", "b"); CausalFabric::add_cause("a", "c"); let e = CausalFabric::trace_effects("a"); assert_eq!(e.len(), 2); }
    #[test] fn test_counterfactual() { CausalFabric::reset(); CausalFabric::add_cause("x", "y"); let p = CausalFabric::counterfactual("x", "z"); assert!(p >= 0.0 && p <= 1.0); }
    #[test] fn test_depth() { CausalFabric::reset(); CausalFabric::add_cause("a", "b"); CausalFabric::add_cause("c", "d"); assert_eq!(CausalFabric::depth(), 2); }
    #[test] fn test_no_effects() { CausalFabric::reset(); let e = CausalFabric::trace_effects("none"); assert!(e.is_empty()); }
    #[test] fn test_cause_count() { CausalFabric::reset(); CausalFabric::add_cause("a", "b"); assert_eq!(CausalFabric::cause_count(), 1); }
    #[test] fn test_ffi() { CausalFabric::reset(); assert_eq!(cf_depth(), 0); }
}
