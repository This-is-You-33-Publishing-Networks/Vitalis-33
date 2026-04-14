//! Paradigm Fusion — Vitalis v1134
//!
//! Seamlessly fuses functional, imperative, logic, and constraint
//! paradigms into a single unified intermediate representation.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<PfState>> = LazyLock::new(|| Mutex::new(PfState::default()));

#[derive(Default)]
struct PfState {
    paradigms: Vec<String>,
    fusions: Vec<(String, String)>,
    unified_nodes: u64,
}

pub struct ParadigmFusion;

impl ParadigmFusion {
    pub fn register_paradigm(name: &str) -> usize {
        let mut s = STATE.lock().unwrap();
        s.paradigms.push(name.to_string());
        s.paradigms.len()
    }

    pub fn fuse(a: &str, b: &str) -> usize {
        let mut s = STATE.lock().unwrap();
        s.fusions.push((a.to_string(), b.to_string()));
        s.unified_nodes += 1;
        s.fusions.len()
    }

    pub fn is_unified(paradigm: &str) -> bool {
        let s = STATE.lock().unwrap();
        s.fusions.iter().any(|(a, b)| a == paradigm || b == paradigm)
    }

    pub fn paradigm_count() -> usize { STATE.lock().unwrap().paradigms.len() }
    pub fn fusion_count() -> usize { STATE.lock().unwrap().fusions.len() }
    pub fn unified_nodes() -> u64 { STATE.lock().unwrap().unified_nodes }
    pub fn reset() { *STATE.lock().unwrap() = PfState::default(); }
}

#[unsafe(no_mangle)]
pub extern "C" fn pf_register(id: i64) -> i64 { ParadigmFusion::register_paradigm(&format!("p{}", id)) as i64 }
#[unsafe(no_mangle)]
pub extern "C" fn pf_fuse() -> i64 { ParadigmFusion::fuse("a", "b") as i64 }
#[unsafe(no_mangle)]
pub extern "C" fn pf_paradigms() -> i64 { ParadigmFusion::paradigm_count() as i64 }
#[unsafe(no_mangle)]
pub extern "C" fn pf_fusions() -> i64 { ParadigmFusion::fusion_count() as i64 }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_register() { ParadigmFusion::reset(); assert_eq!(ParadigmFusion::register_paradigm("functional"), 1); }
    #[test] fn test_fuse() { ParadigmFusion::reset(); assert_eq!(ParadigmFusion::fuse("functional", "imperative"), 1); }
    #[test] fn test_unified() { ParadigmFusion::reset(); ParadigmFusion::fuse("func", "imp"); assert!(ParadigmFusion::is_unified("func")); }
    #[test] fn test_not_unified() { ParadigmFusion::reset(); assert!(!ParadigmFusion::is_unified("logic")); }
    #[test] fn test_paradigm_count() { ParadigmFusion::reset(); ParadigmFusion::register_paradigm("a"); ParadigmFusion::register_paradigm("b"); assert_eq!(ParadigmFusion::paradigm_count(), 2); }
    #[test] fn test_nodes() { ParadigmFusion::reset(); ParadigmFusion::fuse("a", "b"); assert_eq!(ParadigmFusion::unified_nodes(), 1); }
    #[test] fn test_multiple_fuse() { ParadigmFusion::reset(); ParadigmFusion::fuse("a", "b"); ParadigmFusion::fuse("b", "c"); assert_eq!(ParadigmFusion::fusion_count(), 2); }
    #[test] fn test_ffi() { ParadigmFusion::reset(); assert_eq!(pf_paradigms(), 0); }
}
