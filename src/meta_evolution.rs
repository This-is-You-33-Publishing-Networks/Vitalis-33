//! Meta-Evolution — Evolution of evolution itself.
//!
//! This module allows Vitalis to evolve its own evolution strategies.
//! Instead of just evolving code, the system evolves HOW it evolves:
//! which mutation approaches work best, what fitness functions to use,
//! when to explore vs exploit, and how aggressively to mutate.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    Meta-Evolution                           │
//! │                                                             │
//! │  ┌───────────────┐  ┌───────────────┐  ┌────────────────┐  │
//! │  │  Strategy      │  │  Adaptive     │  │  Cross-        │  │
//! │  │  Registry      │  │  Parameters   │  │  Pollination   │  │
//! │  │                │  │               │  │                │  │
//! │  │  register()    │  │  mutation_rate │  │  combine()     │  │
//! │  │  select()      │  │  explore_rate  │  │  breed()       │  │
//! │  │  score()       │  │  risk_level    │  │  hybridize()   │  │
//! │  └───────┬───────┘  └───────┬───────┘  └───────┬────────┘  │
//! │          │                  │                   │           │
//! │  ┌───────▼──────────────────▼───────────────────▼────────┐  │
//! │  │              Strategy Selection Engine                │  │
//! │  │  multi-armed bandit + Thompson sampling               │  │
//! │  │  explore vs exploit balance                           │  │
//! │  └───────────────────────────────────────────────────────┘  │
//! └─────────────────────────────────────────────────────────────┘
//! ```

use std::collections::HashMap;

// ═══════════════════════════════════════════════════════════════════════
//  EVOLUTION STRATEGY — a particular approach to mutation
// ═══════════════════════════════════════════════════════════════════════

/// A strategy for evolving code — the "how" of mutation.
#[derive(Debug, Clone)]
pub struct EvolutionStrategy {
    /// Unique name of this strategy
    pub name: String,
    /// Description of what this strategy does
    pub description: String,
    /// Parameters that control this strategy's behavior
    pub params: StrategyParams,
    /// How many times has this strategy been used?
    pub total_uses: u64,
    /// How many times has it succeeded?
    pub successes: u64,
    /// Running average fitness improvement when this strategy is used
    pub avg_improvement: f64,
    /// Best single improvement achieved
    pub best_improvement: f64,
    /// Use history: (cycle, fitness_before, fitness_after, success)
    pub history: Vec<StrategyUseRecord>,
    /// Thompson sampling parameters: (alpha, beta) for Beta distribution
    pub thompson_alpha: f64,
    pub thompson_beta: f64,
    /// Whether this strategy was born from combining other strategies
    pub parent_strategies: Vec<String>,
    /// Generation of this strategy itself (meta-meta-evolution)
    pub generation: u64,
}

/// Parameters that control an evolution strategy.
#[derive(Debug, Clone)]
pub struct StrategyParams {
    /// Mutation rate (0.0 = no change, 1.0 = maximum change)
    pub mutation_rate: f64,
    /// Exploration vs exploitation (0.0 = pure exploit, 1.0 = pure explore)
    pub explore_rate: f64,
    /// Risk tolerance (0.0 = conservative, 1.0 = aggressive)
    pub risk_level: f64,
    /// How many candidate mutations to try before selecting
    pub candidate_count: u32,
    /// Minimum fitness improvement to accept (fitness gate)
    pub fitness_threshold: f64,
    /// Whether to use crossover (combine parts of different solutions)
    pub use_crossover: bool,
    /// Whether to use elitism (always keep the best)
    pub use_elitism: bool,
}

impl StrategyParams {
    pub fn default_conservative() -> Self {
        Self {
            mutation_rate: 0.1,
            explore_rate: 0.2,
            risk_level: 0.1,
            candidate_count: 3,
            fitness_threshold: 0.0,
            use_crossover: false,
            use_elitism: true,
        }
    }

    pub fn default_aggressive() -> Self {
        Self {
            mutation_rate: 0.5,
            explore_rate: 0.7,
            risk_level: 0.8,
            candidate_count: 5,
            fitness_threshold: -0.1, // Accept slight regressions
            use_crossover: true,
            use_elitism: false,
        }
    }

    pub fn default_balanced() -> Self {
        Self {
            mutation_rate: 0.3,
            explore_rate: 0.4,
            risk_level: 0.4,
            candidate_count: 3,
            fitness_threshold: 0.0,
            use_crossover: true,
            use_elitism: true,
        }
    }

