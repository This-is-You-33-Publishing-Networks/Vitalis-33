//! v420 — Pass Evolution Engine.
//!
//! Compiler optimization passes that evolve themselves via genetic programming
//! over IR transformation rules. Fitness = speedup × code_size_reduction × correctness.
//! Integrates with `optimizer.rs` pass infrastructure and `evolution_safety_rails.rs`.

use std::collections::HashMap;

/// A rule that transforms one IR pattern into another.
#[derive(Debug, Clone)]
pub struct TransformRule {
    pub name: String,
    /// Pattern hash (simplified: we use a string representation)
    pub pattern: String,
    /// Replacement pattern
    pub replacement: String,
    /// Times this rule was applied successfully
    pub applications: u64,
    /// Measured average speedup factor (1.0 = no change)
    pub avg_speedup: f64,
    /// Measured average code size reduction (1.0 = no change)
    pub avg_size_reduction: f64,
    /// Correctness rate (1.0 = always correct)
    pub correctness_rate: f64,
    /// Generation this rule was born
    pub generation: u64,
}

impl TransformRule {
    pub fn new(name: impl Into<String>, pattern: impl Into<String>, replacement: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            pattern: pattern.into(),
            replacement: replacement.into(),
            applications: 0,
            avg_speedup: 1.0,
            avg_size_reduction: 1.0,
            correctness_rate: 1.0,
            generation: 0,
        }
    }

    /// Combined fitness: geometric mean of speedup, size reduction, and correctness.
    pub fn fitness(&self) -> f64 {
        (self.avg_speedup * self.avg_size_reduction * self.correctness_rate).cbrt()
    }

    /// Record an application outcome.
    pub fn record_outcome(&mut self, speedup: f64, size_reduction: f64, correct: bool) {
        self.applications += 1;
        let n = self.applications as f64;
        // Running average
        self.avg_speedup = self.avg_speedup * ((n - 1.0) / n) + speedup / n;
        self.avg_size_reduction = self.avg_size_reduction * ((n - 1.0) / n) + size_reduction / n;
        let c = if correct { 1.0 } else { 0.0 };
        self.correctness_rate = self.correctness_rate * ((n - 1.0) / n) + c / n;
    }
}

/// A sequence of transform rules forming a complete optimization pass.
#[derive(Debug, Clone)]
pub struct EvolvedPass {
    pub name: String,
    pub rules: Vec<TransformRule>,
    pub generation: u64,
    pub total_applications: u64,
    pub fitness_history: Vec<f64>,
}

impl EvolvedPass {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            rules: Vec::new(),
            generation: 0,
            total_applications: 0,
            fitness_history: Vec::new(),
        }
    }

    pub fn add_rule(&mut self, rule: TransformRule) {
        self.rules.push(rule);
    }

    /// Average fitness across all rules.
    pub fn fitness(&self) -> f64 {
        if self.rules.is_empty() {
            return 0.0;
        }
        self.rules.iter().map(|r| r.fitness()).sum::<f64>() / self.rules.len() as f64
    }

    /// Number of active rules.
    pub fn rule_count(&self) -> usize {
        self.rules.len()
    }

    /// Record fitness for trend analysis.
    pub fn record_fitness(&mut self) {
        self.fitness_history.push(self.fitness());
        if self.fitness_history.len() > 100 {
            self.fitness_history.remove(0);
        }
    }

    /// Fitness trend (positive = improving).
    pub fn fitness_trend(&self) -> f64 {
        if self.fitness_history.len() < 2 {
            return 0.0;
        }
        let n = self.fitness_history.len();
        let recent = self.fitness_history[n - 1];
        let earlier = self.fitness_history[n / 2];
        recent - earlier
    }
}

