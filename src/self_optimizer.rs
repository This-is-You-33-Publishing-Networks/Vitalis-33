//! Self Optimizer — RL-based pass ordering, cost models, and auto-tuning.
//!
//! Provides reinforcement learning for compiler optimization pass ordering,
//! cost models for performance prediction, auto-tuning via Bayesian optimization,
//! adaptive compilation strategies, and real IR analysis / pass dispatch.

use std::collections::HashMap;
use crate::ir::{IrModule, Inst, BlockId};

// ── Optimization Pass ───────────────────────────────────────────────────

/// Compiler optimization pass identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OptPass {
    ConstantFolding,
    DeadCodeElimination,
    CommonSubexprElimination,
    LoopInvariantCodeMotion,
    Inlining,
    StrengthReduction,
    TailCallOptimization,
    RegisterAllocation,
    InstructionScheduling,
    LoopUnrolling,
    Vectorization,
    MemoryToRegister,
    AlgebraicSimplification,
    BranchElimination,
    GlobalValueNumbering,
}

impl OptPass {
    pub fn all() -> &'static [OptPass] {
        &[
            OptPass::ConstantFolding,
            OptPass::DeadCodeElimination,
            OptPass::CommonSubexprElimination,
            OptPass::LoopInvariantCodeMotion,
            OptPass::Inlining,
            OptPass::StrengthReduction,
            OptPass::TailCallOptimization,
            OptPass::RegisterAllocation,
            OptPass::InstructionScheduling,
            OptPass::LoopUnrolling,
            OptPass::Vectorization,
            OptPass::MemoryToRegister,
            OptPass::AlgebraicSimplification,
            OptPass::BranchElimination,
            OptPass::GlobalValueNumbering,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            OptPass::ConstantFolding => "const_fold",
            OptPass::DeadCodeElimination => "dce",
            OptPass::CommonSubexprElimination => "cse",
            OptPass::LoopInvariantCodeMotion => "licm",
            OptPass::Inlining => "inline",
            OptPass::StrengthReduction => "strength_reduce",
            OptPass::TailCallOptimization => "tco",
            OptPass::RegisterAllocation => "regalloc",
            OptPass::InstructionScheduling => "isched",
            OptPass::LoopUnrolling => "unroll",
            OptPass::Vectorization => "vectorize",
            OptPass::MemoryToRegister => "mem2reg",
            OptPass::AlgebraicSimplification => "alg_simp",
            OptPass::BranchElimination => "branch_elim",
            OptPass::GlobalValueNumbering => "gvn",
        }
    }
}

// ── Cost Model ──────────────────────────────────────────────────────────

/// Program features for cost prediction.
#[derive(Debug, Clone, Default)]
pub struct ProgramFeatures {
    pub num_instructions: usize,
    pub num_basic_blocks: usize,
    pub num_branches: usize,
    pub num_loops: usize,
    pub num_calls: usize,
    pub max_loop_depth: usize,
    pub num_memory_ops: usize,
    pub num_arithmetic_ops: usize,
    pub num_phi_nodes: usize,
    pub estimated_register_pressure: usize,
}

/// Cost model for predicting optimization benefit.
#[derive(Debug, Clone)]
pub struct CostModel {
    pub weights: Vec<f64>,
    pub bias: f64,
    pub feature_count: usize,
}

impl CostModel {
    pub fn new(feature_count: usize) -> Self {
        CostModel {
            weights: vec![0.0; feature_count],
            bias: 0.0,
            feature_count,
        }
    }

    /// Predict the cost of a program given its features.
    pub fn predict(&self, features: &[f64]) -> f64 {
        let mut cost = self.bias;
        for (w, f) in self.weights.iter().zip(features.iter()) {
            cost += w * f;
        }
        cost
    }

    /// Train cost model on (features, actual_cost) pairs.
    pub fn train(&mut self, data: &[(Vec<f64>, f64)], learning_rate: f64, epochs: usize) {
        for _ in 0..epochs {
            for (features, target) in data {
                let predicted = self.predict(features);
                let error = predicted - target;

                // SGD update
                self.bias -= learning_rate * error;
                for (i, &f) in features.iter().enumerate() {
                    if i < self.weights.len() {
                        self.weights[i] -= learning_rate * error * f;
                    }
                }
            }
        }
    }

    /// Convert ProgramFeatures to feature vector.
    pub fn features_to_vec(features: &ProgramFeatures) -> Vec<f64> {
        vec![
            features.num_instructions as f64,
            features.num_basic_blocks as f64,
            features.num_branches as f64,
            features.num_loops as f64,
            features.num_calls as f64,
            features.max_loop_depth as f64,
            features.num_memory_ops as f64,
            features.num_arithmetic_ops as f64,
            features.num_phi_nodes as f64,
            features.estimated_register_pressure as f64,
        ]
    }
}

// ── RL Pass Ordering ────────────────────────────────────────────────────

