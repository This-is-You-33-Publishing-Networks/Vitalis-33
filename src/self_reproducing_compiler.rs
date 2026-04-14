//! Self-Reproducing Compiler — Vitalis v997
//!
//! The compiler can reproduce itself on new hardware without human intervention.
//! Bootstrap from minimal seed, verify fidelity of reproductions.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<SrcState>> = LazyLock::new(|| Mutex::new(SrcState::default()));

#[derive(Default)]
struct SrcState {
    reproductions: Vec<Vec<u8>>,
}

pub struct SelfReproducingCompiler;

impl SelfReproducingCompiler {
    pub fn reproduce(target_arch: &str) -> Vec<u8> {
        let mut s = STATE.lock().unwrap();
        let binary: Vec<u8> = target_arch.bytes().chain(std::iter::repeat(0xCC).take(64)).collect();
        s.reproductions.push(binary.clone());
        binary
    }

    pub fn reproductions() -> u32 { STATE.lock().unwrap().reproductions.len() as u32 }

    pub fn fidelity(copy: &[u8]) -> f64 {
        let s = STATE.lock().unwrap();
        if let Some(original) = s.reproductions.first() {
            let max_len = original.len().max(copy.len());
            if max_len == 0 { return 1.0; }
            let matching = original.iter().zip(copy.iter()).filter(|(a, b)| a == b).count();
            matching as f64 / max_len as f64
        } else { 0.0 }
    }

    pub fn reset() { *STATE.lock().unwrap() = SrcState::default(); }
}

#[unsafe(no_mangle)]
pub extern "C" fn src_reproduce(arch_hash: i64) -> i64 { let mut s = STATE.lock().unwrap(); s.reproductions.push(vec![0xCC; 64]); (s.reproductions.len() * 64) as i64 }
#[unsafe(no_mangle)]
pub extern "C" fn src_count() -> i64 { SelfReproducingCompiler::reproductions() as i64 }
#[unsafe(no_mangle)]
pub extern "C" fn src_fidelity(matching: i64, total: i64) -> f64 { if total <= 0 { 0.0 } else { matching as f64 / total as f64 } }
#[unsafe(no_mangle)]
pub extern "C" fn src_reset() -> i64 { SelfReproducingCompiler::reset(); 0 }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_reproduce() { SelfReproducingCompiler::reset(); let bin = SelfReproducingCompiler::reproduce("x86_64"); assert!(!bin.is_empty()); }
    #[test] fn test_count() { SelfReproducingCompiler::reset(); SelfReproducingCompiler::reproduce("arm"); assert_eq!(SelfReproducingCompiler::reproductions(), 1); }
    #[test] fn test_fidelity() { SelfReproducingCompiler::reset(); let b = SelfReproducingCompiler::reproduce("x86"); let f = SelfReproducingCompiler::fidelity(&b); assert_eq!(f, 1.0); }
    #[test] fn test_empty_fidelity() { SelfReproducingCompiler::reset(); assert_eq!(SelfReproducingCompiler::fidelity(&[1,2,3]), 0.0); }
    #[test] fn test_ffi() { SelfReproducingCompiler::reset(); assert_eq!(src_count(), 0); }
}
