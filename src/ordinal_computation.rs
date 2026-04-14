//! Ordinal Computation — Vitalis v1227
//!
//! Computation indexed by transfinite ordinals.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<OrdState>> = LazyLock::new(|| Mutex::new(OrdState::default()));

#[derive(Default)]
struct OrdState {
    ordinals: Vec<(String, u64)>,
    current_ordinal: u64,
    omega_reached: bool,
}

pub struct OrdinalComputation;

impl OrdinalComputation {
    pub fn successor() -> u64 {
        let mut s = STATE.lock().unwrap();
        s.current_ordinal += 1;
        let ord = s.current_ordinal;
        s.ordinals.push((format!("ord_{}", ord), ord));
        ord
    }

    pub fn limit_ordinal(name: &str) -> u64 {
        let mut s = STATE.lock().unwrap();
        s.omega_reached = true;
        let ord = s.current_ordinal + 1000;
        s.current_ordinal = ord;
        s.ordinals.push((name.to_string(), ord));
        ord
    }

    pub fn current() -> u64 { STATE.lock().unwrap().current_ordinal }

    pub fn omega_reached() -> bool { STATE.lock().unwrap().omega_reached }

    pub fn ordinal_count() -> usize { STATE.lock().unwrap().ordinals.len() }

    pub fn reset() { *STATE.lock().unwrap() = OrdState::default(); }
}

#[unsafe(no_mangle)]
pub extern "C" fn ord_successor() -> i64 { OrdinalComputation::successor() as i64 }
#[unsafe(no_mangle)]
pub extern "C" fn ord_limit() -> i64 { OrdinalComputation::limit_ordinal("omega") as i64 }
#[unsafe(no_mangle)]
pub extern "C" fn ord_current() -> i64 { OrdinalComputation::current() as i64 }
#[unsafe(no_mangle)]
pub extern "C" fn ord_omega() -> i64 { if OrdinalComputation::omega_reached() { 1 } else { 0 } }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_successor() { OrdinalComputation::reset(); assert_eq!(OrdinalComputation::successor(), 1); }
    #[test] fn test_successive() { OrdinalComputation::reset(); OrdinalComputation::successor(); assert_eq!(OrdinalComputation::successor(), 2); }
    #[test] fn test_limit() { OrdinalComputation::reset(); let o = OrdinalComputation::limit_ordinal("omega"); assert!(o >= 1000); }
    #[test] fn test_omega() { OrdinalComputation::reset(); assert!(!OrdinalComputation::omega_reached()); OrdinalComputation::limit_ordinal("w"); assert!(OrdinalComputation::omega_reached()); }
    #[test] fn test_current() { OrdinalComputation::reset(); OrdinalComputation::successor(); assert_eq!(OrdinalComputation::current(), 1); }
    #[test] fn test_count() { OrdinalComputation::reset(); OrdinalComputation::successor(); OrdinalComputation::successor(); assert_eq!(OrdinalComputation::ordinal_count(), 2); }
    #[test] fn test_reset() { OrdinalComputation::reset(); OrdinalComputation::successor(); OrdinalComputation::reset(); assert_eq!(OrdinalComputation::current(), 0); }
    #[test] fn test_ffi() { OrdinalComputation::reset(); assert_eq!(ord_successor(), 1); assert_eq!(ord_current(), 1); }
}