/// Q-learning agent for optimization pass ordering.
#[derive(Debug, Clone)]
pub struct PassOrderingAgent {
    /// Q-table: state → action → value
    pub q_table: HashMap<Vec<usize>, Vec<f64>>,
    pub num_passes: usize,
    pub learning_rate: f64,
    pub discount_factor: f64,
    pub epsilon: f64,
    pub episode: usize,
}

impl PassOrderingAgent {
    pub fn new(num_passes: usize, learning_rate: f64, discount_factor: f64, epsilon: f64) -> Self {
        PassOrderingAgent {
            q_table: HashMap::new(),
            num_passes,
            learning_rate,
            discount_factor,
            epsilon,
            episode: 0,
        }
    }

    /// Get Q-values for a state, initializing if needed.
    fn get_q_values(&self, state: &[usize]) -> Vec<f64> {
        self.q_table.get(state).cloned().unwrap_or_else(|| vec![0.0; self.num_passes])
    }

    /// Select action using epsilon-greedy policy.
    pub fn select_action(&self, state: &[usize], available: &[usize], seed: u64) -> usize {
        // Epsilon-greedy
        let mut rng_state = seed.wrapping_add(self.episode as u64);
        rng_state ^= rng_state << 13;
        rng_state ^= rng_state >> 7;
        rng_state ^= rng_state << 17;
        let r = rng_state as f64 / u64::MAX as f64;

        if r < self.epsilon {
            // Random action
            let idx = (rng_state as usize) % available.len();
            available[idx]
        } else {
            // Greedy
            let q_values = self.get_q_values(state);
            let mut best_action = available[0];
            let mut best_value = f64::NEG_INFINITY;
            for &a in available {
                if a < q_values.len() && q_values[a] > best_value {
                    best_value = q_values[a];
                    best_action = a;
                }
            }
            best_action
        }
    }

    /// Update Q-value after observing reward.
    pub fn update(&mut self, state: &[usize], action: usize, reward: f64, next_state: &[usize]) {
        let next_q = self.get_q_values(next_state);
        let max_next_q = next_q.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

        let q_values = self.q_table.entry(state.to_vec()).or_insert_with(|| vec![0.0; self.num_passes]);
        if action < q_values.len() {
            let td_target = reward + self.discount_factor * max_next_q;
            q_values[action] += self.learning_rate * (td_target - q_values[action]);
        }
    }

    /// Decay epsilon for exploration reduction.
    pub fn decay_epsilon(&mut self, min_epsilon: f64, decay_rate: f64) {
        self.epsilon = (self.epsilon * decay_rate).max(min_epsilon);
        self.episode += 1;
    }

    /// Get best known pass ordering for a given state.
    pub fn best_ordering(&self, initial_state: &[usize], max_passes: usize) -> Vec<usize> {
        let mut state = initial_state.to_vec();
        let mut ordering = Vec::new();
        let mut used = std::collections::HashSet::new();

        for _ in 0..max_passes {
            let q_values = self.get_q_values(&state);
            let available: Vec<usize> = (0..self.num_passes)
                .filter(|a| !used.contains(a))
                .collect();
            if available.is_empty() { break; }

            let mut best_action = available[0];
            let mut best_value = f64::NEG_INFINITY;
            for &a in &available {
                if a < q_values.len() && q_values[a] > best_value {
                    best_value = q_values[a];
                    best_action = a;
                }
            }

            ordering.push(best_action);
            used.insert(best_action);
            state.push(best_action);
        }
        ordering
    }
}

// ── Bayesian Optimization for Auto-Tuning ───────────────────────────────

/// Simple Bayesian optimization using upper confidence bound (UCB).
#[derive(Debug, Clone)]
pub struct BayesianOptimizer {
    pub observations: Vec<(Vec<f64>, f64)>, // (params, objective)
    pub param_bounds: Vec<(f64, f64)>,
    pub exploration_weight: f64,
    pub best_params: Vec<f64>,
    pub best_objective: f64,
}

impl BayesianOptimizer {
    pub fn new(param_bounds: Vec<(f64, f64)>, exploration_weight: f64) -> Self {
        BayesianOptimizer {
            observations: Vec::new(),
            param_bounds: param_bounds.clone(),
            exploration_weight,
            best_params: param_bounds.iter().map(|(lo, hi)| (lo + hi) / 2.0).collect(),
            best_objective: f64::NEG_INFINITY,
        }
    }

    /// Record an observation.
    pub fn observe(&mut self, params: Vec<f64>, objective: f64) {
        if objective > self.best_objective {
            self.best_objective = objective;
            self.best_params = params.clone();
        }
        self.observations.push((params, objective));
    }

    /// Suggest next parameter point to evaluate (using simplified UCB).
    pub fn suggest(&self, seed: u64) -> Vec<f64> {
        let n_candidates = 100;
        let mut best_candidate = self.best_params.clone();
        let mut best_ucb = f64::NEG_INFINITY;

        let mut rng_state = seed;
        for _ in 0..n_candidates {
            // Random candidate within bounds
            let candidate: Vec<f64> = self.param_bounds.iter().map(|&(lo, hi)| {
                rng_state ^= rng_state << 13;
                rng_state ^= rng_state >> 7;
                rng_state ^= rng_state << 17;
                let r = rng_state as f64 / u64::MAX as f64;
                lo + r * (hi - lo)
            }).collect();

            // Compute UCB: mean + exploration * uncertainty
            let (mean, variance) = self.predict(&candidate);
            let ucb = mean + self.exploration_weight * variance.sqrt();

            if ucb > best_ucb {
                best_ucb = ucb;
                best_candidate = candidate;
            }
        }
        best_candidate
    }