/// Crossover two passes: interleave rules from each.
pub fn crossover_passes(a: &EvolvedPass, b: &EvolvedPass) -> EvolvedPass {
    let mut child = EvolvedPass::new(format!("{}_x_{}", a.name, b.name));
    child.generation = a.generation.max(b.generation) + 1;
    let max_len = a.rules.len().max(b.rules.len());
    for i in 0..max_len {
        if i < a.rules.len() && (i % 2 == 0 || i >= b.rules.len()) {
            child.rules.push(a.rules[i].clone());
        } else if i < b.rules.len() {
            child.rules.push(b.rules[i].clone());
        }
    }
    child
}

/// Mutate a pass by perturbing one rule.
pub fn mutate_pass(pass: &EvolvedPass, seed: u64) -> EvolvedPass {
    let mut mutated = pass.clone();
    mutated.generation += 1;
    if mutated.rules.is_empty() {
        return mutated;
    }
    let idx = (seed as usize) % mutated.rules.len();
    let kind = seed % 3;
    match kind {
        0 => {
            // Perturb replacement
            mutated.rules[idx].replacement = format!("{}_mut{}", mutated.rules[idx].replacement, seed % 100);
        }
        1 => {
            // Swap two rules if possible
            if mutated.rules.len() > 1 {
                let other = ((seed / 7) as usize) % mutated.rules.len();
                mutated.rules.swap(idx, other);
            }
        }
        _ => {
            // Duplicate a rule
            let dup = mutated.rules[idx].clone();
            mutated.rules.push(dup);
        }
    }
    mutated
}

/// The Pass Evolution Engine: manages a population of evolved passes.
pub struct PassEvolutionEngine {
    pub passes: HashMap<String, EvolvedPass>,
    pub generation: u64,
    pub population_cap: usize,
    pub elitism_count: usize,
    pub total_evolutions: u64,
    pub total_improvements: u64,
}

impl PassEvolutionEngine {
    pub fn new() -> Self {
        Self {
            passes: HashMap::new(),
            generation: 0,
            population_cap: 50,
            elitism_count: 5,
            total_evolutions: 0,
            total_improvements: 0,
        }
    }

    /// Register a pass.
    pub fn register_pass(&mut self, pass: EvolvedPass) {
        self.passes.insert(pass.name.clone(), pass);
    }

    /// Get a pass by name.
    pub fn get_pass(&self, name: &str) -> Option<&EvolvedPass> {
        self.passes.get(name)
    }

    /// Get all pass names sorted by fitness (descending).
    pub fn ranked_passes(&self) -> Vec<(String, f64)> {
        let mut ranked: Vec<_> = self.passes.iter()
            .map(|(name, pass)| (name.clone(), pass.fitness()))
            .collect();
        ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        ranked
    }

    /// Best pass by fitness.
    pub fn best_pass(&self) -> Option<&EvolvedPass> {
        self.passes.values()
            .max_by(|a, b| a.fitness().partial_cmp(&b.fitness()).unwrap_or(std::cmp::Ordering::Equal))
    }

    /// Run one evolution cycle: select parents, crossover, mutate, evaluate.
    pub fn evolve_cycle(&mut self, seed: u64) -> EvolutionCycleResult {
        self.generation += 1;
        self.total_evolutions += 1;

        let ranked = self.ranked_passes();
        if ranked.len() < 2 {
            return EvolutionCycleResult {
                generation: self.generation,
                new_passes: 0,
                best_fitness: ranked.first().map(|r| r.1).unwrap_or(0.0),
                improvement: false,
            };
        }

        let best_before = ranked[0].1;

        // Crossover top 2
        let parent_a = &self.passes[&ranked[0].0];
        let parent_b = &self.passes[&ranked[1].0];
        let child = crossover_passes(parent_a, parent_b);
        let child_name = child.name.clone();
        self.passes.insert(child_name, child);

        // Mutate a random pass
        let mut_idx = (seed as usize) % ranked.len();
        let parent = self.passes[&ranked[mut_idx].0].clone();
        let mutant = mutate_pass(&parent, seed);
        let mutant_name = mutant.name.clone();
        self.passes.insert(mutant_name, mutant);

        // Cull if over population cap
        while self.passes.len() > self.population_cap {
            let worst = self.ranked_passes().last().map(|r| r.0.clone());
            if let Some(name) = worst {
                self.passes.remove(&name);
            }
        }

        let best_after = self.ranked_passes().first().map(|r| r.1).unwrap_or(0.0);
        let improvement = best_after > best_before;
        if improvement {
            self.total_improvements += 1;
        }

        EvolutionCycleResult {
            generation: self.generation,
            new_passes: 2,
            best_fitness: best_after,
            improvement,
        }
    }

