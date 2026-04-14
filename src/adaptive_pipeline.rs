//! Adaptive Pipeline — Vitalis v627
//!
//! Dynamically adjusts the compilation pipeline based on code characteristics:
//! - Pipeline stage ordering and selection
//! - Optimization level adaptation per function
//! - Pass skipping for trivial code
//! - Budget-aware compilation (time/memory constraints)
//! - Feedback-driven pipeline tuning

// ── Pipeline Configuration ───────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PassId {
    ConstantFolding,
    DeadCodeElimination,
    CommonSubexprElim,
    LoopInvariantMotion,
    Inlining,
    TailCallOpt,
    RegisterAllocation,
    InstructionCombine,
    StrengthReduction,
    LoopUnrolling,
    Vectorization,
    Custom(String),
}

impl PassId {
    pub fn label(&self) -> &str {
        match self {
            PassId::ConstantFolding => "const_fold",
            PassId::DeadCodeElimination => "dce",
            PassId::CommonSubexprElim => "cse",
            PassId::LoopInvariantMotion => "licm",
            PassId::Inlining => "inline",
            PassId::TailCallOpt => "tco",
            PassId::RegisterAllocation => "regalloc",
            PassId::InstructionCombine => "instcombine",
            PassId::StrengthReduction => "strength_reduce",
            PassId::LoopUnrolling => "unroll",
            PassId::Vectorization => "vectorize",
            PassId::Custom(s) => s,
        }
    }

    pub fn default_cost(&self) -> u64 {
        match self {
            PassId::ConstantFolding => 5,
            PassId::DeadCodeElimination => 10,
            PassId::CommonSubexprElim => 30,
            PassId::LoopInvariantMotion => 25,
            PassId::Inlining => 50,
            PassId::TailCallOpt => 8,
            PassId::RegisterAllocation => 40,
            PassId::InstructionCombine => 15,
            PassId::StrengthReduction => 12,
            PassId::LoopUnrolling => 35,
            PassId::Vectorization => 60,
            PassId::Custom(_) => 20,
        }
    }
}

/// Heuristic code characteristics used for pipeline decisions.
#[derive(Debug, Clone)]
pub struct CodeCharacteristics {
    pub instruction_count: usize,
    pub loop_count: usize,
    pub branch_count: usize,
    pub call_count: usize,
    pub is_hot: bool,          // Frequently executed
    pub is_leaf: bool,         // No function calls
    pub has_recursion: bool,
    pub has_simd_potential: bool,
    pub estimated_complexity: usize, // Rough O(n^k) estimate
}

/// A pass with its recorded effectiveness history.
#[derive(Debug, Clone)]
pub struct PassRecord {
    pub pass: PassId,
    pub total_runs: u32,
    pub total_reductions: i64,
    pub total_time_us: u64,
}

impl PassRecord {
    pub fn new(pass: PassId) -> Self {
        Self { pass, total_runs: 0, total_reductions: 0, total_time_us: 0 }
    }

    pub fn record_run(&mut self, reduction: i64, time_us: u64) {
        self.total_runs += 1;
        self.total_reductions += reduction;
        self.total_time_us += time_us;
    }

    pub fn avg_reduction(&self) -> f64 {
        if self.total_runs == 0 { 0.0 }
        else { self.total_reductions as f64 / self.total_runs as f64 }
    }

    pub fn avg_time_us(&self) -> f64 {
        if self.total_runs == 0 { 0.0 }
        else { self.total_time_us as f64 / self.total_runs as f64 }
    }

    pub fn cost_effectiveness(&self) -> f64 {
        let time = self.avg_time_us();
        if time < 1.0 { return self.avg_reduction(); }
        self.avg_reduction() / time * 1000.0
    }
}

// ── Adaptive Pipeline ────────────────────────────────────────────────

pub struct AdaptivePipeline {
    records: std::collections::HashMap<String, PassRecord>,
    time_budget_us: u64,
    memory_budget_bytes: usize,
}

impl AdaptivePipeline {
    pub fn new(time_budget_us: u64, memory_budget_bytes: usize) -> Self {
        Self {
            records: std::collections::HashMap::new(),
            time_budget_us,
            memory_budget_bytes,
        }
    }

    pub fn record_pass(&mut self, pass: &PassId, reduction: i64, time_us: u64) {
        let entry = self.records.entry(pass.label().to_string())
            .or_insert_with(|| PassRecord::new(pass.clone()));
        entry.record_run(reduction, time_us);
    }