    /// Simple kernel-based prediction (nearest neighbor weighted).
    fn predict(&self, params: &[f64]) -> (f64, f64) {
        if self.observations.is_empty() {
            return (0.0, 1.0);
        }

        let mut weighted_sum = 0.0;
        let mut weight_total = 0.0;
        let mut sq_sum = 0.0;

        for (obs_params, obs_val) in &self.observations {
            let dist: f64 = params.iter().zip(obs_params.iter())
                .map(|(a, b)| (a - b) * (a - b))
                .sum::<f64>()
                .sqrt();
            let weight = (-dist * 2.0).exp(); // RBF kernel
            weighted_sum += weight * obs_val;
            sq_sum += weight * obs_val * obs_val;
            weight_total += weight;
        }

        if weight_total > 0.0 {
            let mean = weighted_sum / weight_total;
            let variance = (sq_sum / weight_total - mean * mean).max(0.001);
            (mean, variance)
        } else {
            (0.0, 1.0)
        }
    }
}

// ── Adaptive Compilation ────────────────────────────────────────────────

/// Compilation tier for tiered JIT.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CompilationTier {
    Interpreted,
    BaselineJIT,
    OptimizedJIT,
    FullyOptimized,
}

/// Compilation strategy decision.
#[derive(Debug, Clone)]
pub struct CompilationDecision {
    pub tier: CompilationTier,
    pub passes: Vec<OptPass>,
    pub estimated_speedup: f64,
    pub estimated_compile_time_ms: f64,
}

/// Adaptive compilation manager.
#[derive(Debug, Clone)]
pub struct AdaptiveCompiler {
    pub hot_functions: HashMap<String, usize>, // function → call count
    pub tier_thresholds: [usize; 3],
    pub history: Vec<(String, CompilationTier, f64)>, // (func, tier, runtime)
}

impl AdaptiveCompiler {
    pub fn new() -> Self {
        AdaptiveCompiler {
            hot_functions: HashMap::new(),
            tier_thresholds: [10, 100, 1000], // baseline, optimized, fully-optimized
            history: Vec::new(),
        }
    }

    /// Record a function call and return recommended compilation tier.
    pub fn record_call(&mut self, function: &str) -> CompilationTier {
        let count = self.hot_functions.entry(function.to_string()).or_insert(0);
        *count += 1;

        if *count >= self.tier_thresholds[2] {
            CompilationTier::FullyOptimized
        } else if *count >= self.tier_thresholds[1] {
            CompilationTier::OptimizedJIT
        } else if *count >= self.tier_thresholds[0] {
            CompilationTier::BaselineJIT
        } else {
            CompilationTier::Interpreted
        }
    }

    /// Get recommended pass sequence for a tier.
    pub fn passes_for_tier(&self, tier: CompilationTier) -> Vec<OptPass> {
        match tier {
            CompilationTier::Interpreted => vec![],
            CompilationTier::BaselineJIT => vec![
                OptPass::ConstantFolding,
                OptPass::DeadCodeElimination,
            ],
            CompilationTier::OptimizedJIT => vec![
                OptPass::ConstantFolding,
                OptPass::DeadCodeElimination,
                OptPass::CommonSubexprElimination,
                OptPass::Inlining,
                OptPass::MemoryToRegister,
            ],
            CompilationTier::FullyOptimized => vec![
                OptPass::ConstantFolding,
                OptPass::DeadCodeElimination,
                OptPass::CommonSubexprElimination,
                OptPass::Inlining,
                OptPass::MemoryToRegister,
                OptPass::LoopInvariantCodeMotion,
                OptPass::LoopUnrolling,
                OptPass::GlobalValueNumbering,
                OptPass::Vectorization,
                OptPass::InstructionScheduling,
                OptPass::RegisterAllocation,
            ],
        }
    }

    /// Decide compilation strategy for a function.
    pub fn decide(&mut self, function: &str) -> CompilationDecision {
        let tier = self.record_call(function);
        let passes = self.passes_for_tier(tier);
        let estimated_speedup = match tier {
            CompilationTier::Interpreted => 1.0,
            CompilationTier::BaselineJIT => 5.0,
            CompilationTier::OptimizedJIT => 20.0,
            CompilationTier::FullyOptimized => 50.0,
        };
        let estimated_compile_time_ms = match tier {
            CompilationTier::Interpreted => 0.0,
            CompilationTier::BaselineJIT => 1.0,
            CompilationTier::OptimizedJIT => 10.0,
            CompilationTier::FullyOptimized => 100.0,
        };

        CompilationDecision { tier, passes, estimated_speedup, estimated_compile_time_ms }
    }
}

