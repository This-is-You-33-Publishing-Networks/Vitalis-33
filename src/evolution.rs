//! Evolution Engine — tracks `@evolvable` functions, supports hot-swap and fitness scoring.\n//!\n//! This module allows the evolution engine (Python-side) to:
//! - Register evolvable functions with their source code
//! - Submit new variants (mutations) and re-compile them
//! - Track fitness scores for each variant
//! - Query generation history
//!
//! # Architecture
//!
//! ```text
//! Python Evolution Engine
//!     │
//!     ▼ (FFI bridge)
//! EvolutionRegistry
//!     ├── EvolvedFunction { source, generation, fitness, hash }
//!     └── compile_variant() → Cranelift JIT → native code
//! ```

use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

// ─── Types ──────────────────────────────────────────────────────────────

/// Registry that tracks all `@evolvable` functions and their evolution history.
pub struct EvolutionRegistry {
    /// Active evolvable functions: name → current variant
    functions: HashMap<String, EvolvedFunction>,
    /// Full history: name → list of all variants (for rollback)
    history: HashMap<String, Vec<EvolvedFunction>>,
    /// Base source template (non-evolvable parts of the program)
    base_source: Option<String>,
}

/// A single variant of an evolvable function.
#[derive(Debug, Clone)]
pub struct EvolvedFunction {
    pub name: String,
    pub source: String,
    pub generation: u64,
    pub parent_hash: Option<u64>,
    pub hash: u64,
    pub fitness: Option<f64>,
}

/// Result of executing an evolved variant.
#[derive(Debug, Clone)]
pub struct EvolveResult {
    pub name: String,
    pub generation: u64,
    pub hash: u64,
    pub exec_result: Option<i64>,
    pub error: Option<String>,
}

/// Population-wide summary statistics.
#[derive(Debug, Clone)]
pub struct PopulationSummary {
    pub total_functions: usize,
    pub total_generations: u64,
    pub avg_fitness: f64,
    pub best_fitness: f64,
    pub worst_fitness: f64,
    pub diversity: f64,
    pub total_variants: usize,
}

impl PopulationSummary {
    pub fn to_json(&self) -> String {
        format!(
            concat!(
                "{{\"total_functions\":{},\"total_generations\":{},",
                "\"avg_fitness\":{:.4},\"best_fitness\":{:.4},\"worst_fitness\":{:.4},",
                "\"diversity\":{:.4},\"total_variants\":{}}}"
            ),
            self.total_functions, self.total_generations,
            self.avg_fitness, self.best_fitness, self.worst_fitness,
            self.diversity, self.total_variants
        )
    }
}

// ─── Implementation ─────────────────────────────────────────────────────