    /// Mutate the strategy parameters themselves (meta-mutation).
    pub fn mutate(&self, intensity: f64) -> Self {
        let jitter = |value: f64, amt: f64| -> f64 {
            // Simple deterministic mutation using value itself as seed
            let offset = ((value * 1000.0).sin() * amt).abs();
            let direction = if (value * 7919.0).cos() > 0.0 { 1.0 } else { -1.0 };
            (value + direction * offset).clamp(0.0, 1.0)
        };

        Self {
            mutation_rate: jitter(self.mutation_rate, intensity * 0.2),
            explore_rate: jitter(self.explore_rate, intensity * 0.3),
            risk_level: jitter(self.risk_level, intensity * 0.25),
            candidate_count: (self.candidate_count as f64
                + if intensity > 0.5 { 1.0 } else { -0.5 }).max(1.0) as u32,
            fitness_threshold: jitter(self.fitness_threshold + 0.5, intensity * 0.1) - 0.5,
            use_crossover: if intensity > 0.7 { !self.use_crossover } else { self.use_crossover },
            use_elitism: if intensity > 0.8 { !self.use_elitism } else { self.use_elitism },
        }
    }

    /// Combine two strategy parameter sets (crossover).
    pub fn crossover(&self, other: &Self) -> Self {
        Self {
            mutation_rate: (self.mutation_rate + other.mutation_rate) / 2.0,
            explore_rate: (self.explore_rate + other.explore_rate) / 2.0,
            risk_level: (self.risk_level + other.risk_level) / 2.0,
            candidate_count: (self.candidate_count + other.candidate_count) / 2,
            fitness_threshold: (self.fitness_threshold + other.fitness_threshold) / 2.0,
            use_crossover: self.use_crossover || other.use_crossover,
            use_elitism: self.use_elitism && other.use_elitism,
        }
    }
}

/// Record of a single use of a strategy.
#[derive(Debug, Clone)]
pub struct StrategyUseRecord {
    pub cycle: u64,
    pub function_name: String,
    pub fitness_before: f64,
    pub fitness_after: f64,
    pub success: bool,
}

// ═══════════════════════════════════════════════════════════════════════
//  META-EVOLUTION ENGINE
// ═══════════════════════════════════════════════════════════════════════

/// The meta-evolution engine — evolves evolution strategies.
pub struct MetaEvolutionEngine {
    /// All known strategies
    strategies: HashMap<String, EvolutionStrategy>,
    /// Current active strategy
    active_strategy: Option<String>,
    /// Total meta-evolution cycles
    meta_cycles: u64,
    /// Strategy breeding events
    breed_count: u64,
    /// Strategy extinction events (removed due to poor performance)
    extinctions: u64,
    /// Best strategy ever observed
    best_strategy_name: Option<String>,
    best_strategy_score: f64,
}

impl MetaEvolutionEngine {
    pub fn new() -> Self {
        let mut engine = Self {
            strategies: HashMap::new(),
            active_strategy: None,
            meta_cycles: 0,
            breed_count: 0,
            extinctions: 0,
            best_strategy_name: None,
            best_strategy_score: 0.0,
        };

        // Seed with initial strategies
        engine.register_default_strategies();
        engine
    }

    /// Register the default set of initial strategies.
    fn register_default_strategies(&mut self) {
        self.register_strategy(
            "conservative",
            "Small, safe mutations. Prioritizes stability.",
            StrategyParams::default_conservative(),
        );
        self.register_strategy(
            "aggressive",
            "Large, exploratory mutations. Seeks breakthroughs.",
            StrategyParams::default_aggressive(),
        );
        self.register_strategy(
            "balanced",
            "Balanced approach with crossover and elitism.",
            StrategyParams::default_balanced(),
        );

        self.active_strategy = Some("balanced".to_string());
    }

    /// Register a new evolution strategy.
    pub fn register_strategy(&mut self, name: &str, description: &str, params: StrategyParams) {
        let strategy = EvolutionStrategy {
            name: name.to_string(),
            description: description.to_string(),
            params,
            total_uses: 0,
            successes: 0,
            avg_improvement: 0.0,
            best_improvement: 0.0,
            history: Vec::new(),
            thompson_alpha: 1.0,
            thompson_beta: 1.0,
            parent_strategies: Vec::new(),
            generation: 0,
        };
        self.strategies.insert(name.to_string(), strategy);
    }

    /// Select the best strategy to use right now.
    /// Uses Thompson Sampling (multi-armed bandit) for explore/exploit balance.
    pub fn select_strategy(&mut self) -> Option<&EvolutionStrategy> {
        if self.strategies.is_empty() {
            return None;
        }

        // Thompson Sampling: sample from Beta(alpha, beta) for each strategy
        // Pick the one with highest sample
        let mut best_name = String::new();
        let mut best_sample = f64::NEG_INFINITY;

        for (name, strategy) in &self.strategies {
            // Simplified Thompson sampling using deterministic approximation
            // (real implementation would use random sampling from Beta distribution)
            let mean = strategy.thompson_alpha
                / (strategy.thompson_alpha + strategy.thompson_beta);
            let uncertainty = 1.0
                / (strategy.thompson_alpha + strategy.thompson_beta).sqrt();

            // Score = mean + exploration bonus
            let score = mean + uncertainty * 0.5;

            if score > best_sample {
                best_sample = score;
                best_name = name.clone();
            }
        }

        self.active_strategy = Some(best_name.clone());
        self.strategies.get(&best_name)
    }

    /// Get the currently active strategy.
    pub fn active(&self) -> Option<&EvolutionStrategy> {
        self.active_strategy.as_ref()
            .and_then(|name| self.strategies.get(name))
    }

