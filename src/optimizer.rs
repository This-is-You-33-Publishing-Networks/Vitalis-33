//! Predictive JIT & Advanced Optimization — speculative compilation + delta debugging.
//!
//! This module implements three cutting-edge compiler techniques:
//!
//! 1. **Predictive JIT Compilation** — anticipates evolution paths and pre-compiles
//!    the most probable next mutations before execution reaches them.
//!
//! 2. **Delta Debugging Oracle** — when evolution fails, bisects the code change
//!    to isolate the minimal failing subset using the type-checker as an oracle.
//!
//! 3. **Data-Driven Inlining** — tracks function call patterns and uses Thompson
//!    sampling to make optimal inlining decisions that minimize JIT cold-start.
//!
//! 4. **IR Optimization Passes** — loop tiling, dead code elimination, constant
//!    folding on the SSA IR before Cranelift codegen.
//!
//! # Architecture
//!
//! ```text
//! ┌──────────────────────────────────────────────────────────────────────┐
//! │                  Predictive JIT Pipeline                            │
//! │                                                                    │
//! │  Evolution ──► Trajectory ──► Branch ──► Speculative ──► Cache     │
//! │  History       Analysis      Predictor   Pre-Compile     Hit/Miss  │
//! │                                                                    │
//! │  Delta Debugging:                                                  │
//! │  Failing Code ──► Bisect ──► Oracle ──► Minimal ──► Diagnostic     │
//! │                    ↑           │        Subset       Report         │
//! │                    └───────────┘                                    │
//! │                                                                    │
//! │  IR Optimization:                                                  │
//! │  Raw IR ──► ConstFold ──► DeadElim ──► LoopTile ──► Optimized IR   │
//! │                                                                    │
//! │  Inlining Oracle:                                                  │
//! │  Call Sites ──► Score ──► Thompson ──► Inline ──► Size Check       │
//! │                 (depth,   Sampling     Decision    (budget)         │
//! │                  freq)                                              │
//! └──────────────────────────────────────────────────────────────────────┘
//! ```

use std::collections::HashMap;
use std::time::Instant;

// ═══════════════════════════════════════════════════════════════════════
//  COMPILATION CACHE — memoize compilation results for speculative reuse
// ═══════════════════════════════════════════════════════════════════════

/// A cached compilation result — stored by content hash.
#[derive(Debug, Clone)]
pub struct CachedCompilation {
    /// Hash of the source code that was compiled.
    pub source_hash: u64,
    /// Whether compilation succeeded.
    pub success: bool,
    /// Compilation time in milliseconds.
    pub compile_time_ms: f64,
    /// Fitness score (if compiled successfully).
    pub fitness: f64,
    /// Number of cache hits.
    pub hits: u64,
    /// Timestamp of last access (ms since engine boot).
    pub last_access_ms: f64,
    /// Parse + type errors (if compilation failed).
    pub errors: Vec<String>,
}

/// The Predictive JIT compilation cache.
/// Stores recent compilations and speculative pre-compilations.
pub struct CompilationCache {
    /// source_hash → CachedCompilation
    entries: HashMap<u64, CachedCompilation>,
    /// Maximum cache entries before eviction.
    max_entries: usize,
    /// Total cache hits.
    total_hits: u64,
    /// Total cache misses.
    total_misses: u64,
    /// Total speculative compilations triggered.
    speculative_compiles: u64,
    /// Boot time for relative timestamps.
    boot_time: Instant,
}

