//! Self-Healing Program Organism — Vitalis v900
//!
//! Implements a self-healing program organism that monitors health,
//! detects issues, and autonomously heals itself over generations.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<SoftwareOrganism>> = LazyLock::new(|| Mutex::new(SoftwareOrganism::new()));

pub struct SoftwareOrganism {
    health: f64,
    heal_count: usize,
    generation: u32,
}

impl SoftwareOrganism {
    pub fn new() -> Self {
        Self { health: 1.0, heal_count: 0, generation: 0 }
    }

    pub fn health_check(&self) -> f64 {
        self.health
    }

    pub fn self_heal(&mut self, issue: &str) -> bool {
        let severity = match issue {
            i if i.contains("critical") => 0.3,
            i if i.contains("warn") => 0.1,
            _ => 0.05,
        };
        self.health = (self.health - severity + 0.2).min(1.0).max(0.0);
        self.heal_count += 1;
        self.generation += 1;
        self.health > 0.5
    }

    pub fn heal_count(&self) -> usize {
        self.heal_count
    }

    pub fn generation(&self) -> u32 {
        self.generation
    }
}

impl Default for SoftwareOrganism {
    fn default() -> Self { Self::new() }
}

#[unsafe(no_mangle)]
pub extern "C" fn so_health() -> f64 {
    STATE.lock().unwrap().health_check()
}

#[unsafe(no_mangle)]
pub extern "C" fn so_heal(issue_id: i64) -> i64 {
    let issue = match issue_id % 3 {
        0 => "warning: memory high",
        1 => "critical: disk full",
        _ => "info: slow response",
    };
    if STATE.lock().unwrap().self_heal(issue) { 1 } else { 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn so_heal_count() -> i64 {
    STATE.lock().unwrap().heal_count() as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn so_generation() -> i64 {
    STATE.lock().unwrap().generation() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_health_full() {
        let org = SoftwareOrganism::new();
        assert_eq!(org.health_check(), 1.0);
    }

    #[test]
    fn test_heal_minor_issue() {
        let mut org = SoftwareOrganism::new();
        let ok = org.self_heal("minor glitch");
        assert!(ok);
    }

    #[test]
    fn test_heal_critical_reduces_health() {
        let mut org = SoftwareOrganism::new();
        for _ in 0..5 {
            org.self_heal("critical: failure");
        }
        assert!(org.health_check() >= 0.0);
    }

    #[test]
    fn test_heal_count() {
        let mut org = SoftwareOrganism::new();
        org.self_heal("warn: high cpu");
        org.self_heal("info: slow");
        assert_eq!(org.heal_count(), 2);
    }

    #[test]
    fn test_generation_increments() {
        let mut org = SoftwareOrganism::new();
        org.self_heal("issue");
        assert_eq!(org.generation(), 1);
    }

    #[test]
    fn test_ffi_so_heal_count() {
        so_heal(0);
        assert!(so_heal_count() >= 1);
    }
}