    /// Get the name of the currently active strategy.
    pub fn active_strategy_name(&self) -> Option<String> {
        self.active_strategy.clone()
    }

    /// Get the active strategy's parameters (or default).
    pub fn active_params(&self) -> StrategyParams {
        self.active()
            .map(|s| s.params.clone())
            .unwrap_or_else(StrategyParams::default_balanced)
    }

    /// Record the result of using a strategy.
    pub fn record_result(&mut self, strategy_name: &str, function_name: &str,
                          fitness_before: f64, fitness_after: f64,
                          success: bool, cycle: u64) {
        if let Some(strategy) = self.strategies.get_mut(strategy_name) {
            strategy.total_uses += 1;
            if success {
                strategy.successes += 1;
                strategy.thompson_alpha += 1.0;
            } else {
                strategy.thompson_beta += 1.0;
            }

            let improvement = fitness_after - fitness_before;
            if improvement > strategy.best_improvement {
                strategy.best_improvement = improvement;
            }

            // Running average
            strategy.avg_improvement = (strategy.avg_improvement * (strategy.total_uses - 1) as f64
                + improvement) / strategy.total_uses as f64;

            strategy.history.push(StrategyUseRecord {
                cycle,
                function_name: function_name.to_string(),
                fitness_before,
                fitness_after,
                success,
            });

            // Keep history bounded
            if strategy.history.len() > 500 {
                strategy.history.drain(0..strategy.history.len() - 500);
            }

            // Update best strategy tracking
            let success_rate = strategy.successes as f64 / strategy.total_uses as f64;
            let score = success_rate * 0.6 + (strategy.avg_improvement / 10.0).min(1.0) * 0.4;
            if score > self.best_strategy_score && strategy.total_uses >= 5 {
                self.best_strategy_score = score;
                self.best_strategy_name = Some(strategy_name.to_string());
            }
        }
    }

    /// Run a meta-evolution cycle — evolve the strategies themselves.
    pub fn meta_evolve(&mut self) -> MetaCycleResult {
        self.meta_cycles += 1;

        let mut bred = 0;
        let mut mutated = 0;
        let mut extinct = 0;

        // 1. Identify best and worst strategies (with minimum uses)
        let mut ranked: Vec<(String, f64)> = self.strategies.iter()
            .filter(|(_, s)| s.total_uses >= 3)
            .map(|(name, s)| {
                let success_rate = s.successes as f64 / s.total_uses.max(1) as f64;
                let score = success_rate * 0.5 + s.avg_improvement.max(0.0) / 10.0 * 0.5;
                (name.clone(), score)
            })
            .collect();

        ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // 2. Breed top strategies (crossover)
        if ranked.len() >= 2 {
            let parent_a = ranked[0].0.clone();
            let parent_b = ranked[1].0.clone();

            if let (Some(a), Some(b)) = (
                self.strategies.get(&parent_a).cloned(),
                self.strategies.get(&parent_b).cloned(),
            ) {
                let child_params = a.params.crossover(&b.params);
                let child_name = format!("hybrid_g{}_{}", self.meta_cycles, self.breed_count);

                let child = EvolutionStrategy {
                    name: child_name.clone(),
                    description: format!("Bred from {} × {}", parent_a, parent_b),
                    params: child_params,
                    total_uses: 0,
                    successes: 0,
                    avg_improvement: 0.0,
                    best_improvement: 0.0,
                    history: Vec::new(),
                    thompson_alpha: 1.0,
                    thompson_beta: 1.0,
                    parent_strategies: vec![parent_a, parent_b],
                    generation: a.generation.max(b.generation) + 1,
                };

                self.strategies.insert(child_name, child);
                self.breed_count += 1;
                bred += 1;
            }
        }

        // 3. Mutate a random strategy
        if let Some((name, strategy)) = self.strategies.iter()
            .filter(|(_, s)| s.total_uses >= 3)
            .max_by(|a, b| a.1.total_uses.cmp(&b.1.total_uses))
        {
            let mutated_params = strategy.params.mutate(0.3);
            let mutant_name = format!("mutant_g{}_{}", self.meta_cycles, name);
            let mutant = EvolutionStrategy {
                name: mutant_name.clone(),
                description: format!("Mutation of {}", name),
                params: mutated_params,
                total_uses: 0,
                successes: 0,
                avg_improvement: 0.0,
                best_improvement: 0.0,
                history: Vec::new(),
                thompson_alpha: 1.0,
                thompson_beta: 1.0,
                parent_strategies: vec![name.clone()],
                generation: strategy.generation + 1,
            };
            self.strategies.insert(mutant_name, mutant);
            mutated += 1;
        }

        // 4. Extinct poorly performing strategies (but keep at least 3)
        if self.strategies.len() > 5 {
            let to_remove: Vec<String> = ranked.iter()
                .rev()
                .take_while(|(_, score)| *score < 0.1)
                .map(|(name, _)| name.clone())
                .take(self.strategies.len().saturating_sub(3))
                .collect();

            for name in &to_remove {
                // Never remove the 3 defaults
                if name != "conservative" && name != "aggressive" && name != "balanced" {
                    self.strategies.remove(name);
                    self.extinctions += 1;
                    extinct += 1;
                }
            }
        }

        MetaCycleResult {
            meta_cycle: self.meta_cycles,
            strategies_bred: bred,
            strategies_mutated: mutated,
            strategies_extinct: extinct,
            total_strategies: self.strategies.len(),
            best_strategy: self.best_strategy_name.clone(),
        }
    }