    /// Select passes for a function based on its characteristics and budget.
    pub fn select_passes(&self, chars: &CodeCharacteristics) -> Vec<PassId> {
        let mut passes = Vec::new();
        let mut remaining_budget = self.time_budget_us;

        // Always include cheap fundamentals
        for p in &[PassId::ConstantFolding, PassId::DeadCodeElimination] {
            let cost = self.estimated_cost(p);
            if cost <= remaining_budget {
                passes.push(p.clone());
                remaining_budget = remaining_budget.saturating_sub(cost);
            }
        }

        // Skip expensive passes for trivial functions
        if chars.instruction_count < 10 {
            return passes;
        }

        // Add CSE for code with moderate complexity
        if chars.instruction_count >= 20 {
            let p = PassId::CommonSubexprElim;
            let cost = self.estimated_cost(&p);
            if cost <= remaining_budget {
                passes.push(p);
                remaining_budget = remaining_budget.saturating_sub(cost);
            }
        }

        // Loop optimizations for loopy code
        if chars.loop_count > 0 {
            for p in &[PassId::LoopInvariantMotion, PassId::StrengthReduction] {
                let cost = self.estimated_cost(p);
                if cost <= remaining_budget {
                    passes.push(p.clone());
                    remaining_budget = remaining_budget.saturating_sub(cost);
                }
            }
        }

        // Loop unrolling for hot loops
        if chars.loop_count > 0 && chars.is_hot {
            let p = PassId::LoopUnrolling;
            let cost = self.estimated_cost(&p);
            if cost <= remaining_budget {
                passes.push(p);
                remaining_budget = remaining_budget.saturating_sub(cost);
            }
        }

        // Inlining for small callees
        if chars.call_count > 0 && !chars.has_recursion {
            let p = PassId::Inlining;
            let cost = self.estimated_cost(&p);
            if cost <= remaining_budget {
                passes.push(p);
                remaining_budget = remaining_budget.saturating_sub(cost);
            }
        }

        // TCO for recursive functions
        if chars.has_recursion {
            let p = PassId::TailCallOpt;
            let cost = self.estimated_cost(&p);
            if cost <= remaining_budget {
                passes.push(p);
                remaining_budget = remaining_budget.saturating_sub(cost);
            }
        }

        // Vectorization for SIMD-capable hot code
        if chars.has_simd_potential && chars.is_hot {
            let p = PassId::Vectorization;
            let cost = self.estimated_cost(&p);
            if cost <= remaining_budget {
                passes.push(p);
                remaining_budget = remaining_budget.saturating_sub(cost);
            }
        }

        let _ = remaining_budget; // suppress unused warning
        passes
    }

    fn estimated_cost(&self, pass: &PassId) -> u64 {
        if let Some(record) = self.records.get(pass.label()) {
            record.avg_time_us() as u64
        } else {
            pass.default_cost()
        }
    }

    /// Get pass effectiveness rankings.
    pub fn effectiveness_ranking(&self) -> Vec<(String, f64)> {
        let mut ranked: Vec<_> = self.records.iter()
            .map(|(name, rec)| (name.clone(), rec.cost_effectiveness()))
            .collect();
        ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        ranked
    }

    pub fn time_budget(&self) -> u64 { self.time_budget_us }
    pub fn memory_budget(&self) -> usize { self.memory_budget_bytes }
    pub fn recorded_passes(&self) -> usize { self.records.len() }
}

impl Default for AdaptivePipeline {
    fn default() -> Self { Self::new(10_000, 256 * 1024 * 1024) }
}