impl EvolutionRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self {
            functions: HashMap::new(),
            history: HashMap::new(),
            base_source: None,
        }
    }

    /// Load a full program, extracting `@evolvable` functions into the registry.
    /// The base source (non-evolvable parts) is stored for re-assembly.
    pub fn load_program(&mut self, source: &str) {
        self.base_source = Some(source.to_string());

        // Parse and extract @evolvable function names
        let (program, _errors) = crate::parser::parse(source);
        for item in &program.items {
            if let crate::ast::TopLevel::Annotated { annotations, item, .. } = item {
                let is_evolvable = annotations.iter().any(|a| a.name == "evolvable");
                if is_evolvable {
                    if let crate::ast::TopLevel::Function(f) = item.as_ref() {
                        let func_source = extract_function_source(source, &f.name);
                        let hash = hash_source(&func_source);
                        let ef = EvolvedFunction {
                            name: f.name.clone(),
                            source: func_source,
                            generation: 0,
                            parent_hash: None,
                            hash,
                            fitness: None,
                        };
                        self.functions.insert(f.name.clone(), ef.clone());
                        self.history.entry(f.name.clone()).or_default().push(ef);
                    }
                }
            }
        }
    }

    /// Register a function as evolvable with its source code.
    pub fn register(&mut self, name: &str, source: &str) {
        let hash = hash_source(source);
        let ef = EvolvedFunction {
            name: name.to_string(),
            source: source.to_string(),
            generation: 0,
            parent_hash: None,
            hash,
            fitness: None,
        };
        self.functions.insert(name.to_string(), ef.clone());
        self.history.entry(name.to_string()).or_default().push(ef);
    }

    /// Submit a new variant (mutation) for an evolvable function.
    /// Returns the new generation number and hash.
    pub fn evolve(&mut self, name: &str, new_source: &str) -> Result<(u64, u64), String> {
        let (parent_hash, generation) = match self.functions.get(name) {
            Some(current) => (Some(current.hash), current.generation + 1),
            None => (None, 0),
        };

        let hash = hash_source(new_source);
        let ef = EvolvedFunction {
            name: name.to_string(),
            source: new_source.to_string(),
            generation,
            parent_hash,
            hash,
            fitness: None,
        };

        self.functions.insert(name.to_string(), ef.clone());
        self.history.entry(name.to_string()).or_default().push(ef);

        Ok((generation, hash))
    }

    /// Compile and execute the current program with all evolved variants substituted.
    /// Returns the i64 result of main().
    pub fn compile_and_run(&self) -> Result<i64, String> {
        let source = self.assemble_source()?;
        crate::codegen::compile_and_run(&source)
    }

    /// Compile and run, then record the result as fitness for a specific function.
    pub fn evaluate_variant(&mut self, name: &str, fitness_fn: impl FnOnce(i64) -> f64) -> Result<EvolveResult, String> {
        let source = self.assemble_source()?;
        match crate::codegen::compile_and_run(&source) {
            Ok(result) => {
                let fitness = fitness_fn(result);
                if let Some(ef) = self.functions.get_mut(name) {
                    ef.fitness = Some(fitness);
                }
                // Also update in history
                if let Some(history) = self.history.get_mut(name) {
                    if let Some(last) = history.last_mut() {
                        last.fitness = Some(fitness);
                    }
                }
                let generation = self.functions.get(name).map_or(0, |f| f.generation);
                let hash = self.functions.get(name).map_or(0, |f| f.hash);
                Ok(EvolveResult {
                    name: name.to_string(),
                    generation,
                    hash,
                    exec_result: Some(result),
                    error: None,
                })
            }
            Err(e) => {
                let generation = self.functions.get(name).map_or(0, |f| f.generation);
                let hash = self.functions.get(name).map_or(0, |f| f.hash);
                Ok(EvolveResult {
                    name: name.to_string(),
                    generation,
                    hash,
                    exec_result: None,
                    error: Some(e),
                })
            }
        }
    }

    /// Set fitness score for a function variant.
    pub fn set_fitness(&mut self, name: &str, score: f64) {
        if let Some(ef) = self.functions.get_mut(name) {
            ef.fitness = Some(score);
        }
        if let Some(history) = self.history.get_mut(name) {
            if let Some(last) = history.last_mut() {
                last.fitness = Some(score);
            }
        }
    }

    /// Get fitness score for the current variant of a function.
    pub fn get_fitness(&self, name: &str) -> Option<f64> {
        self.functions.get(name).and_then(|f| f.fitness)
    }

    /// Get generation number for a function.
    pub fn get_generation(&self, name: &str) -> u64 {
        self.functions.get(name).map_or(0, |f| f.generation)
    }

    /// List all evolvable function names.
    pub fn list_evolvable(&self) -> Vec<String> {
        self.functions.keys().cloned().collect()
    }

    /// Get the current source of an evolvable function.
    pub fn get_source(&self, name: &str) -> Option<&str> {
        self.functions.get(name).map(|f| f.source.as_str())
    }

    /// Get full evolution history for a function.
    pub fn get_history(&self, name: &str) -> Vec<&EvolvedFunction> {
        self.history
            .get(name)
            .map_or_else(Vec::new, |h| h.iter().collect())
    }

    /// Rollback an evolvable function to a previous generation.
    pub fn rollback(&mut self, name: &str, generation: u64) -> Result<(), String> {
        let history = self.history.get(name).ok_or_else(|| format!("no history for '{}'", name))?;
        let variant = history
            .iter()
            .find(|v| v.generation == generation)
            .ok_or_else(|| format!("generation {} not found for '{}'", generation, name))?
            .clone();
        self.functions.insert(name.to_string(), variant);
        Ok(())
    }

    /// Get the best variant (highest fitness) for a function.
    pub fn best_variant(&self, name: &str) -> Option<&EvolvedFunction> {
        self.history.get(name).and_then(|h| {
            h.iter()
                .filter(|v| v.fitness.is_some())
                .max_by(|a, b| a.fitness.partial_cmp(&b.fitness).unwrap_or(std::cmp::Ordering::Equal))
        })
    }

    /// Compute diversity score across all tracked functions (0.0 = monoculture, 1.0 = max diversity).
    /// Measures hash uniqueness across the latest generation of each function.
    pub fn diversity_score(&self) -> f64 {
        let hashes: Vec<u64> = self.functions.values().map(|f| f.hash).collect();
        if hashes.len() <= 1 {
            return 0.0;
        }
        let unique: std::collections::HashSet<u64> = hashes.iter().copied().collect();
        unique.len() as f64 / hashes.len() as f64
    }

    /// Get a summary of the entire evolution population.
    pub fn population_summary(&self) -> PopulationSummary {
        let total_functions = self.functions.len();
        let total_generations: u64 = self.functions.values().map(|f| f.generation).sum();
        let fitnesses: Vec<f64> = self.functions.values().filter_map(|f| f.fitness).collect();
        let avg_fitness = if fitnesses.is_empty() {
            0.0
        } else {
            fitnesses.iter().sum::<f64>() / fitnesses.len() as f64
        };
        let best_fitness = fitnesses.iter().cloned().fold(0.0f64, f64::max);
        let worst_fitness = fitnesses.iter().cloned().fold(f64::MAX, f64::min);
        let diversity = self.diversity_score();
        let total_history: usize = self.history.values().map(|h| h.len()).sum();

        PopulationSummary {
            total_functions,
            total_generations,
            avg_fitness,
            best_fitness,
            worst_fitness: if worst_fitness == f64::MAX { 0.0 } else { worst_fitness },
            diversity,
            total_variants: total_history,
        }
    }

    // ─── Quantum-Inspired Selection Methods ──────────────────────────────

    /// Select the next function to evolve using Bayesian UCB (Upper Confidence Bound).
    ///
    /// Returns the name of the function with the highest UCB score, balancing
    /// exploitation (high mean fitness) with exploration (fewer trials).
    pub fn ucb_select_next(&self, kappa: f64) -> Option<String> {
        if self.functions.is_empty() {
            return None;
        }

        let total_trials: u64 = self.history.values()
            .map(|h| h.len() as u64)
            .sum();

        let mut best_name = None;
        let mut best_score = f64::NEG_INFINITY;

        for (name, _) in &self.functions {
            let history = self.history.get(name);
            let num_trials = history.map_or(0, |h| h.len() as u64);

            // Compute mean fitness across all variants
            let mean_fitness = history
                .map(|h| {
                    let scored: Vec<f64> = h.iter().filter_map(|v| v.fitness).collect();
                    if scored.is_empty() { 0.0 }
                    else { scored.iter().sum::<f64>() / scored.len() as f64 }
                })
                .unwrap_or(0.0);

            let score = crate::hotpath::hotpath_bayesian_ucb(
                mean_fitness, num_trials, total_trials, kappa
            );

            if score > best_score {
                best_score = score;
                best_name = Some(name.clone());
            }
        }

        best_name
    }

    /// Compute Boltzmann selection probabilities across all evolvable functions.
    ///
    /// Returns a Vec of (name, probability) pairs sorted by probability descending.
    pub fn boltzmann_probabilities(&self, temperature: f64) -> Vec<(String, f64)> {
        let names: Vec<String> = self.functions.keys().cloned().collect();
        if names.is_empty() {
            return Vec::new();
        }

        let fitnesses: Vec<f64> = names.iter()
            .map(|n| self.functions.get(n).and_then(|f| f.fitness).unwrap_or(0.0))
            .collect();

        let mut probs = vec![0.0f64; names.len()];
        unsafe {
            crate::hotpath::hotpath_boltzmann_select(
                fitnesses.as_ptr(), names.len(), temperature, probs.as_mut_ptr()
            );
        }

        let mut result: Vec<(String, f64)> = names.into_iter().zip(probs).collect();
        result.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        result
    }

    /// Compute the Shannon entropy diversity of the current fitness distribution.
    ///
    /// 1.0 = maximum diversity (uniform fitness), 0.0 = monoculture.
    pub fn fitness_diversity(&self) -> f64 {
        let fitnesses: Vec<f64> = self.functions.values()
            .filter_map(|f| f.fitness)
            .collect();
        if fitnesses.len() <= 1 {
            return 0.0;
        }
        // Normalize fitnesses to probability distribution
        let sum: f64 = fitnesses.iter().sum();
        if sum <= 0.0 {
            return 0.0;
        }
        let probs: Vec<f64> = fitnesses.iter().map(|&f| f / sum).collect();
        unsafe {
            crate::hotpath::hotpath_shannon_diversity(probs.as_ptr(), probs.len())
        }
    }

    /// Compute adaptive multi-objective fitness for a function variant.
    ///
    /// Uses generation-adaptive weighting: early generations prioritize
    /// correctness and simplicity; later generations prioritize speed and security.
    pub fn adaptive_fitness(&self, name: &str, speed: f64, correctness: f64,
                            complexity: f64, security: f64) -> f64 {
        let generation = self.get_generation(name);
        crate::hotpath::hotpath_adaptive_fitness(speed, correctness, complexity, security, generation)
    }

    /// Compute the fitness EMA (exponential moving average) across a function's history.
    ///
    /// Returns the EMA of fitness values in chronological order.
    pub fn fitness_trend(&self, name: &str, alpha: f64) -> f64 {
        let history = match self.history.get(name) {
            Some(h) => h,
            None => return 0.0,
        };
        let mut ema = 0.0;
        let mut initialized = false;
        for variant in history {
            if let Some(fitness) = variant.fitness {
                if !initialized {
                    ema = fitness;
                    initialized = true;
                } else {
                    ema = crate::hotpath::hotpath_ema_update(ema, fitness, alpha);
                }
            }
        }
        ema
    }

    /// Assemble the full program source by substituting evolved variants.
    ///
    /// Correctly handles multiple `@evolvable` functions by scanning all
    /// occurrences instead of stopping at the first match.
    fn assemble_source(&self) -> Result<String, String> {
        let base = self.base_source.as_deref().ok_or("no base source loaded")?;

        let mut source = base.to_string();
        for (name, ef) in &self.functions {
            // Scan forward through ALL @evolvable annotations to find the one
            // that precedes `fn <name>`. This fixes the multi-@evolvable bug
            // where previously only the first annotation was ever matched.
            let marker = "@evolvable";
            let fn_sig = format!("fn {}", name);
            let mut search_from = 0;
            loop {
                let remaining = &source[search_from..];
                let rel_start = match remaining.find(marker) {
                    Some(pos) => pos,
                    None => break,
                };
                let abs_start = search_from + rel_start;
                let after_annotation = &source[abs_start..];

                // Check whether *this* @evolvable is followed by fn <name>
                if let Some(fn_pos) = after_annotation.find(&fn_sig) {
                    // Make sure there's no intervening @evolvable between the
                    // annotation and fn (that would mean it belongs to a
                    // different function).
                    let gap = &after_annotation[marker.len()..fn_pos];
                    if !gap.contains(marker) {
                        let fn_body_start = after_annotation[fn_pos..].find('{');
                        if let Some(brace_start) = fn_body_start {
                            let abs_brace = abs_start + fn_pos + brace_start;
                            if let Some(end) = find_matching_brace(&source, abs_brace) {
                                let replacement = format!("@evolvable\n{}", ef.source);
                                source = format!(
                                    "{}{}{}",
                                    &source[..abs_start],
                                    replacement,
                                    &source[end + 1..]
                                );
                                break; // Substitution done for this function
                            }
                        }
                    }
                }
                // Move past this @evolvable and keep scanning
                search_from = abs_start + marker.len();
            }
        }

        Ok(source)
    }
}

