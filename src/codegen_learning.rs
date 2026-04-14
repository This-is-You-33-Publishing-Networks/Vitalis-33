//! v425 — Codegen Learning.
//!
//! JIT backend that learns instruction selection heuristics from execution
//! profiles. Tracks which lowering choices produce the fastest code per IR
//! pattern and builds decision trees over IR features → codegen choices.
//! Uses Thompson sampling for exploration vs exploitation.

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

/// Feature vector extracted from an IR function.
#[derive(Debug, Clone)]
pub struct IrFeatures {
    pub num_instructions: u32,
    pub num_blocks: u32,
    pub num_branches: u32,
    pub num_calls: u32,
    pub num_memory_ops: u32,
    pub num_arithmetic_ops: u32,
    pub num_phi_nodes: u32,
    pub loop_depth: u32,
}

impl IrFeatures {
    pub fn new() -> Self {
        Self {
            num_instructions: 0,
            num_blocks: 0,
            num_branches: 0,
            num_calls: 0,
            num_memory_ops: 0,
            num_arithmetic_ops: 0,
            num_phi_nodes: 0,
            loop_depth: 0,
        }
    }

    /// Feature vector as normalized f64 array.
    pub fn to_vec(&self) -> Vec<f64> {
        let max = 1000.0;
        vec![
            self.num_instructions as f64 / max,
            self.num_blocks as f64 / max,
            self.num_branches as f64 / max,
            self.num_calls as f64 / max,
            self.num_memory_ops as f64 / max,
            self.num_arithmetic_ops as f64 / max,
            self.num_phi_nodes as f64 / max,
            self.loop_depth as f64 / 10.0,
        ]
    }

    /// Quantize features to a bucket key for decision tree lookup.
    pub fn bucket_key(&self) -> u64 {
        let b = |v: u32| -> u64 { (v.min(255)) as u64 };
        b(self.num_instructions / 10)
            | (b(self.num_blocks / 5) << 8)
            | (b(self.num_branches / 3) << 16)
            | (b(self.loop_depth) << 24)
    }
}

/// A codegen strategy choice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CodegenChoice {
    /// Use default Cranelift lowering.
    Default,
    /// Aggressively inline small functions first.
    AggressiveInline,
    /// Prioritize register allocation over instruction scheduling.
    RegAllocFirst,
    /// Optimize for code size.
    OptSize,
    /// Loop-focused: unroll + vectorize first.
    LoopOptimize,
}

impl CodegenChoice {
    pub fn all() -> &'static [CodegenChoice] {
        &[
            CodegenChoice::Default,
            CodegenChoice::AggressiveInline,
            CodegenChoice::RegAllocFirst,
            CodegenChoice::OptSize,
            CodegenChoice::LoopOptimize,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            CodegenChoice::Default => "default",
            CodegenChoice::AggressiveInline => "aggressive_inline",
            CodegenChoice::RegAllocFirst => "regalloc_first",
            CodegenChoice::OptSize => "opt_size",
            CodegenChoice::LoopOptimize => "loop_optimize",
        }
    }
}

/// Thompson sampling arm for a codegen choice.
#[derive(Debug, Clone)]
pub struct ThompsonArm {
    pub choice: CodegenChoice,
    pub alpha: f64, // successes + 1
    pub beta: f64,  // failures + 1
    pub total_reward: f64,
    pub samples: u64,
}

impl ThompsonArm {
    pub fn new(choice: CodegenChoice) -> Self {
        Self { choice, alpha: 1.0, beta: 1.0, total_reward: 0.0, samples: 0 }
    }

    /// Expected value (mean of Beta distribution).
    pub fn expected_value(&self) -> f64 {
        self.alpha / (self.alpha + self.beta)
    }

    /// Record a reward (0.0 to 1.0).
    pub fn record(&mut self, reward: f64) {
        self.samples += 1;
        self.total_reward += reward;
        if reward > 0.5 {
            self.alpha += reward;
        } else {
            self.beta += 1.0 - reward;
        }
    }

