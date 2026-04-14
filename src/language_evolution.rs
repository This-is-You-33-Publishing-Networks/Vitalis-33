//! Language Syntax Evolution — Vitalis v770
//!
//! Evolves programming language syntax through fitness-based selection.
//! Proposes new syntax constructs and selects the most fit.

use std::sync::{LazyLock, Mutex};
use std::collections::HashMap;

static STATE: LazyLock<Mutex<LanguageEvolution>> = LazyLock::new(|| Mutex::new(LanguageEvolution::new()));

pub struct LanguageEvolution {
    proposals: HashMap<u32, (String, String, f64)>, // id -> (name, pattern, fitness)
    next_id: u32,
    generation: u32,
}

impl LanguageEvolution {
    pub fn new() -> Self {
        Self { proposals: HashMap::new(), next_id: 0, generation: 0 }
    }

    pub fn propose_syntax(&mut self, name: &str, pattern: &str) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        self.proposals.insert(id, (name.to_string(), pattern.to_string(), 0.0));
        id
    }

    pub fn evaluate_fitness(&mut self, id: u32, score: f64) {
        if let Some(entry) = self.proposals.get_mut(&id) {
            entry.2 = score;
        }
        self.generation += 1;
    }

    pub fn select_best(&self) -> Option<u32> {
        self.proposals.iter()
            .max_by(|a, b| a.1.2.partial_cmp(&b.1.2).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(&id, _)| id)
    }

    pub fn generation(&self) -> u32 {
        self.generation
    }
}

impl Default for LanguageEvolution {
    fn default() -> Self { Self::new() }
}

#[unsafe(no_mangle)]
pub extern "C" fn le_propose(name_len: i64, pattern_len: i64) -> i64 {
    let name = "syn_".to_string() + &"x".repeat(name_len.max(0) as usize);
    let pattern = "p".repeat(pattern_len.max(0) as usize);
    STATE.lock().unwrap().propose_syntax(&name, &pattern) as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn le_evaluate(id: i64, score: f64) -> i64 {
    STATE.lock().unwrap().evaluate_fitness(id as u32, score);
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn le_best() -> i64 {
    STATE.lock().unwrap().select_best().map(|id| id as i64).unwrap_or(-1)
}

#[unsafe(no_mangle)]
pub extern "C" fn le_generation() -> i64 {
    STATE.lock().unwrap().generation() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_propose_returns_id() {
        let mut le = LanguageEvolution::new();
        let id = le.propose_syntax("match_expr", "match * { }");
        assert_eq!(id, 0);
    }

    #[test]
    fn test_evaluate_fitness() {
        let mut le = LanguageEvolution::new();
        let id = le.propose_syntax("if_expr", "if _ then _");
        le.evaluate_fitness(id, 0.85);
        assert_eq!(le.generation(), 1);
    }

    #[test]
    fn test_select_best() {
        let mut le = LanguageEvolution::new();
        let a = le.propose_syntax("a", "p1");
        let b = le.propose_syntax("b", "p2");
        le.evaluate_fitness(a, 0.5);
        le.evaluate_fitness(b, 0.9);
        assert_eq!(le.select_best(), Some(b));
    }

    #[test]
    fn test_no_proposals_no_best() {
        let le = LanguageEvolution::new();
        assert_eq!(le.select_best(), None);
    }

    #[test]
    fn test_generation_increments() {
        let mut le = LanguageEvolution::new();
        let id = le.propose_syntax("x", "y");
        le.evaluate_fitness(id, 0.5);
        le.evaluate_fitness(id, 0.6);
        assert_eq!(le.generation(), 2);
    }

    #[test]
    fn test_ffi_le_propose_and_best() {
        let id = le_propose(3, 5);
        le_evaluate(id, 0.7);
        assert!(le_best() >= 0);
    }
}