// ─── Helpers ────────────────────────────────────────────────────────────

/// Hash source code for version tracking.
fn hash_source(source: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    source.hash(&mut hasher);
    hasher.finish()
}

/// Extract a function's source from a program by name.
/// Looks for `fn <name>(...) ... { ... }`.
fn extract_function_source(source: &str, name: &str) -> String {
    let pattern = format!("fn {}", name);
    if let Some(start) = source.find(&pattern) {
        if let Some(brace_start) = source[start..].find('{') {
            let abs_brace = start + brace_start;
            if let Some(end) = find_matching_brace(source, abs_brace) {
                return source[start..=end].to_string();
            }
        }
    }
    // Fallback
    format!("fn {}() -> i64 {{ 0 }}", name)
}

/// Find the position of the matching closing brace.
fn find_matching_brace(source: &str, open_pos: usize) -> Option<usize> {
    let bytes = source.as_bytes();
    let mut depth = 0;
    for i in open_pos..bytes.len() {
        match bytes[i] {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
    }
    None
}

// ─── Global Registry (thread-local for safety) ─────────────────────────

use std::cell::RefCell;

thread_local! {
    static GLOBAL_REGISTRY: RefCell<EvolutionRegistry> = RefCell::new(EvolutionRegistry::new());
}

/// Access the global evolution registry.
pub fn with_registry<F, R>(f: F) -> R
where
    F: FnOnce(&mut EvolutionRegistry) -> R,
{
    GLOBAL_REGISTRY.with(|reg| f(&mut reg.borrow_mut()))
}

// ═══════════════════════════════════════════════════════════════════════
//  v66: Built-in Mutation Strategies
// ═══════════════════════════════════════════════════════════════════════

/// Categories of source-level mutations that can be applied to function bodies.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MutationKind {
    /// Swap an arithmetic operator for another (+ ↔ -, * ↔ /)
    SwapOperator,
    /// Replace an integer literal with a nearby value
    PerturbConstant,
    /// Duplicate a statement
    DuplicateStatement,
    /// Remove a statement (if safe)
    RemoveStatement,
    /// Swap two adjacent statements
    SwapStatements,
    /// Negate a boolean condition
    NegateCondition,
}

