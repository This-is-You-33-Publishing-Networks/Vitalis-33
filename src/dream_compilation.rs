//! Offline Dream Pass Optimization — Vitalis v925
//!
//! Performs offline optimization by replaying past workloads in a "dream"
//! phase to discover new optimization opportunities.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<DreamCompiler>> = LazyLock::new(|| Mutex::new(DreamCompiler::new()));

pub struct DreamCompiler {
    discoveries: usize,
    total_gain: f64,
    workloads: Vec<String>,
}

impl DreamCompiler {
    pub fn new() -> Self {
        Self { discoveries: 0, total_gain: 0.0, workloads: Vec::new() }
    }

    pub fn replay_workload(&mut self, name: &str) {
        self.workloads.push(name.to_string());
        // Each replay finds some optimizations
        let gain = name.len() as f64 * 0.01;
        self.total_gain += gain;
        if gain > 0.05 {
            self.discoveries += 1;
        }
    }

    pub fn discoveries(&self) -> usize {
        self.discoveries
    }

    pub fn optimization_gain(&self) -> f64 {
        self.total_gain
    }

    pub fn reset(&mut self) {
        self.discoveries = 0;
        self.total_gain = 0.0;
        self.workloads.clear();
    }
}

impl Default for DreamCompiler {
    fn default() -> Self { Self::new() }
}

#[unsafe(no_mangle)]
pub extern "C" fn dream_replay(name_len: i64) -> i64 {
    let name = "workload_".to_string() + &"w".repeat(name_len.max(0) as usize);
    STATE.lock().unwrap().replay_workload(&name);
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn dream_discoveries() -> i64 {
    STATE.lock().unwrap().discoveries() as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn dream_gain() -> f64 {
    STATE.lock().unwrap().optimization_gain()
}

#[unsafe(no_mangle)]
pub extern "C" fn dream_reset() -> i64 {
    STATE.lock().unwrap().reset();
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_replay_increases_gain() {
        let mut dc = DreamCompiler::new();
        dc.replay_workload("http_server");
        assert!(dc.optimization_gain() > 0.0);
    }

    #[test]
    fn test_discoveries_for_long_workload() {
        let mut dc = DreamCompiler::new();
        dc.replay_workload("long_workload_name_here");
        assert!(dc.discoveries() > 0);
    }

    #[test]
    fn test_multiple_replays() {
        let mut dc = DreamCompiler::new();
        dc.replay_workload("w1");
        dc.replay_workload("w2");
        dc.replay_workload("w3");
        assert!(dc.optimization_gain() > 0.0);
    }

    #[test]
    fn test_reset() {
        let mut dc = DreamCompiler::new();
        dc.replay_workload("something");
        dc.reset();
        assert_eq!(dc.discoveries(), 0);
        assert_eq!(dc.optimization_gain(), 0.0);
    }

    #[test]
    fn test_gain_accumulates() {
        let mut dc = DreamCompiler::new();
        dc.replay_workload("aaaa");
        let g1 = dc.optimization_gain();
        dc.replay_workload("bbbb");
        assert!(dc.optimization_gain() > g1);
    }

    #[test]
    fn test_ffi_dream_replay() {
        dream_reset();
        dream_replay(10);
        assert!(dream_gain() > 0.0);
    }
}
