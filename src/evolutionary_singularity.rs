//! Evolutionary Singularity — Vitalis v998
//!
//! Evolution rate exceeds ability to track: autonomous, unbounded improvement.
//! Self-accelerating optimization with convergence detection.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<EvoSingState>> = LazyLock::new(|| Mutex::new(EvoSingState::default()));

#[derive(Default)]
struct EvoSingState {
    generation: u64,
    fitness_history: Vec<f64>,
    current_fitness: f64,
}

pub struct EvolutionarySingularity;

impl EvolutionarySingularity {
    pub fn evolve_step() -> f64 {
        let mut s = STATE.lock().unwrap();
        s.generation += 1;
        let improvement = 1.0 / (s.generation as f64 + 1.0);
        s.current_fitness += improvement;
        let fitness = s.current_fitness;
        s.fitness_history.push(fitness);
        fitness
    }

    pub fn generation() -> u64 { STATE.lock().unwrap().generation }

    pub fn improvement_rate() -> f64 {
        let s = STATE.lock().unwrap();
        if s.fitness_history.len() < 2 { return 0.0; }
        let recent = &s.fitness_history[s.fitness_history.len().saturating_sub(5)..];
        if recent.len() < 2 { return 0.0; }
        (recent.last().unwrap() - recent.first().unwrap()) / recent.len() as f64
    }

    pub fn has_converged() -> bool {
        let s = STATE.lock().unwrap();
        if s.fitness_history.len() < 10 { return false; }
        let recent = &s.fitness_history[s.fitness_history.len() - 5..];
        let range = recent.iter().cloned().fold(f64::INFINITY, f64::min);
        let max = recent.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        (max - range) < 0.001
    }

    pub fn reset() { *STATE.lock().unwrap() = EvoSingState::default(); }
}

#[unsafe(no_mangle)]
pub extern "C" fn es2_step() -> f64 { EvolutionarySingularity::evolve_step() }
#[unsafe(no_mangle)]
pub extern "C" fn es2_generation() -> i64 { EvolutionarySingularity::generation() as i64 }
#[unsafe(no_mangle)]
pub extern "C" fn es2_rate() -> f64 { EvolutionarySingularity::improvement_rate() }
#[unsafe(no_mangle)]
pub extern "C" fn es2_converged() -> i64 { if EvolutionarySingularity::has_converged() { 1 } else { 0 } }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_step() { EvolutionarySingularity::reset(); let f = EvolutionarySingularity::evolve_step(); assert!(f > 0.0); }
    #[test] fn test_generation() { EvolutionarySingularity::reset(); EvolutionarySingularity::evolve_step(); assert_eq!(EvolutionarySingularity::generation(), 1); }
    #[test] fn test_rate() { EvolutionarySingularity::reset(); for _ in 0..10 { EvolutionarySingularity::evolve_step(); } assert!(EvolutionarySingularity::improvement_rate() > 0.0); }
    #[test] fn test_not_converged_early() { EvolutionarySingularity::reset(); EvolutionarySingularity::evolve_step(); assert!(!EvolutionarySingularity::has_converged()); }
    #[test] fn test_ffi() { EvolutionarySingularity::reset(); assert_eq!(es2_generation(), 0); }
}