/// A concrete mutation applied to source code, with undo information.
#[derive(Debug, Clone)]
pub struct Mutation {
    pub kind: MutationKind,
    pub description: String,
    pub original_fragment: String,
    pub mutated_fragment: String,
}

/// Deterministic PRNG for reproducible mutations (xorshift64).
pub struct MutationRng {
    state: u64,
}

impl MutationRng {
    pub fn new(seed: u64) -> Self {
        Self { state: if seed == 0 { 0xDEADBEEF } else { seed } }
    }

    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }

    pub fn next_usize(&mut self, bound: usize) -> usize {
        if bound == 0 { return 0; }
        (self.next_u64() % bound as u64) as usize
    }

    pub fn next_f64(&mut self) -> f64 {
        (self.next_u64() & 0x1FFFFFFFFFFFFF) as f64 / (1u64 << 53) as f64
    }
}

/// Apply a random mutation to function source code.
/// Returns the mutated source and a description of the mutation.
pub fn mutate_source(source: &str, rng: &mut MutationRng) -> (String, Mutation) {
    let strategies: &[fn(&str, &mut MutationRng) -> Option<(String, Mutation)>] = &[
        mutate_swap_operator,
        mutate_perturb_constant,
        mutate_swap_statements,
        mutate_negate_condition,
    ];

    // Try each strategy in random order until one succeeds
    let start = rng.next_usize(strategies.len());
    for i in 0..strategies.len() {
        let idx = (start + i) % strategies.len();
        if let Some(result) = strategies[idx](source, rng) {
            return result;
        }
    }

    // Fallback: identity mutation
    (source.to_string(), Mutation {
        kind: MutationKind::PerturbConstant,
        description: "no mutation applied (source too simple)".to_string(),
        original_fragment: String::new(),
        mutated_fragment: String::new(),
    })
}

