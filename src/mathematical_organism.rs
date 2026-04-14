//! Mathematical Organism — Vitalis v1269
//!
//! Compiler as mathematical organism with metabolism (optimization) and reproduction.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<MathematicalOrganismState>> = LazyLock::new(|| Mutex::new(MathematicalOrganismState::default()));

#[derive(Default)]
struct MathematicalOrganismState {
    metabolism_rate: f64,
    offspring: Vec<String>,
    generation: u64,
    energy: f64,
}

pub struct MathematicalOrganism;

impl MathematicalOrganism {
    pub fn metabolize(rate: f64) -> f64 {
        let mut s = STATE.lock().unwrap();
        s.metabolism_rate = rate;
        s.energy += rate * 0.8;
        s.energy
    }
    pub fn reproduce(name: &str) -> u64 {
        let mut s = STATE.lock().unwrap();
        s.offspring.push(name.to_string());
        s.generation += 1;
        s.generation
    }
    pub fn energy() -> f64 { STATE.lock().unwrap().energy }
    pub fn offspring_count() -> usize { STATE.lock().unwrap().offspring.len() }
    pub fn generation() -> u64 { STATE.lock().unwrap().generation }
    pub fn reset() { *STATE.lock().unwrap() = MathematicalOrganismState::default(); }
}

#[unsafe(no_mangle)]
pub extern "C" fn mo_metabolize(rate: f64) -> f64 { MathematicalOrganism::metabolize(rate) }
#[unsafe(no_mangle)]
pub extern "C" fn mo_reproduce() -> u64 { MathematicalOrganism::reproduce("child") }
#[unsafe(no_mangle)]
pub extern "C" fn mo_energy() -> f64 { MathematicalOrganism::energy() }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_metabolize() { MathematicalOrganism::reset(); let e = MathematicalOrganism::metabolize(10.0); assert!(e > 0.0); }
    #[test] fn test_reproduce() { MathematicalOrganism::reset(); let g = MathematicalOrganism::reproduce("alpha"); assert_eq!(g, 1); }
    #[test] fn test_energy_accumulates() { MathematicalOrganism::reset(); MathematicalOrganism::metabolize(5.0); MathematicalOrganism::metabolize(5.0); assert!(MathematicalOrganism::energy() > 7.0); }
    #[test] fn test_offspring_count() { MathematicalOrganism::reset(); MathematicalOrganism::reproduce("a"); MathematicalOrganism::reproduce("b"); assert_eq!(MathematicalOrganism::offspring_count(), 2); }
    #[test] fn test_generation() { MathematicalOrganism::reset(); MathematicalOrganism::reproduce("x"); assert_eq!(MathematicalOrganism::generation(), 1); }
    #[test] fn test_initial_energy() { MathematicalOrganism::reset(); assert!((MathematicalOrganism::energy() - 0.0).abs() < 1e-9); }
    #[test] fn test_reset() { MathematicalOrganism::reset(); MathematicalOrganism::metabolize(10.0); MathematicalOrganism::reset(); assert!((MathematicalOrganism::energy() - 0.0).abs() < 1e-9); }
}
