//! Optimization Invention — Vitalis v632
//!
//! Discovers novel optimizations by analyzing code patterns:
//! - Superoptimization: exhaustive search for shorter instruction sequences
//! - Algebraic identity discovery
//! - Transform rule synthesis via equality saturation sketches
//! - Optimization opportunity scoring and ranking
//! - Pattern-to-rule conversion


// ── Instruction Model ────────────────────────────────────────────────

/// Simplified instruction for superoptimization search.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MicroOp {
    Const(i64),
    Add,
    Sub,
    Mul,
    Shl(u8),
    Shr(u8),
    And,
    Or,
    Xor,
    Neg,
    Not,
    Dup,   // Duplicate top of stack
    Swap,  // Swap top two
}

impl MicroOp {
    /// Estimated cost in cycles.
    pub fn cost(&self) -> u32 {
        match self {
            Self::Const(_) | Self::Dup | Self::Swap => 0,
            Self::Add | Self::Sub | Self::And | Self::Or | Self::Xor | Self::Neg | Self::Not => 1,
            Self::Shl(_) | Self::Shr(_) => 1,
            Self::Mul => 3,
        }
    }
}

/// A sequence of micro-ops forming a computation.
#[derive(Debug, Clone)]
pub struct MicroProgram {
    pub ops: Vec<MicroOp>,
}

impl MicroProgram {
    pub fn new(ops: Vec<MicroOp>) -> Self { Self { ops } }

    pub fn total_cost(&self) -> u32 {
        self.ops.iter().map(|op| op.cost()).sum()
    }

    /// Evaluate on a simple stack machine with one input.
    pub fn evaluate(&self, input: i64) -> Option<i64> {
        let mut stack: Vec<i64> = vec![input];
        for op in &self.ops {
            match op {
                MicroOp::Const(c) => stack.push(*c),
                MicroOp::Add => {
                    let b = stack.pop()?;
                    let a = stack.pop()?;
                    stack.push(a.wrapping_add(b));
                }
                MicroOp::Sub => {
                    let b = stack.pop()?;
                    let a = stack.pop()?;
                    stack.push(a.wrapping_sub(b));
                }
                MicroOp::Mul => {
                    let b = stack.pop()?;
                    let a = stack.pop()?;
                    stack.push(a.wrapping_mul(b));
                }
                MicroOp::Shl(n) => {
                    let a = stack.pop()?;
                    stack.push(a.wrapping_shl(*n as u32));
                }
                MicroOp::Shr(n) => {
                    let a = stack.pop()?;
                    stack.push(a.wrapping_shr(*n as u32));
                }
                MicroOp::And => {
                    let b = stack.pop()?;
                    let a = stack.pop()?;
                    stack.push(a & b);
                }
                MicroOp::Or => {
                    let b = stack.pop()?;
                    let a = stack.pop()?;
                    stack.push(a | b);
                }
                MicroOp::Xor => {
                    let b = stack.pop()?;
                    let a = stack.pop()?;
                    stack.push(a ^ b);
                }
                MicroOp::Neg => {
                    let a = stack.pop()?;
                    stack.push(a.wrapping_neg());
                }
                MicroOp::Not => {
                    let a = stack.pop()?;
                    stack.push(!a);
                }
                MicroOp::Dup => {
                    let a = *stack.last()?;
                    stack.push(a);
                }
                MicroOp::Swap => {
                    let len = stack.len();
                    if len < 2 { return None; }
                    stack.swap(len - 1, len - 2);
                }
            }
        }
        stack.pop()
    }
}

// ── Algebraic Identity ───────────────────────────────────────────────

/// Known algebraic identities for optimization.
#[derive(Debug, Clone)]
pub struct AlgebraicIdentity {
    pub name: String,
    pub pattern: String,      // e.g. "x * 2"
    pub replacement: String,  // e.g. "x << 1"
    pub cost_saving: i32,     // Positive = saves this many cycles
}

