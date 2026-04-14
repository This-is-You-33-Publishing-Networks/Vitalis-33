//! Abstraction Lattice — Vitalis v1140
//!
//! Lattice of all abstractions enabling automatic movement
//! between abstraction levels — from bits to concepts.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<AlState>> = LazyLock::new(|| Mutex::new(AlState::default()));

#[derive(Default)]
struct AlState {
    levels: Vec<(String, u32)>,
    transitions: Vec<(String, String)>,
    current_level: u32,
}

pub struct AbstractionLattice;

impl AbstractionLattice {
    pub fn add_level(name: &str, rank: u32) -> usize { let mut s = STATE.lock().unwrap(); s.levels.push((name.to_string(), rank)); s.levels.len() }

    pub fn ascend(from: &str, to: &str) -> bool {
        let mut s = STATE.lock().unwrap();
        let from_rank = s.levels.iter().find(|(n, _)| n == from).map(|(_, r)| *r);
        let to_rank = s.levels.iter().find(|(n, _)| n == to).map(|(_, r)| *r);
        if let (Some(fr), Some(tr)) = (from_rank, to_rank) {
            if tr > fr { s.transitions.push((from.to_string(), to.to_string())); s.current_level = tr; return true; }
        }
        false
    }

    pub fn descend(from: &str, to: &str) -> bool {
        let mut s = STATE.lock().unwrap();
        let from_rank = s.levels.iter().find(|(n, _)| n == from).map(|(_, r)| *r);
        let to_rank = s.levels.iter().find(|(n, _)| n == to).map(|(_, r)| *r);
        if let (Some(fr), Some(tr)) = (from_rank, to_rank) {
            if tr < fr { s.transitions.push((from.to_string(), to.to_string())); s.current_level = tr; return true; }
        }
        false
    }

    pub fn current_level() -> u32 { STATE.lock().unwrap().current_level }
    pub fn level_count() -> usize { STATE.lock().unwrap().levels.len() }
    pub fn transition_count() -> usize { STATE.lock().unwrap().transitions.len() }
    pub fn reset() { *STATE.lock().unwrap() = AlState::default(); }
}

#[unsafe(no_mangle)]
pub extern "C" fn al_add(rank: i64) -> i64 { AbstractionLattice::add_level(&format!("l{}", rank), rank as u32) as i64 }
#[unsafe(no_mangle)]
pub extern "C" fn al_level() -> i64 { AbstractionLattice::current_level() as i64 }
#[unsafe(no_mangle)]
pub extern "C" fn al_levels() -> i64 { AbstractionLattice::level_count() as i64 }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_add() { AbstractionLattice::reset(); assert_eq!(AbstractionLattice::add_level("bits", 0), 1); }
    #[test] fn test_ascend() { AbstractionLattice::reset(); AbstractionLattice::add_level("low", 1); AbstractionLattice::add_level("high", 5); assert!(AbstractionLattice::ascend("low", "high")); }
    #[test] fn test_ascend_fail() { AbstractionLattice::reset(); AbstractionLattice::add_level("a", 5); AbstractionLattice::add_level("b", 1); assert!(!AbstractionLattice::ascend("a", "b")); }
    #[test] fn test_descend() { AbstractionLattice::reset(); AbstractionLattice::add_level("high", 10); AbstractionLattice::add_level("low", 1); assert!(AbstractionLattice::descend("high", "low")); }
    #[test] fn test_current() { AbstractionLattice::reset(); AbstractionLattice::add_level("a", 1); AbstractionLattice::add_level("b", 5); AbstractionLattice::ascend("a", "b"); assert_eq!(AbstractionLattice::current_level(), 5); }
    #[test] fn test_ffi() { AbstractionLattice::reset(); assert_eq!(al_level(), 0); }
}
