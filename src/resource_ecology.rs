//! Resource Ecology — Vitalis v1289
//!
//! Ecological resource management with balanced usage.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<ResourceEcologyState>> = LazyLock::new(|| Mutex::new(ResourceEcologyState::default()));

#[derive(Default)]
struct ResourceEcologyState {
    resources: Vec<(String, f64, f64)>,
    balance_score: f64,
    cycles: u64,
}

pub struct ResourceEcology;

impl ResourceEcology {
    pub fn add_resource(name: &str, capacity: f64, usage: f64) {
        let mut s = STATE.lock().unwrap();
        s.resources.push((name.to_string(), capacity, usage));
    }
    pub fn cycle() -> u64 {
        let mut s = STATE.lock().unwrap();
        s.cycles += 1;
        if !s.resources.is_empty() {
            let ratios: Vec<f64> = s.resources.iter().map(|r| r.2 / r.1.max(0.001)).collect();
            s.balance_score = 1.0 - ratios.iter().map(|r| (r - 0.5).abs()).sum::<f64>() / ratios.len() as f64;
        }
        s.cycles
    }
    pub fn balance_score() -> f64 { STATE.lock().unwrap().balance_score }
    pub fn resource_count() -> usize { STATE.lock().unwrap().resources.len() }
    pub fn reset() { *STATE.lock().unwrap() = ResourceEcologyState::default(); }
}

#[unsafe(no_mangle)]
pub extern "C" fn reco_cycle() -> u64 { ResourceEcology::cycle() }
#[unsafe(no_mangle)]
pub extern "C" fn reco_balance_score() -> f64 { ResourceEcology::balance_score() }
#[unsafe(no_mangle)]
pub extern "C" fn reco_resource_count() -> usize { ResourceEcology::resource_count() }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_add_resource() { ResourceEcology::reset(); ResourceEcology::add_resource("cpu", 100.0, 50.0); assert_eq!(ResourceEcology::resource_count(), 1); }
    #[test] fn test_cycle() { ResourceEcology::reset(); let c = ResourceEcology::cycle(); assert_eq!(c, 1); }
    #[test] fn test_balance_with_resources() { ResourceEcology::reset(); ResourceEcology::add_resource("mem", 100.0, 50.0); ResourceEcology::cycle(); assert!(ResourceEcology::balance_score() > 0.0); }
    #[test] fn test_initial_balance() { ResourceEcology::reset(); assert!((ResourceEcology::balance_score() - 0.0).abs() < 1e-9); }
    #[test] fn test_multiple_resources() { ResourceEcology::reset(); ResourceEcology::add_resource("a", 10.0, 5.0); ResourceEcology::add_resource("b", 20.0, 10.0); assert_eq!(ResourceEcology::resource_count(), 2); }
    #[test] fn test_cycles_accumulate() { ResourceEcology::reset(); ResourceEcology::cycle(); let c = ResourceEcology::cycle(); assert_eq!(c, 2); }
    #[test] fn test_reset() { ResourceEcology::reset(); ResourceEcology::add_resource("x", 1.0, 0.5); ResourceEcology::reset(); assert_eq!(ResourceEcology::resource_count(), 0); }
}