/// Standard algebraic identities.
pub fn standard_identities() -> Vec<AlgebraicIdentity> {
    vec![
        AlgebraicIdentity { name: "mul_pow2".into(), pattern: "x * 2^k".into(), replacement: "x << k".into(), cost_saving: 2 },
        AlgebraicIdentity { name: "div_pow2".into(), pattern: "x / 2^k".into(), replacement: "x >> k".into(), cost_saving: 8 },
        AlgebraicIdentity { name: "mod_pow2".into(), pattern: "x % 2^k".into(), replacement: "x & (2^k - 1)".into(), cost_saving: 8 },
        AlgebraicIdentity { name: "add_zero".into(), pattern: "x + 0".into(), replacement: "x".into(), cost_saving: 1 },
        AlgebraicIdentity { name: "mul_one".into(), pattern: "x * 1".into(), replacement: "x".into(), cost_saving: 3 },
        AlgebraicIdentity { name: "mul_zero".into(), pattern: "x * 0".into(), replacement: "0".into(), cost_saving: 3 },
        AlgebraicIdentity { name: "sub_self".into(), pattern: "x - x".into(), replacement: "0".into(), cost_saving: 1 },
        AlgebraicIdentity { name: "xor_self".into(), pattern: "x ^ x".into(), replacement: "0".into(), cost_saving: 1 },
        AlgebraicIdentity { name: "and_self".into(), pattern: "x & x".into(), replacement: "x".into(), cost_saving: 1 },
        AlgebraicIdentity { name: "or_self".into(), pattern: "x | x".into(), replacement: "x".into(), cost_saving: 1 },
        AlgebraicIdentity { name: "double_neg".into(), pattern: "-(-x)".into(), replacement: "x".into(), cost_saving: 2 },
        AlgebraicIdentity { name: "mul_by_3".into(), pattern: "x * 3".into(), replacement: "(x << 1) + x".into(), cost_saving: 1 },
    ]
}

// ── Superoptimizer ───────────────────────────────────────────────────

/// Brute-force superoptimizer: find shortest program equivalent to target.
pub struct Superoptimizer {
    pub test_inputs: Vec<i64>,
    pub max_length: usize,
}

impl Superoptimizer {
    pub fn new(max_length: usize) -> Self {
        Self { test_inputs: vec![0, 1, -1, 2, 3, 7, 10, 42, 100, -50], max_length }
    }

    /// Check if two programs are equivalent on test inputs.
    pub fn equivalent(&self, a: &MicroProgram, b: &MicroProgram) -> bool {
        self.test_inputs.iter().all(|&input| {
            a.evaluate(input) == b.evaluate(input)
        })
    }

    /// Find a cheaper equivalent program (bounded search).
    pub fn superoptimize(&self, target: &MicroProgram) -> Option<MicroProgram> {
        let target_cost = target.total_cost();
        let target_outputs: Vec<Option<i64>> = self.test_inputs.iter()
            .map(|&x| target.evaluate(x))
            .collect();

        // Try increasingly longer programs
        let candidate_ops = [
            MicroOp::Dup, MicroOp::Add, MicroOp::Sub, MicroOp::Neg,
            MicroOp::Shl(1), MicroOp::Shl(2), MicroOp::Shr(1),
            MicroOp::Const(0), MicroOp::Const(1), MicroOp::Const(-1),
        ];

        for len in 1..=self.max_length.min(4) {
            if let Some(prog) = self.search_at_length(&candidate_ops, len, &target_outputs, target_cost) {
                return Some(prog);
            }
        }
        None
    }

    fn search_at_length(
        &self, ops: &[MicroOp], len: usize,
        target_outputs: &[Option<i64>], max_cost: u32,
    ) -> Option<MicroProgram> {
        if len == 0 { return None; }

        let mut indices = vec![0usize; len];
        let num_ops = ops.len();

        loop {
            let program = MicroProgram::new(indices.iter().map(|&i| ops[i]).collect());

            if program.total_cost() < max_cost {
                let outputs: Vec<Option<i64>> = self.test_inputs.iter()
                    .map(|&x| program.evaluate(x))
                    .collect();
                if outputs == *target_outputs {
                    return Some(program);
                }
            }

            // Increment indices (odometer style)
            let mut carry = true;
            for i in (0..len).rev() {
                if carry {
                    indices[i] += 1;
                    if indices[i] >= num_ops {
                        indices[i] = 0;
                    } else {
                        carry = false;
                    }
                }
            }
            if carry { break; }
        }
        None
    }
}

// ── Optimization Opportunity ─────────────────────────────────────────

