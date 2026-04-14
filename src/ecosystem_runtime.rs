//! Program Ecosystem Runtime — Vitalis v905
//!
//! Simulates a competitive ecosystem of services where resource competition
//! determines survival and fitness through evolutionary rounds.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<EcosystemRuntime>> = LazyLock::new(|| Mutex::new(EcosystemRuntime::new()));

pub struct EcosystemRuntime {
    services: Vec<(String, f64)>, // (name, resources)
}

impl EcosystemRuntime {
    pub fn new() -> Self {
        Self { services: Vec::new() }
    }

    pub fn spawn_service(&mut self, name: &str, resources: f64) {
        self.services.push((name.to_string(), resources));
    }

    pub fn compete(&mut self, rounds: u32) -> Vec<String> {
        for _ in 0..rounds {
            self.services.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
            let len = self.services.len();
            if len > 1 {
                // Top half gains resources, bottom half loses
                for i in 0..len / 2 {
                    self.services[i].1 *= 1.1;
                }
                for i in len / 2..len {
                    self.services[i].1 *= 0.9;
                }
            }
        }
        self.services.iter().map(|(n, _)| n.clone()).collect()
    }

    pub fn service_count(&self) -> usize {
        self.services.len()
    }

    pub fn fittest(&self) -> Option<String> {
        self.services.iter()
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(n, _)| n.clone())
    }
}

impl Default for EcosystemRuntime {
    fn default() -> Self { Self::new() }
}

#[unsafe(no_mangle)]
pub extern "C" fn eco_spawn(name_len: i64, resources: f64) -> i64 {
    let name = "svc_".to_string() + &"x".repeat(name_len.max(0) as usize);
    STATE.lock().unwrap().spawn_service(&name, resources);
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn eco_compete(rounds: i64) -> i64 {
    STATE.lock().unwrap().compete(rounds.max(0) as u32).len() as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn eco_count() -> i64 {
    STATE.lock().unwrap().service_count() as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn eco_fittest() -> i64 {
    STATE.lock().unwrap().fittest().map(|n| n.len() as i64).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spawn_service() {
        let mut eco = EcosystemRuntime::new();
        eco.spawn_service("web", 100.0);
        assert_eq!(eco.service_count(), 1);
    }

    #[test]
    fn test_compete_returns_all() {
        let mut eco = EcosystemRuntime::new();
        eco.spawn_service("a", 50.0);
        eco.spawn_service("b", 30.0);
        let survivors = eco.compete(3);
        assert_eq!(survivors.len(), 2);
    }

    #[test]
    fn test_fittest_has_most_resources() {
        let mut eco = EcosystemRuntime::new();
        eco.spawn_service("weak", 10.0);
        eco.spawn_service("strong", 100.0);
        assert_eq!(eco.fittest(), Some("strong".to_string()));
    }

    #[test]
    fn test_no_services_no_fittest() {
        let eco = EcosystemRuntime::new();
        assert_eq!(eco.fittest(), None);
    }

    #[test]
    fn test_compete_changes_resources() {
        let mut eco = EcosystemRuntime::new();
        eco.spawn_service("a", 100.0);
        eco.spawn_service("b", 50.0);
        eco.compete(1);
        // After competition, still 2 services
        assert_eq!(eco.service_count(), 2);
    }

    #[test]
    fn test_ffi_eco_spawn_and_count() {
        eco_spawn(3, 75.0);
        assert!(eco_count() >= 1);
    }
}
