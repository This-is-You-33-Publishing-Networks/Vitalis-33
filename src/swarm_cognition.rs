//! Swarm Cognition — Vitalis v1007
//!
//! Emergent intelligence from a swarm of simple compiler agents.
//! Each agent follows local rules; collective behavior produces optimization.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<SwarmState>> = LazyLock::new(|| Mutex::new(SwarmState::default()));

#[derive(Default)]
struct SwarmState {
    agents: Vec<(String, f64)>,
    pheromones: Vec<(String, f64)>,
    iterations: u64,
    best_fitness: f64,
}

pub struct SwarmCognition;

impl SwarmCognition {
    pub fn spawn_agent(name: &str, fitness: f64) -> usize {
        let mut s = STATE.lock().unwrap();
        s.agents.push((name.to_string(), fitness));
        if fitness > s.best_fitness { s.best_fitness = fitness; }
        s.agents.len()
    }

    pub fn deposit_pheromone(path: &str, strength: f64) {
        let mut s = STATE.lock().unwrap();
        if let Some(p) = s.pheromones.iter_mut().find(|(p, _)| p == path) {
            p.1 += strength;
        } else {
            s.pheromones.push((path.to_string(), strength));
        }
    }

    pub fn iterate() -> f64 {
        let mut s = STATE.lock().unwrap();
        s.iterations += 1;
        // Evaporate pheromones
        for p in s.pheromones.iter_mut() { p.1 *= 0.95; }
        s.best_fitness
    }

    pub fn best_path() -> Option<String> {
        let s = STATE.lock().unwrap();
        s.pheromones.iter().max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal)).map(|(p, _)| p.clone())
    }

    pub fn best_fitness() -> f64 { STATE.lock().unwrap().best_fitness }
    pub fn agent_count() -> usize { STATE.lock().unwrap().agents.len() }
    pub fn reset() { *STATE.lock().unwrap() = SwarmState::default(); }
}

#[unsafe(no_mangle)]
pub extern "C" fn swcog_spawn(fitness: f64) -> i64 { SwarmCognition::spawn_agent("ffi_agent", fitness) as i64 }
#[unsafe(no_mangle)]
pub extern "C" fn swcog_iterate() -> f64 { SwarmCognition::iterate() }
#[unsafe(no_mangle)]
pub extern "C" fn swcog_best() -> f64 { SwarmCognition::best_fitness() }
#[unsafe(no_mangle)]
pub extern "C" fn swcog_agents() -> i64 { SwarmCognition::agent_count() as i64 }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_spawn() { SwarmCognition::reset(); assert_eq!(SwarmCognition::spawn_agent("a", 0.5), 1); }
    #[test] fn test_pheromone() { SwarmCognition::reset(); SwarmCognition::deposit_pheromone("path1", 1.0); assert_eq!(SwarmCognition::best_path(), Some("path1".to_string())); }
    #[test] fn test_iterate() { SwarmCognition::reset(); SwarmCognition::spawn_agent("a", 0.8); let f = SwarmCognition::iterate(); assert!((f - 0.8).abs() < 0.01); }
    #[test] fn test_evaporation() { SwarmCognition::reset(); SwarmCognition::deposit_pheromone("p", 1.0); SwarmCognition::iterate(); let s = STATE.lock().unwrap(); assert!(s.pheromones[0].1 < 1.0); }
    #[test] fn test_accumulate() { SwarmCognition::reset(); SwarmCognition::deposit_pheromone("p", 1.0); SwarmCognition::deposit_pheromone("p", 1.0); let s = STATE.lock().unwrap(); assert!((s.pheromones[0].1 - 2.0).abs() < 0.01); }
    #[test] fn test_agent_count() { SwarmCognition::reset(); SwarmCognition::spawn_agent("x", 0.1); SwarmCognition::spawn_agent("y", 0.2); assert_eq!(SwarmCognition::agent_count(), 2); }
    #[test] fn test_ffi() { SwarmCognition::reset(); assert_eq!(swcog_agents(), 0); }
}
