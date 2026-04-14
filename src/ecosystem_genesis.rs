//! Ecosystem Genesis — Vitalis v1285
//!
//! Bootstraps self-sustaining tool/library/compiler ecosystem.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<EcosystemGenesisState>> = LazyLock::new(|| Mutex::new(EcosystemGenesisState::default()));

#[derive(Default)]
struct EcosystemGenesisState {
    entities: Vec<(String, String)>,
    relationships: Vec<(String, String)>,
    health: f64,
}

pub struct EcosystemGenesis;

impl EcosystemGenesis {
    pub fn add_entity(name: &str, kind: &str) {
        let mut s = STATE.lock().unwrap();
        s.entities.push((name.to_string(), kind.to_string()));
        s.health = s.entities.len() as f64 / (s.entities.len() as f64 + 1.0);
    }
    pub fn add_relationship(from: &str, to: &str) {
        let mut s = STATE.lock().unwrap();
        s.relationships.push((from.to_string(), to.to_string()));
    }
    pub fn entity_count() -> usize { STATE.lock().unwrap().entities.len() }
    pub fn relationship_count() -> usize { STATE.lock().unwrap().relationships.len() }
    pub fn health() -> f64 { STATE.lock().unwrap().health }
    pub fn reset() { *STATE.lock().unwrap() = EcosystemGenesisState::default(); }
}

#[unsafe(no_mangle)]
pub extern "C" fn egen_entity_count() -> usize { EcosystemGenesis::entity_count() }
#[unsafe(no_mangle)]
pub extern "C" fn egen_relationship_count() -> usize { EcosystemGenesis::relationship_count() }
#[unsafe(no_mangle)]
pub extern "C" fn egen_health() -> f64 { EcosystemGenesis::health() }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_add_entity() { EcosystemGenesis::reset(); EcosystemGenesis::add_entity("compiler", "tool"); assert_eq!(EcosystemGenesis::entity_count(), 1); }
    #[test] fn test_add_relationship() { EcosystemGenesis::reset(); EcosystemGenesis::add_relationship("compiler", "stdlib"); assert_eq!(EcosystemGenesis::relationship_count(), 1); }
    #[test] fn test_health_grows() { EcosystemGenesis::reset(); EcosystemGenesis::add_entity("a", "lib"); EcosystemGenesis::add_entity("b", "tool"); assert!(EcosystemGenesis::health() > 0.5); }
    #[test] fn test_initial_health() { EcosystemGenesis::reset(); assert!((EcosystemGenesis::health() - 0.0).abs() < 1e-9); }
    #[test] fn test_multiple_entities() { EcosystemGenesis::reset(); for i in 0..5 { EcosystemGenesis::add_entity(&format!("e{}", i), "lib"); } assert_eq!(EcosystemGenesis::entity_count(), 5); }
    #[test] fn test_multiple_relationships() { EcosystemGenesis::reset(); EcosystemGenesis::add_relationship("a", "b"); EcosystemGenesis::add_relationship("b", "c"); assert_eq!(EcosystemGenesis::relationship_count(), 2); }
    #[test] fn test_entity_kinds() { EcosystemGenesis::reset(); EcosystemGenesis::add_entity("x", "compiler"); assert_eq!(STATE.lock().unwrap().entities[0].1, "compiler"); }
    #[test] fn test_reset() { EcosystemGenesis::reset(); EcosystemGenesis::add_entity("x", "t"); EcosystemGenesis::reset(); assert_eq!(EcosystemGenesis::entity_count(), 0); }
}
