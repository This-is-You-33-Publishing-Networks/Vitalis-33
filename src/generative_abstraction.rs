//! Generative Abstraction — Vitalis v1283
//!
//! Generates new abstraction mechanisms from problem structure.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<GenerativeAbstractionState>> = LazyLock::new(|| Mutex::new(GenerativeAbstractionState::default()));

#[derive(Default)]
struct GenerativeAbstractionState {
    abstractions: Vec<(String, u32)>,
    generated: u64,
    reuse_count: u64,
}

pub struct GenerativeAbstraction;

impl GenerativeAbstraction {
    pub fn generate(name: &str, level: u32) {
        let mut s = STATE.lock().unwrap();
        s.abstractions.push((name.to_string(), level));
        s.generated += 1;
    }
    pub fn reuse(name: &str) -> u64 {
        let mut s = STATE.lock().unwrap();
        s.reuse_count += 1;
        for a in s.abstractions.iter_mut() { if a.0 == name { a.1 += 1; } }
        s.reuse_count
    }
    pub fn abstraction_count() -> usize { STATE.lock().unwrap().abstractions.len() }
    pub fn total_generated() -> u64 { STATE.lock().unwrap().generated }
    pub fn total_reuses() -> u64 { STATE.lock().unwrap().reuse_count }
    pub fn reset() { *STATE.lock().unwrap() = GenerativeAbstractionState::default(); }
}

#[unsafe(no_mangle)]
pub extern "C" fn ga_generate() -> u64 { GenerativeAbstraction::generate("auto", 1); GenerativeAbstraction::total_generated() }
#[unsafe(no_mangle)]
pub extern "C" fn ga_abstraction_count() -> usize { GenerativeAbstraction::abstraction_count() }
#[unsafe(no_mangle)]
pub extern "C" fn ga_total_reuses() -> u64 { GenerativeAbstraction::total_reuses() }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_generate() { GenerativeAbstraction::reset(); GenerativeAbstraction::generate("functor", 2); assert_eq!(GenerativeAbstraction::abstraction_count(), 1); }
    #[test] fn test_reuse() { GenerativeAbstraction::reset(); GenerativeAbstraction::generate("monad", 1); let r = GenerativeAbstraction::reuse("monad"); assert_eq!(r, 1); }
    #[test] fn test_total_generated() { GenerativeAbstraction::reset(); GenerativeAbstraction::generate("a", 1); GenerativeAbstraction::generate("b", 2); assert_eq!(GenerativeAbstraction::total_generated(), 2); }
    #[test] fn test_reuse_increments_level() { GenerativeAbstraction::reset(); GenerativeAbstraction::generate("x", 1); GenerativeAbstraction::reuse("x"); assert_eq!(STATE.lock().unwrap().abstractions[0].1, 2); }
    #[test] fn test_empty() { GenerativeAbstraction::reset(); assert_eq!(GenerativeAbstraction::abstraction_count(), 0); }
    #[test] fn test_total_reuses() { GenerativeAbstraction::reset(); GenerativeAbstraction::generate("y", 1); GenerativeAbstraction::reuse("y"); GenerativeAbstraction::reuse("y"); assert_eq!(GenerativeAbstraction::total_reuses(), 2); }
    #[test] fn test_reset() { GenerativeAbstraction::reset(); GenerativeAbstraction::generate("z", 1); GenerativeAbstraction::reset(); assert_eq!(GenerativeAbstraction::abstraction_count(), 0); }
}
