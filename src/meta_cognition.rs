//! Meta-Cognition — Vitalis v985
//!
//! The compiler reasons about its own reasoning: resource allocation,
//! strategy adjustment, and self-reflective optimization monitoring.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<MetaCogState>> = LazyLock::new(|| Mutex::new(MetaCogState::default()));

#[derive(Default)]
struct MetaCogState {
    observations: Vec<String>,
    strategies: Vec<String>,
    reflections: u64,
    last_evaluation: f64,
}

pub struct MetaCognition;

impl MetaCognition {
    pub fn observe_reasoning(step: &str) {
        let mut s = STATE.lock().unwrap();
        s.observations.push(step.to_string());
        s.reflections += 1;
    }

    pub fn evaluate_reasoning() -> f64 {
        let mut s = STATE.lock().unwrap();
        s.reflections += 1;
        let score = if s.observations.is_empty() { 0.0 } else { 1.0 - (1.0 / (s.observations.len() as f64 + 1.0)) };
        s.last_evaluation = score;
        score
    }

    pub fn adjust_strategy(strat: &str) {
        let mut s = STATE.lock().unwrap();
        s.strategies.push(strat.to_string());
        s.reflections += 1;
    }

    pub fn reflections() -> u64 { STATE.lock().unwrap().reflections }
    pub fn reset() { *STATE.lock().unwrap() = MetaCogState::default(); }
}

#[unsafe(no_mangle)]
pub extern "C" fn mc_observe(step_len: i64) -> i64 { let mut s = STATE.lock().unwrap(); s.observations.push(format!("step_{}", step_len)); s.reflections += 1; s.reflections as i64 }
#[unsafe(no_mangle)]
pub extern "C" fn mc_evaluate() -> f64 { MetaCognition::evaluate_reasoning() }
#[unsafe(no_mangle)]
pub extern "C" fn mc_adjust(strat_id: i64) -> i64 { let mut s = STATE.lock().unwrap(); s.strategies.push(format!("strat_{}", strat_id)); s.reflections += 1; 1 }
#[unsafe(no_mangle)]
pub extern "C" fn mc_reflections() -> i64 { MetaCognition::reflections() as i64 }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_observe() { MetaCognition::reset(); MetaCognition::observe_reasoning("check"); assert_eq!(MetaCognition::reflections(), 1); }
    #[test] fn test_evaluate() { MetaCognition::reset(); MetaCognition::observe_reasoning("a"); let e = MetaCognition::evaluate_reasoning(); assert!(e > 0.0 && e < 1.0); }
    #[test] fn test_adjust() { MetaCognition::reset(); MetaCognition::adjust_strategy("greedy"); assert_eq!(MetaCognition::reflections(), 1); }
    #[test] fn test_empty_eval() { MetaCognition::reset(); assert_eq!(MetaCognition::evaluate_reasoning(), 0.0); }
    #[test] fn test_ffi() { MetaCognition::reset(); assert_eq!(mc_reflections(), 0); }
}
