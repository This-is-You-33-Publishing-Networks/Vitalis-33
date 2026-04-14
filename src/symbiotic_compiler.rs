//! Symbiotic Compiler — Vitalis v1033
//!
//! Compiler and program co-evolve, each improving the other.
//! The program teaches the compiler; the compiler teaches the program.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<SymState>> = LazyLock::new(|| Mutex::new(SymState::default()));

#[derive(Default)]
struct SymState {
    compiler_fitness: f64,
    program_fitness: f64,
    co_evolution_steps: u64,
    synergy: f64,
}

pub struct SymbioticCompiler;

impl SymbioticCompiler {
    pub fn co_evolve() -> (f64, f64) {
        let mut s = STATE.lock().unwrap();
        s.co_evolution_steps += 1;
        let boost = 0.1 / (s.co_evolution_steps as f64).sqrt();
        s.compiler_fitness += boost * (1.0 + s.program_fitness * 0.1);
        s.program_fitness += boost * (1.0 + s.compiler_fitness * 0.1);
        s.synergy = s.compiler_fitness * s.program_fitness;
        (s.compiler_fitness, s.program_fitness)
    }

    pub fn teach_compiler(lesson: &str) -> f64 {
        let mut s = STATE.lock().unwrap();
        s.compiler_fitness += lesson.len() as f64 * 0.01;
        s.compiler_fitness
    }

    pub fn teach_program(optimization: &str) -> f64 {
        let mut s = STATE.lock().unwrap();
        s.program_fitness += optimization.len() as f64 * 0.01;
        s.program_fitness
    }

    pub fn synergy() -> f64 { STATE.lock().unwrap().synergy }
    pub fn steps() -> u64 { STATE.lock().unwrap().co_evolution_steps }
    pub fn reset() { *STATE.lock().unwrap() = SymState::default(); }
}

#[unsafe(no_mangle)]
pub extern "C" fn sym_coevolve() -> f64 { let (c, p) = SymbioticCompiler::co_evolve(); c + p }
#[unsafe(no_mangle)]
pub extern "C" fn sym_synergy() -> f64 { SymbioticCompiler::synergy() }
#[unsafe(no_mangle)]
pub extern "C" fn sym_steps() -> i64 { SymbioticCompiler::steps() as i64 }
#[unsafe(no_mangle)]
pub extern "C" fn sym_teach_compiler(len: i64) -> f64 { SymbioticCompiler::teach_compiler(&"x".repeat(len as usize)) }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_coevolve() { SymbioticCompiler::reset(); let (c, p) = SymbioticCompiler::co_evolve(); assert!(c > 0.0 && p > 0.0); }
    #[test] fn test_teach_compiler() { SymbioticCompiler::reset(); let f = SymbioticCompiler::teach_compiler("inlining"); assert!(f > 0.0); }
    #[test] fn test_teach_program() { SymbioticCompiler::reset(); let f = SymbioticCompiler::teach_program("vectorize"); assert!(f > 0.0); }
    #[test] fn test_synergy() { SymbioticCompiler::reset(); SymbioticCompiler::co_evolve(); assert!(SymbioticCompiler::synergy() > 0.0); }
    #[test] fn test_steps() { SymbioticCompiler::reset(); SymbioticCompiler::co_evolve(); SymbioticCompiler::co_evolve(); assert_eq!(SymbioticCompiler::steps(), 2); }
    #[test] fn test_mutual_benefit() { SymbioticCompiler::reset(); for _ in 0..10 { SymbioticCompiler::co_evolve(); } assert!(SymbioticCompiler::synergy() > 0.0); }
    #[test] fn test_reset() { SymbioticCompiler::reset(); SymbioticCompiler::co_evolve(); SymbioticCompiler::reset(); assert_eq!(SymbioticCompiler::steps(), 0); }
    #[test] fn test_ffi() { SymbioticCompiler::reset(); assert_eq!(sym_steps(), 0); }
}