impl Default for AdaptiveCompiler {
    fn default() -> Self { Self::new() }
}

// ── Real IR Analysis & Pass Dispatch ────────────────────────────────────

/// Extract ProgramFeatures from a real IrModule by analyzing its IR.
pub fn extract_features(module: &IrModule) -> ProgramFeatures {
    let mut features = ProgramFeatures::default();

    for func in &module.functions {
        features.num_basic_blocks += func.blocks.len();
        for block in &func.blocks {
            features.num_instructions += block.insts.len();
            for inst in &block.insts {
                match inst {
                    Inst::Branch { .. } => features.num_branches += 1,
                    Inst::Jump { .. } => {} // unconditional — not a branch decision
                    Inst::Call { .. } => features.num_calls += 1,
                    Inst::Load { .. } | Inst::Store { .. }
                    | Inst::Alloca { .. } => features.num_memory_ops += 1,
                    Inst::BinOp { .. } | Inst::UnOp { .. } => features.num_arithmetic_ops += 1,
                    Inst::Phi { .. } => features.num_phi_nodes += 1,
                    _ => {}
                }
            }
        }
    }

    // Estimate register pressure: max live values across any block
    for func in &module.functions {
        for block in &func.blocks {
            let mut defs = 0usize;
            for inst in &block.insts {
                // Count instructions that define a new value
                match inst {
                    Inst::IConst { .. } | Inst::FConst { .. } | Inst::BConst { .. }
                    | Inst::StrConst { .. } | Inst::BinOp { .. } | Inst::UnOp { .. }
                    | Inst::ICmp { .. } | Inst::FCmp { .. } | Inst::Phi { .. }
                    | Inst::Copy { .. } | Inst::Load { .. } | Inst::Alloca { .. }
                    | Inst::Call { .. } | Inst::ArrayAlloc { .. } | Inst::ArrayGet { .. }
                    | Inst::ArrayLen { .. } | Inst::StructAlloc { .. }
                    | Inst::FieldGet { .. } | Inst::ClosureAlloc { .. } => {
                        defs += 1;
                    }
                    _ => {}
                }
            }
            if defs > features.estimated_register_pressure {
                features.estimated_register_pressure = defs;
            }
        }
    }

    // Simple loop detection: count back-edges (branch/jump to earlier block)
    for func in &module.functions {
        let block_ids: Vec<BlockId> = func.blocks.iter().map(|b| b.id).collect();
        for (idx, block) in func.blocks.iter().enumerate() {
            for inst in &block.insts {
                let targets: Vec<BlockId> = match inst {
                    Inst::Jump { target } => vec![*target],
                    Inst::Branch { then_bb, else_bb, .. } => vec![*then_bb, *else_bb],
                    _ => vec![],
                };
                for target in targets {
                    // Back-edge: target appears before current block
                    if let Some(target_idx) = block_ids.iter().position(|&b| b == target) {
                        if target_idx <= idx {
                            features.num_loops += 1;
                        }
                    }
                }
            }
        }
    }

    features
}

/// Apply a single optimization pass to an IrModule, returning number of changes.
/// Dispatches to real optimizer.rs functions for implemented passes.
pub fn apply_pass(pass: OptPass, module: &mut IrModule) -> u32 {
    let mut changes = 0u32;
    match pass {
        OptPass::ConstantFolding => {
            for func in &mut module.functions {
                changes += crate::optimizer::constant_fold(func);
            }
        }
        OptPass::DeadCodeElimination => {
            for func in &mut module.functions {
                changes += crate::optimizer::dead_code_eliminate(func);
            }
        }
        OptPass::CommonSubexprElimination => {
            for func in &mut module.functions {
                changes += crate::optimizer::cse(func);
            }
        }
        OptPass::StrengthReduction => {
            for func in &mut module.functions {
                changes += crate::optimizer::strength_reduce(func);
            }
        }
        // Passes without real implementations yet — no-op
        _ => {}
    }
    changes
}

/// Apply a sequence of passes and return total changes per pass.
pub fn apply_pass_sequence(passes: &[OptPass], module: &mut IrModule) -> Vec<(OptPass, u32)> {
    passes.iter().map(|&pass| {
        let changes = apply_pass(pass, module);
        (pass, changes)
    }).collect()
}

/// Measure the improvement ratio between features before and after optimization.
/// Returns a score in [0, 1] where higher = more improvement.
pub fn measure_improvement(before: &ProgramFeatures, after: &ProgramFeatures) -> f64 {
    if before.num_instructions == 0 {
        return 0.0;
    }
    // Reduction in instruction count (primary metric)
    let inst_reduction = if after.num_instructions < before.num_instructions {
        (before.num_instructions - after.num_instructions) as f64 / before.num_instructions as f64
    } else {
        0.0
    };
    // Reduction in register pressure
    let reg_reduction = if before.estimated_register_pressure > 0
        && after.estimated_register_pressure < before.estimated_register_pressure
    {
        (before.estimated_register_pressure - after.estimated_register_pressure) as f64
            / before.estimated_register_pressure as f64
    } else {
        0.0
    };
    // Weighted: 70% instruction reduction + 30% register pressure reduction
    (inst_reduction * 0.7 + reg_reduction * 0.3).min(1.0)
}