    /// Get the strategy landscape — all strategies with their performance.
    pub fn landscape_json(&self) -> String {
        let strategies: Vec<String> = self.strategies.values().map(|s| {
            let success_rate = if s.total_uses > 0 {
                s.successes as f64 / s.total_uses as f64
            } else {
                0.0
            };
            format!(
                concat!(
                    "{{",
                    "\"name\":\"{}\",",
                    "\"generation\":{},",
                    "\"uses\":{},",
                    "\"successes\":{},",
                    "\"success_rate\":{:.4},",
                    "\"avg_improvement\":{:.4},",
                    "\"best_improvement\":{:.4},",
                    "\"mutation_rate\":{:.3},",
                    "\"explore_rate\":{:.3},",
                    "\"risk_level\":{:.3},",
                    "\"parents\":[{}]",
                    "}}"
                ),
                s.name,
                s.generation,
                s.total_uses,
                s.successes,
                success_rate,
                s.avg_improvement,
                s.best_improvement,
                s.params.mutation_rate,
                s.params.explore_rate,
                s.params.risk_level,
                s.parent_strategies.iter()
                    .map(|p| format!("\"{}\"", p))
                    .collect::<Vec<_>>()
                    .join(","),
            )
        }).collect();

        format!("[{}]", strategies.join(","))
    }

    /// Get meta-evolution statistics as JSON.
    pub fn stats_json(&self) -> String {
        let active = self.active_strategy.as_deref().unwrap_or("none");
        let best = self.best_strategy_name.as_deref().unwrap_or("none");

        format!(
            concat!(
                "{{",
                "\"meta_cycles\":{},",
                "\"total_strategies\":{},",
                "\"breed_count\":{},",
                "\"extinctions\":{},",
                "\"active_strategy\":\"{}\",",
                "\"best_strategy\":\"{}\",",
                "\"best_score\":{:.4}",
                "}}"
            ),
            self.meta_cycles,
            self.strategies.len(),
            self.breed_count,
            self.extinctions,
            active,
            best,
            self.best_strategy_score,
        )
    }

    /// List all strategy names.
    pub fn list_strategies(&self) -> Vec<String> {
        self.strategies.keys().cloned().collect()
    }

    /// Get a specific strategy by name.
    pub fn get_strategy(&self, name: &str) -> Option<&EvolutionStrategy> {
        self.strategies.get(name)
    }

