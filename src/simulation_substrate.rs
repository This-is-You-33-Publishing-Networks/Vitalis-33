//! Simulation Substrate — Vitalis v1075
//!
//! Programs can instantiate sub-universes with their own physics
//! and execution rules. Nested simulation management.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<SsubState>> = LazyLock::new(|| Mutex::new(SsubState::default()));

#[derive(Default)]
struct SsubState {
    universes: Vec<(String, Vec<String>, bool)>,
    tick_count: u64,
}

pub struct SimulationSubstrate;

impl SimulationSubstrate {
    pub fn create_universe(name: &str, rules: &[&str]) -> usize {
        let mut s = STATE.lock().unwrap();
        s.universes.push((name.to_string(), rules.iter().map(|r| r.to_string()).collect(), true));
        s.universes.len()
    }

    pub fn tick(name: &str) -> bool {
        let mut s = STATE.lock().unwrap();
        s.tick_count += 1;
        s.universes.iter().any(|(n, _, active)| n == name && *active)
    }

    pub fn halt_universe(name: &str) -> bool {
        let mut s = STATE.lock().unwrap();
        if let Some(u) = s.universes.iter_mut().find(|(n, _, _)| n == name) { u.2 = false; true } else { false }
    }

    pub fn active_universes() -> usize { STATE.lock().unwrap().universes.iter().filter(|(_, _, a)| *a).count() }
    pub fn universe_count() -> usize { STATE.lock().unwrap().universes.len() }
    pub fn total_ticks() -> u64 { STATE.lock().unwrap().tick_count }
    pub fn reset() { *STATE.lock().unwrap() = SsubState::default(); }
}

#[unsafe(no_mangle)]
pub extern "C" fn ssub_create(id: i64) -> i64 { SimulationSubstrate::create_universe(&format!("u{}", id), &["gravity"]) as i64 }
#[unsafe(no_mangle)]
pub extern "C" fn ssub_tick(id: i64) -> i64 { if SimulationSubstrate::tick(&format!("u{}", id)) { 1 } else { 0 } }
#[unsafe(no_mangle)]
pub extern "C" fn ssub_active() -> i64 { SimulationSubstrate::active_universes() as i64 }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_create() { SimulationSubstrate::reset(); assert_eq!(SimulationSubstrate::create_universe("u1", &["gravity"]), 1); }
    #[test] fn test_tick() { SimulationSubstrate::reset(); SimulationSubstrate::create_universe("u1", &[]); assert!(SimulationSubstrate::tick("u1")); }
    #[test] fn test_tick_nonexistent() { SimulationSubstrate::reset(); assert!(!SimulationSubstrate::tick("nope")); }
    #[test] fn test_halt() { SimulationSubstrate::reset(); SimulationSubstrate::create_universe("u1", &[]); assert!(SimulationSubstrate::halt_universe("u1")); assert!(!SimulationSubstrate::tick("u1")); }
    #[test] fn test_active() { SimulationSubstrate::reset(); SimulationSubstrate::create_universe("u1", &[]); SimulationSubstrate::create_universe("u2", &[]); SimulationSubstrate::halt_universe("u1"); assert_eq!(SimulationSubstrate::active_universes(), 1); }
    #[test] fn test_total_ticks() { SimulationSubstrate::reset(); SimulationSubstrate::create_universe("u1", &[]); SimulationSubstrate::tick("u1"); assert_eq!(SimulationSubstrate::total_ticks(), 1); }
    #[test] fn test_universe_count() { SimulationSubstrate::reset(); SimulationSubstrate::create_universe("a", &[]); assert_eq!(SimulationSubstrate::universe_count(), 1); }
    #[test] fn test_ffi() { SimulationSubstrate::reset(); assert_eq!(ssub_active(), 0); }
}