/// Run the RL agent to find an optimized pass ordering for a module.
/// Trains the agent over `episodes` using feature-based simulation
/// and returns the best ordering found.
pub fn rl_optimize(module: &IrModule, episodes: usize, seed: u64) -> Vec<OptPass> {
    let implemented = [
        OptPass::ConstantFolding,
        OptPass::DeadCodeElimination,
        OptPass::CommonSubexprElimination,
        OptPass::StrengthReduction,
    ];
    let num_passes = implemented.len();
    let mut agent = PassOrderingAgent::new(num_passes, 0.1, 0.9, 0.3);

    let features = extract_features(module);
    let initial_state = vec![features.num_instructions, features.num_basic_blocks];

    for ep in 0..episodes {
        let mut state = initial_state.clone();
        let mut available: Vec<usize> = (0..num_passes).collect();

        // Simulate pass ordering using feature estimates (no module mutation)
        let mut simulated_insts = features.num_instructions;
        let simulated_blocks = features.num_basic_blocks;

        for _ in 0..num_passes {
            if available.is_empty() { break; }
            let action = agent.select_action(&state, &available, seed.wrapping_add(ep as u64));

            // Estimate reward based on pass type and current features
            let reward = match implemented[action] {
                OptPass::ConstantFolding => {
                    let r = (features.num_arithmetic_ops as f64 * 0.1).min(0.5);
                    simulated_insts = simulated_insts.saturating_sub((r * simulated_insts as f64) as usize);
                    r
                }
                OptPass::DeadCodeElimination => {
                    let r = 0.15_f64.min(simulated_insts as f64 * 0.01);
                    simulated_insts = simulated_insts.saturating_sub(1);
                    r
                }
                OptPass::CommonSubexprElimination => {
                    let r = (features.num_arithmetic_ops as f64 * 0.05).min(0.3);
                    simulated_insts = simulated_insts.saturating_sub((r * simulated_insts as f64) as usize);
                    r
                }
                OptPass::StrengthReduction => {
                    let r = (features.num_arithmetic_ops as f64 * 0.08).min(0.4);
                    simulated_insts = simulated_insts.saturating_sub((r * simulated_insts as f64) as usize);
                    r
                }
                _ => 0.0,
            };

            let next_state = vec![simulated_insts, simulated_blocks];
            agent.update(&state, action, reward, &next_state);
            state = next_state;
            available.retain(|&a| a != action);
        }

        agent.decay_epsilon(0.01, 0.95);
    }

    // Get best ordering from trained agent
    let ordering_indices = agent.best_ordering(&initial_state, num_passes);
    ordering_indices.iter()
        .filter_map(|&i| implemented.get(i).copied())
        .collect()
}

