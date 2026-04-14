//! Godel Numbering — Vitalis v1209
//!
//! Assigns Godel numbers to compiler constructs for self-reference.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<GodelState>> = LazyLock::new(|| Mutex::new(GodelState::default()));

#[derive(Default)]
struct GodelState {
    assignments: Vec<(String, u64)>,
    next_number: u64,
}

pub struct GodelNumbering;

impl GodelNumbering {
    pub fn assign(name: &str) -> u64 {
        let mut s = STATE.lock().unwrap();
        s.next_number += 1;
        let num = s.next_number;
        s.assignments.push((name.to_string(), num));
        num
    }

    pub fn lookup(name: &str) -> Option<u64> {
        let s = STATE.lock().unwrap();
        s.assignments.iter().find(|(n, _)| n == name).map(|(_, v)| *v)
    }

    pub fn decode(number: u64) -> Option<String> {
        let s = STATE.lock().unwrap();
        s.assignments.iter().find(|(_, v)| *v == number).map(|(n, _)| n.clone())
    }

    pub fn count() -> usize { STATE.lock().unwrap().assignments.len() }

    pub fn next_number() -> u64 { STATE.lock().unwrap().next_number }

    pub fn reset() { *STATE.lock().unwrap() = GodelState::default(); }
}

#[unsafe(no_mangle)]
pub extern "C" fn gn_assign() -> i64 { GodelNumbering::assign("ffi_construct") as i64 }
#[unsafe(no_mangle)]
pub extern "C" fn gn_count() -> i64 { GodelNumbering::count() as i64 }
#[unsafe(no_mangle)]
pub extern "C" fn gn_next() -> i64 { GodelNumbering::next_number() as i64 }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_assign() { GodelNumbering::reset(); let n = GodelNumbering::assign("fn_main"); assert_eq!(n, 1); }
    #[test] fn test_assign_increments() { GodelNumbering::reset(); GodelNumbering::assign("a"); let n = GodelNumbering::assign("b"); assert_eq!(n, 2); }
    #[test] fn test_lookup() { GodelNumbering::reset(); GodelNumbering::assign("x"); assert_eq!(GodelNumbering::lookup("x"), Some(1)); }
    #[test] fn test_lookup_missing() { GodelNumbering::reset(); assert_eq!(GodelNumbering::lookup("z"), None); }
    #[test] fn test_decode() { GodelNumbering::reset(); GodelNumbering::assign("hello"); assert_eq!(GodelNumbering::decode(1), Some("hello".to_string())); }
    #[test] fn test_count() { GodelNumbering::reset(); GodelNumbering::assign("a"); GodelNumbering::assign("b"); assert_eq!(GodelNumbering::count(), 2); }
    #[test] fn test_reset() { GodelNumbering::reset(); GodelNumbering::assign("a"); GodelNumbering::reset(); assert_eq!(GodelNumbering::count(), 0); }
    #[test] fn test_ffi() { GodelNumbering::reset(); let n = gn_assign(); assert_eq!(n, 1); assert_eq!(gn_count(), 1); }
}
