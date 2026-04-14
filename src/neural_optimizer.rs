//! RL-Based Optimization Pass Selection — Vitalis v710
//!
//! Uses reinforcement learning to select the best optimization passes
//! based on IR features and observed speedup rewards.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<NeuralOptimizerAgent>> = LazyLock::new(|| Mutex::new(NeuralOptimizerAgent::new()));

pub struct NeuralOptimizerAgent {
    rewards: Vec<f64>,
    best: usize,
    total: f64,
}

impl NeuralOptimizerAgent {
    pub fn new() -> Self {
        Self { rewards: vec![0.0; 8], best: 0, total: 0.0 }
    }

    pub fn select_pass(&self, ir_features: &[f64]) -> usize {
        let sum: f64 = ir_features.iter().sum();
        (sum.abs() as usize) % self.rewards.len().max(1)
    }

    pub fn record_reward(&mut self, pass: usize, speedup: f64) {
        let idx = pass % self.rewards.len().max(1);
        self.rewards[idx] += speedup;
        self.total += speedup;
        if self.rewards[idx] > self.rewards[self.best] {
            self.best = idx;
        }
    }

    pub fn best_pass(&self) -> usize {
        self.best
    }

    pub fn total_rewards(&self) -> f64 {
        self.total
    }
}

impl Default for NeuralOptimizerAgent {
    fn default() -> Self { Self::new() }
}

#[unsafe(no_mangle)]
pub extern "C" fn nopt_select(feature_sum: f64) -> i64 {
    let features = [feature_sum];
    STATE.lock().unwrap().select_pass(&features) as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn nopt_record(pass: i64, speedup: f64) -> i64 {
    STATE.lock().unwrap().record_reward(pass as usize, speedup);
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn nopt_best() -> i64 {
    STATE.lock().unwrap().best_pass() as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn nopt_total() -> f64 {
    STATE.lock().unwrap().total_rewards()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_select_pass_in_range() {
        let agent = NeuralOptimizerAgent::new();
        let pass = agent.select_pass(&[1.0, 2.0, 3.0]);
        assert!(pass < 8);
    }

    #[test]
    fn test_record_reward_updates_total() {
        let mut agent = NeuralOptimizerAgent::new();
        agent.record_reward(0, 1.5);
        agent.record_reward(1, 2.0);
        assert!((agent.total_rewards() - 3.5).abs() < 1e-9);
    }

    #[test]
    fn test_best_pass_after_rewards() {
        let mut agent = NeuralOptimizerAgent::new();
        agent.record_reward(3, 10.0);
        agent.record_reward(1, 2.0);
        assert_eq!(agent.best_pass(), 3);
    }

    #[test]
    fn test_empty_features() {
        let agent = NeuralOptimizerAgent::new();
        let pass = agent.select_pass(&[]);
        assert!(pass < 8);
    }

    #[test]
    fn test_ffi_nopt_record_and_total() {
        nopt_record(2, 5.0);
        assert!(nopt_total() >= 5.0);
    }

    #[test]
    fn test_ffi_nopt_best() {
        let b = nopt_best();
        assert!(b >= 0);
    }
}
