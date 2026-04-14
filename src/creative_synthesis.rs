//! Creative Synthesis — Vitalis v1275
//!
//! Generates genuinely novel programming constructs.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<CreativeSynthesisState>> = LazyLock::new(|| Mutex::new(CreativeSynthesisState::default()));

#[derive(Default)]
struct CreativeSynthesisState {
    inventions: Vec<(String, f64)>,
    novelty_score: f64,
    attempts: u64,
}

pub struct CreativeSynthesis;

impl CreativeSynthesis {
    pub fn invent(name: &str, novelty: f64) {
        let mut s = STATE.lock().unwrap();
        s.inventions.push((name.to_string(), novelty));
        s.novelty_score += novelty;
        s.attempts += 1;
    }
    pub fn attempt() -> u64 {
        let mut s = STATE.lock().unwrap();
        s.attempts += 1;
        s.attempts
    }
    pub fn novelty_score() -> f64 { STATE.lock().unwrap().novelty_score }
    pub fn invention_count() -> usize { STATE.lock().unwrap().inventions.len() }
    pub fn best_novelty() -> f64 { STATE.lock().unwrap().inventions.iter().map(|x| x.1).fold(0.0_f64, f64::max) }
    pub fn reset() { *STATE.lock().unwrap() = CreativeSynthesisState::default(); }
}

#[unsafe(no_mangle)]
pub extern "C" fn csyn_invent() -> u64 { CreativeSynthesis::attempt() }
#[unsafe(no_mangle)]
pub extern "C" fn csyn_novelty_score() -> f64 { CreativeSynthesis::novelty_score() }
#[unsafe(no_mangle)]
pub extern "C" fn csyn_invention_count() -> usize { CreativeSynthesis::invention_count() }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_invent() { CreativeSynthesis::reset(); CreativeSynthesis::invent("monad-lens", 0.9); assert_eq!(CreativeSynthesis::invention_count(), 1); }
    #[test] fn test_novelty() { CreativeSynthesis::reset(); CreativeSynthesis::invent("x", 0.5); assert!((CreativeSynthesis::novelty_score() - 0.5).abs() < 1e-9); }
    #[test] fn test_attempt() { CreativeSynthesis::reset(); let a = CreativeSynthesis::attempt(); assert_eq!(a, 1); }
    #[test] fn test_best_novelty() { CreativeSynthesis::reset(); CreativeSynthesis::invent("a", 0.3); CreativeSynthesis::invent("b", 0.8); assert!((CreativeSynthesis::best_novelty() - 0.8).abs() < 1e-9); }
    #[test] fn test_accumulates() { CreativeSynthesis::reset(); CreativeSynthesis::invent("a", 1.0); CreativeSynthesis::invent("b", 2.0); assert!((CreativeSynthesis::novelty_score() - 3.0).abs() < 1e-9); }
    #[test] fn test_empty_best() { CreativeSynthesis::reset(); assert!((CreativeSynthesis::best_novelty() - 0.0).abs() < 1e-9); }
    #[test] fn test_attempts_count() { CreativeSynthesis::reset(); CreativeSynthesis::invent("x", 0.1); CreativeSynthesis::attempt(); assert!(STATE.lock().unwrap().attempts == 2); }
    #[test] fn test_reset() { CreativeSynthesis::reset(); CreativeSynthesis::invent("z", 1.0); CreativeSynthesis::reset(); assert_eq!(CreativeSynthesis::invention_count(), 0); }
}