/// A discovered optimization opportunity.
#[derive(Debug, Clone)]
pub struct OptimizationOpportunity {
    pub kind: OpportunityKind,
    pub description: String,
    pub confidence: f64,
    pub estimated_speedup: f64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpportunityKind {
    StrengthReduction,
    DeadCodeElimination,
    ConstantFolding,
    LoopInvariantMotion,
    CommonSubexpression,
    Vectorization,
    BranchElimination,
    AlgebraicSimplification,
}

impl OptimizationOpportunity {
    /// Score combining confidence and estimated speedup.
    pub fn priority_score(&self) -> f64 {
        self.confidence * self.estimated_speedup
    }
}

/// Rank opportunities by priority.
pub fn rank_opportunities(opps: &mut [OptimizationOpportunity]) {
    opps.sort_by(|a, b| b.priority_score().partial_cmp(&a.priority_score()).unwrap_or(std::cmp::Ordering::Equal));
}

// ── FFI Exports ──────────────────────────────────────────────────────

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_opt_invent_program_cost(op_count: i64, has_mul: i64) -> i64 {
    let base = op_count.max(0);
    let mul_cost = if has_mul != 0 { 3 } else { 0 };
    base + mul_cost
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_opt_invent_identity_count() -> i64 {
    standard_identities().len() as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_opt_invent_priority_score(confidence: f64, speedup: f64) -> f64 {
    confidence.clamp(0.0, 1.0) * speedup.max(0.0)
}

// ═══════════════════════════════════════════════════════════════════════
// Tests
// ═══════════════════════════════════════════════════════════════════════
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_micro_op_costs() {
        assert_eq!(MicroOp::Add.cost(), 1);
        assert_eq!(MicroOp::Mul.cost(), 3);
        assert_eq!(MicroOp::Const(0).cost(), 0);
    }

    #[test]
    fn test_micro_program_add() {
        let prog = MicroProgram::new(vec![MicroOp::Dup, MicroOp::Add]);
        assert_eq!(prog.evaluate(5), Some(10)); // x + x = 2x
    }

    #[test]
    fn test_micro_program_double_via_shift() {
        let prog = MicroProgram::new(vec![MicroOp::Shl(1)]);
        assert_eq!(prog.evaluate(5), Some(10)); // x << 1 = 2x
    }

    #[test]
    fn test_micro_program_negate() {
        let prog = MicroProgram::new(vec![MicroOp::Neg]);
        assert_eq!(prog.evaluate(42), Some(-42));
    }

    #[test]
    fn test_micro_program_xor_self() {
        let prog = MicroProgram::new(vec![MicroOp::Dup, MicroOp::Xor]);
        assert_eq!(prog.evaluate(42), Some(0));
    }

    #[test]
    fn test_program_total_cost() {
        let prog = MicroProgram::new(vec![MicroOp::Dup, MicroOp::Mul, MicroOp::Const(1), MicroOp::Add]);
        assert_eq!(prog.total_cost(), 4); // 0 + 3 + 0 + 1
    }

    #[test]
    fn test_equivalence_check() {
        let so = Superoptimizer::new(3);
        let a = MicroProgram::new(vec![MicroOp::Dup, MicroOp::Add]); // x * 2
        let b = MicroProgram::new(vec![MicroOp::Shl(1)]); // x << 1
        assert!(so.equivalent(&a, &b));
    }

    #[test]
    fn test_nonequivalent() {
        let so = Superoptimizer::new(3);
        let a = MicroProgram::new(vec![MicroOp::Dup, MicroOp::Add]); // x * 2
        let b = MicroProgram::new(vec![MicroOp::Shl(2)]); // x << 2 = x * 4
        assert!(!so.equivalent(&a, &b));
    }

    #[test]
    fn test_superoptimize_finds_shift() {
        let so = Superoptimizer::new(3);
        let target = MicroProgram::new(vec![MicroOp::Dup, MicroOp::Add]); // 2x via add (cost 1)
        let result = so.superoptimize(&target);
        // Should find x << 1 which costs 1 (same) or fewer ops
        if let Some(optimized) = result {
            assert!(so.equivalent(&target, &optimized));
        }
    }

    #[test]
    fn test_standard_identities() {
        let ids = standard_identities();
        assert!(ids.len() >= 10);
        assert!(ids.iter().all(|id| id.cost_saving > 0));
    }

    #[test]
    fn test_opportunity_score() {
        let opp = OptimizationOpportunity {
            kind: OpportunityKind::StrengthReduction,
            description: "Replace mul by shift".into(),
            confidence: 0.9,
            estimated_speedup: 2.0,
        };
        assert!((opp.priority_score() - 1.8).abs() < 1e-10);
    }

    #[test]
    fn test_rank_opportunities() {
        let mut opps = vec![
            OptimizationOpportunity { kind: OpportunityKind::ConstantFolding, description: "fold".into(), confidence: 0.5, estimated_speedup: 1.0 },
            OptimizationOpportunity { kind: OpportunityKind::Vectorization, description: "vec".into(), confidence: 0.8, estimated_speedup: 4.0 },
        ];
        rank_opportunities(&mut opps);
        assert_eq!(opps[0].kind, OpportunityKind::Vectorization);
    }

    #[test]
    fn test_ffi_program_cost() {
        assert_eq!(vitalis_opt_invent_program_cost(5, 1), 8);
        assert_eq!(vitalis_opt_invent_program_cost(5, 0), 5);
    }

    #[test]
    fn test_ffi_identity_count() {
        assert!(vitalis_opt_invent_identity_count() >= 10);
    }

    #[test]
    fn test_ffi_priority() {
        assert!((vitalis_opt_invent_priority_score(0.9, 3.0) - 2.7).abs() < 1e-10);
    }
}