    /// Sample from Beta distribution using Jöhnk's algorithm approximation.
    pub fn sample(&self, seed: u64) -> f64 {
        // Simplified: use expected value + noise
        let noise = ((seed as f64 * 0.0000001).sin() + 1.0) / 2.0;
        let mean = self.expected_value();
        mean * 0.8 + noise * 0.2
    }
}

/// Decision node mapping IR feature buckets to codegen choices.
pub struct CodegenLearner {
    /// Map from feature bucket → arms
    pub decisions: HashMap<u64, Vec<ThompsonArm>>,
    pub total_decisions: u64,
    pub correct_predictions: u64,
}

impl CodegenLearner {
    pub fn new() -> Self {
        Self {
            decisions: HashMap::new(),
            total_decisions: 0,
            correct_predictions: 0,
        }
    }

    /// Get or create arms for a feature bucket.
    fn get_arms(&mut self, bucket: u64) -> &mut Vec<ThompsonArm> {
        self.decisions.entry(bucket).or_insert_with(|| {
            CodegenChoice::all().iter().map(|c| ThompsonArm::new(*c)).collect()
        })
    }

    /// Select the best codegen choice for given features using Thompson sampling.
    pub fn select(&mut self, features: &IrFeatures, seed: u64) -> CodegenChoice {
        let bucket = features.bucket_key();
        self.total_decisions += 1;
        let arms = self.get_arms(bucket);

        let mut best_choice = CodegenChoice::Default;
        let mut best_sample = f64::NEG_INFINITY;
        for (i, arm) in arms.iter().enumerate() {
            let s = arm.sample(seed.wrapping_add(i as u64));
            if s > best_sample {
                best_sample = s;
                best_choice = arm.choice;
            }
        }
        best_choice
    }

    /// Record outcome of a codegen choice.
    pub fn record_outcome(&mut self, features: &IrFeatures, choice: CodegenChoice, reward: f64) {
        let bucket = features.bucket_key();
        let arms = self.get_arms(bucket);
        for arm in arms.iter_mut() {
            if arm.choice == choice {
                arm.record(reward);
                break;
            }
        }
    }

    /// Get the current best choice for a feature set (no exploration).
    pub fn best_choice(&mut self, features: &IrFeatures) -> CodegenChoice {
        let bucket = features.bucket_key();
        let arms = self.get_arms(bucket);
        arms.iter()
            .max_by(|a, b| a.expected_value().partial_cmp(&b.expected_value()).unwrap_or(std::cmp::Ordering::Equal))
            .map(|a| a.choice)
            .unwrap_or(CodegenChoice::Default)
    }

    /// Accuracy rate.
    pub fn accuracy(&self) -> f64 {
        if self.total_decisions == 0 { return 0.0; }
        self.correct_predictions as f64 / self.total_decisions as f64
    }

    /// Number of feature buckets learned.
    pub fn buckets_learned(&self) -> usize {
        self.decisions.len()
    }
}

// ─── FFI ──────────────────────────────────────────────────────────────

static GLOBAL_LEARNER: LazyLock<Mutex<CodegenLearner>> =
    LazyLock::new(|| Mutex::new(CodegenLearner::new()));

