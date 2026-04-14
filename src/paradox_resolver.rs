//! Paradox Resolver — Vitalis v1219
//!
//! Detects and resolves paradoxes in self-referential code.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<ParadoxState>> = LazyLock::new(|| Mutex::new(ParadoxState::default()));

#[derive(Default)]
struct ParadoxState {
    paradoxes: Vec<String>,
    resolutions: Vec<String>,
    unresolved: u32,
}

pub struct ParadoxResolver;

impl ParadoxResolver {
    pub fn detect_paradox(name: &str) {
        let mut s = STATE.lock().unwrap();
        s.paradoxes.push(name.to_string());
        s.unresolved += 1;
    }

    pub fn resolve(name: &str, resolution: &str) -> bool {
        let mut s = STATE.lock().unwrap();
        if s.paradoxes.iter().any(|p| p == name) {
            s.resolutions.push(format!("{}: {}", name, resolution));
            if s.unresolved > 0 { s.unresolved -= 1; }
            true
        } else {
            false
        }
    }

    pub fn paradox_count() -> usize { STATE.lock().unwrap().paradoxes.len() }

    pub fn resolution_count() -> usize { STATE.lock().unwrap().resolutions.len() }

    pub fn unresolved() -> u32 { STATE.lock().unwrap().unresolved }

    pub fn reset() { *STATE.lock().unwrap() = ParadoxState::default(); }
}

#[unsafe(no_mangle)]
pub extern "C" fn para_detect() -> i64 { ParadoxResolver::detect_paradox("ffi_paradox"); ParadoxResolver::paradox_count() as i64 }
#[unsafe(no_mangle)]
pub extern "C" fn para_resolve() -> i64 { if ParadoxResolver::resolve("ffi_paradox", "dissolve") { 1 } else { 0 } }
#[unsafe(no_mangle)]
pub extern "C" fn para_unresolved() -> i64 { ParadoxResolver::unresolved() as i64 }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_detect() { ParadoxResolver::reset(); ParadoxResolver::detect_paradox("liar"); assert_eq!(ParadoxResolver::paradox_count(), 1); }
    #[test] fn test_resolve() { ParadoxResolver::reset(); ParadoxResolver::detect_paradox("liar"); assert!(ParadoxResolver::resolve("liar", "levels")); }
    #[test] fn test_resolve_unknown() { ParadoxResolver::reset(); assert!(!ParadoxResolver::resolve("none", "x")); }
    #[test] fn test_unresolved() { ParadoxResolver::reset(); ParadoxResolver::detect_paradox("a"); assert_eq!(ParadoxResolver::unresolved(), 1); }
    #[test] fn test_unresolved_after_resolve() { ParadoxResolver::reset(); ParadoxResolver::detect_paradox("a"); ParadoxResolver::resolve("a", "fix"); assert_eq!(ParadoxResolver::unresolved(), 0); }
    #[test] fn test_resolution_count() { ParadoxResolver::reset(); ParadoxResolver::detect_paradox("x"); ParadoxResolver::resolve("x", "r"); assert_eq!(ParadoxResolver::resolution_count(), 1); }
    #[test] fn test_reset() { ParadoxResolver::reset(); ParadoxResolver::detect_paradox("p"); ParadoxResolver::reset(); assert_eq!(ParadoxResolver::paradox_count(), 0); }
    #[test] fn test_ffi() { ParadoxResolver::reset(); para_detect(); assert_eq!(para_unresolved(), 1); }
}