/// Swap an arithmetic operator: + ↔ -, * ↔ /
fn mutate_swap_operator(source: &str, rng: &mut MutationRng) -> Option<(String, Mutation)> {
    let swaps: &[(&str, &str)] = &[
        (" + ", " - "), (" - ", " + "),
        (" * ", " / "), (" / ", " * "),
        (" > ", " < "), (" < ", " > "),
        (" >= ", " <= "), (" <= ", " >= "),
    ];

    // Find all swap-applicable positions
    let mut candidates: Vec<(usize, &str, &str)> = Vec::new();
    for &(from, to) in swaps {
        let mut search_from = 0;
        while let Some(pos) = source[search_from..].find(from) {
            candidates.push((search_from + pos, from, to));
            search_from += pos + from.len();
        }
    }

    if candidates.is_empty() { return None; }

    let (pos, from, to) = candidates[rng.next_usize(candidates.len())];
    let mut result = String::with_capacity(source.len());
    result.push_str(&source[..pos]);
    result.push_str(to);
    result.push_str(&source[pos + from.len()..]);

    Some((result, Mutation {
        kind: MutationKind::SwapOperator,
        description: format!("swapped '{}' → '{}'", from.trim(), to.trim()),
        original_fragment: from.to_string(),
        mutated_fragment: to.to_string(),
    }))
}

/// Perturb an integer constant by ±1..5
fn mutate_perturb_constant(source: &str, rng: &mut MutationRng) -> Option<(String, Mutation)> {
    // Find integer literals (sequences of digits not part of identifiers)
    let bytes = source.as_bytes();
    let mut candidates: Vec<(usize, usize)> = Vec::new(); // (start, end)
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i].is_ascii_digit() {
            // Check not preceded by alphanumeric or underscore (part of identifier)
            if i > 0 && (bytes[i-1].is_ascii_alphanumeric() || bytes[i-1] == b'_') {
                i += 1;
                continue;
            }
            let start = i;
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                i += 1;
            }
            // Check not followed by alphanumeric or underscore
            if i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                continue;
            }
            candidates.push((start, i));
        } else {
            i += 1;
        }
    }

    if candidates.is_empty() { return None; }

    let (start, end) = candidates[rng.next_usize(candidates.len())];
    let num_str = &source[start..end];
    let num: i64 = num_str.parse().ok()?;
    let delta = (rng.next_usize(5) as i64 + 1) * if rng.next_u64() % 2 == 0 { 1 } else { -1 };
    let new_num = num.saturating_add(delta);
    let new_str = new_num.to_string();

    let mut result = String::with_capacity(source.len());
    result.push_str(&source[..start]);
    result.push_str(&new_str);
    result.push_str(&source[end..]);

    Some((result, Mutation {
        kind: MutationKind::PerturbConstant,
        description: format!("perturbed {} → {} (delta {})", num, new_num, delta),
        original_fragment: num_str.to_string(),
        mutated_fragment: new_str,
    }))
}

/// Swap two adjacent statements in the function body
fn mutate_swap_statements(source: &str, rng: &mut MutationRng) -> Option<(String, Mutation)> {
    // Split body lines (inside { ... })
    let open = source.find('{')?;
    let close = find_matching_brace(source, open)?;
    let body = &source[open+1..close];
    let lines: Vec<&str> = body.lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect();

    if lines.len() < 2 { return None; }

    let idx = rng.next_usize(lines.len() - 1);
    let mut new_lines: Vec<&str> = lines.clone();
    new_lines.swap(idx, idx + 1);

    let new_body = new_lines.iter()
        .map(|l| format!("    {}", l))
        .collect::<Vec<_>>()
        .join("\n");

    let mut result = String::with_capacity(source.len());
    result.push_str(&source[..open+1]);
    result.push('\n');
    result.push_str(&new_body);
    result.push('\n');
    result.push_str(&source[close..]);

    Some((result, Mutation {
        kind: MutationKind::SwapStatements,
        description: format!("swapped statements at lines {} and {}", idx, idx+1),
        original_fragment: lines[idx].to_string(),
        mutated_fragment: lines[idx+1].to_string(),
    }))
}

/// Negate a boolean/comparison condition
fn mutate_negate_condition(source: &str, rng: &mut MutationRng) -> Option<(String, Mutation)> {
    let negations: &[(&str, &str)] = &[
        ("== ", "!= "), ("!= ", "== "),
        ("true", "false"), ("false", "true"),
    ];

    let mut candidates: Vec<(usize, &str, &str)> = Vec::new();
    for &(from, to) in negations {
        let mut search_from = 0;
        while let Some(pos) = source[search_from..].find(from) {
            candidates.push((search_from + pos, from, to));
            search_from += pos + from.len();
        }
    }

    if candidates.is_empty() { return None; }

    let (pos, from, to) = candidates[rng.next_usize(candidates.len())];
    let mut result = String::with_capacity(source.len());
    result.push_str(&source[..pos]);
    result.push_str(to);
    result.push_str(&source[pos + from.len()..]);

    Some((result, Mutation {
        kind: MutationKind::NegateCondition,
        description: format!("negated '{}' → '{}'", from, to),
        original_fragment: from.to_string(),
        mutated_fragment: to.to_string(),
    }))
}

