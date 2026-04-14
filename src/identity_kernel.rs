//! Identity Kernel — Vitalis v1293
//!
//! Irreducible core identity of Vitalis across all versions.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<IdentityKernelState>> = LazyLock::new(|| Mutex::new(IdentityKernelState::default()));

#[derive(Default)]
struct IdentityKernelState {
    invariants: Vec<(String, bool)>,
    version_count: u32,
    essence_hash: u64,
}

pub struct IdentityKernel;

impl IdentityKernel {
    pub fn add_invariant(name: &str, holds: bool) {
        let mut s = STATE.lock().unwrap();
        s.invariants.push((name.to_string(), holds));
        s.essence_hash = s.essence_hash.wrapping_mul(31).wrapping_add(name.len() as u64);
    }
    pub fn register_version() -> u32 {
        let mut s = STATE.lock().unwrap();
        s.version_count += 1;
        s.version_count
    }
    pub fn invariant_count() -> usize { STATE.lock().unwrap().invariants.len() }
    pub fn all_hold() -> bool { STATE.lock().unwrap().invariants.iter().all(|i| i.1) }
    pub fn essence_hash() -> u64 { STATE.lock().unwrap().essence_hash }
    pub fn reset() { *STATE.lock().unwrap() = IdentityKernelState::default(); }
}

#[unsafe(no_mangle)]
pub extern "C" fn idk_register_version() -> u32 { IdentityKernel::register_version() }
#[unsafe(no_mangle)]
pub extern "C" fn idk_invariant_count() -> usize { IdentityKernel::invariant_count() }
#[unsafe(no_mangle)]
pub extern "C" fn idk_all_hold() -> bool { IdentityKernel::all_hold() }
#[unsafe(no_mangle)]
pub extern "C" fn idk_essence_hash() -> u64 { IdentityKernel::essence_hash() }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_add_invariant() { IdentityKernel::reset(); IdentityKernel::add_invariant("type_safety", true); assert_eq!(IdentityKernel::invariant_count(), 1); }
    #[test] fn test_all_hold_true() { IdentityKernel::reset(); IdentityKernel::add_invariant("a", true); IdentityKernel::add_invariant("b", true); assert!(IdentityKernel::all_hold()); }
    #[test] fn test_all_hold_false() { IdentityKernel::reset(); IdentityKernel::add_invariant("a", true); IdentityKernel::add_invariant("b", false); assert!(!IdentityKernel::all_hold()); }
    #[test] fn test_register_version() { IdentityKernel::reset(); let v = IdentityKernel::register_version(); assert_eq!(v, 1); }
    #[test] fn test_essence_hash_changes() { IdentityKernel::reset(); let h1 = IdentityKernel::essence_hash(); IdentityKernel::add_invariant("x", true); assert_ne!(IdentityKernel::essence_hash(), h1); }
    #[test] fn test_multiple_versions() { IdentityKernel::reset(); IdentityKernel::register_version(); let v = IdentityKernel::register_version(); assert_eq!(v, 2); }
    #[test] fn test_empty_all_hold() { IdentityKernel::reset(); assert!(IdentityKernel::all_hold()); }
    #[test] fn test_reset() { IdentityKernel::reset(); IdentityKernel::add_invariant("z", true); IdentityKernel::reset(); assert_eq!(IdentityKernel::invariant_count(), 0); }
}