// ── FFI Interface ───────────────────────────────────────────────────────

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_selfopt_cost_predict(features: *const f64, feature_count: i64, weights: *const f64, bias: f64) -> f64 {
    let f = unsafe { std::slice::from_raw_parts(features, feature_count as usize) };
    let w = unsafe { std::slice::from_raw_parts(weights, feature_count as usize) };
    let model = CostModel { weights: w.to_vec(), bias, feature_count: feature_count as usize };
    model.predict(f)
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_selfopt_num_passes() -> i64 {
    OptPass::all().len() as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_selfopt_tier(call_count: i64) -> i64 {
    if call_count >= 1000 { 3 }
    else if call_count >= 100 { 2 }
    else if call_count >= 10 { 1 }
    else { 0 }
}

// ── Tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_opt_pass_count() {
        assert_eq!(OptPass::all().len(), 15);
    }

    #[test]
    fn test_cost_model_predict() {
        let mut model = CostModel::new(3);
        model.weights = vec![1.0, 2.0, 3.0];
        model.bias = 0.5;
        assert!((model.predict(&[1.0, 1.0, 1.0]) - 6.5).abs() < 1e-10);
    }

    #[test]
    fn test_cost_model_train() {
        let mut model = CostModel::new(2);
        let data = vec![
            (vec![1.0, 0.0], 2.0),
            (vec![0.0, 1.0], 3.0),
            (vec![1.0, 1.0], 5.0),
        ];
        model.train(&data, 0.01, 100);
        let pred = model.predict(&[1.0, 1.0]);
        assert!((pred - 5.0).abs() < 1.0); // Should be approximately 5.0
    }

    #[test]
    fn test_features_to_vec() {
        let features = ProgramFeatures {
            num_instructions: 100,
            num_basic_blocks: 10,
            ..Default::default()
        };
        let vec = CostModel::features_to_vec(&features);
        assert_eq!(vec.len(), 10);
        assert_eq!(vec[0], 100.0);
        assert_eq!(vec[1], 10.0);
    }

    #[test]
    fn test_pass_ordering_agent() {
        let mut agent = PassOrderingAgent::new(5, 0.1, 0.9, 0.1);
        let state = vec![0, 1];
        let action = agent.select_action(&state, &[2, 3, 4], 42);
        assert!([2, 3, 4].contains(&action));

        // Update Q-value
        agent.update(&state, action, 1.0, &[0, 1, action]);
    }

    #[test]
    fn test_pass_ordering_best() {
        let mut agent = PassOrderingAgent::new(3, 0.5, 0.9, 0.0); // No exploration
        // Set Q-values manually
        agent.q_table.insert(vec![], vec![3.0, 1.0, 2.0]);
        agent.q_table.insert(vec![0], vec![0.0, 1.0, 5.0]);

        let ordering = agent.best_ordering(&[], 2);
        assert_eq!(ordering[0], 0); // Highest Q in empty state
        assert_eq!(ordering[1], 2); // Highest Q after choosing 0
    }

    #[test]
    fn test_epsilon_decay() {
        let mut agent = PassOrderingAgent::new(5, 0.1, 0.9, 1.0);
        agent.decay_epsilon(0.01, 0.95);
        assert!((agent.epsilon - 0.95).abs() < 1e-10);
        for _ in 0..100 {
            agent.decay_epsilon(0.01, 0.95);
        }
        assert!(agent.epsilon >= 0.01);
    }

    #[test]
    fn test_bayesian_optimizer() {
        let bounds = vec![(0.0, 1.0), (0.0, 10.0)];
        let mut opt = BayesianOptimizer::new(bounds, 2.0);

        opt.observe(vec![0.5, 5.0], 10.0);
        opt.observe(vec![0.3, 7.0], 15.0);

        let suggestion = opt.suggest(42);
        assert_eq!(suggestion.len(), 2);
        assert!(suggestion[0] >= 0.0 && suggestion[0] <= 1.0);
        assert!(suggestion[1] >= 0.0 && suggestion[1] <= 10.0);
    }

    #[test]
    fn test_bayesian_best_tracking() {
        let bounds = vec![(0.0, 1.0)];
        let mut opt = BayesianOptimizer::new(bounds, 1.0);
        opt.observe(vec![0.1], 5.0);
        opt.observe(vec![0.9], 20.0);
        opt.observe(vec![0.5], 10.0);
        assert!((opt.best_objective - 20.0).abs() < 1e-10);
        assert!((opt.best_params[0] - 0.9).abs() < 1e-10);
    }

    #[test]
    fn test_adaptive_compiler_tiers() {
        let mut compiler = AdaptiveCompiler::new();
        for _ in 0..9 {
            assert_eq!(compiler.decide("f").tier, CompilationTier::Interpreted);
        }
        assert_eq!(compiler.decide("f").tier, CompilationTier::BaselineJIT);
        for _ in 0..90 {
            compiler.decide("f");
        }
        assert_eq!(compiler.decide("f").tier, CompilationTier::OptimizedJIT);
    }

    #[test]
    fn test_adaptive_pass_sequences() {
        let compiler = AdaptiveCompiler::new();
        assert_eq!(compiler.passes_for_tier(CompilationTier::Interpreted).len(), 0);
        assert_eq!(compiler.passes_for_tier(CompilationTier::BaselineJIT).len(), 2);
        assert!(compiler.passes_for_tier(CompilationTier::FullyOptimized).len() > 5);
    }

    #[test]
    fn test_compilation_decision() {
        let mut compiler = AdaptiveCompiler::new();
        for _ in 0..15 {
            compiler.decide("hot_func");
        }
        let decision = compiler.decide("hot_func");
        assert!(decision.estimated_speedup > 1.0);
        assert!(decision.passes.len() > 0);
    }

    #[test]
    fn test_ffi_cost_predict() {
        let features = [10.0f64, 20.0];
        let weights = [1.0f64, 0.5];
        let result = vitalis_selfopt_cost_predict(features.as_ptr(), 2, weights.as_ptr(), 1.0);
        assert!((result - 21.0).abs() < 1e-10); // 10*1 + 20*0.5 + 1.0
    }

    #[test]
    fn test_ffi_num_passes() {
        assert_eq!(vitalis_selfopt_num_passes(), 15);
    }

    #[test]
    fn test_ffi_tier() {
        assert_eq!(vitalis_selfopt_tier(5), 0);
        assert_eq!(vitalis_selfopt_tier(50), 1);
        assert_eq!(vitalis_selfopt_tier(500), 2);
        assert_eq!(vitalis_selfopt_tier(5000), 3);
    }

    #[test]
    fn test_pass_names() {
        for pass in OptPass::all() {
            assert!(!pass.name().is_empty());
        }
    }

    // ── v68: Real pass wiring tests ──────────────────────────────────────

    #[test]
    fn test_extract_features_empty_module() {
        let module = crate::ir::IrModule::new();
        let features = super::extract_features(&module);
        assert_eq!(features.num_instructions, 0);
        assert_eq!(features.num_basic_blocks, 0);
    }

    #[test]
    fn test_extract_features_with_ir() {
        use crate::ir::*;
        let mut module = IrModule::new();
        let mut func = IrFunction {
            name: "test".to_string(),
            params: vec![],
            ret_type: IrType::I64,
            blocks: vec![],
            entry: BlockId(0),
        };
        let mut block = BasicBlock::new(BlockId(0));
        block.insts.push(Inst::IConst { result: Value(0), value: 10, ty: IrType::I64 });
        block.insts.push(Inst::IConst { result: Value(1), value: 20, ty: IrType::I64 });
        block.insts.push(Inst::BinOp {
            result: Value(2), op: IrBinOp::Add, lhs: Value(0), rhs: Value(1), ty: IrType::I64,
        });
        block.insts.push(Inst::Return { value: Some(Value(2)) });
        func.blocks.push(block);
        module.functions.push(func);

        let features = super::extract_features(&module);
        assert_eq!(features.num_instructions, 4);
        assert_eq!(features.num_basic_blocks, 1);
        assert_eq!(features.num_arithmetic_ops, 1);
        assert!(features.estimated_register_pressure >= 3);
    }

    #[test]
    fn test_apply_pass_constant_folding() {
        use crate::ir::*;
        let mut module = IrModule::new();
        let mut func = IrFunction {
            name: "fold_me".to_string(),
            params: vec![],
            ret_type: IrType::I64,
            blocks: vec![],
            entry: BlockId(0),
        };
        let mut block = BasicBlock::new(BlockId(0));
        block.insts.push(Inst::IConst { result: Value(0), value: 3, ty: IrType::I64 });
        block.insts.push(Inst::IConst { result: Value(1), value: 7, ty: IrType::I64 });
        block.insts.push(Inst::BinOp {
            result: Value(2), op: IrBinOp::Add, lhs: Value(0), rhs: Value(1), ty: IrType::I64,
        });
        block.insts.push(Inst::Return { value: Some(Value(2)) });
        func.blocks.push(block);
        module.functions.push(func);

        let changes = super::apply_pass(OptPass::ConstantFolding, &mut module);
        assert!(changes > 0, "constant folding should fold 3+7");
        // The BinOp should now be an IConst with value 10
        let inst = &module.functions[0].blocks[0].insts[2];
        match inst {
            Inst::IConst { value: 10, .. } => {} // correct
            other => panic!("Expected IConst(10), got {:?}", other),
        }
    }

    #[test]
    fn test_apply_pass_dce() {
        use crate::ir::*;
        let mut module = IrModule::new();
        let mut func = IrFunction {
            name: "dce_me".to_string(),
            params: vec![],
            ret_type: IrType::I64,
            blocks: vec![],
            entry: BlockId(0),
        };
        let mut block = BasicBlock::new(BlockId(0));
        // v0 = 42 (used in return)
        block.insts.push(Inst::IConst { result: Value(0), value: 42, ty: IrType::I64 });
        // v1 = 99 (dead — never used)
        block.insts.push(Inst::IConst { result: Value(1), value: 99, ty: IrType::I64 });
        block.insts.push(Inst::Return { value: Some(Value(0)) });
        func.blocks.push(block);
        module.functions.push(func);

        let changes = super::apply_pass(OptPass::DeadCodeElimination, &mut module);
        assert_eq!(changes, 1, "DCE should remove the unused IConst(99)");
        assert_eq!(module.functions[0].blocks[0].insts.len(), 2); // IConst(42) + Return
    }

    #[test]
    fn test_apply_pass_strength_reduce() {
        use crate::ir::*;
        let mut module = IrModule::new();
        let mut func = IrFunction {
            name: "sr_me".to_string(),
            params: vec![],
            ret_type: IrType::I64,
            blocks: vec![],
            entry: BlockId(0),
        };
        let mut block = BasicBlock::new(BlockId(0));
        block.insts.push(Inst::IConst { result: Value(0), value: 5, ty: IrType::I64 });
        block.insts.push(Inst::IConst { result: Value(1), value: 1, ty: IrType::I64 });
        // 5 * 1 should be strength-reduced to Copy
        block.insts.push(Inst::BinOp {
            result: Value(2), op: IrBinOp::Mul, lhs: Value(0), rhs: Value(1), ty: IrType::I64,
        });
        block.insts.push(Inst::Return { value: Some(Value(2)) });
        func.blocks.push(block);
        module.functions.push(func);

        let changes = super::apply_pass(OptPass::StrengthReduction, &mut module);
        assert!(changes > 0, "x * 1 should be reduced to copy");
    }

    #[test]
    fn test_apply_pass_cse() {
        use crate::ir::*;
        let mut module = IrModule::new();
        let mut func = IrFunction {
            name: "cse_me".to_string(),
            params: vec![],
            ret_type: IrType::I64,
            blocks: vec![],
            entry: BlockId(0),
        };
        let mut block = BasicBlock::new(BlockId(0));
        block.insts.push(Inst::IConst { result: Value(0), value: 3, ty: IrType::I64 });
        block.insts.push(Inst::IConst { result: Value(1), value: 4, ty: IrType::I64 });
        // Same operation twice: 3 + 4
        block.insts.push(Inst::BinOp {
            result: Value(2), op: IrBinOp::Add, lhs: Value(0), rhs: Value(1), ty: IrType::I64,
        });
        block.insts.push(Inst::BinOp {
            result: Value(3), op: IrBinOp::Add, lhs: Value(0), rhs: Value(1), ty: IrType::I64,
        });
        block.insts.push(Inst::Return { value: Some(Value(3)) });
        func.blocks.push(block);
        module.functions.push(func);

        let changes = super::apply_pass(OptPass::CommonSubexprElimination, &mut module);
        assert_eq!(changes, 1, "CSE should eliminate duplicate add");
    }

    #[test]
    fn test_apply_pass_sequence() {
        use crate::ir::*;
        let mut module = IrModule::new();
        let mut func = IrFunction {
            name: "seq_opt".to_string(),
            params: vec![],
            ret_type: IrType::I64,
            blocks: vec![],
            entry: BlockId(0),
        };
        let mut block = BasicBlock::new(BlockId(0));
        block.insts.push(Inst::IConst { result: Value(0), value: 5, ty: IrType::I64 });
        block.insts.push(Inst::IConst { result: Value(1), value: 0, ty: IrType::I64 });
        // 5 + 0 → strength reduce to copy of v0
        block.insts.push(Inst::BinOp {
            result: Value(2), op: IrBinOp::Add, lhs: Value(0), rhs: Value(1), ty: IrType::I64,
        });
        block.insts.push(Inst::Return { value: Some(Value(2)) });
        func.blocks.push(block);
        module.functions.push(func);

        let results = super::apply_pass_sequence(
            &[OptPass::StrengthReduction, OptPass::DeadCodeElimination],
            &mut module,
        );
        assert_eq!(results.len(), 2);
        // Strength reduction changes x+0 to copy
        assert!(results[0].1 > 0);
    }

    #[test]
    fn test_measure_improvement() {
        let before = ProgramFeatures {
            num_instructions: 100,
            estimated_register_pressure: 20,
            ..Default::default()
        };
        let after = ProgramFeatures {
            num_instructions: 70,
            estimated_register_pressure: 15,
            ..Default::default()
        };
        let score = super::measure_improvement(&before, &after);
        // 30% instruction reduction * 0.7 + 25% reg reduction * 0.3 = 0.21 + 0.075 = 0.285
        assert!(score > 0.2 && score < 0.4, "score={}", score);
    }

    #[test]
    fn test_measure_improvement_no_change() {
        let features = ProgramFeatures {
            num_instructions: 50,
            ..Default::default()
        };
        assert_eq!(super::measure_improvement(&features, &features), 0.0);
    }

    #[test]
    fn test_rl_optimize_basic() {
        use crate::ir::*;
        let mut module = IrModule::new();
        let mut func = IrFunction {
            name: "rl_test".to_string(),
            params: vec![],
            ret_type: IrType::I64,
            blocks: vec![],
            entry: BlockId(0),
        };
        let mut block = BasicBlock::new(BlockId(0));
        block.insts.push(Inst::IConst { result: Value(0), value: 10, ty: IrType::I64 });
        block.insts.push(Inst::IConst { result: Value(1), value: 0, ty: IrType::I64 });
        block.insts.push(Inst::BinOp {
            result: Value(2), op: IrBinOp::Add, lhs: Value(0), rhs: Value(1), ty: IrType::I64,
        });
        block.insts.push(Inst::Return { value: Some(Value(2)) });
        func.blocks.push(block);
        module.functions.push(func);

        let ordering = super::rl_optimize(&module, 5, 42);
        // Should return some ordering of the 4 implemented passes
        assert!(!ordering.is_empty());
        assert!(ordering.len() <= 4);
    }

    #[test]
    fn test_extract_features_branches_and_loops() {
        use crate::ir::*;
        let mut module = IrModule::new();
        let mut func = IrFunction {
            name: "loop_fn".to_string(),
            params: vec![],
            ret_type: IrType::I64,
            blocks: vec![],
            entry: BlockId(0),
        };
        // Block 0: entry
        let mut b0 = BasicBlock::new(BlockId(0));
        b0.insts.push(Inst::IConst { result: Value(0), value: 1, ty: IrType::Bool });
        b0.insts.push(Inst::Branch { cond: Value(0), then_bb: BlockId(1), else_bb: BlockId(2) });
        func.blocks.push(b0);
        // Block 1: loop body — jumps back to block 0 (back-edge)
        let mut b1 = BasicBlock::new(BlockId(1));
        b1.insts.push(Inst::Jump { target: BlockId(0) });
        func.blocks.push(b1);
        // Block 2: exit
        let mut b2 = BasicBlock::new(BlockId(2));
        b2.insts.push(Inst::IConst { result: Value(1), value: 0, ty: IrType::I64 });
        b2.insts.push(Inst::Return { value: Some(Value(1)) });
        func.blocks.push(b2);
        module.functions.push(func);

        let features = super::extract_features(&module);
        assert_eq!(features.num_basic_blocks, 3);
        assert_eq!(features.num_branches, 1);
        assert!(features.num_loops >= 1, "should detect back-edge as loop");
    }
}
