//! Cantor Hierarchy — Vitalis v1233
//!
//! Infinite cardinality hierarchies for type expressiveness.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<CantorState>> = LazyLock::new(|| Mutex::new(CantorState::default()));

#[derive(Default)]
struct CantorState {
    levels: Vec<(String, u32)>,
    current_aleph: u32,
}

pub struct CantorHierarchy;

impl CantorHierarchy {
    pub fn ascend(name: &str) -> u32 {
        let mut s = STATE.lock().unwrap();
        s.current_aleph += 1;
        let aleph = s.current_aleph;
        s.levels.push((name.to_string(), aleph));
        aleph
    }

    pub fn current_aleph() -> u32 { STATE.lock().unwrap().current_aleph }

    pub fn level_count() -> usize { STATE.lock().unwrap().levels.len() }

    pub fn power_set() -> u32 {
        let mut s = STATE.lock().unwrap();
        s.current_aleph += 1;
        s.current_aleph
    }

    pub fn has_level(name: &str) -> bool {
        STATE.lock().unwrap().levels.iter().any(|(n, _)| n == name)
    }

    pub fn reset() { *STATE.lock().unwrap() = CantorState::default(); }
}

#[unsafe(no_mangle)]
pub extern "C" fn cant_ascend() -> i64 { CantorHierarchy::ascend("ffi_level") as i64 }
#[unsafe(no_mangle)]
pub extern "C" fn cant_aleph() -> i64 { CantorHierarchy::current_aleph() as i64 }
#[unsafe(no_mangle)]
pub extern "C" fn cant_levels() -> i64 { CantorHierarchy::level_count() as i64 }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_ascend() { CantorHierarchy::reset(); assert_eq!(CantorHierarchy::ascend("aleph0"), 1); }
    #[test] fn test_current() { CantorHierarchy::reset(); CantorHierarchy::ascend("a"); assert_eq!(CantorHierarchy::current_aleph(), 1); }
    #[test] fn test_multiple() { CantorHierarchy::reset(); CantorHierarchy::ascend("a"); CantorHierarchy::ascend("b"); assert_eq!(CantorHierarchy::current_aleph(), 2); }
    #[test] fn test_power_set() { CantorHierarchy::reset(); CantorHierarchy::ascend("a"); let p = CantorHierarchy::power_set(); assert!(p > 1); }
    #[test] fn test_has_level() { CantorHierarchy::reset(); CantorHierarchy::ascend("continuum"); assert!(CantorHierarchy::has_level("continuum")); }
    #[test] fn test_reset() { CantorHierarchy::reset(); CantorHierarchy::ascend("x"); CantorHierarchy::reset(); assert_eq!(CantorHierarchy::level_count(), 0); }
    #[test] fn test_ffi() { CantorHierarchy::reset(); cant_ascend(); assert_eq!(cant_aleph(), 1); }
}