/// Crossover: combine body parts of two function variants.
/// Takes the first half from parent_a and second half from parent_b.
pub fn crossover_sources(parent_a: &str, parent_b: &str) -> Option<String> {
    let open_a = parent_a.find('{')?;
    let close_a = find_matching_brace(parent_a, open_a)?;
    let open_b = parent_b.find('{')?;
    let close_b = find_matching_brace(parent_b, open_b)?;

    let body_a = &parent_a[open_a+1..close_a];
    let body_b = &parent_b[open_b+1..close_b];

    let lines_a: Vec<&str> = body_a.lines().filter(|l| !l.trim().is_empty()).collect();
    let lines_b: Vec<&str> = body_b.lines().filter(|l| !l.trim().is_empty()).collect();

    let mid_a = lines_a.len() / 2;
    let mid_b = lines_b.len() / 2;

    let mut child_lines: Vec<&str> = Vec::new();
    child_lines.extend_from_slice(&lines_a[..mid_a.min(lines_a.len())]);
    child_lines.extend_from_slice(&lines_b[mid_b.min(lines_b.len())..]);

    // Use parent_a's signature
    let sig = &parent_a[..open_a];
    let child_body = child_lines.join("\n");
    Some(format!("{}{{\n{}\n}}", sig, child_body))
}

// Add mutation methods to the registry
impl EvolutionRegistry {
    /// Apply a random mutation to a registered function and evolve it.
    pub fn mutate_and_evolve(&mut self, name: &str, seed: u64) -> Result<(u64, u64, Mutation), String> {
        let source = self.get_source(name)
            .ok_or_else(|| format!("function '{}' not registered", name))?
            .to_string();
        let mut rng = MutationRng::new(seed);
        let (mutated, mutation) = mutate_source(&source, &mut rng);
        let (generation, hash) = self.evolve(name, &mutated)?;
        Ok((generation, hash, mutation))
    }