    /// Get the number of strategies.
    pub fn strategy_count(&self) -> usize {
        self.strategies.len()
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  META-CYCLE RESULT
// ═══════════════════════════════════════════════════════════════════════

/// Result of a meta-evolution cycle.
#[derive(Debug, Clone)]
pub struct MetaCycleResult {
    pub meta_cycle: u64,
    pub strategies_bred: usize,
    pub strategies_mutated: usize,
    pub strategies_extinct: usize,
    pub total_strategies: usize,
    pub best_strategy: Option<String>,
}

impl MetaCycleResult {
    pub fn to_json(&self) -> String {
        let best = match &self.best_strategy {
            Some(s) => format!("\"{}\"", s),
            None => "null".to_string(),
        };
        format!(
            concat!(
                "{{",
                "\"meta_cycle\":{},",
                "\"bred\":{},",
                "\"mutated\":{},",
                "\"extinct\":{},",
                "\"total_strategies\":{},",
                "\"best_strategy\":{}",
                "}}"
            ),
            self.meta_cycle,
            self.strategies_bred,
            self.strategies_mutated,
            self.strategies_extinct,
            self.total_strategies,
            best,
        )
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  GLOBAL META-EVOLUTION ENGINE (thread-local)
// ═══════════════════════════════════════════════════════════════════════

use std::cell::RefCell;

thread_local! {
    static GLOBAL_META: RefCell<MetaEvolutionEngine> =
        RefCell::new(MetaEvolutionEngine::new());
}

/// Access the global meta-evolution engine.
pub fn with_meta<F, R>(f: F) -> R
where
    F: FnOnce(&mut MetaEvolutionEngine) -> R,
{
    GLOBAL_META.with(|m| f(&mut m.borrow_mut()))
}

// ═══════════════════════════════════════════════════════════════════════
//  v67: PRNG-based Thompson Sampling + Strategy Archiving
// ═══════════════════════════════════════════════════════════════════════

/// Xorshift64 PRNG for stochastic selection (same one used in evolution.rs).
pub struct MetaRng {
    state: u64,
}

impl MetaRng {
    pub fn new(seed: u64) -> Self {
        Self { state: if seed == 0 { 0xCAFEBABE } else { seed } }
    }

    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }

    pub fn next_f64(&mut self) -> f64 {
        (self.next_u64() & 0x1FFFFFFFFFFFFF) as f64 / (1u64 << 53) as f64
    }

    /// Sample from Beta(alpha, beta) using Jöhnk's algorithm.
    /// Returns a value in [0, 1].
    pub fn sample_beta(&mut self, alpha: f64, beta: f64) -> f64 {
        if alpha <= 0.0 || beta <= 0.0 { return 0.5; }

        // For alpha=1, beta=1 (uniform): just return random
        if (alpha - 1.0).abs() < 1e-9 && (beta - 1.0).abs() < 1e-9 {
            return self.next_f64();
        }

        // Gamma variate via Marsaglia-Tsang for alpha >= 1
        let ga = self.sample_gamma(alpha);
        let gb = self.sample_gamma(beta);
        let sum = ga + gb;
        if sum <= 0.0 { return 0.5; }
        ga / sum
    }

    /// Sample from Gamma(shape, 1.0) using Marsaglia-Tsang method.
    fn sample_gamma(&mut self, shape: f64) -> f64 {
        if shape < 1.0 {
            // Boost: Gamma(a) = Gamma(a+1) * U^(1/a)
            let u = self.next_f64().max(1e-15);
            return self.sample_gamma(shape + 1.0) * u.powf(1.0 / shape);
        }

        let d = shape - 1.0 / 3.0;
        let c = 1.0 / (9.0 * d).sqrt();

        loop {
            // Generate normal via Box-Muller
            let u1 = self.next_f64().max(1e-15);
            let u2 = self.next_f64();
            let z = (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos();

            let v = (1.0 + c * z).powi(3);
            if v <= 0.0 { continue; }

            let u = self.next_f64().max(1e-15);
            let zz = z * z;

            if u.ln() < 0.5 * zz + d * (1.0 - v + v.ln()) {
                return d * v;
            }
        }
    }
}

/// An archived strategy that was extinct but preserved for reference.
#[derive(Debug, Clone)]
pub struct ArchivedStrategy {
    pub name: String,
    pub generation: u64,
    pub total_uses: u64,
    pub successes: u64,
    pub avg_improvement: f64,
    pub params: StrategyParams,
    pub extinction_cycle: u64,
    pub extinction_reason: String,
}

impl MetaEvolutionEngine {
    /// Select strategy using real PRNG-based Thompson sampling.
    pub fn select_strategy_stochastic(&mut self, rng: &mut MetaRng) -> Option<String> {
        if self.strategies.is_empty() { return None; }

        let mut best_name = String::new();
        let mut best_sample = f64::NEG_INFINITY;

        for (name, strategy) in &self.strategies {
            let sample = rng.sample_beta(strategy.thompson_alpha, strategy.thompson_beta);
            if sample > best_sample {
                best_sample = sample;
                best_name = name.clone();
            }
        }

        self.active_strategy = Some(best_name.clone());
        Some(best_name)
    }

    /// Archive a strategy before extinction (preserves knowledge).
    pub fn archive_strategy(&self, name: &str, cycle: u64, reason: &str) -> Option<ArchivedStrategy> {
        self.strategies.get(name).map(|s| ArchivedStrategy {
            name: s.name.clone(),
            generation: s.generation,
            total_uses: s.total_uses,
            successes: s.successes,
            avg_improvement: s.avg_improvement,
            params: s.params.clone(),
            extinction_cycle: cycle,
            extinction_reason: reason.to_string(),
        })
    }

    /// Run meta-evolution with real PRNG and archiving.
    pub fn meta_evolve_with_rng(&mut self, rng: &mut MetaRng, archive: &mut Vec<ArchivedStrategy>) -> MetaCycleResult {
        self.meta_cycles += 1;
        let cycle = self.meta_cycles;

        let mut bred = 0;
        let mut mutated = 0;
        let mut extinct = 0;

        // 1. Rank strategies
        let mut ranked: Vec<(String, f64)> = self.strategies.iter()
            .filter(|(_, s)| s.total_uses >= 3)
            .map(|(name, s)| {
                let success_rate = s.successes as f64 / s.total_uses.max(1) as f64;
                let score = success_rate * 0.5 + s.avg_improvement.max(0.0) / 10.0 * 0.5;
                (name.clone(), score)
            })
            .collect();
        ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // 2. Breed top strategies with stochastic parent selection
        if ranked.len() >= 2 {
            // Use rng to sometimes pick non-top parents (diversity)
            let parent_a_idx = if rng.next_f64() < 0.7 { 0 } else { rng.next_u64() as usize % ranked.len() };
            let mut parent_b_idx = if rng.next_f64() < 0.7 { 1 } else { rng.next_u64() as usize % ranked.len() };
            if parent_b_idx == parent_a_idx { parent_b_idx = (parent_a_idx + 1) % ranked.len(); }

            let parent_a = ranked[parent_a_idx].0.clone();
            let parent_b = ranked[parent_b_idx].0.clone();

            if let (Some(a), Some(b)) = (
                self.strategies.get(&parent_a).cloned(),
                self.strategies.get(&parent_b).cloned(),
            ) {
                let mut child_params = a.params.crossover(&b.params);
                // Stochastic mutation on child
                let mutation_intensity = rng.next_f64() * 0.3;
                child_params = child_params.mutate(mutation_intensity);

                let child_name = format!("hybrid_g{}_{}", cycle, self.breed_count);
                let child = EvolutionStrategy {
                    name: child_name.clone(),
                    description: format!("Bred from {} × {} (rng)", parent_a, parent_b),
                    params: child_params,
                    total_uses: 0,
                    successes: 0,
                    avg_improvement: 0.0,
                    best_improvement: 0.0,
                    history: Vec::new(),
                    thompson_alpha: 1.0,
                    thompson_beta: 1.0,
                    parent_strategies: vec![parent_a, parent_b],
                    generation: a.generation.max(b.generation) + 1,
                };
                self.strategies.insert(child_name, child);
                self.breed_count += 1;
                bred += 1;
            }
        }

        // 3. Mutate using PRNG intensity
        if let Some((name, strategy)) = self.strategies.iter()
            .filter(|(_, s)| s.total_uses >= 3)
            .max_by(|a, b| a.1.total_uses.cmp(&b.1.total_uses))
        {
            let intensity = rng.next_f64() * 0.6 + 0.1;
            let mutated_params = strategy.params.mutate(intensity);
            let mutant_name = format!("mutant_g{}_{}", cycle, name);
            let mutant = EvolutionStrategy {
                name: mutant_name.clone(),
                description: format!("Mutation of {} (intensity {:.2})", name, intensity),
                params: mutated_params,
                total_uses: 0,
                successes: 0,
                avg_improvement: 0.0,
                best_improvement: 0.0,
                history: Vec::new(),
                thompson_alpha: 1.0,
                thompson_beta: 1.0,
                parent_strategies: vec![name.clone()],
                generation: strategy.generation + 1,
            };
            self.strategies.insert(mutant_name, mutant);
            mutated += 1;
        }

        // 4. Extinct poorly performing strategies — archive before removing
        if self.strategies.len() > 5 {
            let to_remove: Vec<String> = ranked.iter()
                .rev()
                .take_while(|(_, score)| *score < 0.1)
                .map(|(name, _)| name.clone())
                .filter(|n| n != "conservative" && n != "aggressive" && n != "balanced")
                .take(self.strategies.len().saturating_sub(3))
                .collect();

            for name in &to_remove {
                if let Some(archived) = self.archive_strategy(name, cycle, "low performance score") {
                    archive.push(archived);
                }
                self.strategies.remove(name);
                self.extinctions += 1;
                extinct += 1;
            }
        }

        MetaCycleResult {
            meta_cycle: cycle,
            strategies_bred: bred,
            strategies_mutated: mutated,
            strategies_extinct: extinct,
            total_strategies: self.strategies.len(),
            best_strategy: self.best_strategy_name.clone(),
        }
    }

    /// Compute multi-objective score for a strategy.
    pub fn multi_objective_score(strategy: &EvolutionStrategy) -> f64 {
        if strategy.total_uses == 0 { return 0.0; }
        let success_rate = strategy.successes as f64 / strategy.total_uses as f64;
        let improvement = strategy.avg_improvement.max(0.0);
        let consistency = if strategy.total_uses >= 10 {
            // Variance of success: lower is more consistent
            1.0 - (success_rate * (1.0 - success_rate)).sqrt()
        } else {
            0.5
        };
        // Combined score: 40% success, 30% improvement, 30% consistency
        success_rate * 0.4 + (improvement / 10.0).min(1.0) * 0.3 + consistency * 0.3
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  TESTS
// ═══════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strategy_params_default() {
        let cons = StrategyParams::default_conservative();
        assert!(cons.mutation_rate < 0.3);
        assert!(cons.risk_level < 0.3);
        assert!(cons.use_elitism);

        let aggr = StrategyParams::default_aggressive();
        assert!(aggr.mutation_rate > 0.3);
        assert!(aggr.risk_level > 0.5);
        assert!(aggr.use_crossover);
    }

    #[test]
    fn test_strategy_params_mutation() {
        let params = StrategyParams::default_balanced();
        let mutated = params.mutate(0.5);

        // Mutation should change at least something
        let unchanged = (mutated.mutation_rate - params.mutation_rate).abs() < 0.001
            && (mutated.explore_rate - params.explore_rate).abs() < 0.001
            && (mutated.risk_level - params.risk_level).abs() < 0.001;
        // Note: deterministic mutation based on sin() may not change everything
        // but at least some values should differ
        assert!(!unchanged || mutated.candidate_count != params.candidate_count,
            "Mutation should change parameters");
    }

    #[test]
    fn test_strategy_params_crossover() {
        let a = StrategyParams::default_conservative();
        let b = StrategyParams::default_aggressive();
        let child = a.crossover(&b);

        // Child should be between parents
        assert!(child.mutation_rate >= a.mutation_rate.min(b.mutation_rate));
        assert!(child.mutation_rate <= a.mutation_rate.max(b.mutation_rate));
    }

    #[test]
    fn test_meta_engine_new() {
        let engine = MetaEvolutionEngine::new();
        assert_eq!(engine.strategy_count(), 3); // conservative, aggressive, balanced
        assert!(engine.active().is_some());
    }

    #[test]
    fn test_meta_engine_register() {
        let mut engine = MetaEvolutionEngine::new();
        engine.register_strategy(
            "experimental",
            "A test strategy",
            StrategyParams::default_balanced(),
        );
        assert_eq!(engine.strategy_count(), 4);
    }

    #[test]
    fn test_meta_engine_select() {
        let mut engine = MetaEvolutionEngine::new();
        let strategy = engine.select_strategy();
        assert!(strategy.is_some());
        assert!(engine.active_strategy.is_some());
    }

    #[test]
    fn test_meta_engine_record_success() {
        let mut engine = MetaEvolutionEngine::new();
        engine.record_result("balanced", "func_a", 50.0, 60.0, true, 1);

        let strategy = engine.get_strategy("balanced").unwrap();
        assert_eq!(strategy.total_uses, 1);
        assert_eq!(strategy.successes, 1);
        assert!(strategy.avg_improvement > 0.0);
    }

    #[test]
    fn test_meta_engine_record_failure() {
        let mut engine = MetaEvolutionEngine::new();
        engine.record_result("conservative", "func_b", 50.0, 45.0, false, 1);

        let strategy = engine.get_strategy("conservative").unwrap();
        assert_eq!(strategy.total_uses, 1);
        assert_eq!(strategy.successes, 0);
    }

    #[test]
    fn test_meta_engine_thompson_sampling() {
        let mut engine = MetaEvolutionEngine::new();

        // Make "balanced" clearly the best
        for i in 0..20 {
            engine.record_result("balanced", "f", 50.0, 60.0, true, i);
        }
        for i in 0..20 {
            engine.record_result("aggressive", "f", 50.0, 45.0, false, i);
        }

        // Select should prefer balanced
        let selected = engine.select_strategy().unwrap();
        // With 20 successes vs 20 failures, Thompson should strongly prefer balanced
        // (the deterministic approximation should also prefer it)
        assert!(selected.name == "balanced" || selected.name == "conservative",
            "Should prefer successful strategies");
    }

    #[test]
    fn test_meta_evolve_breeds() {
        let mut engine = MetaEvolutionEngine::new();

        // Give strategies enough data
        for i in 0..5 {
            engine.record_result("balanced", "f", 50.0, 60.0, true, i);
            engine.record_result("conservative", "f", 50.0, 55.0, true, i);
            engine.record_result("aggressive", "f", 50.0, 45.0, false, i);
        }

        let result = engine.meta_evolve();
        assert!(result.strategies_bred > 0 || result.strategies_mutated > 0,
            "Meta-evolution should breed or mutate");
        assert!(result.total_strategies > 3, "Should have created new strategies");
    }

    #[test]
    fn test_meta_evolve_cycle_result() {
        let result = MetaCycleResult {
            meta_cycle: 1,
            strategies_bred: 2,
            strategies_mutated: 1,
            strategies_extinct: 0,
            total_strategies: 6,
            best_strategy: Some("balanced".to_string()),
        };
        let json = result.to_json();
        assert!(json.contains("\"meta_cycle\":1"));
        assert!(json.contains("\"bred\":2"));
    }

    #[test]
    fn test_meta_engine_landscape_json() {
        let engine = MetaEvolutionEngine::new();
        let json = engine.landscape_json();
        assert!(json.contains("conservative"));
        assert!(json.contains("aggressive"));
        assert!(json.contains("balanced"));
    }

    #[test]
    fn test_meta_engine_stats_json() {
        let engine = MetaEvolutionEngine::new();
        let json = engine.stats_json();
        assert!(json.contains("\"meta_cycles\":0"));
        assert!(json.contains("\"total_strategies\":3"));
    }

    #[test]
    fn test_meta_engine_list_strategies() {
        let engine = MetaEvolutionEngine::new();
        let names = engine.list_strategies();
        assert_eq!(names.len(), 3);
    }

    #[test]
    fn test_meta_engine_active_params() {
        let engine = MetaEvolutionEngine::new();
        let params = engine.active_params();
        // Default active is balanced
        assert!(params.mutation_rate > 0.0);
    }

    #[test]
    fn test_global_meta() {
        with_meta(|m| {
            assert_eq!(m.strategy_count(), 3);
        });
    }

    #[test]
    fn test_meta_evolution_lifecycle() {
        let mut engine = MetaEvolutionEngine::new();

        // Simulate 50 evolution cycles with strategy selection
        for cycle in 0..50 {
            let strategy_name = engine.select_strategy()
                .map(|s| s.name.clone())
                .unwrap_or_else(|| "balanced".to_string());

            let success = cycle % 3 != 0; // 66% success rate
            let improvement = if success { 5.0 } else { -2.0 };

            engine.record_result(
                &strategy_name,
                &format!("func_{}", cycle % 5),
                50.0,
                50.0 + improvement,
                success,
                cycle,
            );

            // Meta-evolve every 10 cycles
            if cycle % 10 == 9 {
                let result = engine.meta_evolve();
                assert!(result.total_strategies >= 3);
            }
        }

        // After 50 cycles, should have learned something
        let stats = engine.stats_json();
        assert!(stats.contains("\"meta_cycles\""));
        assert!(engine.strategy_count() >= 3);

        let landscape = engine.landscape_json();
        assert!(!landscape.is_empty());
    }

    #[test]
    fn test_strategy_history_bounded() {
        let mut engine = MetaEvolutionEngine::new();

        // Record many results
        for i in 0..600 {
            engine.record_result("balanced", "f", 50.0, 55.0, true, i);
        }

        let strategy = engine.get_strategy("balanced").unwrap();
        assert!(strategy.history.len() <= 500, "History should be bounded");
    }

    // ─── v67: PRNG + Archiving Tests ──────────────────────────────────

    #[test]
    fn test_meta_rng_deterministic() {
        let mut rng1 = MetaRng::new(42);
        let mut rng2 = MetaRng::new(42);
        for _ in 0..50 {
            assert_eq!(rng1.next_u64(), rng2.next_u64());
        }
    }

    #[test]
    fn test_meta_rng_f64_range() {
        let mut rng = MetaRng::new(123);
        for _ in 0..100 {
            let v = rng.next_f64();
            assert!(v >= 0.0 && v < 1.0, "f64 out of range: {}", v);
        }
    }

    #[test]
    fn test_beta_sampling_range() {
        let mut rng = MetaRng::new(999);
        for _ in 0..100 {
            let sample = rng.sample_beta(2.0, 5.0);
            assert!(sample >= 0.0 && sample <= 1.0, "Beta sample out of range: {}", sample);
        }
    }

    #[test]
    fn test_beta_sampling_bias() {
        // Beta(10, 2) should produce high values on average
        let mut rng = MetaRng::new(77);
        let mut sum = 0.0;
        let n = 200;
        for _ in 0..n {
            sum += rng.sample_beta(10.0, 2.0);
        }
        let mean = sum / n as f64;
        // Expected mean = alpha/(alpha+beta) = 10/12 ≈ 0.833
        assert!(mean > 0.6, "Beta(10,2) mean too low: {}", mean);
    }

    #[test]
    fn test_stochastic_select() {
        let mut engine = MetaEvolutionEngine::new();
        let mut rng = MetaRng::new(42);

        // Make balanced strongly preferred
        for i in 0..20 {
            engine.record_result("balanced", "f", 50.0, 60.0, true, i);
            engine.record_result("aggressive", "f", 50.0, 45.0, false, i);
        }

        let mut balanced_count = 0;
        for _ in 0..50 {
            let name = engine.select_strategy_stochastic(&mut rng).unwrap();
            if name == "balanced" { balanced_count += 1; }
        }
        // Balanced should win most of the time
        assert!(balanced_count > 25, "balanced only selected {} / 50 times", balanced_count);
    }

    #[test]
    fn test_archive_strategy() {
        let mut engine = MetaEvolutionEngine::new();
        engine.record_result("conservative", "f", 50.0, 55.0, true, 1);

        let archived = engine.archive_strategy("conservative", 10, "test extinction");
        assert!(archived.is_some());
        let archived = archived.unwrap();
        assert_eq!(archived.name, "conservative");
        assert_eq!(archived.extinction_cycle, 10);
        assert_eq!(archived.total_uses, 1);
    }

    #[test]
    fn test_meta_evolve_with_rng_and_archive() {
        let mut engine = MetaEvolutionEngine::new();
        let mut rng = MetaRng::new(42);
        let mut archive: Vec<ArchivedStrategy> = Vec::new();

        for i in 0..5 {
            engine.record_result("balanced", "f", 50.0, 60.0, true, i);
            engine.record_result("conservative", "f", 50.0, 55.0, true, i);
            engine.record_result("aggressive", "f", 50.0, 45.0, false, i);
        }

        let result = engine.meta_evolve_with_rng(&mut rng, &mut archive);
        assert!(result.strategies_bred > 0 || result.strategies_mutated > 0);
        assert!(result.total_strategies >= 3);
    }

    #[test]
    fn test_multi_objective_score() {
        let strategy = EvolutionStrategy {
            name: "test".to_string(),
            description: String::new(),
            params: StrategyParams::default_balanced(),
            total_uses: 20,
            successes: 16,  // 80% success rate
            avg_improvement: 5.0,
            best_improvement: 10.0,
            history: Vec::new(),
            thompson_alpha: 17.0,
            thompson_beta: 5.0,
            parent_strategies: Vec::new(),
            generation: 0,
        };
        let score = MetaEvolutionEngine::multi_objective_score(&strategy);
        assert!(score > 0.3, "multi-objective score too low: {}", score);
    }
}
