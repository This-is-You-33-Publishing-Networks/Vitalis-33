//! Homotopy Types — Vitalis v1154
//!
//! Homotopy type theory: types as spaces, proofs as paths, equivalences as homotopies.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<HttState>> = LazyLock::new(|| Mutex::new(HttState::default()));

#[derive(Default)]
struct HttState { spaces: Vec<String>, paths: Vec<(String, String, String)>, homotopies: u64 }

pub struct HomotopyTypes;

impl HomotopyTypes {
    pub fn add_space(name: &str) -> usize { let mut s = STATE.lock().unwrap(); s.spaces.push(name.to_string()); s.spaces.len() }
    pub fn add_path(name: &str, from: &str, to: &str) -> usize { let mut s = STATE.lock().unwrap(); s.paths.push((name.to_string(), from.to_string(), to.to_string())); s.paths.len() }
    pub fn homotopy(path_a: &str, path_b: &str) -> bool {
        let mut s = STATE.lock().unwrap();
        let a = s.paths.iter().find(|(n, _, _)| n == path_a).map(|(_, f, t)| (f.clone(), t.clone()));
        let b = s.paths.iter().find(|(n, _, _)| n == path_b).map(|(_, f, t)| (f.clone(), t.clone()));
        if let (Some((af, at)), Some((bf, bt))) = (a, b) { if af == bf && at == bt { s.homotopies += 1; return true; } }
        false
    }
    pub fn space_count() -> usize { STATE.lock().unwrap().spaces.len() }
    pub fn path_count() -> usize { STATE.lock().unwrap().paths.len() }
    pub fn homotopy_count() -> u64 { STATE.lock().unwrap().homotopies }
    pub fn reset() { *STATE.lock().unwrap() = HttState::default(); }
}

#[unsafe(no_mangle)]
pub extern "C" fn htt_space(id: i64) -> i64 { HomotopyTypes::add_space(&format!("S{}", id)) as i64 }
#[unsafe(no_mangle)]
pub extern "C" fn htt_path() -> i64 { HomotopyTypes::add_path("p", "a", "b") as i64 }
#[unsafe(no_mangle)]
pub extern "C" fn htt_spaces() -> i64 { HomotopyTypes::space_count() as i64 }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_space() { HomotopyTypes::reset(); assert_eq!(HomotopyTypes::add_space("S1"), 1); }
    #[test] fn test_path() { HomotopyTypes::reset(); assert_eq!(HomotopyTypes::add_path("p", "a", "b"), 1); }
    #[test] fn test_homotopy() { HomotopyTypes::reset(); HomotopyTypes::add_path("p1", "a", "b"); HomotopyTypes::add_path("p2", "a", "b"); assert!(HomotopyTypes::homotopy("p1", "p2")); }
    #[test] fn test_no_homotopy() { HomotopyTypes::reset(); HomotopyTypes::add_path("p1", "a", "b"); HomotopyTypes::add_path("p2", "c", "d"); assert!(!HomotopyTypes::homotopy("p1", "p2")); }
    #[test] fn test_homotopy_count() { HomotopyTypes::reset(); HomotopyTypes::add_path("p1", "a", "b"); HomotopyTypes::add_path("p2", "a", "b"); HomotopyTypes::homotopy("p1", "p2"); assert_eq!(HomotopyTypes::homotopy_count(), 1); }
    #[test] fn test_counts() { HomotopyTypes::reset(); HomotopyTypes::add_space("S"); HomotopyTypes::add_path("p", "a", "b"); assert_eq!(HomotopyTypes::space_count(), 1); assert_eq!(HomotopyTypes::path_count(), 1); }
    #[test] fn test_ffi() { HomotopyTypes::reset(); assert_eq!(htt_spaces(), 0); }
}