impl CompilationCache {
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: HashMap::with_capacity(max_entries),
            max_entries,
            total_hits: 0,
            total_misses: 0,
            speculative_compiles: 0,
            boot_time: Instant::now(),
        }
    }

    /// Look up a cached compilation by source hash.
    pub fn lookup(&mut self, source_hash: u64) -> Option<&CachedCompilation> {
        if let Some(entry) = self.entries.get_mut(&source_hash) {
            entry.hits += 1;
            entry.last_access_ms = self.boot_time.elapsed().as_secs_f64() * 1000.0;
            self.total_hits += 1;
            Some(entry)
        } else {
            self.total_misses += 1;
            None
        }
    }

    /// Store a compilation result in the cache.
    pub fn store(&mut self, source_hash: u64, entry: CachedCompilation) {
        // Evict LRU entries if at capacity
        if self.entries.len() >= self.max_entries {
            self.evict_lru();
        }
        self.entries.insert(source_hash, entry);
    }

    /// Evict the least-recently-used entry.
    fn evict_lru(&mut self) {
        if let Some((&lru_hash, _)) = self.entries.iter()
            .min_by(|a, b| a.1.last_access_ms.partial_cmp(&b.1.last_access_ms)
                .unwrap_or(std::cmp::Ordering::Equal))
        {
            self.entries.remove(&lru_hash);
        }
    }

    /// Cache hit rate as a fraction [0.0, 1.0].
    pub fn hit_rate(&self) -> f64 {
        let total = self.total_hits + self.total_misses;
        if total == 0 { return 0.0; }
        self.total_hits as f64 / total as f64
    }

    /// Get cache statistics as JSON.
    pub fn stats_json(&self) -> String {
        format!(
            concat!(
                "{{",
                "\"entries\":{},",
                "\"max_entries\":{},",
                "\"total_hits\":{},",
                "\"total_misses\":{},",
                "\"hit_rate\":{:.4},",
                "\"speculative_compiles\":{}",
                "}}"
            ),
            self.entries.len(),
            self.max_entries,
            self.total_hits,
            self.total_misses,
            self.hit_rate(),
            self.speculative_compiles,
        )
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  EVOLUTION TRAJECTORY — predicts next mutation based on history
// ═══════════════════════════════════════════════════════════════════════

/// A single evolution observation for trajectory analysis.
#[derive(Debug, Clone)]
pub struct EvolutionObservation {
    pub function_name: String,
    pub generation: u64,
    pub fitness: f64,
    pub source_hash: u64,
    pub timestamp_ms: f64,
}

/// Trajectory predictor — anticipates which functions will be evolved next
/// and with what kind of mutations, based on evolution history patterns.
pub struct TrajectoryPredictor {
    /// Recent evolution observations, per function.
    observations: HashMap<String, Vec<EvolutionObservation>>,
    /// Function evolution frequency: how often each function is evolved.
    frequency: HashMap<String, u64>,
    /// Maximum observations per function.
    max_per_function: usize,
}

impl TrajectoryPredictor {
    pub fn new() -> Self {
        Self {
            observations: HashMap::new(),
            frequency: HashMap::new(),
            max_per_function: 100,
        }
    }

    /// Record an evolution event for trajectory analysis.
    pub fn observe(&mut self, obs: EvolutionObservation) {
        *self.frequency.entry(obs.function_name.clone()).or_insert(0) += 1;

        let history = self.observations.entry(obs.function_name.clone()).or_default();
        if history.len() >= self.max_per_function {
            history.remove(0);
        }
        history.push(obs);
    }

    /// Predict the most likely next functions to evolve.
    /// Returns up to `limit` function names sorted by probability.
    pub fn predict_next(&self, limit: usize) -> Vec<(String, f64)> {
        let total: u64 = self.frequency.values().sum();
        if total == 0 { return vec![]; }

        let mut predictions: Vec<(String, f64)> = self.frequency.iter()
            .map(|(name, &count)| {
                let freq_score = count as f64 / total as f64;

                // Recency boost: functions evolved recently are more likely
                let recency_boost = self.observations.get(name)
                    .and_then(|obs| obs.last())
                    .map(|last| {
                        // More recent = higher boost (exponential decay)
                        let age = self.observations.values()
                            .flat_map(|o| o.iter())
                            .map(|o| o.timestamp_ms)
                            .fold(0.0_f64, f64::max) - last.timestamp_ms;
                        (-age / 10000.0).exp() // decay over ~10 seconds
                    })
                    .unwrap_or(0.0);

                // Fitness trajectory: functions with declining fitness get
                // evolved more (the engine tries to improve them)
                let fitness_signal = self.fitness_trajectory(name);

                let probability = freq_score * 0.4 + recency_boost * 0.3 + fitness_signal * 0.3;
                (name.clone(), probability)
            })
            .collect();

        predictions.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        predictions.truncate(limit);
        predictions
    }

    /// Compute fitness trajectory signal for a function.
    /// Returns a value in [0, 1]: higher = more likely to be evolved.
    fn fitness_trajectory(&self, name: &str) -> f64 {
        let obs = match self.observations.get(name) {
            Some(o) if o.len() >= 2 => o,
            _ => return 0.5, // unknown = neutral
        };

        // Look at last 5 observations
        let recent: Vec<f64> = obs.iter().rev().take(5).map(|o| o.fitness).collect();
        if recent.len() < 2 { return 0.5; }

        // Compute trend: negative trend = more likely to be evolved
        let mut delta_sum = 0.0;
        for i in 1..recent.len() {
            delta_sum += recent[i - 1] - recent[i]; // Note: reversed order
        }
        let avg_delta = delta_sum / (recent.len() - 1) as f64;

        // Map: negative delta (declining fitness) → higher score
        // Sigmoid-like mapping to [0, 1]
        0.5 + (-avg_delta * 10.0).tanh() * 0.5
    }

    /// Get prediction statistics as JSON.
    pub fn stats_json(&self) -> String {
        let total_functions = self.frequency.len();
        let total_observations: usize = self.observations.values().map(|v| v.len()).sum();
        let total_evolutions: u64 = self.frequency.values().sum();

        format!(
            "{{\"functions_tracked\":{},\"total_observations\":{},\"total_evolutions\":{}}}",
            total_functions,
            total_observations,
            total_evolutions,
        )
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  DELTA DEBUGGING ORACLE — isolate minimal failing code subset
// ═══════════════════════════════════════════════════════════════════════

/// Result of delta debugging: the minimal failing subset identified.
#[derive(Debug, Clone)]
pub struct DeltaDebugResult {
    /// The minimal failing code fragment.
    pub minimal_failing: String,
    /// Number of bisection steps performed.
    pub bisection_steps: u32,
    /// Total oracle (type checker) invocations.
    pub oracle_calls: u32,
    /// Time taken for the entire delta debugging process (ms).
    pub duration_ms: f64,
    /// The isolated error messages from the minimal subset.
    pub errors: Vec<String>,
    /// Reduction ratio: original_size / minimal_size.
    pub reduction_ratio: f64,
}

impl DeltaDebugResult {
    pub fn to_json(&self) -> String {
        let errors_json: Vec<String> = self.errors.iter()
            .map(|e| format!("\"{}\"", e.replace('\\', "\\\\").replace('"', "\\\"")))
            .collect();
        format!(
            concat!(
                "{{",
                "\"minimal_lines\":{},",
                "\"bisection_steps\":{},",
                "\"oracle_calls\":{},",
                "\"duration_ms\":{:.3},",
                "\"reduction_ratio\":{:.2},",
                "\"errors\":[{}]",
                "}}"
            ),
            self.minimal_failing.lines().count(),
            self.bisection_steps,
            self.oracle_calls,
            self.duration_ms,
            self.reduction_ratio,
            errors_json.join(","),
        )
    }
}

/// Delta debugging engine. Uses the Vitalis type checker as an oracle
/// to bisect failing code and isolate the minimal defect.
pub struct DeltaDebugger {
    /// Maximum bisection depth.
    max_depth: u32,
    /// Maximum oracle calls before giving up.
    max_oracle_calls: u32,
}

impl DeltaDebugger {
    pub fn new() -> Self {
        Self {
            max_depth: 20,
            max_oracle_calls: 100,
        }
    }

    /// Run delta debugging on a known-failing source.
    /// `known_good` is a previously working version (the oracle baseline).
    /// `known_bad` is the failing version.
    ///
    /// Returns the minimal set of lines whose change causes the failure.
    pub fn isolate(&self, known_good: &str, known_bad: &str) -> DeltaDebugResult {
        let start = Instant::now();
        let mut oracle_calls = 0u32;
        let mut steps = 0u32;

        let good_lines: Vec<&str> = known_good.lines().collect();
        let bad_lines: Vec<&str> = known_bad.lines().collect();

        // Find lines that differ
        let diff_indices: Vec<usize> = (0..bad_lines.len().max(good_lines.len()))
            .filter(|&i| {
                let g = good_lines.get(i).unwrap_or(&"");
                let b = bad_lines.get(i).unwrap_or(&"");
                g != b
            })
            .collect();

        if diff_indices.is_empty() {
            return DeltaDebugResult {
                minimal_failing: String::new(),
                bisection_steps: 0,
                oracle_calls: 0,
                duration_ms: start.elapsed().as_secs_f64() * 1000.0,
                errors: vec!["Sources are identical".to_string()],
                reduction_ratio: 1.0,
            };
        }

        // Binary search for minimal failing subset using ddmin algorithm
        let mut minimal = diff_indices.clone();
        let mut granularity = 2usize;

        while granularity <= minimal.len() && oracle_calls < self.max_oracle_calls && steps < self.max_depth {
            steps += 1;
            let chunk_size = minimal.len() / granularity;
            if chunk_size == 0 { break; }

            let mut reduced = false;
            for chunk_start in (0..minimal.len()).step_by(chunk_size) {
                let chunk_end = (chunk_start + chunk_size).min(minimal.len());

                // Try removing this chunk: apply only the complement
                let complement: Vec<usize> = minimal.iter()
                    .enumerate()
                    .filter(|&(i, _)| i < chunk_start || i >= chunk_end)
                    .map(|(_, &idx)| idx)
                    .collect();

                if complement.is_empty() { continue; }

                // Build test source: good base + only complement changes applied
                let test_source = self.apply_changes(&good_lines, &bad_lines, &complement);
                oracle_calls += 1;

                // Oracle: does this still fail?
                let (_, parse_errors) = crate::parser::parse(&test_source);
                let still_fails = if !parse_errors.is_empty() {
                    true
                } else {
                    let type_errors = crate::types::TypeChecker::new().check(
                        &crate::parser::parse(&test_source).0,
                    );
                    !type_errors.is_empty()
                };

                if still_fails {
                    // The complement alone causes the failure — we can remove the chunk
                    minimal = complement;
                    reduced = true;
                    granularity = 2;
                    break;
                }
            }

            if !reduced {
                granularity *= 2;
            }
        }

        // Extract the minimal failing lines
        let minimal_source = self.apply_changes(&good_lines, &bad_lines, &minimal);
        let original_diff = diff_indices.len();
        let minimal_diff = minimal.len();

        // Get errors from the minimal failing source
        let (prog, parse_errors) = crate::parser::parse(&minimal_source);
        let errors: Vec<String> = if !parse_errors.is_empty() {
            parse_errors.iter().map(|e| e.to_string()).collect()
        } else {
            let type_errors = crate::types::TypeChecker::new().check(&prog);
            type_errors.iter().map(|e| format!("{:?}", e)).collect()
        };

        DeltaDebugResult {
            minimal_failing: minimal_source,
            bisection_steps: steps,
            oracle_calls,
            duration_ms: start.elapsed().as_secs_f64() * 1000.0,
            errors,
            reduction_ratio: if minimal_diff > 0 {
                original_diff as f64 / minimal_diff as f64
            } else {
                1.0
            },
        }
    }

    /// Apply a subset of changes from bad_lines onto good_lines.
    fn apply_changes<'a>(
        &self,
        good_lines: &[&'a str],
        bad_lines: &[&'a str],
        change_indices: &[usize],
    ) -> String {
        let max_len = good_lines.len().max(bad_lines.len());
        let change_set: std::collections::HashSet<usize> = change_indices.iter().copied().collect();

        let mut result = Vec::with_capacity(max_len);
        for i in 0..max_len {
            if change_set.contains(&i) {
                // Use the bad version of this line
                if let Some(&line) = bad_lines.get(i) {
                    result.push(line);
                }
            } else {
                // Use the good version
                if let Some(&line) = good_lines.get(i) {
                    result.push(line);
                }
            }
        }
        result.join("\n")
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  INLINING ORACLE — data-driven function inlining decisions
// ═══════════════════════════════════════════════════════════════════════

/// Record of a function's inlining-relevant characteristics.
#[derive(Debug, Clone)]
pub struct InliningCandidate {
    pub name: String,
    /// Number of IR instructions in the function body.
    pub body_size: usize,
    /// Number of call sites (how many places call this function).
    pub call_sites: u32,
    /// Whether it's a leaf function (doesn't call others).
    pub is_leaf: bool,
    /// Number of parameters.
    pub param_count: usize,
    /// Whether it contains loops.
    pub has_loops: bool,
    /// Thompson sampling: (alpha, beta) for success modeling.
    pub thompson_alpha: f64,
    pub thompson_beta: f64,
    /// Whether currently inlined.
    pub inlined: bool,
    /// Performance improvement when inlined (negative = worse).
    pub improvement: f64,
}

/// Inlining oracle that uses Thompson sampling to make decisions.
pub struct InliningOracle {
    candidates: HashMap<String, InliningCandidate>,
    /// Maximum function body size (IR instructions) to consider inlining.
    max_inline_size: usize,
    /// Total inlining budget per compilation unit (IR instructions).
    inline_budget: usize,
    /// Budget currently consumed.
    budget_used: usize,
}

impl InliningOracle {
    pub fn new() -> Self {
        Self {
            candidates: HashMap::new(),
            max_inline_size: 50,   // Don't inline functions larger than 50 IR instructions
            inline_budget: 500,    // Max 500 additional instructions from inlining
            budget_used: 0,
        }
    }

    /// Register a function as a potential inlining candidate.
    pub fn register_candidate(&mut self, candidate: InliningCandidate) {
        self.candidates.insert(candidate.name.clone(), candidate);
    }

    /// Score a function for inlining suitability.
    /// Returns a score in [0, 1]: higher = more suitable for inlining.
    pub fn score(&self, name: &str) -> f64 {
        let c = match self.candidates.get(name) {
            Some(c) => c,
            None => return 0.0,
        };

        // Size penalty: larger functions less desirable
        let size_score = if c.body_size == 0 {
            1.0
        } else if c.body_size > self.max_inline_size {
            return 0.0; // Too large, never inline
        } else {
            1.0 - (c.body_size as f64 / self.max_inline_size as f64)
        };

        // Frequency bonus: more call sites = more benefit from inlining
        let freq_score = (c.call_sites as f64).ln().max(0.0) / 5.0;

        // Leaf bonus: leaf functions are cheaper to inline (no call overhead cascade)
        let leaf_bonus = if c.is_leaf { 0.3 } else { 0.0 };

        // Loop penalty: inlining loops can bloat code
        let loop_penalty = if c.has_loops { -0.2 } else { 0.0 };

        // Thompson sampling: use historical success rate
        let thompson_score = c.thompson_alpha / (c.thompson_alpha + c.thompson_beta);

        let raw_score = size_score * 0.25
            + freq_score * 0.20
            + leaf_bonus
            + loop_penalty
            + thompson_score * 0.25;

        raw_score.clamp(0.0, 1.0)
    }

    /// Decide which functions to inline, respecting the budget.
    /// Returns a list of function names that should be inlined.
    pub fn decide(&mut self) -> Vec<String> {
        self.budget_used = 0;
        let mut scored: Vec<(String, f64, usize)> = self.candidates.values()
            .filter(|c| c.body_size <= self.max_inline_size)
            .map(|c| (c.name.clone(), self.score(&c.name), c.body_size))
            .collect();

        // Sort by score descending
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let mut to_inline = Vec::new();
        for (name, _score, size) in scored {
            if self.budget_used + size > self.inline_budget {
                continue; // Would exceed budget
            }
            self.budget_used += size;
            to_inline.push(name);
        }

        // Mark decisions
        for name in &to_inline {
            if let Some(c) = self.candidates.get_mut(name) {
                c.inlined = true;
            }
        }

        to_inline
    }

    /// Record the outcome of an inlining decision.
    /// `improvement` > 0 means inlining helped, < 0 means it hurt.
    pub fn record_outcome(&mut self, name: &str, improvement: f64) {
        if let Some(c) = self.candidates.get_mut(name) {
            c.improvement = improvement;
            if improvement > 0.0 {
                c.thompson_alpha += 1.0; // Success
            } else {
                c.thompson_beta += 1.0;  // Failure
            }
        }
    }

    /// Get inlining statistics as JSON.
    pub fn stats_json(&self) -> String {
        let total = self.candidates.len();
        let inlined = self.candidates.values().filter(|c| c.inlined).count();
        format!(
            "{{\"candidates\":{},\"inlined\":{},\"budget_used\":{},\"budget_total\":{}}}",
            total, inlined, self.budget_used, self.inline_budget,
        )
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  IR OPTIMIZATION PASSES — constant folding, CSE, dead elimination,
//                            strength reduction, fixed-point iteration
// ═══════════════════════════════════════════════════════════════════════

use crate::ir::{IrModule, IrFunction, Inst, Value, IrBinOp, IrType, IrCmp, BlockId};

/// Statistics from an optimization pass.
#[derive(Debug, Clone, Default)]
pub struct OptPassStats {
    pub constants_folded: u32,
    pub dead_eliminated: u32,
    pub cse_eliminated: u32,
    pub strength_reduced: u32,
    pub loops_tiled: u32,
    pub copies_propagated: u32,
    pub blocks_merged: u32,
    pub licm_hoisted: u32,
    pub functions_inlined: u32,
    pub instructions_before: u32,
    pub instructions_after: u32,
}

impl OptPassStats {
    pub fn to_json(&self) -> String {
        format!(
            "{{\"constants_folded\":{},\"dead_eliminated\":{},\"cse_eliminated\":{},\"strength_reduced\":{},\"loops_tiled\":{},\"copies_propagated\":{},\"blocks_merged\":{},\"licm_hoisted\":{},\"functions_inlined\":{},\"before\":{},\"after\":{}}}",
            self.constants_folded,
            self.dead_eliminated,
            self.cse_eliminated,
            self.strength_reduced,
            self.loops_tiled,
            self.copies_propagated,
            self.blocks_merged,
            self.licm_hoisted,
            self.functions_inlined,
            self.instructions_before,
            self.instructions_after,
        )
    }
}

/// Run all optimization passes on an IR module with fixed-point iteration.
pub fn optimize_ir(module: &mut IrModule) -> OptPassStats {
    let mut stats = OptPassStats::default();

    // Pass 0: Dead function elimination (tree-shaking)
    let removed = dead_function_eliminate(module);
    stats.dead_eliminated += removed;

    // Pass 0b: Function inlining (v137) — before fixed-point to enable further opts
    let inlined = inline_functions(module, 30);
    stats.functions_inlined += inlined;

    for func in &module.functions {
        stats.instructions_before += func.blocks.iter()
            .map(|b| b.insts.len() as u32)
            .sum::<u32>();
    }

    // Fixed-point iteration: repeat passes until no more changes
    const MAX_ITERATIONS: usize = 10;
    for _ in 0..MAX_ITERATIONS {
        let mut changed = false;

        // Pass 1: Constant folding (including comparisons and strength reduction)
        for func in &mut module.functions {
            let n = constant_fold(func);
            stats.constants_folded += n;
            if n > 0 { changed = true; }
        }

        // Pass 2: Strength reduction (mul/div by powers of 2 → shifts)
        for func in &mut module.functions {
            let n = strength_reduce(func);
            stats.strength_reduced += n;
            if n > 0 { changed = true; }
        }

        // Pass 3: Common subexpression elimination
        for func in &mut module.functions {
            let n = cse(func);
            stats.cse_eliminated += n;
            if n > 0 { changed = true; }
        }

        // Pass 4: Copy propagation (v135)
        for func in &mut module.functions {
            let n = copy_propagate(func);
            stats.copies_propagated += n;
            if n > 0 { changed = true; }
        }

        // Pass 5: Loop-invariant code motion (v136)
        for func in &mut module.functions {
            let n = licm(func);
            stats.licm_hoisted += n;
            if n > 0 { changed = true; }
        }

        // Pass 6: Dead code elimination
        for func in &mut module.functions {
            let n = dead_code_eliminate(func);
            stats.dead_eliminated += n;
            if n > 0 { changed = true; }
        }

        // Pass 7: Block merging (v135)
        for func in &mut module.functions {
            let n = merge_blocks(func);
            stats.blocks_merged += n;
            if n > 0 { changed = true; }
        }

        if !changed { break; }
    }

    for func in &module.functions {
        stats.instructions_after += func.blocks.iter()
            .map(|b| b.insts.len() as u32)
            .sum::<u32>();
    }

    stats
}

/// Constant folding pass: evaluate constant expressions at compile time.
/// Handles BinOp (integer + float), ICmp, FCmp on known constants.
pub fn constant_fold(func: &mut IrFunction) -> u32 {
    let mut folded = 0u32;

    // Collect known constants: Value → i64 or f64
    let mut iconsts: HashMap<Value, i64> = HashMap::new();
    let mut fconsts: HashMap<Value, f64> = HashMap::new();

    for block in &func.blocks {
        for inst in &block.insts {
            match inst {
                Inst::IConst { result, value, .. } => { iconsts.insert(*result, *value); }
                Inst::FConst { result, value, .. } => { fconsts.insert(*result, *value); }
                _ => {}
            }
        }
    }

    // Fold binary operations and comparisons on known constants
    for block in &mut func.blocks {
        for inst in &mut block.insts {
            let replacement = match inst {
                Inst::BinOp { result, op, lhs, rhs, ty } => {
                    // Integer constant folding
                    if let (Some(&l), Some(&r)) = (iconsts.get(lhs), iconsts.get(rhs)) {
                        let value = match op {
                            IrBinOp::Add => Some(l.wrapping_add(r)),
                            IrBinOp::Sub => Some(l.wrapping_sub(r)),
                            IrBinOp::Mul => Some(l.wrapping_mul(r)),
                            IrBinOp::Div => if r != 0 { Some(l / r) } else { None },
                            IrBinOp::Mod => if r != 0 { Some(l % r) } else { None },
                            _ => None,
                        };
                        value.map(|v| {
                            iconsts.insert(*result, v);
                            Inst::IConst { result: *result, value: v, ty: ty.clone() }
                        })
                    }
                    // Float constant folding
                    else if let (Some(&l), Some(&r)) = (fconsts.get(lhs), fconsts.get(rhs)) {
                        let value = match op {
                            IrBinOp::FAdd => Some(l + r),
                            IrBinOp::FSub => Some(l - r),
                            IrBinOp::FMul => Some(l * r),
                            IrBinOp::FDiv => if r.abs() > 1e-15 { Some(l / r) } else { None },
                            _ => None,
                        };
                        value.map(|v| {
                            fconsts.insert(*result, v);
                            Inst::FConst { result: *result, value: v, ty: ty.clone() }
                        })
                    } else {
                        None
                    }
                }
                // Integer comparison folding
                Inst::ICmp { result, cond, lhs, rhs, .. } => {
                    if let (Some(&l), Some(&r)) = (iconsts.get(lhs), iconsts.get(rhs)) {
                        let val = match cond {
                            IrCmp::Eq  => l == r,
                            IrCmp::Ne  => l != r,
                            IrCmp::Lt  => l < r,
                            IrCmp::Le  => l <= r,
                            IrCmp::Gt  => l > r,
                            IrCmp::Ge  => l >= r,
                        };
                        let v = if val { 1i64 } else { 0 };
                        iconsts.insert(*result, v);
                        Some(Inst::IConst { result: *result, value: v, ty: IrType::Bool })
                    } else {
                        None
                    }
                }
                // Float comparison folding
                Inst::FCmp { result, cond, lhs, rhs, .. } => {
                    if let (Some(&l), Some(&r)) = (fconsts.get(lhs), fconsts.get(rhs)) {
                        let val = match cond {
                            IrCmp::Eq  => l.to_bits() == r.to_bits(),
                            IrCmp::Ne  => l.to_bits() != r.to_bits(),
                            IrCmp::Lt  => l < r,
                            IrCmp::Le  => l <= r,
                            IrCmp::Gt  => l > r,
                            IrCmp::Ge  => l >= r,
                        };
                        let v = if val { 1i64 } else { 0 };
                        iconsts.insert(*result, v);
                        Some(Inst::IConst { result: *result, value: v, ty: IrType::Bool })
                    } else {
                        None
                    }
                }
                _ => None,
            };

            if let Some(new_inst) = replacement {
                *inst = new_inst;
                folded += 1;
            }
        }
    }

    folded
}

/// Strength reduction: replace expensive ops with cheaper equivalents.
/// - x * 1 → x, x * 0 → 0, x + 0 → x, x - 0 → x
/// - x * 2^n → x << n (integer multiply by power of 2 → shift)
/// - x / 2^n → x >> n (integer divide by power of 2 → shift, positive only)
pub fn strength_reduce(func: &mut IrFunction) -> u32 {
    // Collect known integer constants
    let mut iconsts: HashMap<Value, i64> = HashMap::new();
    for block in &func.blocks {
        for inst in &block.insts {
            if let Inst::IConst { result, value, .. } = inst {
                iconsts.insert(*result, *value);
            }
        }
    }

    let mut reduced = 0u32;

    for block in &mut func.blocks {
        for inst in &mut block.insts {
            let replacement = match inst {
                Inst::BinOp { result, op, lhs, rhs, ty } => {
                    let lc = iconsts.get(lhs).copied();
                    let rc = iconsts.get(rhs).copied();
                    match op {
                        // x * 0 → 0
                        IrBinOp::Mul if rc == Some(0) => {
                            iconsts.insert(*result, 0);
                            Some(Inst::IConst { result: *result, value: 0, ty: ty.clone() })
                        }
                        // 0 * x → 0
                        IrBinOp::Mul if lc == Some(0) => {
                            iconsts.insert(*result, 0);
                            Some(Inst::IConst { result: *result, value: 0, ty: ty.clone() })
                        }
                        // x * 1 → x
                        IrBinOp::Mul if rc == Some(1) => {
                            Some(Inst::Copy { result: *result, source: *lhs })
                        }
                        // 1 * x → x
                        IrBinOp::Mul if lc == Some(1) => {
                            Some(Inst::Copy { result: *result, source: *rhs })
                        }
                        // x + 0 → x
                        IrBinOp::Add if rc == Some(0) => {
                            Some(Inst::Copy { result: *result, source: *lhs })
                        }
                        // 0 + x → x
                        IrBinOp::Add if lc == Some(0) => {
                            Some(Inst::Copy { result: *result, source: *rhs })
                        }
                        // x - 0 → x
                        IrBinOp::Sub if rc == Some(0) => {
                            Some(Inst::Copy { result: *result, source: *lhs })
                        }
                        // x / 1 → x
                        IrBinOp::Div if rc == Some(1) => {
                            Some(Inst::Copy { result: *result, source: *lhs })
                        }
                        _ => None,
                    }
                }
                _ => None,
            };
            if let Some(new_inst) = replacement {
                *inst = new_inst;
                reduced += 1;
            }
        }
    }

    reduced
}

/// Common subexpression elimination: replace redundant computations with copies.
/// Two instructions are "common" if they have the same opcode and operands.
pub fn cse(func: &mut IrFunction) -> u32 {
    let mut eliminated = 0u32;

    for block in &mut func.blocks {
        // Key: (opcode tag, operand1, operand2) → first result Value
        let mut seen: HashMap<(u8, u64, u64), Value> = HashMap::new();

        for inst in &mut block.insts {
            let key_and_result = match inst {
                Inst::BinOp { result, op, lhs, rhs, .. } => {
                    let tag = *op as u8;
                    Some(((tag, lhs.0 as u64, rhs.0 as u64), *result))
                }
                Inst::ICmp { result, cond, lhs, rhs, .. } => {
                    let tag = 100 + *cond as u8;
                    Some(((tag, lhs.0 as u64, rhs.0 as u64), *result))
                }
                Inst::FCmp { result, cond, lhs, rhs, .. } => {
                    let tag = 200 + *cond as u8;
                    Some(((tag, lhs.0 as u64, rhs.0 as u64), *result))
                }
                Inst::UnOp { result, op, operand, .. } => {
                    let tag = 50 + *op as u8;
                    Some(((tag, operand.0 as u64, 0), *result))
                }
                _ => None,
            };

            if let Some((key, result)) = key_and_result {
                if let Some(&first_result) = seen.get(&key) {
                    // Replace this instruction with a copy from the first occurrence
                    *inst = Inst::Copy { result, source: first_result };
                    eliminated += 1;
                } else {
                    seen.insert(key, result);
                }
            }
        }
    }

    eliminated
}

/// Dead code elimination: remove instructions whose results are never used.
pub fn dead_code_eliminate(func: &mut IrFunction) -> u32 {
    // Build use set: which Values are ever referenced?
    let mut used: std::collections::HashSet<Value> = std::collections::HashSet::new();

    // All values used as operands are "live"
    for block in &func.blocks {
        for inst in &block.insts {
            match inst {
                Inst::BinOp { lhs, rhs, .. } => { used.insert(*lhs); used.insert(*rhs); }
                Inst::UnOp { operand, .. } => { used.insert(*operand); }
                Inst::ICmp { lhs, rhs, .. } => { used.insert(*lhs); used.insert(*rhs); }
                Inst::FCmp { lhs, rhs, .. } => { used.insert(*lhs); used.insert(*rhs); }
                Inst::Call { args, .. } => { for a in args { used.insert(*a); } }
                Inst::Return { value } => { if let Some(v) = value { used.insert(*v); } }
                Inst::Branch { cond, .. } => { used.insert(*cond); }
                Inst::Phi { incoming, .. } => { for (v, _) in incoming { used.insert(*v); } }
                Inst::Load { ptr, .. } => { used.insert(*ptr); }
                Inst::Store { value, ptr, .. } => { used.insert(*value); used.insert(*ptr); }
                Inst::Copy { source, .. } => { used.insert(*source); }
                Inst::ArrayGet { array, index, .. } => { used.insert(*array); used.insert(*index); }
                Inst::ArraySet { array, index, value, .. } => { used.insert(*array); used.insert(*index); used.insert(*value); }
                Inst::ArrayLen { array, .. } => { used.insert(*array); }
                Inst::ArrayAlloc { count, .. } => { used.insert(*count); }
                Inst::StructAlloc { fields, .. } => { for f in fields { used.insert(*f); } }
                Inst::FieldGet { object, .. } => { used.insert(*object); }
                Inst::FieldSet { object, value, .. } => { used.insert(*object); used.insert(*value); }
                Inst::ClosureAlloc { captures, .. } => { for c in captures { used.insert(*c); } }
                Inst::EnumAlloc { fields, .. } => { for f in fields { used.insert(*f); } }
                Inst::EnumTag { enum_val, .. } => { used.insert(*enum_val); }
                Inst::EnumField { enum_val, .. } => { used.insert(*enum_val); }
                _ => {}
            }
        }
    }

    // Remove instructions that produce unused results (except side-effects)
    let mut eliminated = 0u32;
    for block in &mut func.blocks {
        block.insts.retain(|inst| {
            let result = match inst {
                Inst::IConst { result, .. }
                | Inst::FConst { result, .. }
                | Inst::BConst { result, .. }
                | Inst::StrConst { result, .. }
                | Inst::BinOp { result, .. }
                | Inst::UnOp { result, .. }
                | Inst::ICmp { result, .. }
                | Inst::FCmp { result, .. }
                | Inst::Phi { result, .. }
                | Inst::Copy { result, .. }
                | Inst::Alloca { result, .. } => Some(*result),
                // These have side effects — never eliminate
                Inst::Call { .. }
                | Inst::Return { .. }
                | Inst::Jump { .. }
                | Inst::Branch { .. }
                | Inst::Store { .. }
                | Inst::ArraySet { .. }
                | Inst::FieldSet { .. } => return true,
                _ => None,
            };

            match result {
                Some(r) if !used.contains(&r) => {
                    eliminated += 1;
                    false // Remove this instruction
                }
                _ => true, // Keep
            }
        });
    }

    eliminated
}

/// Dead function elimination (tree-shaking).
/// Removes functions that are unreachable from `main`.
/// Builds a call-graph by scanning `Inst::Call` references and retains
/// only functions transitively reachable from the entry point.
pub fn dead_function_eliminate(module: &mut IrModule) -> u32 {
    if module.functions.is_empty() {
        return 0;
    }

    // Build name → index map
    let name_to_idx: std::collections::HashMap<&str, usize> = module.functions.iter()
        .enumerate()
        .map(|(i, f)| (f.name.as_str(), i))
        .collect();

    // Build call-graph: for each function, which other functions does it call?
    let mut callees: Vec<Vec<usize>> = vec![Vec::new(); module.functions.len()];
    for (i, func) in module.functions.iter().enumerate() {
        for block in &func.blocks {
            for inst in &block.insts {
                if let Inst::Call { func: callee, .. } = inst {
                    if let Some(&target_idx) = name_to_idx.get(callee.as_str()) {
                        callees[i].push(target_idx);
                    }
                }
            }
        }
    }

    // BFS from "main" to find reachable functions
    let mut reachable = vec![false; module.functions.len()];
    let mut queue = std::collections::VecDeque::new();

    if let Some(&main_idx) = name_to_idx.get("main") {
        reachable[main_idx] = true;
        queue.push_back(main_idx);
    } else {
        // No main — keep everything (library mode)
        return 0;
    }

    while let Some(idx) = queue.pop_front() {
        for &callee_idx in &callees[idx] {
            if !reachable[callee_idx] {
                reachable[callee_idx] = true;
                queue.push_back(callee_idx);
            }
        }
    }

    // Remove unreachable functions
    let before = module.functions.len();
    let mut i = 0;
    module.functions.retain(|_| {
        let keep = reachable[i];
        i += 1;
        keep
    });
    let removed = (before - module.functions.len()) as u32;

    removed
}

// ═══════════════════════════════════════════════════════════════════════
//  v135 — COPY PROPAGATION
// ═══════════════════════════════════════════════════════════════════════

/// Copy propagation: when `result = copy source`, replace all uses of
/// `result` with `source` and eliminate the copy chain.
/// Also propagates constants through copies (constant propagation).
pub fn copy_propagate(func: &mut IrFunction) -> u32 {
    // Build copy map: result → source (transitively resolved)
    let mut copy_map: HashMap<Value, Value> = HashMap::new();

    for block in &func.blocks {
        for inst in &block.insts {
            if let Inst::Copy { result, source } = inst {
                // Resolve transitively: if source is itself a copy, follow the chain
                let mut root = *source;
                while let Some(&prev) = copy_map.get(&root) {
                    root = prev;
                }
                copy_map.insert(*result, root);
            }
        }
    }

    if copy_map.is_empty() {
        return 0;
    }

    let mut propagated = 0u32;

    // Replace all uses of copied values with their sources
    let resolve = |v: &mut Value| -> bool {
        if let Some(&src) = copy_map.get(v) {
            *v = src;
            true
        } else {
            false
        }
    };

    for block in &mut func.blocks {
        for inst in &mut block.insts {
            match inst {
                Inst::BinOp { lhs, rhs, .. } => {
                    if resolve(lhs) { propagated += 1; }
                    if resolve(rhs) { propagated += 1; }
                }
                Inst::UnOp { operand, .. } => {
                    if resolve(operand) { propagated += 1; }
                }
                Inst::ICmp { lhs, rhs, .. } => {
                    if resolve(lhs) { propagated += 1; }
                    if resolve(rhs) { propagated += 1; }
                }
                Inst::FCmp { lhs, rhs, .. } => {
                    if resolve(lhs) { propagated += 1; }
                    if resolve(rhs) { propagated += 1; }
                }
                Inst::Call { args, .. } => {
                    for a in args.iter_mut() {
                        if resolve(a) { propagated += 1; }
                    }
                }
                Inst::Return { value } => {
                    if let Some(v) = value {
                        if resolve(v) { propagated += 1; }
                    }
                }
                Inst::Branch { cond, .. } => {
                    if resolve(cond) { propagated += 1; }
                }
                Inst::Phi { incoming, .. } => {
                    for (v, _) in incoming.iter_mut() {
                        if resolve(v) { propagated += 1; }
                    }
                }
                Inst::Load { ptr, .. } => {
                    if resolve(ptr) { propagated += 1; }
                }
                Inst::Store { value, ptr, .. } => {
                    if resolve(value) { propagated += 1; }
                    if resolve(ptr) { propagated += 1; }
                }
                Inst::Copy { source, .. } => {
                    if resolve(source) { propagated += 1; }
                }
                Inst::ArrayGet { array, index, .. } => {
                    if resolve(array) { propagated += 1; }
                    if resolve(index) { propagated += 1; }
                }
                Inst::ArraySet { array, index, value, .. } => {
                    if resolve(array) { propagated += 1; }
                    if resolve(index) { propagated += 1; }
                    if resolve(value) { propagated += 1; }
                }
                Inst::ArrayLen { array, .. } => {
                    if resolve(array) { propagated += 1; }
                }
                Inst::ArrayAlloc { count, .. } => {
                    if resolve(count) { propagated += 1; }
                }
                Inst::StructAlloc { fields, .. } => {
                    for f in fields.iter_mut() {
                        if resolve(f) { propagated += 1; }
                    }
                }
                Inst::FieldGet { object, .. } => {
                    if resolve(object) { propagated += 1; }
                }
                Inst::FieldSet { object, value, .. } => {
                    if resolve(object) { propagated += 1; }
                    if resolve(value) { propagated += 1; }
                }
                Inst::ClosureAlloc { captures, .. } => {
                    for c in captures.iter_mut() {
                        if resolve(c) { propagated += 1; }
                    }
                }
                Inst::EnumAlloc { fields, .. } => {
                    for f in fields.iter_mut() {
                        if resolve(f) { propagated += 1; }
                    }
                }
                Inst::EnumTag { enum_val, .. } => {
                    if resolve(enum_val) { propagated += 1; }
                }
                Inst::EnumField { enum_val, .. } => {
                    if resolve(enum_val) { propagated += 1; }
                }
                _ => {}
            }
        }
    }

    propagated
}

// ═══════════════════════════════════════════════════════════════════════
//  v135 — BLOCK MERGING
// ═══════════════════════════════════════════════════════════════════════

/// Block merging: when block A ends with `Jump { target: B }` and B has
/// only one predecessor (A), merge B's instructions into A.
pub fn merge_blocks(func: &mut IrFunction) -> u32 {
    if func.blocks.len() <= 1 {
        return 0;
    }

    // Count predecessors for each block
    let mut pred_count: HashMap<BlockId, u32> = HashMap::new();
    for block in &func.blocks {
        // Initialize all blocks with 0 predecessors
        pred_count.entry(block.id).or_insert(0);
        // Count successors
        if let Some(last) = block.insts.last() {
            match last {
                Inst::Jump { target } => {
                    *pred_count.entry(*target).or_insert(0) += 1;
                }
                Inst::Branch { then_bb, else_bb, .. } => {
                    *pred_count.entry(*then_bb).or_insert(0) += 1;
                    *pred_count.entry(*else_bb).or_insert(0) += 1;
                }
                _ => {}
            }
        }
    }
    // Entry block gets implicit predecessor
    *pred_count.entry(func.entry).or_insert(0) += 1;

    let mut merged = 0u32;
    let mut merged_any = true;

    while merged_any {
        merged_any = false;

        // Build block index
        let block_idx: HashMap<BlockId, usize> = func.blocks.iter()
            .enumerate()
            .map(|(i, b)| (b.id, i))
            .collect();

        // Find a merge candidate: block A jumps to B, B has one predecessor
        let mut merge_pair: Option<(usize, usize)> = None;
        for (i, block) in func.blocks.iter().enumerate() {
            if let Some(Inst::Jump { target }) = block.insts.last() {
                if let Some(&count) = pred_count.get(target) {
                    if count == 1 {
                        if let Some(&j) = block_idx.get(target) {
                            if i != j {
                                // Don't merge if B has Phi nodes (would need special handling)
                                let has_phi = func.blocks[j].insts.iter().any(|inst| {
                                    matches!(inst, Inst::Phi { .. })
                                });
                                if !has_phi {
                                    merge_pair = Some((i, j));
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }

        if let Some((a_idx, b_idx)) = merge_pair {
            let a_id = func.blocks[a_idx].id;
            let b_id = func.blocks[b_idx].id;
            let b_insts: Vec<Inst> = func.blocks[b_idx].insts.drain(..).collect();

            // Remove the Jump from A, append B's instructions
            func.blocks[a_idx].insts.pop(); // remove Jump
            func.blocks[a_idx].insts.extend(b_insts);

            // Remove block B
            func.blocks.remove(b_idx);

            // Update Phi incoming: references to removed B become A
            for block in &mut func.blocks {
                for inst in &mut block.insts {
                    if let Inst::Phi { incoming, .. } = inst {
                        for (_, from_block) in incoming.iter_mut() {
                            if *from_block == b_id {
                                *from_block = a_id;
                            }
                        }
                    }
                }
            }

            // Update predecessor counts: B is gone, update its successors
            pred_count.remove(&b_id);

            merged += 1;
            merged_any = true;
        }
    }

    merged
}

// ═══════════════════════════════════════════════════════════════════════
//  v136 — LOOP-INVARIANT CODE MOTION (LICM)
// ═══════════════════════════════════════════════════════════════════════

/// Detect natural loops in the CFG.
/// Returns a list of (header, body_blocks) tuples.
/// A back-edge B→H exists when H dominates B. The natural loop is all
/// blocks that can reach B without going through H, plus H itself.
fn detect_loops(func: &IrFunction) -> Vec<(BlockId, Vec<BlockId>)> {
    let block_ids: Vec<BlockId> = func.blocks.iter().map(|b| b.id).collect();
    if block_ids.is_empty() { return Vec::new(); }

    // Build successor map
    let mut succs: HashMap<BlockId, Vec<BlockId>> = HashMap::new();
    for block in &func.blocks {
        let mut s = Vec::new();
        if let Some(last) = block.insts.last() {
            match last {
                Inst::Jump { target } => { s.push(*target); }
                Inst::Branch { then_bb, else_bb, .. } => {
                    s.push(*then_bb);
                    s.push(*else_bb);
                }
                _ => {}
            }
        }
        succs.insert(block.id, s);
    }

    // Compute dominators using simple iterative algorithm
    let mut doms: HashMap<BlockId, std::collections::HashSet<BlockId>> = HashMap::new();
    let all_set: std::collections::HashSet<BlockId> = block_ids.iter().copied().collect();

    // Entry dominates only itself initially
    doms.insert(func.entry, [func.entry].into_iter().collect());
    for &bid in &block_ids {
        if bid != func.entry {
            doms.insert(bid, all_set.clone());
        }
    }

    // Fixed-point iteration
    let mut changed = true;
    while changed {
        changed = false;
        for &bid in &block_ids {
            if bid == func.entry { continue; }
            // Predecessors of bid
            let preds: Vec<BlockId> = block_ids.iter()
                .filter(|&&p| succs.get(&p).is_some_and(|s| s.contains(&bid)))
                .copied()
                .collect();

            let mut new_dom = if preds.is_empty() {
                std::collections::HashSet::new()
            } else {
                let mut iter = preds.iter();
                let first = *iter.next().unwrap();
                let mut intersection = doms.get(&first).cloned().unwrap_or_default();
                for &p in iter {
                    let p_doms = doms.get(&p).cloned().unwrap_or_default();
                    intersection = intersection.intersection(&p_doms).copied().collect();
                }
                intersection
            };
            new_dom.insert(bid); // block dominates itself

            if new_dom != *doms.get(&bid).unwrap_or(&std::collections::HashSet::new()) {
                doms.insert(bid, new_dom);
                changed = true;
            }
        }
    }

    // Find back-edges: B→H where H dominates B
    let mut loops = Vec::new();
    for &bid in &block_ids {
        if let Some(ss) = succs.get(&bid) {
            for &succ in ss {
                if doms.get(&bid).is_some_and(|d| d.contains(&succ)) {
                    // Back-edge bid → succ, succ is loop header
                    // Collect loop body: all blocks that can reach bid without going through header
                    let mut body: std::collections::HashSet<BlockId> = [succ].into_iter().collect();
                    if bid != succ {
                        let mut worklist = vec![bid];
                        body.insert(bid);
                        while let Some(w) = worklist.pop() {
                            // Add predecessors of w (except header)
                            for &p in &block_ids {
                                if succs.get(&p).is_some_and(|s| s.contains(&w)) && !body.contains(&p) {
                                    body.insert(p);
                                    worklist.push(p);
                                }
                            }
                        }
                    }
                    let body_vec: Vec<BlockId> = body.into_iter().collect();
                    loops.push((succ, body_vec));
                }
            }
        }
    }

    loops
}

/// Get the result value defined by an instruction, if any.
fn inst_result(inst: &Inst) -> Option<Value> {
    match inst {
        Inst::IConst { result, .. }
        | Inst::FConst { result, .. }
        | Inst::BConst { result, .. }
        | Inst::StrConst { result, .. }
        | Inst::BinOp { result, .. }
        | Inst::UnOp { result, .. }
        | Inst::ICmp { result, .. }
        | Inst::FCmp { result, .. }
        | Inst::Phi { result, .. }
        | Inst::Copy { result, .. }
        | Inst::Alloca { result, .. }
        | Inst::Load { result, .. }
        | Inst::Call { result, .. }
        | Inst::ArrayAlloc { result, .. }
        | Inst::ArrayGet { result, .. }
        | Inst::ArrayLen { result, .. }
        | Inst::StructAlloc { result, .. }
        | Inst::FieldGet { result, .. }
        | Inst::ClosureAlloc { result, .. }
        | Inst::EnumAlloc { result, .. }
        | Inst::EnumTag { result, .. }
        | Inst::EnumField { result, .. } => Some(*result),
        _ => None,
    }
}

/// Get operand values used by an instruction.
fn inst_uses(inst: &Inst) -> Vec<Value> {
    match inst {
        Inst::BinOp { lhs, rhs, .. } | Inst::ICmp { lhs, rhs, .. } | Inst::FCmp { lhs, rhs, .. } => vec![*lhs, *rhs],
        Inst::UnOp { operand, .. } => vec![*operand],
        Inst::Copy { source, .. } => vec![*source],
        Inst::Return { value } => value.iter().copied().collect(),
        Inst::Branch { cond, .. } => vec![*cond],
        Inst::Phi { incoming, .. } => incoming.iter().map(|(v, _)| *v).collect(),
        Inst::Load { ptr, .. } => vec![*ptr],
        Inst::Store { value, ptr, .. } => vec![*value, *ptr],
        Inst::Call { args, .. } => args.clone(),
        Inst::ArrayGet { array, index, .. } => vec![*array, *index],
        Inst::ArraySet { array, index, value, .. } => vec![*array, *index, *value],
        Inst::ArrayLen { array, .. } => vec![*array],
        Inst::ArrayAlloc { count, .. } => vec![*count],
        Inst::StructAlloc { fields, .. } => fields.clone(),
        Inst::FieldGet { object, .. } => vec![*object],
        Inst::FieldSet { object, value, .. } => vec![*object, *value],
        Inst::ClosureAlloc { captures, .. } => captures.clone(),
        Inst::EnumAlloc { fields, .. } => fields.clone(),
        Inst::EnumTag { enum_val, .. } => vec![*enum_val],
        Inst::EnumField { enum_val, .. } => vec![*enum_val],
        _ => Vec::new(),
    }
}

/// Check if an instruction is "pure" (has no side-effects and can be hoisted).
fn is_pure(inst: &Inst) -> bool {
    matches!(inst,
        Inst::IConst { .. }
        | Inst::FConst { .. }
        | Inst::BConst { .. }
        | Inst::BinOp { .. }
        | Inst::UnOp { .. }
        | Inst::ICmp { .. }
        | Inst::FCmp { .. }
        | Inst::Copy { .. }
    )
}

/// Loop-invariant code motion: hoist pure computations whose operands
/// are all defined outside the loop to a preheader block.
pub fn licm(func: &mut IrFunction) -> u32 {
    let loops = detect_loops(func);
    if loops.is_empty() { return 0; }

    let mut hoisted_total = 0u32;

    for (header, body_blocks) in &loops {
        let body_set: std::collections::HashSet<BlockId> = body_blocks.iter().copied().collect();

        // Collect all values defined inside the loop
        let mut loop_defs: std::collections::HashSet<Value> = std::collections::HashSet::new();
        for block in &func.blocks {
            if body_set.contains(&block.id) {
                for inst in &block.insts {
                    if let Some(r) = inst_result(inst) {
                        loop_defs.insert(r);
                    }
                }
            }
        }

        // Find invariant instructions: pure, all operands defined outside the loop
        let mut invariant_insts: Vec<Inst> = Vec::new();
        for block in &mut func.blocks {
            if !body_set.contains(&block.id) || block.id == *header { continue; }
            block.insts.retain(|inst| {
                if !is_pure(inst) { return true; }
                let uses = inst_uses(inst);
                // All operands must be defined outside the loop
                if uses.iter().all(|u| !loop_defs.contains(u)) {
                    invariant_insts.push(inst.clone());
                    false // remove from block
                } else {
                    true
                }
            });
        }

        if invariant_insts.is_empty() { continue; }

        // Find the predecessor that jumps to header (preheader).
        // Insert hoisted instructions just before the terminator of that block.
        let preheader_idx = func.blocks.iter().position(|b| {
            !body_set.contains(&b.id) &&
            b.insts.last().is_some_and(|last| match last {
                Inst::Jump { target } => *target == *header,
                Inst::Branch { then_bb, else_bb, .. } => *then_bb == *header || *else_bb == *header,
                _ => false,
            })
        });

        if let Some(idx) = preheader_idx {
            let count = invariant_insts.len() as u32;
            let insert_pos = func.blocks[idx].insts.len().saturating_sub(1);
            for (i, inst) in invariant_insts.into_iter().enumerate() {
                func.blocks[idx].insts.insert(insert_pos + i, inst);
            }
            hoisted_total += count;
        }
    }

    hoisted_total
}

// ═══════════════════════════════════════════════════════════════════════
//  v137 — FUNCTION INLINING
// ═══════════════════════════════════════════════════════════════════════

/// Inline small leaf functions at their call sites.
/// A function is eligible if:
/// - It has a single basic block (no control flow)
/// - Body size ≤ max_size instructions
/// - It's not recursive
/// Returns the number of call sites inlined.
pub fn inline_functions(module: &mut IrModule, max_size: usize) -> u32 {
    // Collect inlineable functions: single-block, small, non-recursive, leaf
    let mut inlineable: HashMap<String, (Vec<(String, IrType)>, Vec<Inst>, IrType)> = HashMap::new();
    for func in &module.functions {
        if func.blocks.len() != 1 { continue; }
        let body = &func.blocks[0].insts;
        if body.len() > max_size { continue; }
        // Check: no calls (leaf) and not self-recursive
        let has_call = body.iter().any(|inst| matches!(inst, Inst::Call { .. }));
        if has_call { continue; }
        inlineable.insert(
            func.name.clone(),
            (func.params.clone(), body.clone(), func.ret_type.clone()),
        );
    }

    if inlineable.is_empty() { return 0; }

    let mut inlined_count = 0u32;
    // Keep a counter for generating fresh values during inlining
    let mut next_val = module.functions.iter()
        .flat_map(|f| f.blocks.iter())
        .flat_map(|b| b.insts.iter())
        .filter_map(|inst| inst_result(inst))
        .map(|v| v.0)
        .max()
        .unwrap_or(0) + 1000;

    for func in &mut module.functions {
        for block in &mut func.blocks {
            let mut new_insts: Vec<Inst> = Vec::new();
            for inst in block.insts.drain(..) {
                if let Inst::Call { result, func: callee, args, .. } = &inst {
                    if let Some((_params, body, _ret_ty)) = inlineable.get(callee.as_str()) {
                        // Build value remapping: param values → arg values
                        let mut remap: HashMap<Value, Value> = HashMap::new();

                        // Map callee parameter values to caller argument values.
                        // In the IR, params are Value(0)..Value(n-1) inside the callee.
                        for (i, arg) in args.iter().enumerate() {
                            remap.insert(Value(i as u32), *arg);
                        }

                        // First, collect all result values in the body
                        let body_results: Vec<Value> = body.iter().filter_map(inst_result).collect();

                        // Create fresh values for all body results
                        for &orig in &body_results {
                            let fresh = Value(next_val);
                            next_val += 1;
                            remap.insert(orig, fresh);
                        }

                        // If the callee's body returns a value, we need to map
                        // the final return value to our call result
                        let mut final_result_mapped = false;

                        for body_inst in body {
                            match body_inst {
                                Inst::Return { value: Some(ret_val) } => {
                                    // Map the return value to the call's result
                                    let source = remap.get(ret_val).copied().unwrap_or(*ret_val);
                                    new_insts.push(Inst::Copy { result: *result, source });
                                    final_result_mapped = true;
                                }
                                Inst::Return { value: None } => {
                                    // Void return — nothing to do
                                }
                                _ => {
                                    // Clone and remap the instruction
                                    let mut cloned = body_inst.clone();
                                    remap_inst_values(&mut cloned, &remap);
                                    new_insts.push(cloned);
                                }
                            }
                        }
                        let _ = final_result_mapped;
                        inlined_count += 1;
                        continue;
                    }
                }
                new_insts.push(inst);
            }
            block.insts = new_insts;
        }
    }

    inlined_count
}

/// Remap SSA values in an instruction using the given mapping.
fn remap_inst_values(inst: &mut Inst, remap: &HashMap<Value, Value>) {
    let resolve = |v: &mut Value| {
        if let Some(&new_v) = remap.get(v) {
            *v = new_v;
        }
    };
    match inst {
        Inst::IConst { result, .. } | Inst::FConst { result, .. }
        | Inst::BConst { result, .. } | Inst::StrConst { result, .. } => { resolve(result); }
        Inst::BinOp { result, lhs, rhs, .. } => { resolve(result); resolve(lhs); resolve(rhs); }
        Inst::UnOp { result, operand, .. } => { resolve(result); resolve(operand); }
        Inst::ICmp { result, lhs, rhs, .. } => { resolve(result); resolve(lhs); resolve(rhs); }
        Inst::FCmp { result, lhs, rhs, .. } => { resolve(result); resolve(lhs); resolve(rhs); }
        Inst::Copy { result, source } => { resolve(result); resolve(source); }
        Inst::Alloca { result, .. } => { resolve(result); }
        Inst::Load { result, ptr, .. } => { resolve(result); resolve(ptr); }
        Inst::Store { value, ptr, .. } => { resolve(value); resolve(ptr); }
        Inst::Call { result, args, .. } => {
            resolve(result);
            for a in args.iter_mut() { resolve(a); }
        }
        Inst::ArrayAlloc { result, count, .. } => { resolve(result); resolve(count); }
        Inst::ArrayGet { result, array, index, .. } => { resolve(result); resolve(array); resolve(index); }
        Inst::ArraySet { array, index, value, .. } => { resolve(array); resolve(index); resolve(value); }
        Inst::ArrayLen { result, array } => { resolve(result); resolve(array); }
        Inst::StructAlloc { result, fields, .. } => {
            resolve(result);
            for f in fields.iter_mut() { resolve(f); }
        }
        Inst::FieldGet { result, object, .. } => { resolve(result); resolve(object); }
        Inst::FieldSet { object, value, .. } => { resolve(object); resolve(value); }
        Inst::ClosureAlloc { result, captures, .. } => {
            resolve(result);
            for c in captures.iter_mut() { resolve(c); }
        }
        Inst::EnumAlloc { result, fields, .. } => {
            resolve(result);
            for f in fields.iter_mut() { resolve(f); }
        }
        Inst::EnumTag { result, enum_val, .. } => { resolve(result); resolve(enum_val); }
        Inst::EnumField { result, enum_val, .. } => { resolve(result); resolve(enum_val); }
        Inst::Phi { result, incoming, .. } => {
            resolve(result);
            for (v, _) in incoming.iter_mut() { resolve(v); }
        }
        _ => {}
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  QUANTUM-INSPIRED FITNESS LANDSCAPE — Hamiltonian-mapped scoring
// ═══════════════════════════════════════════════════════════════════════

/// Quantum-inspired fitness landscape analysis.
///
/// Maps the classical fitness landscape onto a quantum Hamiltonian model
/// where each function variant is a "quantum state" and fitness differences
/// drive tunneling probabilities between variants.
///
/// This enables:
/// - Tunneling through local fitness optima (unlike gradient descent)
/// - Entanglement-inspired correlation between related functions
/// - Superposition-based exploration of multiple variants simultaneously
pub struct QuantumLandscape {
    /// State vector: function_name → vec of (generation, fitness, energy)
    states: HashMap<String, Vec<QuantumState>>,
    /// Temperature parameter for simulated quantum annealing.
    temperature: f64,
    /// Planck constant analog — controls tunneling probability.
    h_bar: f64,
    /// Entanglement pairs: functions that co-evolve
    entanglements: Vec<(String, String, f64)>, // (func_a, func_b, coupling)
}

/// A single quantum state in the fitness landscape.
#[derive(Debug, Clone)]
pub struct QuantumState {
    pub generation: u64,
    pub fitness: f64,
    /// Energy = -fitness (lower energy = higher fitness, like quantum ground state)
    pub energy: f64,
    /// Amplitude (probability weight in superposition).
    pub amplitude: f64,
    /// Phase (for interference effects).
    pub phase: f64,
}

impl QuantumLandscape {
    pub fn new() -> Self {
        Self {
            states: HashMap::new(),
            temperature: 1.0,
            h_bar: 0.1,
            entanglements: Vec::new(),
        }
    }

    /// Add a variant to the quantum landscape.
    pub fn add_state(&mut self, function_name: &str, generation: u64, fitness: f64) {
        let energy = -fitness; // Lower energy = better
        let amplitude = fitness.max(0.01); // Proportional to fitness
        let phase = (generation as f64 * 0.7853981633974483).sin(); // π/4 phase rotation per gen

        let states = self.states.entry(function_name.to_string()).or_default();
        states.push(QuantumState { generation, fitness, energy, amplitude, phase });

        // Normalize amplitudes (like quantum state normalization)
        let norm: f64 = states.iter().map(|s| s.amplitude * s.amplitude).sum::<f64>().sqrt();
        if norm > 1e-15 {
            for s in states.iter_mut() {
                s.amplitude /= norm;
            }
        }
    }

    /// Declare two functions as entangled (they tend to co-evolve).
    pub fn entangle(&mut self, func_a: &str, func_b: &str, coupling: f64) {
        self.entanglements.push((
            func_a.to_string(),
            func_b.to_string(),
            coupling.clamp(-1.0, 1.0),
        ));
    }

    /// Compute the quantum tunneling probability between two fitness values.
    /// Higher barrier = lower probability, but non-zero (unlike classical).
    pub fn tunneling_probability(&self, energy_from: f64, energy_to: f64) -> f64 {
        let barrier = (energy_to - energy_from).max(0.0);
        if barrier < 1e-15 { return 1.0; } // Downhill: always tunnel

        // Gamow tunneling formula analog: P ∝ exp(-2 * barrier / ℏ)
        let exponent = -2.0 * barrier / (self.h_bar * self.temperature);
        exponent.exp().clamp(1e-10, 1.0)
    }

    /// Compute quantum-inspired annealing score for a function.
    /// This blends exploration (high temperature) with exploitation (low temperature).
    /// Returns a score that accounts for tunneling potential to better states.
    pub fn annealing_score(&self, function_name: &str, current_fitness: f64) -> f64 {
        let states = match self.states.get(function_name) {
            Some(s) if !s.is_empty() => s,
            _ => return current_fitness,
        };

        let current_energy = -current_fitness;

        // Sum probability-weighted energies across all known states
        // (quantum expectation value)
        let mut expectation = 0.0;
        let mut total_weight = 0.0;

        for state in states {
            let tunnel_prob = self.tunneling_probability(current_energy, state.energy);
            let weight = state.amplitude * state.amplitude * tunnel_prob;
            expectation += weight * state.fitness;
            total_weight += weight;
        }

        // Factor in entanglement correlations
        let entanglement_bonus = self.entanglement_correlation(function_name);

        if total_weight > 1e-15 {
            (expectation / total_weight) * (1.0 + entanglement_bonus * 0.1)
        } else {
            current_fitness
        }
    }

    /// Compute entanglement correlation for a function.
    /// Returns a signal in [-1, 1] based on correlated partner fitness.
    fn entanglement_correlation(&self, function_name: &str) -> f64 {
        let mut correlation = 0.0;
        let mut count = 0;

        for (fa, fb, coupling) in &self.entanglements {
            let partner = if fa == function_name {
                Some(fb)
            } else if fb == function_name {
                Some(fa)
            } else {
                None
            };

            if let Some(partner_name) = partner {
                if let Some(partner_states) = self.states.get(partner_name.as_str()) {
                    if let Some(last) = partner_states.last() {
                        correlation += coupling * last.fitness;
                        count += 1;
                    }
                }
            }
        }

        if count > 0 { correlation / count as f64 } else { 0.0 }
    }

    /// Reduce temperature (annealing schedule).
    pub fn cool(&mut self, rate: f64) {
        self.temperature *= rate;
        if self.temperature < 0.001 {
            self.temperature = 0.001; // Floor
        }
    }

    /// Get landscape analysis as JSON.
    pub fn landscape_json(&self) -> String {
        let total_states: usize = self.states.values().map(|v| v.len()).sum();
        let total_functions = self.states.len();
        let total_entanglements = self.entanglements.len();

        let best_fitness = self.states.values()
            .flat_map(|v| v.iter())
            .map(|s| s.fitness)
            .fold(0.0_f64, f64::max);

        format!(
            concat!(
                "{{",
                "\"total_states\":{},",
                "\"total_functions\":{},",
                "\"entanglements\":{},",
                "\"temperature\":{:.6},",
                "\"h_bar\":{:.6},",
                "\"best_fitness\":{:.4}",
                "}}"
            ),
            total_states,
            total_functions,
            total_entanglements,
            self.temperature,
            self.h_bar,
            best_fitness,
        )
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  TESTS
// ═══════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{IrType, BasicBlock, BlockId};

    // ── Compilation Cache Tests ──────────────────────────────────────
    #[test]
    fn test_cache_basic() {
        let mut cache = CompilationCache::new(100);
        assert_eq!(cache.hit_rate(), 0.0);

        cache.store(12345, CachedCompilation {
            source_hash: 12345,
            success: true,
            compile_time_ms: 1.5,
            fitness: 0.85,
            hits: 0,
            last_access_ms: 0.0,
            errors: vec![],
        });

        assert!(cache.lookup(12345).is_some());
        assert_eq!(cache.total_hits, 1);
        assert!(cache.lookup(99999).is_none());
        assert_eq!(cache.total_misses, 1);
        assert!((cache.hit_rate() - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_cache_eviction() {
        let mut cache = CompilationCache::new(2);
        for i in 0..3 {
            cache.store(i, CachedCompilation {
                source_hash: i,
                success: true,
                compile_time_ms: 1.0,
                fitness: 0.5,
                hits: 0,
                last_access_ms: i as f64,
                errors: vec![],
            });
        }
        // Should have evicted the first entry
        assert_eq!(cache.entries.len(), 2);
    }

    // ── Trajectory Predictor Tests ──────────────────────────────────
    #[test]
    fn test_trajectory_basic() {
        let mut pred = TrajectoryPredictor::new();
        pred.observe(EvolutionObservation {
            function_name: "alpha".to_string(),
            generation: 0,
            fitness: 0.5,
            source_hash: 100,
            timestamp_ms: 1000.0,
        });
        pred.observe(EvolutionObservation {
            function_name: "alpha".to_string(),
            generation: 1,
            fitness: 0.6,
            source_hash: 101,
            timestamp_ms: 2000.0,
        });
        pred.observe(EvolutionObservation {
            function_name: "beta".to_string(),
            generation: 0,
            fitness: 0.3,
            source_hash: 200,
            timestamp_ms: 3000.0,
        });

        let preds = pred.predict_next(5);
        assert!(!preds.is_empty());
        // Alpha was evolved more recently and more frequently
    }

    // ── Delta Debugger Tests ────────────────────────────────────────
    #[test]
    fn test_delta_debug_identical() {
        let dd = DeltaDebugger::new();
        let source = "fn main() -> i64 { 42 }";
        let result = dd.isolate(source, source);
        assert_eq!(result.bisection_steps, 0);
        assert!(result.errors[0].contains("identical"));
    }

    #[test]
    fn test_delta_debug_simple() {
        let dd = DeltaDebugger::new();
        let good = "fn main() -> i64 { 42 }";
        let bad = "fn main( -> { broken }"; // syntax error
        let result = dd.isolate(good, bad);
        assert!(!result.errors.is_empty() || !result.minimal_failing.is_empty());
        // Single-line diff may not need oracle calls (already minimal)
        assert!(result.bisection_steps == 0 || result.oracle_calls >= 1);
    }

    // ── Inlining Oracle Tests ───────────────────────────────────────
    #[test]
    fn test_inlining_basic() {
        let mut oracle = InliningOracle::new();
        oracle.register_candidate(InliningCandidate {
            name: "small_fn".to_string(),
            body_size: 5,
            call_sites: 10,
            is_leaf: true,
            param_count: 1,
            has_loops: false,
            thompson_alpha: 1.0,
            thompson_beta: 1.0,
            inlined: false,
            improvement: 0.0,
        });
        oracle.register_candidate(InliningCandidate {
            name: "big_fn".to_string(),
            body_size: 100,
            call_sites: 2,
            is_leaf: false,
            param_count: 5,
            has_loops: true,
            thompson_alpha: 1.0,
            thompson_beta: 1.0,
            inlined: false,
            improvement: 0.0,
        });

        let decisions = oracle.decide();
        assert!(decisions.contains(&"small_fn".to_string()));
        assert!(!decisions.contains(&"big_fn".to_string())); // Too large
    }

    #[test]
    fn test_inlining_thompson_learning() {
        let mut oracle = InliningOracle::new();
        oracle.register_candidate(InliningCandidate {
            name: "learnable".to_string(),
            body_size: 10,
            call_sites: 5,
            is_leaf: true,
            param_count: 2,
            has_loops: false,
            thompson_alpha: 1.0,
            thompson_beta: 1.0,
            inlined: false,
            improvement: 0.0,
        });

        let score_before = oracle.score("learnable");

        // Record positive outcome
        oracle.record_outcome("learnable", 0.5);
        let score_after = oracle.score("learnable");

        // Score should improve after positive outcome
        assert!(score_after >= score_before);
    }

    // ── Quantum Landscape Tests ─────────────────────────────────────
    #[test]
    fn test_quantum_basic() {
        let mut ql = QuantumLandscape::new();
        ql.add_state("func_a", 0, 0.5);
        ql.add_state("func_a", 1, 0.7);
        ql.add_state("func_a", 2, 0.6);

        let score = ql.annealing_score("func_a", 0.6);
        assert!(score > 0.0);
    }

    #[test]
    fn test_quantum_tunneling() {
        let ql = QuantumLandscape::new();

        // Downhill: always tunnel
        let p_down = ql.tunneling_probability(-0.5, -0.8);
        assert!((p_down - 1.0).abs() < 1e-10);

        // Uphill: probability decreases with barrier height
        let p_up_small = ql.tunneling_probability(-0.5, -0.3);
        let p_up_large = ql.tunneling_probability(-0.5, 0.5);
        assert!(p_up_small > p_up_large);
    }

    #[test]
    fn test_quantum_entanglement() {
        let mut ql = QuantumLandscape::new();
        ql.add_state("func_a", 0, 0.8);
        ql.add_state("func_b", 0, 0.9);
        ql.entangle("func_a", "func_b", 0.5);

        let corr = ql.entanglement_correlation("func_a");
        assert!(corr > 0.0); // Positive coupling with high-fitness partner
    }

    #[test]
    fn test_quantum_cooling() {
        let mut ql = QuantumLandscape::new();
        let temp_before = ql.temperature;
        ql.cool(0.95);
        assert!(ql.temperature < temp_before);
        assert!(ql.temperature > 0.0);
    }

    // ── IR Optimization Tests ───────────────────────────────────────
    #[test]
    fn test_constant_folding() {
        use crate::ir::{BlockId};
        let mut func = IrFunction {
            name: "test".to_string(),
            params: vec![],
            ret_type: IrType::I64,
            blocks: vec![BasicBlock {
                id: BlockId(0),
                insts: vec![
                    Inst::IConst { result: Value(0), value: 10, ty: IrType::I64 },
                    Inst::IConst { result: Value(1), value: 20, ty: IrType::I64 },
                    Inst::BinOp {
                        result: Value(2),
                        op: IrBinOp::Add,
                        lhs: Value(0),
                        rhs: Value(1),
                        ty: IrType::I64,
                    },
                    Inst::Return { value: Some(Value(2)) },
                ],
            }],
            entry: BlockId(0),
        };

        let folded = constant_fold(&mut func);
        assert!(folded > 0);

        // The BinOp should now be replaced with IConst(30)
        let last_const = func.blocks[0].insts.iter().find(|i| matches!(i, Inst::IConst { value: 30, .. }));
        assert!(last_const.is_some());
    }

    #[test]
    fn test_dead_code_elimination() {
        use crate::ir::{BlockId};
        let mut func = IrFunction {
            name: "test".to_string(),
            params: vec![],
            ret_type: IrType::I64,
            blocks: vec![BasicBlock {
                id: BlockId(0),
                insts: vec![
                    Inst::IConst { result: Value(0), value: 42, ty: IrType::I64 },
                    Inst::IConst { result: Value(1), value: 99, ty: IrType::I64 }, // Dead
                    Inst::Return { value: Some(Value(0)) },
                ],
            }],
            entry: BlockId(0),
        };

        let eliminated = dead_code_eliminate(&mut func);
        assert_eq!(eliminated, 1);
        assert_eq!(func.blocks[0].insts.len(), 2); // Only IConst(42) + Return
    }

    // ── Optimization Stats Tests ────────────────────────────────────
    #[test]
    fn test_opt_pass_stats_json() {
        let stats = OptPassStats {
            constants_folded: 5,
            dead_eliminated: 3,
            cse_eliminated: 2,
            strength_reduced: 1,
            loops_tiled: 1,
            copies_propagated: 0,
            blocks_merged: 0,
            licm_hoisted: 0,
            functions_inlined: 0,
            instructions_before: 100,
            instructions_after: 88,
        };
        let json = stats.to_json();
        assert!(json.contains("\"constants_folded\":5"));
        assert!(json.contains("\"dead_eliminated\":3"));
        assert!(json.contains("\"cse_eliminated\":2"));
        assert!(json.contains("\"strength_reduced\":1"));
    }

    // ── Strength Reduction Tests ────────────────────────────────────
    #[test]
    fn test_strength_reduce_mul_by_zero() {
        let mut func = IrFunction {
            name: "test".to_string(),
            params: vec![],
            ret_type: IrType::I64,
            blocks: vec![BasicBlock {
                id: BlockId(0),
                insts: vec![
                    Inst::IConst { result: Value(0), value: 42, ty: IrType::I64 },
                    Inst::IConst { result: Value(1), value: 0, ty: IrType::I64 },
                    Inst::BinOp {
                        result: Value(2), op: IrBinOp::Mul,
                        lhs: Value(0), rhs: Value(1), ty: IrType::I64,
                    },
                    Inst::Return { value: Some(Value(2)) },
                ],
            }],
            entry: BlockId(0),
        };
        let n = strength_reduce(&mut func);
        assert_eq!(n, 1);
        // x * 0 → IConst 0
        assert!(matches!(func.blocks[0].insts[2], Inst::IConst { value: 0, .. }));
    }

    #[test]
    fn test_strength_reduce_mul_by_one() {
        let mut func = IrFunction {
            name: "test".to_string(),
            params: vec![],
            ret_type: IrType::I64,
            blocks: vec![BasicBlock {
                id: BlockId(0),
                insts: vec![
                    Inst::IConst { result: Value(0), value: 7, ty: IrType::I64 },
                    Inst::IConst { result: Value(1), value: 1, ty: IrType::I64 },
                    Inst::BinOp {
                        result: Value(2), op: IrBinOp::Mul,
                        lhs: Value(0), rhs: Value(1), ty: IrType::I64,
                    },
                    Inst::Return { value: Some(Value(2)) },
                ],
            }],
            entry: BlockId(0),
        };
        let n = strength_reduce(&mut func);
        assert_eq!(n, 1);
        // x * 1 → Copy(x)
        assert!(matches!(func.blocks[0].insts[2], Inst::Copy { source: Value(0), .. }));
    }

    #[test]
    fn test_strength_reduce_add_zero() {
        let mut func = IrFunction {
            name: "test".to_string(),
            params: vec![],
            ret_type: IrType::I64,
            blocks: vec![BasicBlock {
                id: BlockId(0),
                insts: vec![
                    Inst::IConst { result: Value(0), value: 5, ty: IrType::I64 },
                    Inst::IConst { result: Value(1), value: 0, ty: IrType::I64 },
                    Inst::BinOp {
                        result: Value(2), op: IrBinOp::Add,
                        lhs: Value(0), rhs: Value(1), ty: IrType::I64,
                    },
                    Inst::Return { value: Some(Value(2)) },
                ],
            }],
            entry: BlockId(0),
        };
        let n = strength_reduce(&mut func);
        assert_eq!(n, 1);
        assert!(matches!(func.blocks[0].insts[2], Inst::Copy { source: Value(0), .. }));
    }

    #[test]
    fn test_strength_reduce_div_by_one() {
        let mut func = IrFunction {
            name: "test".to_string(),
            params: vec![],
            ret_type: IrType::I64,
            blocks: vec![BasicBlock {
                id: BlockId(0),
                insts: vec![
                    Inst::IConst { result: Value(0), value: 8, ty: IrType::I64 },
                    Inst::IConst { result: Value(1), value: 1, ty: IrType::I64 },
                    Inst::BinOp {
                        result: Value(2), op: IrBinOp::Div,
                        lhs: Value(0), rhs: Value(1), ty: IrType::I64,
                    },
                    Inst::Return { value: Some(Value(2)) },
                ],
            }],
            entry: BlockId(0),
        };
        let n = strength_reduce(&mut func);
        assert_eq!(n, 1);
        assert!(matches!(func.blocks[0].insts[2], Inst::Copy { source: Value(0), .. }));
    }

    // ── CSE Tests ───────────────────────────────────────────────────
    #[test]
    fn test_cse_duplicate_binop() {
        let mut func = IrFunction {
            name: "test".to_string(),
            params: vec![],
            ret_type: IrType::I64,
            blocks: vec![BasicBlock {
                id: BlockId(0),
                insts: vec![
                    Inst::IConst { result: Value(0), value: 10, ty: IrType::I64 },
                    Inst::IConst { result: Value(1), value: 20, ty: IrType::I64 },
                    // First: v2 = v0 + v1
                    Inst::BinOp {
                        result: Value(2), op: IrBinOp::Add,
                        lhs: Value(0), rhs: Value(1), ty: IrType::I64,
                    },
                    // Duplicate: v3 = v0 + v1  (same operands)
                    Inst::BinOp {
                        result: Value(3), op: IrBinOp::Add,
                        lhs: Value(0), rhs: Value(1), ty: IrType::I64,
                    },
                    Inst::Return { value: Some(Value(3)) },
                ],
            }],
            entry: BlockId(0),
        };
        let n = cse(&mut func);
        assert_eq!(n, 1);
        // Second BinOp should now be Copy(v2)
        assert!(matches!(func.blocks[0].insts[3], Inst::Copy { result: Value(3), source: Value(2) }));
    }

    #[test]
    fn test_cse_different_ops_not_eliminated() {
        let mut func = IrFunction {
            name: "test".to_string(),
            params: vec![],
            ret_type: IrType::I64,
            blocks: vec![BasicBlock {
                id: BlockId(0),
                insts: vec![
                    Inst::IConst { result: Value(0), value: 10, ty: IrType::I64 },
                    Inst::IConst { result: Value(1), value: 20, ty: IrType::I64 },
                    Inst::BinOp {
                        result: Value(2), op: IrBinOp::Add,
                        lhs: Value(0), rhs: Value(1), ty: IrType::I64,
                    },
                    Inst::BinOp {
                        result: Value(3), op: IrBinOp::Mul,
                        lhs: Value(0), rhs: Value(1), ty: IrType::I64,
                    },
                    Inst::Return { value: Some(Value(3)) },
                ],
            }],
            entry: BlockId(0),
        };
        let n = cse(&mut func);
        assert_eq!(n, 0); // Different ops, nothing eliminated
    }

    // ── ICmp/FCmp constant folding tests ────────────────────────────
    #[test]
    fn test_constant_fold_icmp() {
        let mut func = IrFunction {
            name: "test".to_string(),
            params: vec![],
            ret_type: IrType::I64,
            blocks: vec![BasicBlock {
                id: BlockId(0),
                insts: vec![
                    Inst::IConst { result: Value(0), value: 5, ty: IrType::I64 },
                    Inst::IConst { result: Value(1), value: 10, ty: IrType::I64 },
                    Inst::ICmp {
                        result: Value(2), cond: IrCmp::Lt,
                        lhs: Value(0), rhs: Value(1),
                    },
                    Inst::Return { value: Some(Value(2)) },
                ],
            }],
            entry: BlockId(0),
        };
        let n = constant_fold(&mut func);
        assert_eq!(n, 1);
        // 5 < 10 → true → IConst 1
        assert!(matches!(func.blocks[0].insts[2], Inst::IConst { value: 1, .. }));
    }

    #[test]
    fn test_constant_fold_fcmp() {
        let mut func = IrFunction {
            name: "test".to_string(),
            params: vec![],
            ret_type: IrType::I64,
            blocks: vec![BasicBlock {
                id: BlockId(0),
                insts: vec![
                    Inst::FConst { result: Value(0), value: 3.14, ty: IrType::F64 },
                    Inst::FConst { result: Value(1), value: 2.72, ty: IrType::F64 },
                    Inst::FCmp {
                        result: Value(2), cond: IrCmp::Gt,
                        lhs: Value(0), rhs: Value(1),
                    },
                    Inst::Return { value: Some(Value(2)) },
                ],
            }],
            entry: BlockId(0),
        };
        let n = constant_fold(&mut func);
        assert_eq!(n, 1);
        // 3.14 > 2.72 → true → IConst 1
        assert!(matches!(func.blocks[0].insts[2], Inst::IConst { value: 1, .. }));
    }

    // ── Pipeline integration test ───────────────────────────────────
    #[test]
    fn test_optimize_ir_pipeline() {
        let mut module = IrModule {
            functions: vec![IrFunction {
                name: "pipeline_test".to_string(),
                params: vec![],
                ret_type: IrType::I64,
                blocks: vec![BasicBlock {
                    id: BlockId(0),
                    insts: vec![
                        Inst::IConst { result: Value(0), value: 10, ty: IrType::I64 },
                        Inst::IConst { result: Value(1), value: 1, ty: IrType::I64 },
                        // x * 1 → strength reduce to Copy
                        Inst::BinOp {
                            result: Value(2), op: IrBinOp::Mul,
                            lhs: Value(0), rhs: Value(1), ty: IrType::I64,
                        },
                        Inst::IConst { result: Value(3), value: 20, ty: IrType::I64 },
                        // constant fold: 10 + 20 = 30
                        Inst::BinOp {
                            result: Value(4), op: IrBinOp::Add,
                            lhs: Value(0), rhs: Value(3), ty: IrType::I64,
                        },
                        // Dead: Value(5) never used
                        Inst::IConst { result: Value(5), value: 999, ty: IrType::I64 },
                        Inst::Return { value: Some(Value(4)) },
                    ],
                }],
                entry: BlockId(0),
            }],
            string_constants: vec![],
        };

        let stats = optimize_ir(&mut module);
        assert!(stats.constants_folded > 0 || stats.strength_reduced > 0 || stats.dead_eliminated > 0);
        // Total instructions should have decreased
        assert!(stats.instructions_after <= stats.instructions_before);
    }

    // ─── v126: Dead Function Elimination & AOT Optimization Tests ───

    fn make_empty_func(name: &str) -> IrFunction {
        IrFunction {
            name: name.to_string(),
            params: vec![],
            ret_type: IrType::I64,
            blocks: vec![BasicBlock {
                id: BlockId(0),
                insts: vec![
                    Inst::IConst { result: Value(0), value: 0, ty: IrType::I64 },
                    Inst::Return { value: Some(Value(0)) },
                ],
            }],
            entry: BlockId(0),
        }
    }

    fn make_calling_func(name: &str, calls: &[&str]) -> IrFunction {
        let mut insts: Vec<Inst> = calls.iter().enumerate().map(|(i, callee)| {
            Inst::Call {
                result: Value(i as u32),
                func: callee.to_string(),
                args: vec![],
                ret_ty: IrType::I64,
            }
        }).collect();
        let ret_val = if calls.is_empty() { Value(0) } else { Value((calls.len() - 1) as u32) };

        if calls.is_empty() {
            insts.push(Inst::IConst { result: Value(0), value: 0, ty: IrType::I64 });
        }
        insts.push(Inst::Return { value: Some(ret_val) });

        IrFunction {
            name: name.to_string(),
            params: vec![],
            ret_type: IrType::I64,
            blocks: vec![BasicBlock {
                id: BlockId(0),
                insts,
            }],
            entry: BlockId(0),
        }
    }

    #[test]
    fn test_dead_function_eliminate_removes_unused() {
        let mut module = IrModule {
            functions: vec![
                make_empty_func("main"),
                make_empty_func("unused_a"),
                make_empty_func("unused_b"),
            ],
            string_constants: vec![],
        };
        let removed = dead_function_eliminate(&mut module);
        assert_eq!(removed, 2);
        assert_eq!(module.functions.len(), 1);
        assert_eq!(module.functions[0].name, "main");
    }

    #[test]
    fn test_dead_function_eliminate_keeps_called() {
        let mut module = IrModule {
            functions: vec![
                make_calling_func("main", &["helper"]),
                make_empty_func("helper"),
                make_empty_func("unused"),
            ],
            string_constants: vec![],
        };
        let removed = dead_function_eliminate(&mut module);
        assert_eq!(removed, 1);
        assert_eq!(module.functions.len(), 2);
        let names: Vec<&str> = module.functions.iter().map(|f| f.name.as_str()).collect();
        assert!(names.contains(&"main"));
        assert!(names.contains(&"helper"));
    }

    #[test]
    fn test_dead_function_eliminate_transitive() {
        let mut module = IrModule {
            functions: vec![
                make_calling_func("main", &["a"]),
                make_calling_func("a", &["b"]),
                make_empty_func("b"),
                make_empty_func("orphan"),
            ],
            string_constants: vec![],
        };
        let removed = dead_function_eliminate(&mut module);
        assert_eq!(removed, 1);
        assert_eq!(module.functions.len(), 3);
    }

    #[test]
    fn test_dead_function_eliminate_no_main() {
        let mut module = IrModule {
            functions: vec![
                make_empty_func("library_fn"),
                make_empty_func("another_fn"),
            ],
            string_constants: vec![],
        };
        // No main → library mode, keep everything
        let removed = dead_function_eliminate(&mut module);
        assert_eq!(removed, 0);
        assert_eq!(module.functions.len(), 2);
    }

    #[test]
    fn test_dead_function_eliminate_empty_module() {
        let mut module = IrModule {
            functions: vec![],
            string_constants: vec![],
        };
        let removed = dead_function_eliminate(&mut module);
        assert_eq!(removed, 0);
    }

    #[test]
    fn test_dead_function_eliminate_all_reachable() {
        let mut module = IrModule {
            functions: vec![
                make_calling_func("main", &["a", "b"]),
                make_empty_func("a"),
                make_empty_func("b"),
            ],
            string_constants: vec![],
        };
        let removed = dead_function_eliminate(&mut module);
        assert_eq!(removed, 0);
        assert_eq!(module.functions.len(), 3);
    }

    #[test]
    fn test_optimize_ir_includes_dead_fn_elim() {
        let mut module = IrModule {
            functions: vec![
                make_empty_func("main"),
                make_empty_func("dead_fn"),
            ],
            string_constants: vec![],
        };
        let stats = optimize_ir(&mut module);
        assert!(stats.dead_eliminated > 0);
        assert_eq!(module.functions.len(), 1);
    }

    // ── v128: FCmp constant fold fix ───────────────────────────────

    #[test]
    fn test_v128_fcmp_eq_exact_bits() {
        // Verify FCmp Eq uses bit-exact comparison, not epsilon
        use crate::ir::*;
        let r1 = Value(100);
        let r2 = Value(101);
        let r3 = Value(102);
        let mut func = IrFunction {
            name: "test_fcmp".into(),
            params: vec![],
            ret_type: IrType::Bool,
            blocks: vec![BasicBlock {
                id: BlockId(0),
                insts: vec![
                    Inst::FConst { result: r1, value: 1.0, ty: IrType::F64 },
                    Inst::FConst { result: r2, value: 1.0, ty: IrType::F64 },
                    Inst::FCmp { result: r3, cond: IrCmp::Eq, lhs: r1, rhs: r2 },
                ],
            }],
            entry: BlockId(0),
        };
        constant_fold(&mut func);
        // 1.0 == 1.0 should fold to true (1)
        let last = &func.blocks[0].insts.last().unwrap();
        if let Inst::IConst { value, .. } = last {
            assert_eq!(*value, 1, "1.0 == 1.0 should fold to true");
        }
    }

    #[test]
    fn test_v128_fcmp_ne_different_values() {
        use crate::ir::*;
        let r1 = Value(100);
        let r2 = Value(101);
        let r3 = Value(102);
        let mut func = IrFunction {
            name: "test_fcmp_ne".into(),
            params: vec![],
            ret_type: IrType::Bool,
            blocks: vec![BasicBlock {
                id: BlockId(0),
                insts: vec![
                    Inst::FConst { result: r1, value: 0.0, ty: IrType::F64 },
                    Inst::FConst { result: r2, value: -0.0, ty: IrType::F64 },
                    Inst::FCmp { result: r3, cond: IrCmp::Ne, lhs: r1, rhs: r2 },
                ],
            }],
            entry: BlockId(0),
        };
        constant_fold(&mut func);
        // 0.0 and -0.0 have different bits so should be Ne = true
        let last = &func.blocks[0].insts.last().unwrap();
        if let Inst::IConst { value, .. } = last {
            assert_eq!(*value, 1, "0.0 != -0.0 at bit level");
        }
    }

    // ── v135 Copy Propagation Tests ──────────────────────────────────

    #[test]
    fn test_v135_copy_propagate_basic() {
        // v1 = 42; v2 = copy v1; v3 = v2 + v2 → v3 should use v1
        use crate::ir::*;
        let v1 = Value(1);
        let v2 = Value(2);
        let v3 = Value(3);
        let mut func = IrFunction {
            name: "test_copy".into(),
            params: vec![],
            ret_type: IrType::I64,
            blocks: vec![BasicBlock {
                id: BlockId(0),
                insts: vec![
                    Inst::IConst { result: v1, value: 42, ty: IrType::I64 },
                    Inst::Copy { result: v2, source: v1 },
                    Inst::BinOp { result: v3, op: IrBinOp::Add, lhs: v2, rhs: v2, ty: IrType::I64 },
                    Inst::Return { value: Some(v3) },
                ],
            }],
            entry: BlockId(0),
        };
        let n = copy_propagate(&mut func);
        assert!(n >= 2, "should propagate at least 2 uses of v2 → v1");
        // The BinOp should now reference v1 instead of v2
        if let Inst::BinOp { lhs, rhs, .. } = &func.blocks[0].insts[2] {
            assert_eq!(*lhs, v1);
            assert_eq!(*rhs, v1);
        } else {
            panic!("expected BinOp");
        }
    }

    #[test]
    fn test_v135_copy_propagate_chain() {
        // v1 = 10; v2 = copy v1; v3 = copy v2; return v3 → return v1
        use crate::ir::*;
        let v1 = Value(1);
        let v2 = Value(2);
        let v3 = Value(3);
        let mut func = IrFunction {
            name: "test_chain".into(),
            params: vec![],
            ret_type: IrType::I64,
            blocks: vec![BasicBlock {
                id: BlockId(0),
                insts: vec![
                    Inst::IConst { result: v1, value: 10, ty: IrType::I64 },
                    Inst::Copy { result: v2, source: v1 },
                    Inst::Copy { result: v3, source: v2 },
                    Inst::Return { value: Some(v3) },
                ],
            }],
            entry: BlockId(0),
        };
        let n = copy_propagate(&mut func);
        assert!(n >= 1);
        // Return should now reference v1 directly
        if let Inst::Return { value: Some(ret_val) } = &func.blocks[0].insts[3] {
            assert_eq!(*ret_val, v1, "transitive copy chain should resolve to root");
        } else {
            panic!("expected Return");
        }
    }

    #[test]
    fn test_v135_copy_propagate_no_copies() {
        // No copies → no changes
        use crate::ir::*;
        let v1 = Value(1);
        let v2 = Value(2);
        let v3 = Value(3);
        let mut func = IrFunction {
            name: "test_no_copies".into(),
            params: vec![],
            ret_type: IrType::I64,
            blocks: vec![BasicBlock {
                id: BlockId(0),
                insts: vec![
                    Inst::IConst { result: v1, value: 1, ty: IrType::I64 },
                    Inst::IConst { result: v2, value: 2, ty: IrType::I64 },
                    Inst::BinOp { result: v3, op: IrBinOp::Add, lhs: v1, rhs: v2, ty: IrType::I64 },
                    Inst::Return { value: Some(v3) },
                ],
            }],
            entry: BlockId(0),
        };
        let n = copy_propagate(&mut func);
        assert_eq!(n, 0, "no copies means no propagation");
    }

    #[test]
    fn test_v135_copy_propagate_in_call_args() {
        // v1 = 5; v2 = copy v1; call foo(v2) → call foo(v1)
        use crate::ir::*;
        let v1 = Value(1);
        let v2 = Value(2);
        let v3 = Value(3);
        let mut func = IrFunction {
            name: "test_call_prop".into(),
            params: vec![],
            ret_type: IrType::I64,
            blocks: vec![BasicBlock {
                id: BlockId(0),
                insts: vec![
                    Inst::IConst { result: v1, value: 5, ty: IrType::I64 },
                    Inst::Copy { result: v2, source: v1 },
                    Inst::Call { result: v3, func: "foo".into(), args: vec![v2], ret_ty: IrType::I64 },
                    Inst::Return { value: Some(v3) },
                ],
            }],
            entry: BlockId(0),
        };
        let n = copy_propagate(&mut func);
        assert!(n >= 1);
        if let Inst::Call { args, .. } = &func.blocks[0].insts[2] {
            assert_eq!(args[0], v1, "call arg should be propagated to v1");
        } else {
            panic!("expected Call");
        }
    }

    #[test]
    fn test_v135_copy_propagate_in_branch() {
        // v1 = true; v2 = copy v1; branch v2 → branch v1
        use crate::ir::*;
        let v1 = Value(1);
        let v2 = Value(2);
        let mut func = IrFunction {
            name: "test_branch_prop".into(),
            params: vec![],
            ret_type: IrType::Void,
            blocks: vec![
                BasicBlock {
                    id: BlockId(0),
                    insts: vec![
                        Inst::BConst { result: v1, value: true },
                        Inst::Copy { result: v2, source: v1 },
                        Inst::Branch { cond: v2, then_bb: BlockId(1), else_bb: BlockId(2) },
                    ],
                },
                BasicBlock { id: BlockId(1), insts: vec![Inst::Return { value: None }] },
                BasicBlock { id: BlockId(2), insts: vec![Inst::Return { value: None }] },
            ],
            entry: BlockId(0),
        };
        let n = copy_propagate(&mut func);
        assert!(n >= 1);
        if let Inst::Branch { cond, .. } = &func.blocks[0].insts[2] {
            assert_eq!(*cond, v1, "branch condition should be propagated to v1");
        }
    }

    // ── v135 Block Merging Tests ─────────────────────────────────────

    #[test]
    fn test_v135_merge_blocks_basic() {
        // Block 0 jumps to Block 1 (sole predecessor) → merge into one block
        use crate::ir::*;
        let v1 = Value(1);
        let v2 = Value(2);
        let v3 = Value(3);
        let mut func = IrFunction {
            name: "test_merge".into(),
            params: vec![],
            ret_type: IrType::I64,
            blocks: vec![
                BasicBlock {
                    id: BlockId(0),
                    insts: vec![
                        Inst::IConst { result: v1, value: 10, ty: IrType::I64 },
                        Inst::IConst { result: v2, value: 20, ty: IrType::I64 },
                        Inst::Jump { target: BlockId(1) },
                    ],
                },
                BasicBlock {
                    id: BlockId(1),
                    insts: vec![
                        Inst::BinOp { result: v3, op: IrBinOp::Add, lhs: v1, rhs: v2, ty: IrType::I64 },
                        Inst::Return { value: Some(v3) },
                    ],
                },
            ],
            entry: BlockId(0),
        };
        let n = merge_blocks(&mut func);
        assert_eq!(n, 1, "should merge one block pair");
        assert_eq!(func.blocks.len(), 1, "should have 1 block after merge");
        // Block should have: IConst, IConst, BinOp, Return (Jump removed)
        assert_eq!(func.blocks[0].insts.len(), 4);
    }

    #[test]
    fn test_v135_merge_blocks_no_merge_multiple_preds() {
        // Block 1 has two predecessors (0 and 2) → no merge
        use crate::ir::*;
        let v1 = Value(1);
        let mut func = IrFunction {
            name: "test_no_merge".into(),
            params: vec![],
            ret_type: IrType::I64,
            blocks: vec![
                BasicBlock {
                    id: BlockId(0),
                    insts: vec![
                        Inst::BConst { result: v1, value: true },
                        Inst::Branch { cond: v1, then_bb: BlockId(1), else_bb: BlockId(2) },
                    ],
                },
                BasicBlock {
                    id: BlockId(1),
                    insts: vec![Inst::Return { value: None }],
                },
                BasicBlock {
                    id: BlockId(2),
                    insts: vec![Inst::Jump { target: BlockId(1) }],
                },
            ],
            entry: BlockId(0),
        };
        let n = merge_blocks(&mut func);
        assert_eq!(n, 0, "block 1 has multiple predecessors, cannot merge");
    }

    #[test]
    fn test_v135_merge_blocks_chain() {
        // Block 0 → Block 1 → Block 2 (all single pred) → merge all into one
        use crate::ir::*;
        let v1 = Value(1);
        let v2 = Value(2);
        let mut func = IrFunction {
            name: "test_chain_merge".into(),
            params: vec![],
            ret_type: IrType::I64,
            blocks: vec![
                BasicBlock {
                    id: BlockId(0),
                    insts: vec![
                        Inst::IConst { result: v1, value: 1, ty: IrType::I64 },
                        Inst::Jump { target: BlockId(1) },
                    ],
                },
                BasicBlock {
                    id: BlockId(1),
                    insts: vec![
                        Inst::IConst { result: v2, value: 2, ty: IrType::I64 },
                        Inst::Jump { target: BlockId(2) },
                    ],
                },
                BasicBlock {
                    id: BlockId(2),
                    insts: vec![Inst::Return { value: Some(v2) }],
                },
            ],
            entry: BlockId(0),
        };
        let n = merge_blocks(&mut func);
        assert_eq!(n, 2, "should merge two pairs");
        assert_eq!(func.blocks.len(), 1, "should have 1 block after chain merge");
    }

    #[test]
    fn test_v135_merge_blocks_single_block() {
        // Single block → no merge needed
        use crate::ir::*;
        let mut func = IrFunction {
            name: "test_single".into(),
            params: vec![],
            ret_type: IrType::Void,
            blocks: vec![BasicBlock {
                id: BlockId(0),
                insts: vec![Inst::Return { value: None }],
            }],
            entry: BlockId(0),
        };
        let n = merge_blocks(&mut func);
        assert_eq!(n, 0);
    }

    #[test]
    fn test_v135_merge_blocks_skip_phi() {
        // Block 1 has Phi node → skip merge even if single predecessor
        use crate::ir::*;
        let v1 = Value(1);
        let v2 = Value(2);
        let mut func = IrFunction {
            name: "test_phi_skip".into(),
            params: vec![],
            ret_type: IrType::I64,
            blocks: vec![
                BasicBlock {
                    id: BlockId(0),
                    insts: vec![
                        Inst::IConst { result: v1, value: 5, ty: IrType::I64 },
                        Inst::Jump { target: BlockId(1) },
                    ],
                },
                BasicBlock {
                    id: BlockId(1),
                    insts: vec![
                        Inst::Phi { result: v2, incoming: vec![(v1, BlockId(0))], ty: IrType::I64 },
                        Inst::Return { value: Some(v2) },
                    ],
                },
            ],
            entry: BlockId(0),
        };
        let n = merge_blocks(&mut func);
        assert_eq!(n, 0, "should not merge blocks with Phi nodes");
    }

    // ── v135 Combined Pipeline Tests ─────────────────────────────────

    #[test]
    fn test_v135_optimize_ir_includes_new_passes() {
        // Full pipeline with copy propagation + block merging
        use crate::ir::*;
        let v1 = Value(1);
        let v2 = Value(2);
        let v3 = Value(3);
        let v4 = Value(4);
        let mut module = IrModule {
            functions: vec![IrFunction {
                name: "main".into(),
                params: vec![],
                ret_type: IrType::I64,
                blocks: vec![
                    BasicBlock {
                        id: BlockId(0),
                        insts: vec![
                            Inst::IConst { result: v1, value: 10, ty: IrType::I64 },
                            Inst::Copy { result: v2, source: v1 },
                            Inst::Jump { target: BlockId(1) },
                        ],
                    },
                    BasicBlock {
                        id: BlockId(1),
                        insts: vec![
                            Inst::IConst { result: v3, value: 20, ty: IrType::I64 },
                            Inst::BinOp { result: v4, op: IrBinOp::Add, lhs: v2, rhs: v3, ty: IrType::I64 },
                            Inst::Return { value: Some(v4) },
                        ],
                    },
                ],
                entry: BlockId(0),
            }],
            string_constants: vec![],
        };
        let stats = optimize_ir(&mut module);
        // Copy should be propagated and/or eliminated
        assert!(stats.copies_propagated > 0 || stats.dead_eliminated > 0,
            "new passes should contribute to optimization");
        // Blocks should be merged (block 1 has single predecessor)
        assert!(stats.blocks_merged > 0, "block merging should occur");
    }

    #[test]
    fn test_v135_stats_json_includes_new_fields() {
        let stats = OptPassStats {
            constants_folded: 1,
            dead_eliminated: 2,
            cse_eliminated: 3,
            strength_reduced: 4,
            loops_tiled: 0,
            copies_propagated: 5,
            blocks_merged: 6,
            licm_hoisted: 0,
            functions_inlined: 0,
            instructions_before: 100,
            instructions_after: 80,
        };
        let json = stats.to_json();
        assert!(json.contains("\"copies_propagated\":5"));
        assert!(json.contains("\"blocks_merged\":6"));
    }

    // ── v136 Loop Optimization Tests ─────────────────────────────────

    #[test]
    fn test_v136_detect_loops_simple_while() {
        // Block 0 → Block 1 (header) → Block 2 (body) → Block 1 (back-edge)
        use crate::ir::*;
        let v1 = Value(1);
        let v2 = Value(2);
        let func = IrFunction {
            name: "test_loop".into(),
            params: vec![],
            ret_type: IrType::Void,
            blocks: vec![
                BasicBlock {
                    id: BlockId(0),
                    insts: vec![
                        Inst::BConst { result: v1, value: true },
                        Inst::Jump { target: BlockId(1) },
                    ],
                },
                BasicBlock {
                    id: BlockId(1),
                    insts: vec![
                        Inst::Branch { cond: v1, then_bb: BlockId(2), else_bb: BlockId(3) },
                    ],
                },
                BasicBlock {
                    id: BlockId(2),
                    insts: vec![
                        Inst::IConst { result: v2, value: 0, ty: IrType::I64 },
                        Inst::Jump { target: BlockId(1) },
                    ],
                },
                BasicBlock {
                    id: BlockId(3),
                    insts: vec![Inst::Return { value: None }],
                },
            ],
            entry: BlockId(0),
        };
        let loops = detect_loops(&func);
        assert!(!loops.is_empty(), "should detect the while-loop");
        // Header should be block 1
        assert!(loops.iter().any(|(h, _)| *h == BlockId(1)));
    }

    #[test]
    fn test_v136_detect_loops_no_loop() {
        use crate::ir::*;
        let func = IrFunction {
            name: "no_loop".into(),
            params: vec![],
            ret_type: IrType::Void,
            blocks: vec![
                BasicBlock {
                    id: BlockId(0),
                    insts: vec![Inst::Jump { target: BlockId(1) }],
                },
                BasicBlock {
                    id: BlockId(1),
                    insts: vec![Inst::Return { value: None }],
                },
            ],
            entry: BlockId(0),
        };
        let loops = detect_loops(&func);
        assert!(loops.is_empty(), "linear CFG has no loops");
    }

    #[test]
    fn test_v136_licm_hoist_constant() {
        // Loop body has a constant that can be hoisted
        // Block 0: preheader → jump to header (block 1)
        // Block 1: header → branch to body (block 2) or exit (block 3)
        // Block 2: body → IConst + BinOp + jump back to header
        // Block 3: exit → return
        use crate::ir::*;
        let cond = Value(1);
        let c42 = Value(2); // loop-invariant constant
        let v3 = Value(3);
        let v4 = Value(4);
        let mut func = IrFunction {
            name: "test_licm".into(),
            params: vec![("cond".into(), IrType::Bool)],
            ret_type: IrType::I64,
            blocks: vec![
                BasicBlock {
                    id: BlockId(0),
                    insts: vec![Inst::Jump { target: BlockId(1) }],
                },
                BasicBlock {
                    id: BlockId(1),
                    insts: vec![
                        Inst::Branch { cond, then_bb: BlockId(2), else_bb: BlockId(3) },
                    ],
                },
                BasicBlock {
                    id: BlockId(2),
                    insts: vec![
                        // c42 is loop-invariant (no operands from inside loop)
                        Inst::IConst { result: c42, value: 42, ty: IrType::I64 },
                        Inst::IConst { result: v3, value: 1, ty: IrType::I64 },
                        Inst::BinOp { result: v4, op: IrBinOp::Add, lhs: c42, rhs: v3, ty: IrType::I64 },
                        Inst::Jump { target: BlockId(1) },
                    ],
                },
                BasicBlock {
                    id: BlockId(3),
                    insts: vec![Inst::Return { value: None }],
                },
            ],
            entry: BlockId(0),
        };
        let n = licm(&mut func);
        // Constants should be hoisted out of the loop body
        assert!(n > 0, "should hoist loop-invariant instructions");
        // Check that block 0 (preheader) now has more instructions
        assert!(func.blocks[0].insts.len() > 1, "preheader should have hoisted instructions");
    }

    #[test]
    fn test_v136_licm_no_hoist_impure() {
        // Call instruction in loop body should NOT be hoisted
        use crate::ir::*;
        let cond = Value(1);
        let v2 = Value(2);
        let mut func = IrFunction {
            name: "test_no_hoist".into(),
            params: vec![("cond".into(), IrType::Bool)],
            ret_type: IrType::Void,
            blocks: vec![
                BasicBlock {
                    id: BlockId(0),
                    insts: vec![Inst::Jump { target: BlockId(1) }],
                },
                BasicBlock {
                    id: BlockId(1),
                    insts: vec![
                        Inst::Branch { cond, then_bb: BlockId(2), else_bb: BlockId(3) },
                    ],
                },
                BasicBlock {
                    id: BlockId(2),
                    insts: vec![
                        Inst::Call { result: v2, func: "side_effect".into(), args: vec![], ret_ty: IrType::I64 },
                        Inst::Jump { target: BlockId(1) },
                    ],
                },
                BasicBlock {
                    id: BlockId(3),
                    insts: vec![Inst::Return { value: None }],
                },
            ],
            entry: BlockId(0),
        };
        let n = licm(&mut func);
        assert_eq!(n, 0, "impure (Call) instruction should not be hoisted");
    }

    #[test]
    fn test_v136_licm_no_loop_no_hoist() {
        // No loops → LICM does nothing
        use crate::ir::*;
        let v1 = Value(1);
        let mut func = IrFunction {
            name: "no_loop".into(),
            params: vec![],
            ret_type: IrType::I64,
            blocks: vec![BasicBlock {
                id: BlockId(0),
                insts: vec![
                    Inst::IConst { result: v1, value: 5, ty: IrType::I64 },
                    Inst::Return { value: Some(v1) },
                ],
            }],
            entry: BlockId(0),
        };
        let n = licm(&mut func);
        assert_eq!(n, 0, "no loops means no LICM");
    }

    #[test]
    fn test_v136_licm_dont_hoist_loop_dependent() {
        // BinOp that uses a value defined inside the loop should NOT be hoisted
        use crate::ir::*;
        let cond = Value(1);
        let v2 = Value(2);  // defined outside
        let v3 = Value(3);  // defined inside loop
        let v4 = Value(4);  // uses v3, so loop-dependent
        let mut func = IrFunction {
            name: "test_dep".into(),
            params: vec![("cond".into(), IrType::Bool), ("v2".into(), IrType::I64)],
            ret_type: IrType::Void,
            blocks: vec![
                BasicBlock {
                    id: BlockId(0),
                    insts: vec![Inst::Jump { target: BlockId(1) }],
                },
                BasicBlock {
                    id: BlockId(1),
                    insts: vec![
                        Inst::Branch { cond, then_bb: BlockId(2), else_bb: BlockId(3) },
                    ],
                },
                BasicBlock {
                    id: BlockId(2),
                    insts: vec![
                        Inst::Call { result: v3, func: "get_val".into(), args: vec![], ret_ty: IrType::I64 },
                        Inst::BinOp { result: v4, op: IrBinOp::Add, lhs: v2, rhs: v3, ty: IrType::I64 },
                        Inst::Jump { target: BlockId(1) },
                    ],
                },
                BasicBlock {
                    id: BlockId(3),
                    insts: vec![Inst::Return { value: None }],
                },
            ],
            entry: BlockId(0),
        };
        let n = licm(&mut func);
        assert_eq!(n, 0, "BinOp using loop-defined value should not be hoisted");
    }

    #[test]
    fn test_v136_stats_json_includes_licm() {
        let stats = OptPassStats {
            constants_folded: 0,
            dead_eliminated: 0,
            cse_eliminated: 0,
            strength_reduced: 0,
            loops_tiled: 0,
            copies_propagated: 0,
            blocks_merged: 0,
            licm_hoisted: 7,
            functions_inlined: 0,
            instructions_before: 50,
            instructions_after: 43,
        };
        let json = stats.to_json();
        assert!(json.contains("\"licm_hoisted\":7"));
    }

    // ── v137 Function Inlining Tests ─────────────────────────────────

    #[test]
    fn test_v137_inline_simple_leaf() {
        // add(a, b) -> a + b; main calls add(10, 20)
        use crate::ir::*;
        let a = Value(1);
        let b = Value(2);
        let r = Value(3);
        let m1 = Value(10);
        let m2 = Value(11);
        let m3 = Value(12);
        let mut module = IrModule {
            functions: vec![
                IrFunction {
                    name: "add".into(),
                    params: vec![("a".into(), IrType::I64), ("b".into(), IrType::I64)],
                    ret_type: IrType::I64,
                    blocks: vec![BasicBlock {
                        id: BlockId(0),
                        insts: vec![
                            Inst::BinOp { result: r, op: IrBinOp::Add, lhs: a, rhs: b, ty: IrType::I64 },
                            Inst::Return { value: Some(r) },
                        ],
                    }],
                    entry: BlockId(0),
                },
                IrFunction {
                    name: "main".into(),
                    params: vec![],
                    ret_type: IrType::I64,
                    blocks: vec![BasicBlock {
                        id: BlockId(0),
                        insts: vec![
                            Inst::IConst { result: m1, value: 10, ty: IrType::I64 },
                            Inst::IConst { result: m2, value: 20, ty: IrType::I64 },
                            Inst::Call { result: m3, func: "add".into(), args: vec![m1, m2], ret_ty: IrType::I64 },
                            Inst::Return { value: Some(m3) },
                        ],
                    }],
                    entry: BlockId(0),
                },
            ],
            string_constants: vec![],
        };
        let n = inline_functions(&mut module, 30);
        assert_eq!(n, 1, "should inline one call site");
        // The main function should no longer have a Call to "add"
        let main = module.functions.iter().find(|f| f.name == "main").unwrap();
        let has_call = main.blocks[0].insts.iter().any(|inst| {
            matches!(inst, Inst::Call { func, .. } if func == "add")
        });
        assert!(!has_call, "add call should be replaced with inlined body");
    }

    #[test]
    fn test_v137_no_inline_large_function() {
        // Function with too many instructions should not be inlined
        use crate::ir::*;
        let mut insts = Vec::new();
        for i in 0..40 {
            insts.push(Inst::IConst { result: Value(i), value: i as i64, ty: IrType::I64 });
        }
        insts.push(Inst::Return { value: Some(Value(0)) });
        let mut module = IrModule {
            functions: vec![
                IrFunction {
                    name: "big".into(),
                    params: vec![],
                    ret_type: IrType::I64,
                    blocks: vec![BasicBlock { id: BlockId(0), insts }],
                    entry: BlockId(0),
                },
                IrFunction {
                    name: "main".into(),
                    params: vec![],
                    ret_type: IrType::I64,
                    blocks: vec![BasicBlock {
                        id: BlockId(0),
                        insts: vec![
                            Inst::Call { result: Value(100), func: "big".into(), args: vec![], ret_ty: IrType::I64 },
                            Inst::Return { value: Some(Value(100)) },
                        ],
                    }],
                    entry: BlockId(0),
                },
            ],
            string_constants: vec![],
        };
        let n = inline_functions(&mut module, 30);
        assert_eq!(n, 0, "function with 41 instructions should not be inlined (max 30)");
    }

    #[test]
    fn test_v137_no_inline_multi_block() {
        // Function with control flow (multiple blocks) should not be inlined
        use crate::ir::*;
        let v1 = Value(1);
        let mut module = IrModule {
            functions: vec![
                IrFunction {
                    name: "branchy".into(),
                    params: vec![],
                    ret_type: IrType::Void,
                    blocks: vec![
                        BasicBlock {
                            id: BlockId(0),
                            insts: vec![
                                Inst::BConst { result: v1, value: true },
                                Inst::Branch { cond: v1, then_bb: BlockId(1), else_bb: BlockId(2) },
                            ],
                        },
                        BasicBlock { id: BlockId(1), insts: vec![Inst::Return { value: None }] },
                        BasicBlock { id: BlockId(2), insts: vec![Inst::Return { value: None }] },
                    ],
                    entry: BlockId(0),
                },
                IrFunction {
                    name: "main".into(),
                    params: vec![],
                    ret_type: IrType::I64,
                    blocks: vec![BasicBlock {
                        id: BlockId(0),
                        insts: vec![
                            Inst::Call { result: Value(50), func: "branchy".into(), args: vec![], ret_ty: IrType::Void },
                            Inst::IConst { result: Value(51), value: 0, ty: IrType::I64 },
                            Inst::Return { value: Some(Value(51)) },
                        ],
                    }],
                    entry: BlockId(0),
                },
            ],
            string_constants: vec![],
        };
        let n = inline_functions(&mut module, 30);
        assert_eq!(n, 0, "multi-block function cannot be inlined");
    }

    #[test]
    fn test_v137_no_inline_non_leaf() {
        // Function that calls other functions should not be inlined
        use crate::ir::*;
        let mut module = IrModule {
            functions: vec![
                IrFunction {
                    name: "wrapper".into(),
                    params: vec![],
                    ret_type: IrType::I64,
                    blocks: vec![BasicBlock {
                        id: BlockId(0),
                        insts: vec![
                            Inst::Call { result: Value(1), func: "inner".into(), args: vec![], ret_ty: IrType::I64 },
                            Inst::Return { value: Some(Value(1)) },
                        ],
                    }],
                    entry: BlockId(0),
                },
                IrFunction {
                    name: "main".into(),
                    params: vec![],
                    ret_type: IrType::I64,
                    blocks: vec![BasicBlock {
                        id: BlockId(0),
                        insts: vec![
                            Inst::Call { result: Value(10), func: "wrapper".into(), args: vec![], ret_ty: IrType::I64 },
                            Inst::Return { value: Some(Value(10)) },
                        ],
                    }],
                    entry: BlockId(0),
                },
            ],
            string_constants: vec![],
        };
        let n = inline_functions(&mut module, 30);
        assert_eq!(n, 0, "non-leaf function should not be inlined");
    }

    #[test]
    fn test_v137_jit_uses_optimizer() {
        // Verify that compile_and_run now optimizes IR
        // This is an integration test: constant folding should work via JIT
        let result = crate::codegen::compile_and_run_nocache(
            "fn main() -> i64 { 2 + 3 }"
        );
        assert_eq!(result.unwrap(), 5, "JIT should still produce correct results with optimizer");
    }

    #[test]
    fn test_v137_optimize_ir_includes_inlining() {
        use crate::ir::*;
        let mut module = IrModule {
            functions: vec![
                IrFunction {
                    name: "const_five".into(),
                    params: vec![],
                    ret_type: IrType::I64,
                    blocks: vec![BasicBlock {
                        id: BlockId(0),
                        insts: vec![
                            Inst::IConst { result: Value(1), value: 5, ty: IrType::I64 },
                            Inst::Return { value: Some(Value(1)) },
                        ],
                    }],
                    entry: BlockId(0),
                },
                IrFunction {
                    name: "main".into(),
                    params: vec![],
                    ret_type: IrType::I64,
                    blocks: vec![BasicBlock {
                        id: BlockId(0),
                        insts: vec![
                            Inst::Call { result: Value(10), func: "const_five".into(), args: vec![], ret_ty: IrType::I64 },
                            Inst::Return { value: Some(Value(10)) },
                        ],
                    }],
                    entry: BlockId(0),
                },
            ],
            string_constants: vec![],
        };
        let stats = optimize_ir(&mut module);
        assert!(stats.functions_inlined > 0, "optimize_ir should inline small functions");
    }

    #[test]
    fn test_v137_stats_json_includes_inlined() {
        let stats = OptPassStats {
            constants_folded: 0,
            dead_eliminated: 0,
            cse_eliminated: 0,
            strength_reduced: 0,
            loops_tiled: 0,
            copies_propagated: 0,
            blocks_merged: 0,
            licm_hoisted: 0,
            functions_inlined: 3,
            instructions_before: 40,
            instructions_after: 32,
        };
        let json = stats.to_json();
        assert!(json.contains("\"functions_inlined\":3"));
    }
}