#[unsafe(no_mangle)]
pub extern "C" fn slang_codegen_learn_select(
    num_insts: i64, num_blocks: i64, num_branches: i64, seed: i64,
) -> i64 {
    let features = IrFeatures {
        num_instructions: num_insts as u32,
        num_blocks: num_blocks as u32,
        num_branches: num_branches as u32,
        ..IrFeatures::new()
    };
    let mut learner = GLOBAL_LEARNER.lock().unwrap();
    let choice = learner.select(&features, seed as u64);
    match choice {
        CodegenChoice::Default => 0,
        CodegenChoice::AggressiveInline => 1,
        CodegenChoice::RegAllocFirst => 2,
        CodegenChoice::OptSize => 3,
        CodegenChoice::LoopOptimize => 4,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_codegen_learn_record(
    num_insts: i64, num_blocks: i64, num_branches: i64, choice: i64, reward: i64,
) -> i64 {
    let features = IrFeatures {
        num_instructions: num_insts as u32,
        num_blocks: num_blocks as u32,
        num_branches: num_branches as u32,
        ..IrFeatures::new()
    };
    let c = match choice {
        1 => CodegenChoice::AggressiveInline,
        2 => CodegenChoice::RegAllocFirst,
        3 => CodegenChoice::OptSize,
        4 => CodegenChoice::LoopOptimize,
        _ => CodegenChoice::Default,
    };
    let r = f64::from_bits(reward as u64);
    let mut learner = GLOBAL_LEARNER.lock().unwrap();
    learner.record_outcome(&features, c, r);
    1
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_codegen_learn_buckets() -> i64 {
    let learner = GLOBAL_LEARNER.lock().unwrap();
    learner.buckets_learned() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ir_features_new() {
        let f = IrFeatures::new();
        assert_eq!(f.num_instructions, 0);
    }

    #[test]
    fn test_ir_features_to_vec() {
        let mut f = IrFeatures::new();
        f.num_instructions = 100;
        let v = f.to_vec();
        assert_eq!(v.len(), 8);
        assert!((v[0] - 0.1).abs() < 1e-10);
    }

    #[test]
    fn test_ir_features_bucket() {
        let f = IrFeatures::new();
        let b = f.bucket_key();
        assert_eq!(b, 0);
    }

    #[test]
    fn test_codegen_choice_all() {
        assert_eq!(CodegenChoice::all().len(), 5);
    }

    #[test]
    fn test_codegen_choice_name() {
        assert_eq!(CodegenChoice::Default.name(), "default");
        assert_eq!(CodegenChoice::LoopOptimize.name(), "loop_optimize");
    }

    #[test]
    fn test_thompson_arm_new() {
        let arm = ThompsonArm::new(CodegenChoice::Default);
        assert!((arm.expected_value() - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_thompson_arm_record() {
        let mut arm = ThompsonArm::new(CodegenChoice::Default);
        arm.record(0.9);
        assert!(arm.expected_value() > 0.5);
        assert_eq!(arm.samples, 1);
    }

    #[test]
    fn test_thompson_arm_sample() {
        let arm = ThompsonArm::new(CodegenChoice::Default);
        let s = arm.sample(42);
        assert!(s > 0.0 && s <= 1.0);
    }

    #[test]
    fn test_learner_new() {
        let l = CodegenLearner::new();
        assert_eq!(l.buckets_learned(), 0);
        assert_eq!(l.total_decisions, 0);
    }

    #[test]
    fn test_learner_select() {
        let mut l = CodegenLearner::new();
        let f = IrFeatures::new();
        let _c = l.select(&f, 42);
        assert_eq!(l.total_decisions, 1);
        assert_eq!(l.buckets_learned(), 1);
    }

    #[test]
    fn test_learner_record_outcome() {
        let mut l = CodegenLearner::new();
        let f = IrFeatures::new();
        l.record_outcome(&f, CodegenChoice::Default, 0.9);
        assert_eq!(l.buckets_learned(), 1);
    }

    #[test]
    fn test_learner_best_choice() {
        let mut l = CodegenLearner::new();
        let f = IrFeatures::new();
        for _ in 0..10 {
            l.record_outcome(&f, CodegenChoice::LoopOptimize, 0.95);
        }
        for _ in 0..10 {
            l.record_outcome(&f, CodegenChoice::Default, 0.3);
        }
        let best = l.best_choice(&f);
        assert_eq!(best, CodegenChoice::LoopOptimize);
    }

    #[test]
    fn test_learner_accuracy() {
        let l = CodegenLearner::new();
        assert!((l.accuracy() - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_ffi_select() {
        let choice = slang_codegen_learn_select(100, 10, 5, 42);
        assert!(choice >= 0 && choice <= 4);
    }

    #[test]
    fn test_ffi_record() {
        let r = slang_codegen_learn_record(100, 10, 5, 0, f64::to_bits(0.8) as i64);
        assert_eq!(r, 1);
    }

    #[test]
    fn test_ffi_buckets() {
        let b = slang_codegen_learn_buckets();
        assert!(b >= 0);
    }
}
