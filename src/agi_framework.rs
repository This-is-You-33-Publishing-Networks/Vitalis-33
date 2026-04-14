//! AGI Framework — Vitalis v961
//!
//! Framework for building Artificial General Intelligence systems:
//! perception, reasoning, planning, and action loops.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<AgiState>> = LazyLock::new(|| Mutex::new(AgiState::default()));

#[derive(Default)]
struct AgiState {
    percepts: Vec<Vec<f64>>,
    decisions: Vec<String>,
    actions_taken: u64,
    learning_cycles: u64,
}

pub struct AgiFramework;

impl AgiFramework {
    pub fn perceive(observation: &str) -> Vec<f64> {
        let mut s = STATE.lock().unwrap();
        let embedding: Vec<f64> = observation.bytes().map(|b| b as f64 / 255.0).take(16).collect();
        s.percepts.push(embedding.clone());
        s.learning_cycles += 1;
        embedding
    }

    pub fn reason(percepts: &[f64]) -> String {
        let mut s = STATE.lock().unwrap();
        s.learning_cycles += 1;
        let sum: f64 = percepts.iter().sum();
        let decision = if sum > 5.0 { "act_high" } else if sum > 1.0 { "act_medium" } else { "act_low" };
        s.decisions.push(decision.to_string());
        decision.to_string()
    }

    pub fn act(decision: &str) -> bool {
        let mut s = STATE.lock().unwrap();
        s.actions_taken += 1;
        s.learning_cycles += 1;
        !decision.is_empty()
    }

    pub fn learning_cycles() -> u64 {
        STATE.lock().unwrap().learning_cycles
    }

    pub fn reset() {
        let mut s = STATE.lock().unwrap();
        *s = AgiState::default();
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn agi_perceive(obs_len: i64) -> i64 { let mut s = STATE.lock().unwrap(); s.percepts.push(vec![0.5; obs_len.max(0) as usize]); s.learning_cycles += 1; obs_len.max(0) }
#[unsafe(no_mangle)]
pub extern "C" fn agi_reason(sum: f64) -> i64 { let mut s = STATE.lock().unwrap(); s.learning_cycles += 1; if sum > 5.0 { 2 } else if sum > 1.0 { 1 } else { 0 } }
#[unsafe(no_mangle)]
pub extern "C" fn agi_act(decision: i64) -> i64 { let mut s = STATE.lock().unwrap(); s.actions_taken += 1; s.learning_cycles += 1; if decision > 0 { 1 } else { 0 } }
#[unsafe(no_mangle)]
pub extern "C" fn agi_cycles() -> i64 { STATE.lock().unwrap().learning_cycles as i64 }

#[cfg(test)]
mod tests {
    use super::*;

    #[test] fn test_perceive() { AgiFramework::reset(); let e = AgiFramework::perceive("hello"); assert!(!e.is_empty()); }
    #[test] fn test_reason() { AgiFramework::reset(); let d = AgiFramework::reason(&[3.0, 3.0]); assert_eq!(d, "act_high"); }
    #[test] fn test_act() { AgiFramework::reset(); assert!(AgiFramework::act("go")); }
    #[test] fn test_learning_cycles() { AgiFramework::reset(); AgiFramework::perceive("x"); AgiFramework::reason(&[1.0]); assert!(AgiFramework::learning_cycles() >= 2); }
    #[test] fn test_ffi_cycles() { AgiFramework::reset(); assert_eq!(agi_cycles(), 0); }
}
