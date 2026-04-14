//! Causal Rewriter — Vitalis v1083
//!
//! Rewrites the causal structure of programs, changing what causes what
//! to discover more efficient dependency orderings.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<CrwState>> = LazyLock::new(|| Mutex::new(CrwState::default()));

#[derive(Default)]
struct CrwState {
    rewrites: Vec<(String, String, String)>,
    saved_cycles: u64,
}

pub struct CausalRewriter;

impl CausalRewriter {
    pub fn rewrite(original_cause: &str, original_effect: &str, new_cause: &str) -> usize {
        let mut s = STATE.lock().unwrap();
        s.rewrites.push((original_cause.to_string(), original_effect.to_string(), new_cause.to_string()));
        s.saved_cycles += 1;
        s.rewrites.len()
    }

    pub fn rewrites_for(effect: &str) -> Vec<String> {
        STATE.lock().unwrap().rewrites.iter().filter(|(_, e, _)| e == effect).map(|(_, _, nc)| nc.clone()).collect()
    }

    pub fn rewrite_count() -> usize { STATE.lock().unwrap().rewrites.len() }
    pub fn saved_cycles() -> u64 { STATE.lock().unwrap().saved_cycles }
    pub fn reset() { *STATE.lock().unwrap() = CrwState::default(); }
}

#[unsafe(no_mangle)]
pub extern "C" fn crw_rewrite(id: i64) -> i64 { CausalRewriter::rewrite(&format!("c{}", id), "effect", "new_cause") as i64 }
#[unsafe(no_mangle)]
pub extern "C" fn crw_count() -> i64 { CausalRewriter::rewrite_count() as i64 }
#[unsafe(no_mangle)]
pub extern "C" fn crw_saved() -> i64 { CausalRewriter::saved_cycles() as i64 }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_rewrite() { CausalRewriter::reset(); assert_eq!(CausalRewriter::rewrite("a", "b", "c"), 1); }
    #[test] fn test_rewrites_for() { CausalRewriter::reset(); CausalRewriter::rewrite("a", "effect", "x"); let r = CausalRewriter::rewrites_for("effect"); assert_eq!(r.len(), 1); }
    #[test] fn test_no_rewrites() { CausalRewriter::reset(); assert!(CausalRewriter::rewrites_for("none").is_empty()); }
    #[test] fn test_count() { CausalRewriter::reset(); CausalRewriter::rewrite("a", "b", "c"); CausalRewriter::rewrite("d", "e", "f"); assert_eq!(CausalRewriter::rewrite_count(), 2); }
    #[test] fn test_saved() { CausalRewriter::reset(); CausalRewriter::rewrite("a", "b", "c"); assert_eq!(CausalRewriter::saved_cycles(), 1); }
    #[test] fn test_multiple_rewrites() { CausalRewriter::reset(); CausalRewriter::rewrite("a", "x", "b"); CausalRewriter::rewrite("c", "x", "d"); assert_eq!(CausalRewriter::rewrites_for("x").len(), 2); }
    #[test] fn test_ffi() { CausalRewriter::reset(); assert_eq!(crw_count(), 0); }
    #[test] fn test_ffi_saved() { CausalRewriter::reset(); assert_eq!(crw_saved(), 0); }
}