// ── FFI Exports ──────────────────────────────────────────────────────

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_adaptive_select_pass_count(
    inst_count: i64, loop_count: i64, call_count: i64,
    is_hot: i64, has_recursion: i64, budget_us: i64,
) -> i64 {
    let chars = CodeCharacteristics {
        instruction_count: inst_count.max(0) as usize,
        loop_count: loop_count.max(0) as usize,
        branch_count: 0,
        call_count: call_count.max(0) as usize,
        is_hot: is_hot != 0,
        is_leaf: call_count == 0,
        has_recursion: has_recursion != 0,
        has_simd_potential: false,
        estimated_complexity: 1,
    };
    let pipeline = AdaptivePipeline::new(budget_us.max(0) as u64, 256 * 1024 * 1024);
    pipeline.select_passes(&chars).len() as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_adaptive_pass_cost_effectiveness(
    total_reductions: i64, total_time_us: i64, total_runs: i64,
) -> f64 {
    if total_runs <= 0 { return 0.0; }
    let avg_red = total_reductions as f64 / total_runs as f64;
    let avg_time = total_time_us as f64 / total_runs as f64;
    if avg_time < 1.0 { avg_red } else { avg_red / avg_time * 1000.0 }
}

// ═══════════════════════════════════════════════════════════════════════
// Tests
// ═══════════════════════════════════════════════════════════════════════
#[cfg(test)]
mod tests {
    use super::*;

    fn simple_chars() -> CodeCharacteristics {
        CodeCharacteristics {
            instruction_count: 100,
            loop_count: 2,
            branch_count: 5,
            call_count: 3,
            is_hot: true,
            is_leaf: false,
            has_recursion: false,
            has_simd_potential: true,
            estimated_complexity: 2,
        }
    }

    #[test]
    fn test_pass_labels() {
        assert_eq!(PassId::ConstantFolding.label(), "const_fold");
        assert_eq!(PassId::DeadCodeElimination.label(), "dce");
        assert_eq!(PassId::Custom("my_pass".into()).label(), "my_pass");
    }

    #[test]
    fn test_pass_record() {
        let mut rec = PassRecord::new(PassId::ConstantFolding);
        rec.record_run(10, 5);
        rec.record_run(20, 15);
        assert_eq!(rec.total_runs, 2);
        assert!((rec.avg_reduction() - 15.0).abs() < 1e-10);
        assert!((rec.avg_time_us() - 10.0).abs() < 1e-10);
    }

    #[test]
    fn test_pass_cost_effectiveness() {
        let mut rec = PassRecord::new(PassId::DeadCodeElimination);
        rec.record_run(100, 10);
        assert!(rec.cost_effectiveness() > 0.0);
    }

    #[test]
    fn test_select_passes_trivial() {
        let pipeline = AdaptivePipeline::new(10_000, 256 * 1024 * 1024);
        let chars = CodeCharacteristics {
            instruction_count: 5, loop_count: 0, branch_count: 0,
            call_count: 0, is_hot: false, is_leaf: true,
            has_recursion: false, has_simd_potential: false, estimated_complexity: 0,
        };
        let passes = pipeline.select_passes(&chars);
        assert!(passes.len() <= 2); // Only fundamentals
    }

    #[test]
    fn test_select_passes_complex() {
        let pipeline = AdaptivePipeline::new(10_000, 256 * 1024 * 1024);
        let passes = pipeline.select_passes(&simple_chars());
        assert!(passes.len() >= 4); // Fundamentals + loop opts + inline + vectorize
    }

    #[test]
    fn test_select_passes_recursive() {
        let pipeline = AdaptivePipeline::new(10_000, 256 * 1024 * 1024);
        let mut chars = simple_chars();
        chars.has_recursion = true;
        let passes = pipeline.select_passes(&chars);
        assert!(passes.iter().any(|p| *p == PassId::TailCallOpt));
    }

    #[test]
    fn test_select_passes_tight_budget() {
        let pipeline = AdaptivePipeline::new(10, 256 * 1024 * 1024); // Very tight
        let passes = pipeline.select_passes(&simple_chars());
        assert!(passes.len() <= 3); // Budget constrains selection
    }

    #[test]
    fn test_select_passes_with_loops() {
        let pipeline = AdaptivePipeline::new(10_000, 256 * 1024 * 1024);
        let chars = CodeCharacteristics {
            instruction_count: 50, loop_count: 3, branch_count: 2,
            call_count: 0, is_hot: false, is_leaf: true,
            has_recursion: false, has_simd_potential: false, estimated_complexity: 2,
        };
        let passes = pipeline.select_passes(&chars);
        assert!(passes.iter().any(|p| *p == PassId::LoopInvariantMotion));
    }

    #[test]
    fn test_effectiveness_ranking() {
        let mut pipeline = AdaptivePipeline::new(10_000, 256 * 1024 * 1024);
        pipeline.record_pass(&PassId::ConstantFolding, 50, 5);
        pipeline.record_pass(&PassId::DeadCodeElimination, 30, 10);
        let ranking = pipeline.effectiveness_ranking();
        assert_eq!(ranking.len(), 2);
    }

    #[test]
    fn test_pipeline_defaults() {
        let p = AdaptivePipeline::default();
        assert_eq!(p.time_budget(), 10_000);
        assert_eq!(p.recorded_passes(), 0);
    }

    #[test]
    fn test_ffi_select_pass_count() {
        let count = vitalis_adaptive_select_pass_count(100, 2, 3, 1, 0, 10000);
        assert!(count >= 2);
    }

    #[test]
    fn test_ffi_cost_effectiveness() {
        let ce = vitalis_adaptive_pass_cost_effectiveness(100, 10, 2);
        assert!(ce > 0.0);
    }

    #[test]
    fn test_ffi_cost_effectiveness_zero_runs() {
        assert_eq!(vitalis_adaptive_pass_cost_effectiveness(100, 10, 0), 0.0);
    }
}
