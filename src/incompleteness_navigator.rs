//! Incompleteness Navigator — Vitalis v1213
//!
//! Navigates Godel incompleteness by switching axiom systems.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<IncNavState>> = LazyLock::new(|| Mutex::new(IncNavState::default()));

#[derive(Default)]
struct IncNavState {
    axiom_systems: Vec<String>,
    switches: u64,
    unprovable_detected: u64,
}

pub struct IncompletenessNavigator;

impl IncompletenessNavigator {
    pub fn add_axiom_system(name: &str) {
        let mut s = STATE.lock().unwrap();
        s.axiom_systems.push(name.to_string());
    }

    pub fn switch_system(name: &str) -> bool {
        let mut s = STATE.lock().unwrap();
        if s.axiom_systems.iter().any(|a| a == name) {
            s.switches += 1;
            true
        } else {
            false
        }
    }

    pub fn detect_unprovable() -> u64 {
        let mut s = STATE.lock().unwrap();
        s.unprovable_detected += 1;
        s.unprovable_detected
    }

    pub fn system_count() -> usize { STATE.lock().unwrap().axiom_systems.len() }

    pub fn switches() -> u64 { STATE.lock().unwrap().switches }

    pub fn reset() { *STATE.lock().unwrap() = IncNavState::default(); }
}

#[unsafe(no_mangle)]
pub extern "C" fn incn_systems() -> i64 { IncompletenessNavigator::system_count() as i64 }
#[unsafe(no_mangle)]
pub extern "C" fn incn_switches() -> i64 { IncompletenessNavigator::switches() as i64 }
#[unsafe(no_mangle)]
pub extern "C" fn incn_detect() -> i64 { IncompletenessNavigator::detect_unprovable() as i64 }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_add_system() { IncompletenessNavigator::reset(); IncompletenessNavigator::add_axiom_system("ZFC"); assert_eq!(IncompletenessNavigator::system_count(), 1); }
    #[test] fn test_switch() { IncompletenessNavigator::reset(); IncompletenessNavigator::add_axiom_system("PA"); assert!(IncompletenessNavigator::switch_system("PA")); }
    #[test] fn test_switch_unknown() { IncompletenessNavigator::reset(); assert!(!IncompletenessNavigator::switch_system("none")); }
    #[test] fn test_detect() { IncompletenessNavigator::reset(); let n = IncompletenessNavigator::detect_unprovable(); assert_eq!(n, 1); }
    #[test] fn test_switches_count() { IncompletenessNavigator::reset(); IncompletenessNavigator::add_axiom_system("A"); IncompletenessNavigator::switch_system("A"); assert_eq!(IncompletenessNavigator::switches(), 1); }
    #[test] fn test_reset() { IncompletenessNavigator::reset(); IncompletenessNavigator::add_axiom_system("X"); IncompletenessNavigator::reset(); assert_eq!(IncompletenessNavigator::system_count(), 0); }
    #[test] fn test_ffi() { IncompletenessNavigator::reset(); IncompletenessNavigator::add_axiom_system("ffi"); assert_eq!(incn_systems(), 1); }
}