    /// Number of passes in population.
    pub fn population_size(&self) -> usize {
        self.passes.len()
    }

    /// Overall improvement rate.
    pub fn improvement_rate(&self) -> f64 {
        if self.total_evolutions == 0 { return 0.0; }
        self.total_improvements as f64 / self.total_evolutions as f64
    }
}

/// Result of one evolution cycle.
#[derive(Debug)]
pub struct EvolutionCycleResult {
    pub generation: u64,
    pub new_passes: usize,
    pub best_fitness: f64,
    pub improvement: bool,
}

// ─── FFI ──────────────────────────────────────────────────────────────

use std::sync::{LazyLock, Mutex};

static GLOBAL_ENGINE: LazyLock<Mutex<PassEvolutionEngine>> =
    LazyLock::new(|| Mutex::new(PassEvolutionEngine::new()));

#[unsafe(no_mangle)]
pub extern "C" fn slang_pass_evo_register(name_hash: i64) -> i64 {
    let mut engine = GLOBAL_ENGINE.lock().unwrap();
    let name = format!("pass_{}", name_hash);
    engine.register_pass(EvolvedPass::new(name));
    engine.population_size() as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_pass_evo_cycle(seed: i64) -> i64 {
    let mut engine = GLOBAL_ENGINE.lock().unwrap();
    let result = engine.evolve_cycle(seed as u64);
    if result.improvement { 1 } else { 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_pass_evo_best_fitness() -> i64 {
    let engine = GLOBAL_ENGINE.lock().unwrap();
    let f = engine.best_pass().map(|p| p.fitness()).unwrap_or(0.0);
    f64::to_bits(f) as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_pass_evo_population_size() -> i64 {
    let engine = GLOBAL_ENGINE.lock().unwrap();
    engine.population_size() as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_pass_evo_generation() -> i64 {
    let engine = GLOBAL_ENGINE.lock().unwrap();
    engine.generation as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transform_rule_new() {
        let r = TransformRule::new("fold", "add(0, x)", "x");
        assert_eq!(r.name, "fold");
        assert_eq!(r.applications, 0);
        assert!((r.fitness() - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_transform_rule_fitness() {
        let mut r = TransformRule::new("r1", "a", "b");
        r.avg_speedup = 2.0;
        r.avg_size_reduction = 0.5;
        r.correctness_rate = 1.0;
        let expected = (2.0_f64 * 0.5 * 1.0).cbrt();
        assert!((r.fitness() - expected).abs() < 1e-10);
    }

    #[test]
    fn test_transform_rule_record() {
        let mut r = TransformRule::new("r1", "a", "b");
        r.record_outcome(2.0, 0.8, true);
        assert!(r.avg_speedup > 1.0);
        assert_eq!(r.applications, 1);
    }

    #[test]
    fn test_evolved_pass_new() {
        let p = EvolvedPass::new("test_pass");
        assert_eq!(p.name, "test_pass");
        assert_eq!(p.rule_count(), 0);
        assert!((p.fitness() - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_evolved_pass_add_rule() {
        let mut p = EvolvedPass::new("p1");
        p.add_rule(TransformRule::new("r1", "a", "b"));
        p.add_rule(TransformRule::new("r2", "c", "d"));
        assert_eq!(p.rule_count(), 2);
    }

    #[test]
    fn test_evolved_pass_fitness() {
        let mut p = EvolvedPass::new("p1");
        let mut r = TransformRule::new("r1", "a", "b");
        r.avg_speedup = 2.0;
        p.add_rule(r);
        assert!(p.fitness() > 0.0);
    }

    #[test]
    fn test_evolved_pass_trend() {
        let mut p = EvolvedPass::new("p1");
        p.fitness_history = vec![0.5, 0.6, 0.7, 0.8];
        assert!(p.fitness_trend() > 0.0);
    }

    #[test]
    fn test_crossover() {
        let mut a = EvolvedPass::new("a");
        a.add_rule(TransformRule::new("r1", "1", "2"));
        a.add_rule(TransformRule::new("r2", "3", "4"));
        let mut b = EvolvedPass::new("b");
        b.add_rule(TransformRule::new("r3", "5", "6"));
        let child = crossover_passes(&a, &b);
        assert!(!child.rules.is_empty());
        assert_eq!(child.generation, 1);
    }

    #[test]
    fn test_mutate() {
        let mut p = EvolvedPass::new("p1");
        p.add_rule(TransformRule::new("r1", "a", "b"));
        let m = mutate_pass(&p, 42);
        assert_eq!(m.generation, 1);
    }

    #[test]
    fn test_engine_new() {
        let e = PassEvolutionEngine::new();
        assert_eq!(e.population_size(), 0);
        assert_eq!(e.generation, 0);
    }

    #[test]
    fn test_engine_register() {
        let mut e = PassEvolutionEngine::new();
        e.register_pass(EvolvedPass::new("p1"));
        assert_eq!(e.population_size(), 1);
    }

    #[test]
    fn test_engine_ranked() {
        let mut e = PassEvolutionEngine::new();
        let mut p1 = EvolvedPass::new("p1");
        let mut r = TransformRule::new("r", "a", "b");
        r.avg_speedup = 3.0;
        p1.add_rule(r);
        e.register_pass(p1);
        e.register_pass(EvolvedPass::new("p2"));
        let ranked = e.ranked_passes();
        assert_eq!(ranked[0].0, "p1");
    }

    #[test]
    fn test_engine_evolve_insufficient() {
        let mut e = PassEvolutionEngine::new();
        e.register_pass(EvolvedPass::new("p1"));
        let r = e.evolve_cycle(0);
        assert_eq!(r.new_passes, 0); // needs at least 2
    }

    #[test]
    fn test_engine_evolve_cycle() {
        let mut e = PassEvolutionEngine::new();
        let mut p1 = EvolvedPass::new("p1");
        p1.add_rule(TransformRule::new("r1", "a", "b"));
        let mut p2 = EvolvedPass::new("p2");
        p2.add_rule(TransformRule::new("r2", "c", "d"));
        e.register_pass(p1);
        e.register_pass(p2);
        let r = e.evolve_cycle(42);
        assert_eq!(r.new_passes, 2);
        assert!(e.population_size() >= 2);
    }

    #[test]
    fn test_engine_improvement_rate() {
        let e = PassEvolutionEngine::new();
        assert!((e.improvement_rate() - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_engine_best_pass() {
        let mut e = PassEvolutionEngine::new();
        let mut p = EvolvedPass::new("best");
        let mut r = TransformRule::new("r", "a", "b");
        r.avg_speedup = 5.0;
        p.add_rule(r);
        e.register_pass(p);
        assert_eq!(e.best_pass().unwrap().name, "best");
    }

    #[test]
    fn test_ffi_register() {
        let count = slang_pass_evo_register(1);
        assert!(count >= 1);
    }

    #[test]
    fn test_ffi_population_size() {
        let size = slang_pass_evo_population_size();
        assert!(size >= 0);
    }

    #[test]
    fn test_ffi_generation() {
        let generation = slang_pass_evo_generation();
        assert!(generation >= 0);
    }
}
