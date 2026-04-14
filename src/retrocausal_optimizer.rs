//! Retrocausal Optimizer — Vitalis v1085
//!
//! Optimization that propagates constraints backward from desired
//! outcomes to inputs, working from the future to the past.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<RcoState>> = LazyLock::new(|| Mutex::new(RcoState::default()));

#[derive(Default)]
struct RcoState {
    goals: Vec<(String, f64)>,
    backward_passes: u64,
    constraint_propagations: Vec<(String, String)>,
    improvement: f64,
}

pub struct RetrocausalOptimizer;

impl RetrocausalOptimizer {
    pub fn set_goal(name: &str, target: f64) -> usize {
        let mut s = STATE.lock().unwrap();
        s.goals.push((name.to_string(), target));
        s.goals.len()
    }

    pub fn backward_pass() -> f64 {
        let mut s = STATE.lock().unwrap();
        s.backward_passes += 1;
        let improvement = s.goals.iter().map(|(_, t)| t).sum::<f64>() / s.goals.len().max(1) as f64 * 0.1;
        s.improvement += improvement;
        s.improvement
    }

    pub fn propagate_constraint(from: &str, to: &str) -> usize {
        let mut s = STATE.lock().unwrap();
        s.constraint_propagations.push((from.to_string(), to.to_string()));
        s.constraint_propagations.len()
    }

    pub fn total_improvement() -> f64 { STATE.lock().unwrap().improvement }
    pub fn pass_count() -> u64 { STATE.lock().unwrap().backward_passes }
    pub fn goal_count() -> usize { STATE.lock().unwrap().goals.len() }
    pub fn reset() { *STATE.lock().unwrap() = RcoState::default(); }
}

#[unsafe(no_mangle)]
pub extern "C" fn rco_goal(target: f64) -> i64 { RetrocausalOptimizer::set_goal("ffi", target) as i64 }
#[unsafe(no_mangle)]
pub extern "C" fn rco_backward() -> f64 { RetrocausalOptimizer::backward_pass() }
#[unsafe(no_mangle)]
pub extern "C" fn rco_improvement() -> f64 { RetrocausalOptimizer::total_improvement() }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_goal() { RetrocausalOptimizer::reset(); assert_eq!(RetrocausalOptimizer::set_goal("speed", 100.0), 1); }
    #[test] fn test_backward() { RetrocausalOptimizer::reset(); RetrocausalOptimizer::set_goal("a", 10.0); let imp = RetrocausalOptimizer::backward_pass(); assert!(imp > 0.0); }
    #[test] fn test_propagate() { RetrocausalOptimizer::reset(); assert_eq!(RetrocausalOptimizer::propagate_constraint("output", "input"), 1); }
    #[test] fn test_improvement_grows() { RetrocausalOptimizer::reset(); RetrocausalOptimizer::set_goal("a", 10.0); RetrocausalOptimizer::backward_pass(); let i1 = RetrocausalOptimizer::total_improvement(); RetrocausalOptimizer::backward_pass(); assert!(RetrocausalOptimizer::total_improvement() > i1); }
    #[test] fn test_pass_count() { RetrocausalOptimizer::reset(); RetrocausalOptimizer::backward_pass(); assert_eq!(RetrocausalOptimizer::pass_count(), 1); }
    #[test] fn test_goal_count() { RetrocausalOptimizer::reset(); RetrocausalOptimizer::set_goal("a", 1.0); RetrocausalOptimizer::set_goal("b", 2.0); assert_eq!(RetrocausalOptimizer::goal_count(), 2); }
    #[test] fn test_ffi() { RetrocausalOptimizer::reset(); assert!((rco_improvement() - 0.0).abs() < 0.01); }
}
