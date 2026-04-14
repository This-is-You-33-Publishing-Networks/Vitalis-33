//! Limit Breaker — Vitalis v1207
//!
//! Finds creative workarounds to theoretical limits.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<LimitBreakerState>> = LazyLock::new(|| Mutex::new(LimitBreakerState::default()));

#[derive(Default)]
struct LimitBreakerState {
    limits: Vec<String>,
    broken: Vec<String>,
    workarounds: u64,
}

pub struct LimitBreaker;

impl LimitBreaker {
    pub fn register_limit(name: &str) {
        let mut s = STATE.lock().unwrap();
        s.limits.push(name.to_string());
    }

    pub fn break_limit(name: &str) -> bool {
        let mut s = STATE.lock().unwrap();
        if s.limits.iter().any(|l| l == name) {
            s.broken.push(name.to_string());
            s.workarounds += 1;
            true
        } else {
            false
        }
    }

    pub fn limits_count() -> usize { STATE.lock().unwrap().limits.len() }

    pub fn broken_count() -> usize { STATE.lock().unwrap().broken.len() }

    pub fn workarounds() -> u64 { STATE.lock().unwrap().workarounds }

    pub fn reset() { *STATE.lock().unwrap() = LimitBreakerState::default(); }
}

#[unsafe(no_mangle)]
pub extern "C" fn lb_register() -> i64 { LimitBreaker::register_limit("ffi_limit"); LimitBreaker::limits_count() as i64 }
#[unsafe(no_mangle)]
pub extern "C" fn lb_break() -> i64 { if LimitBreaker::break_limit("ffi_limit") { 1 } else { 0 } }
#[unsafe(no_mangle)]
pub extern "C" fn lb_broken() -> i64 { LimitBreaker::broken_count() as i64 }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_register() { LimitBreaker::reset(); LimitBreaker::register_limit("speed"); assert_eq!(LimitBreaker::limits_count(), 1); }
    #[test] fn test_break() { LimitBreaker::reset(); LimitBreaker::register_limit("mem"); assert!(LimitBreaker::break_limit("mem")); }
    #[test] fn test_break_unknown() { LimitBreaker::reset(); assert!(!LimitBreaker::break_limit("none")); }
    #[test] fn test_workarounds() { LimitBreaker::reset(); LimitBreaker::register_limit("x"); LimitBreaker::break_limit("x"); assert_eq!(LimitBreaker::workarounds(), 1); }
    #[test] fn test_reset() { LimitBreaker::reset(); LimitBreaker::register_limit("a"); LimitBreaker::reset(); assert_eq!(LimitBreaker::limits_count(), 0); }
    #[test] fn test_ffi() { LimitBreaker::reset(); lb_register(); assert_eq!(lb_break(), 1); }
}