    /// Run N mutation-evaluation cycles on a function.
    /// Returns the best fitness achieved and the generation that produced it.
    pub fn evolve_n_cycles(&mut self, name: &str, cycles: usize, seed: u64,
                           fitness_fn: impl Fn(i64) -> f64) -> Result<(f64, u64), String> {
        let mut rng = MutationRng::new(seed);
        let mut best_fitness = f64::NEG_INFINITY;
        let mut best_gen = 0u64;

        for _ in 0..cycles {
            let cycle_seed = rng.next_u64();
            let (generation, _hash, _mutation) = self.mutate_and_evolve(name, cycle_seed)?;

            match self.evaluate_variant(name, |r| fitness_fn(r)) {
                Ok(result) => {
                    if let Some(fitness) = self.get_fitness(name) {
                        if fitness > best_fitness {
                            best_fitness = fitness;
                            best_gen = generation;
                        } else {
                            // Revert to previous best if fitness declined
                            let _ = self.rollback(name, best_gen);
                        }
                    }
                    if result.error.is_some() {
                        // Compilation failed — rollback
                        let _ = self.rollback(name, best_gen);
                    }
                }
                Err(_) => {
                    let _ = self.rollback(name, best_gen);
                }
            }
        }

        Ok((best_fitness, best_gen))
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_and_list() {
        let mut reg = EvolutionRegistry::new();
        reg.register("fitness", "fn fitness(x: i64) -> i64 { x * 2 }");
        reg.register("mutate", "fn mutate(x: i64) -> i64 { x + 1 }");
        let names = reg.list_evolvable();
        assert_eq!(names.len(), 2);
    }

    #[test]
    fn test_evolve_increments_generation() {
        let mut reg = EvolutionRegistry::new();
        reg.register("score", "fn score(x: i64) -> i64 { x }");
        assert_eq!(reg.get_generation("score"), 0);

        reg.evolve("score", "fn score(x: i64) -> i64 { x * 2 }").unwrap();
        assert_eq!(reg.get_generation("score"), 1);

        reg.evolve("score", "fn score(x: i64) -> i64 { x * 3 }").unwrap();
        assert_eq!(reg.get_generation("score"), 2);
    }

    #[test]
    fn test_fitness_tracking() {
        let mut reg = EvolutionRegistry::new();
        reg.register("eval", "fn eval(x: i64) -> i64 { x }");
        assert_eq!(reg.get_fitness("eval"), None);

        reg.set_fitness("eval", 0.95);
        assert_eq!(reg.get_fitness("eval"), Some(0.95));
    }

    #[test]
    fn test_hash_changes_on_evolve() {
        let mut reg = EvolutionRegistry::new();
        reg.register("f", "fn f() -> i64 { 1 }");
        let hash1 = reg.functions.get("f").unwrap().hash;

        reg.evolve("f", "fn f() -> i64 { 2 }").unwrap();
        let hash2 = reg.functions.get("f").unwrap().hash;

        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_history() {
        let mut reg = EvolutionRegistry::new();
        reg.register("g", "fn g() -> i64 { 0 }");
        reg.evolve("g", "fn g() -> i64 { 1 }").unwrap();
        reg.evolve("g", "fn g() -> i64 { 2 }").unwrap();

        let history = reg.get_history("g");
        assert_eq!(history.len(), 3);
        assert_eq!(history[0].generation, 0);
        assert_eq!(history[2].generation, 2);
    }

    #[test]
    fn test_rollback() {
        let mut reg = EvolutionRegistry::new();
        reg.register("h", "fn h() -> i64 { 10 }");
        reg.evolve("h", "fn h() -> i64 { 20 }").unwrap();
        reg.evolve("h", "fn h() -> i64 { 30 }").unwrap();

        assert_eq!(reg.get_generation("h"), 2);
        reg.rollback("h", 0).unwrap();
        assert_eq!(reg.get_source("h"), Some("fn h() -> i64 { 10 }"));
    }

    #[test]
    fn test_best_variant() {
        let mut reg = EvolutionRegistry::new();
        reg.register("opt", "fn opt() -> i64 { 1 }");
        reg.set_fitness("opt", 0.5);

        reg.evolve("opt", "fn opt() -> i64 { 2 }").unwrap();
        reg.set_fitness("opt", 0.9);

        reg.evolve("opt", "fn opt() -> i64 { 3 }").unwrap();
        reg.set_fitness("opt", 0.7);

        let best = reg.best_variant("opt").unwrap();
        assert_eq!(best.generation, 1);
        assert_eq!(best.fitness, Some(0.9));
    }

    #[test]
    fn test_compile_and_run_simple() {
        let mut reg = EvolutionRegistry::new();
        reg.base_source = Some("fn main() -> i64 { 42 }".to_string());
        let result = reg.compile_and_run().unwrap();
        assert_eq!(result, 42);
    }

    #[test]
    fn test_load_program() {
        let source = r#"
@evolvable
fn scorer(x: i64) -> i64 {
    x * 2
}

fn main() -> i64 {
    scorer(21)
}
"#;
        let mut reg = EvolutionRegistry::new();
        reg.load_program(source);

        let names = reg.list_evolvable();
        assert_eq!(names.len(), 1);
        assert!(names.contains(&"scorer".to_string()));
        assert_eq!(reg.get_generation("scorer"), 0);
    }

    // ─── Quantum-Inspired Selection Tests ──────────────────────────────

    #[test]
    fn test_ucb_select_untried() {
        let mut reg = EvolutionRegistry::new();
        reg.register("a", "fn a() -> i64 { 1 }");
        reg.set_fitness("a", 0.9);
        reg.register("b", "fn b() -> i64 { 2 }");
        // b has no fitness → 0 mean, but only 1 trial → gets exploration bonus
        // Both have 1 trial (the initial register). UCB prefers higher mean.
        // With kappa=1.414, a should win due to higher fitness
        let next = reg.ucb_select_next(1.414);
        assert!(next.is_some());
    }

    #[test]
    fn test_ucb_select_exploration() {
        let mut reg = EvolutionRegistry::new();
        reg.register("explored", "fn explored() -> i64 { 1 }");
        reg.set_fitness("explored", 0.5);
        // Evolve 'explored' many times
        for i in 0..20 {
            reg.evolve("explored", &format!("fn explored() -> i64 {{ {} }}", i)).unwrap();
            reg.set_fitness("explored", 0.5);
        }
        reg.register("fresh", "fn fresh() -> i64 { 2 }");
        reg.set_fitness("fresh", 0.4); // Lower fitness but much fewer trials

        let next = reg.ucb_select_next(2.0); // high kappa = more exploration
        assert_eq!(next, Some("fresh".to_string()));
    }

    #[test]
    fn test_boltzmann_probabilities() {
        let mut reg = EvolutionRegistry::new();
        reg.register("low", "fn low() -> i64 { 1 }");
        reg.set_fitness("low", 0.2);
        reg.register("high", "fn high() -> i64 { 2 }");
        reg.set_fitness("high", 0.9);

        let probs = reg.boltzmann_probabilities(1.0);
        assert_eq!(probs.len(), 2);
        // "high" should have higher probability
        let high_prob = probs.iter().find(|(n, _)| n == "high").unwrap().1;
        let low_prob = probs.iter().find(|(n, _)| n == "low").unwrap().1;
        assert!(high_prob > low_prob);
        // Probabilities should roughly sum to 1.0
        let sum: f64 = probs.iter().map(|(_, p)| p).sum();
        assert!((sum - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_fitness_diversity() {
        let mut reg = EvolutionRegistry::new();
        reg.register("a", "fn a() -> i64 { 1 }");
        reg.set_fitness("a", 0.5);
        reg.register("b", "fn b() -> i64 { 2 }");
        reg.set_fitness("b", 0.5);
        reg.register("c", "fn c() -> i64 { 3 }");
        reg.set_fitness("c", 0.5);

        // All same fitness → uniform distribution → max diversity
        let diversity = reg.fitness_diversity();
        assert!((diversity - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_fitness_trend() {
        let mut reg = EvolutionRegistry::new();
        reg.register("t", "fn t() -> i64 { 1 }");
        reg.set_fitness("t", 0.3);
        reg.evolve("t", "fn t() -> i64 { 2 }").unwrap();
        reg.set_fitness("t", 0.6);
        reg.evolve("t", "fn t() -> i64 { 3 }").unwrap();
        reg.set_fitness("t", 0.9);

        let trend = reg.fitness_trend("t", 0.5);
        // EMA(0.3, 0.6, 0.5) = 0.5*0.6 + 0.5*0.3 = 0.45
        // EMA(0.45, 0.9, 0.5) = 0.5*0.9 + 0.5*0.45 = 0.675
        assert!((trend - 0.675).abs() < 0.01);
    }

    #[test]
    fn test_adaptive_fitness_early() {
        let mut reg = EvolutionRegistry::new();
        reg.register("af", "fn af() -> i64 { 1 }");
        // Generation 0 → early weights
        let score = reg.adaptive_fitness("af", 0.5, 1.0, 1.0, 0.5);
        // correctness(0.4*1.0) + complexity(0.3*1.0) + speed(0.2*0.5) + security(0.1*0.5) = 0.85
        assert!((score - 0.85).abs() < 0.01);
    }

    #[test]
    fn test_assemble_source_multi_evolvable() {
        // Regression test: assemble_source must substitute ALL @evolvable
        // functions, not just the first one.
        let mut reg = EvolutionRegistry::new();
        let base = concat!(
            "@evolvable\n",
            "fn alpha(x: i64) -> i64 { x }\n\n",
            "@evolvable\n",
            "fn beta(x: i64) -> i64 { x + 1 }\n\n",
            "fn main() -> i64 { alpha(1) + beta(2) }\n"
        );
        reg.load_program(base);

        // Evolve both functions
        reg.evolve("alpha", "fn alpha(x: i64) -> i64 { x * 10 }").unwrap();
        reg.evolve("beta", "fn beta(x: i64) -> i64 { x * 20 }").unwrap();

        let assembled = reg.assemble_source().unwrap();
        // Both substitutions must appear
        assert!(assembled.contains("x * 10"), "alpha was not substituted: {}", assembled);
        assert!(assembled.contains("x * 20"), "beta was not substituted: {}", assembled);
        // Original bodies must be gone
        assert!(!assembled.contains("{ x }"), "original alpha body still present");
        assert!(!assembled.contains("x + 1"), "original beta body still present");
    }

    // ─── v66: Mutation Strategy Tests ──────────────────────────────────

    #[test]
    fn test_mutation_rng_deterministic() {
        let mut rng1 = MutationRng::new(42);
        let mut rng2 = MutationRng::new(42);
        for _ in 0..100 {
            assert_eq!(rng1.next_u64(), rng2.next_u64());
        }
    }

    #[test]
    fn test_mutation_rng_bounded() {
        let mut rng = MutationRng::new(123);
        for _ in 0..100 {
            let v = rng.next_usize(10);
            assert!(v < 10);
        }
    }

    #[test]
    fn test_mutate_swap_operator() {
        let src = "fn f(x: i64) -> i64 { x + 1 }";
        let mut rng = MutationRng::new(1);
        let result = mutate_swap_operator(src, &mut rng);
        assert!(result.is_some());
        let (mutated, mutation) = result.unwrap();
        assert_eq!(mutation.kind, MutationKind::SwapOperator);
        // Should have swapped + to -
        assert!(mutated.contains(" - ") || mutated.contains(" * ") || mutated.contains(" / "),
            "operator not swapped: {}", mutated);
    }

    #[test]
    fn test_mutate_perturb_constant() {
        let src = "fn f() -> i64 { 42 }";
        let mut rng = MutationRng::new(7);
        let result = mutate_perturb_constant(src, &mut rng);
        assert!(result.is_some());
        let (mutated, mutation) = result.unwrap();
        assert_eq!(mutation.kind, MutationKind::PerturbConstant);
        assert!(!mutated.contains("42"), "constant not perturbed: {}", mutated);
    }

    #[test]
    fn test_mutate_negate_condition() {
        let src = "fn f(x: i64) -> i64 { if x == 0 { 1 } else { 0 } }";
        let mut rng = MutationRng::new(3);
        let result = mutate_negate_condition(src, &mut rng);
        assert!(result.is_some());
        let (mutated, mutation) = result.unwrap();
        assert_eq!(mutation.kind, MutationKind::NegateCondition);
        assert!(mutated.contains("!= "), "condition not negated: {}", mutated);
    }

    #[test]
    fn test_mutate_source_always_returns() {
        let src = "fn f() -> i64 { 1 }";
        let mut rng = MutationRng::new(99);
        let (_mutated, mutation) = mutate_source(src, &mut rng);
        // Should always return something (even identity mutation)
        assert!(!mutation.description.is_empty());
    }

    #[test]
    fn test_crossover_sources() {
        let a = "fn f(x: i64) -> i64 {\n    let a = x + 1\n    let b = a * 2\n    b\n}";
        let b = "fn f(x: i64) -> i64 {\n    let c = x - 1\n    let d = c / 2\n    d\n}";
        let child = crossover_sources(a, b);
        assert!(child.is_some());
        let child = child.unwrap();
        assert!(child.starts_with("fn f(x: i64) -> i64 "));
    }

    #[test]
    fn test_mutate_and_evolve() {
        let mut reg = EvolutionRegistry::new();
        reg.register("m", "fn m(x: i64) -> i64 { x + 1 }");
        let result = reg.mutate_and_evolve("m", 42);
        assert!(result.is_ok());
        let (generation, _hash, mutation) = result.unwrap();
        assert_eq!(generation, 1);
        assert!(!mutation.description.is_empty());
    }

    #[test]
    fn test_mutation_preserves_function_name() {
        let src = "fn compute(x: i64) -> i64 { x * 2 + 1 }";
        let mut rng = MutationRng::new(77);
        let (mutated, _) = mutate_source(src, &mut rng);
        assert!(mutated.contains("fn compute("));
    }
}
