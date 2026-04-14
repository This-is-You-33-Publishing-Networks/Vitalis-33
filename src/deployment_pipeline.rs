//! Autonomous CI/CD Pipeline — Vitalis v875
//!
//! Manages autonomous continuous integration and deployment pipelines
//! with stage execution, rollback tracking, and status monitoring.

use std::sync::{LazyLock, Mutex};
use std::collections::{HashMap, HashSet};

static STATE: LazyLock<Mutex<DeployPipeline>> = LazyLock::new(|| Mutex::new(DeployPipeline::new()));

pub struct DeployPipeline {
    stages: Vec<String>,
    passed: HashSet<String>,
    rollbacks: usize,
}

impl DeployPipeline {
    pub fn new() -> Self {
        Self { stages: Vec::new(), passed: HashSet::new(), rollbacks: 0 }
    }

    pub fn add_stage(&mut self, name: &str) {
        self.stages.push(name.to_string());
    }

    pub fn run_stage(&mut self, name: &str) -> bool {
        let success = !name.contains("fail");
        if success {
            self.passed.insert(name.to_string());
        } else {
            self.rollbacks += 1;
        }
        success
    }

    pub fn stages_passed(&self) -> usize {
        self.passed.len()
    }

    pub fn rollback_count(&self) -> usize {
        self.rollbacks
    }
}

impl Default for DeployPipeline {
    fn default() -> Self { Self::new() }
}

#[unsafe(no_mangle)]
pub extern "C" fn dp_add_stage(name_len: i64) -> i64 {
    let name = "stage_".to_string() + &"s".repeat(name_len.max(0) as usize);
    STATE.lock().unwrap().add_stage(&name);
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn dp_run(stage_id: i64) -> i64 {
    let name = format!("stage_{stage_id}");
    if STATE.lock().unwrap().run_stage(&name) { 1 } else { 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn dp_passed() -> i64 {
    STATE.lock().unwrap().stages_passed() as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn dp_rollback() -> i64 {
    STATE.lock().unwrap().rollback_count() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_stage() {
        let mut dp = DeployPipeline::new();
        dp.add_stage("build");
        assert_eq!(dp.stages.len(), 1);
    }

    #[test]
    fn test_run_stage_success() {
        let mut dp = DeployPipeline::new();
        assert!(dp.run_stage("build"));
        assert_eq!(dp.stages_passed(), 1);
    }

    #[test]
    fn test_run_stage_failure() {
        let mut dp = DeployPipeline::new();
        assert!(!dp.run_stage("fail_deploy"));
        assert_eq!(dp.rollback_count(), 1);
    }

    #[test]
    fn test_stages_passed_count() {
        let mut dp = DeployPipeline::new();
        dp.run_stage("build");
        dp.run_stage("test");
        assert_eq!(dp.stages_passed(), 2);
    }

    #[test]
    fn test_rollback_accumulates() {
        let mut dp = DeployPipeline::new();
        dp.run_stage("fail_1");
        dp.run_stage("fail_2");
        assert_eq!(dp.rollback_count(), 2);
    }

    #[test]
    fn test_ffi_dp_run_and_passed() {
        dp_run(10);
        assert!(dp_passed() >= 1);
    }
}
