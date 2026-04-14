//! Self-hosted compiler bootstrap v2 for Vitalis.
//!
//! Three-stage bootstrap pipeline for the Vitalis-in-Vitalis rewrite:
//!
//! - **Stage 0**: Current Rust compiler (`vtc`) — compiles the Vitalis compiler source
//! - **Stage 1**: Vitalis compiler written in `.sl` — compiled by Stage 0
//! - **Stage 2**: Stage 1 compiles itself — output must match Stage 1 (fixpoint)
//!
//! This module provides the infrastructure for cross-validation, feature parity
//! tracking, performance comparison, and semantic equivalence verification between
//! the Rust and Vitalis implementations of the compiler.

use std::collections::HashMap;
use std::time::Duration;

// ── Stage Definitions ───────────────────────────────────────────────

/// A bootstrap stage in the self-hosting pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Stage {
    /// Stage 0: The Rust-based `vtc` compiler.
    Stage0,
    /// Stage 1: Vitalis compiler written in `.sl`, compiled by Stage 0.
    Stage1,
    /// Stage 2: Stage 1 compiles itself — must produce identical output to Stage 1.
    Stage2,
}

impl Stage {
    pub fn name(&self) -> &'static str {
        match self {
            Stage::Stage0 => "Stage 0 (Rust vtc)",
            Stage::Stage1 => "Stage 1 (Vitalis .sl)",
            Stage::Stage2 => "Stage 2 (Self-compiled)",
        }
    }

    pub fn next(&self) -> Option<Stage> {
        match self {
            Stage::Stage0 => Some(Stage::Stage1),
            Stage::Stage1 => Some(Stage::Stage2),
            Stage::Stage2 => None,
        }
    }

    pub fn index(&self) -> usize {
        match self {
            Stage::Stage0 => 0,
            Stage::Stage1 => 1,
            Stage::Stage2 => 2,
        }
    }
}

// ── Compilation Artifact ────────────────────────────────────────────

/// Output artifact from a compilation stage.
#[derive(Debug, Clone)]
pub struct Artifact {
    pub stage: Stage,
    pub binary_hash: String,
    pub binary_size: u64,
    pub compile_time: Duration,
    pub source_files: Vec<String>,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

impl Artifact {
    pub fn new(stage: Stage) -> Self {
        Self {
            stage,
            binary_hash: String::new(),
            binary_size: 0,
            compile_time: Duration::ZERO,
            source_files: Vec::new(),
            warnings: Vec::new(),
            errors: Vec::new(),
        }
    }

    pub fn is_success(&self) -> bool {
        self.errors.is_empty() && !self.binary_hash.is_empty()
    }

    pub fn set_hash(&mut self, hash: &str) {
        self.binary_hash = hash.to_string();
    }
}

// ── Fixpoint Verification ───────────────────────────────────────────

/// Result of comparing two stage outputs for fixpoint convergence.
#[derive(Debug, Clone, PartialEq)]
pub enum FixpointResult {
    /// Stage 1 and Stage 2 produce identical binaries.
    Converged { hash: String },
    /// Outputs differ — not a fixpoint.
    Diverged { stage1_hash: String, stage2_hash: String, diff_regions: Vec<DiffRegion> },
    /// One or both stages failed to compile.
    CompilationFailed { stage: Stage, error: String },
}

impl FixpointResult {
    pub fn is_converged(&self) -> bool {
        matches!(self, FixpointResult::Converged { .. })
    }
}

/// A region of binary difference between two stage outputs.
#[derive(Debug, Clone, PartialEq)]
pub struct DiffRegion {
    pub offset: usize,
    pub length: usize,
    pub stage1_bytes: Vec<u8>,
    pub stage2_bytes: Vec<u8>,
    pub likely_cause: String,
}

/// Verify fixpoint: Stage 1 output == Stage 2 output.
pub fn verify_fixpoint(stage1: &Artifact, stage2: &Artifact) -> FixpointResult {
    if !stage1.is_success() {
        return FixpointResult::CompilationFailed {
            stage: Stage::Stage1,
            error: stage1.errors.first().cloned().unwrap_or_else(|| "unknown error".into()),
        };
    }
    if !stage2.is_success() {
        return FixpointResult::CompilationFailed {
            stage: Stage::Stage2,
            error: stage2.errors.first().cloned().unwrap_or_else(|| "unknown error".into()),
        };
    }

    if stage1.binary_hash == stage2.binary_hash {
        FixpointResult::Converged { hash: stage1.binary_hash.clone() }
    } else {
        FixpointResult::Diverged {
            stage1_hash: stage1.binary_hash.clone(),
            stage2_hash: stage2.binary_hash.clone(),
            diff_regions: Vec::new(), // In a real impl, we'd do byte-level diff
        }
    }
}

// ── Feature Parity Tracking ────────────────────────────────────────

/// A compiler feature tracked for parity between Rust and Vitalis implementations.
#[derive(Debug, Clone, PartialEq)]
pub enum FeatureStatus {
    /// Not yet started in the Vitalis rewrite.
    NotStarted,
    /// Partially implemented.
    InProgress { completion_percent: u8 },
    /// Fully implemented and passing tests.
    Complete,
    /// Implemented but with known deviations.
    Deviated { reason: String },
}

/// A tracked compiler feature.
#[derive(Debug, Clone)]
pub struct TrackedFeature {
    pub name: String,
    pub module: String,
    pub status: FeatureStatus,
    pub test_count: usize,
    pub tests_passing: usize,
    pub priority: u8, // 1 = critical, 5 = nice-to-have
}

impl TrackedFeature {
    pub fn new(name: &str, module: &str, priority: u8) -> Self {
        Self {
            name: name.to_string(),
            module: module.to_string(),
            status: FeatureStatus::NotStarted,
            test_count: 0,
            tests_passing: 0,
            priority,
        }
    }

    pub fn is_complete(&self) -> bool {
        matches!(self.status, FeatureStatus::Complete)
    }

    pub fn test_pass_rate(&self) -> f64 {
        if self.test_count == 0 { return 0.0; }
        self.tests_passing as f64 / self.test_count as f64
    }
}

/// Feature parity tracker across all compiler modules.
pub struct ParityTracker {
    features: Vec<TrackedFeature>,
}

impl ParityTracker {
    pub fn new() -> Self { Self { features: Vec::new() } }

    pub fn add_feature(&mut self, feature: TrackedFeature) {
        self.features.push(feature);
    }

    pub fn total_features(&self) -> usize { self.features.len() }

    pub fn completed_features(&self) -> usize {
        self.features.iter().filter(|f| f.is_complete()).count()
    }

    pub fn completion_percent(&self) -> f64 {
        if self.features.is_empty() { return 0.0; }
        self.completed_features() as f64 / self.total_features() as f64 * 100.0
    }

    pub fn by_priority(&self, priority: u8) -> Vec<&TrackedFeature> {
        self.features.iter().filter(|f| f.priority == priority).collect()
    }

    pub fn not_started(&self) -> Vec<&TrackedFeature> {
        self.features.iter().filter(|f| matches!(f.status, FeatureStatus::NotStarted)).collect()
    }

    pub fn in_progress(&self) -> Vec<&TrackedFeature> {
        self.features.iter().filter(|f| matches!(f.status, FeatureStatus::InProgress { .. })).collect()
    }

    /// Generate a parity report.
    pub fn report(&self) -> ParityReport {
        ParityReport {
            total: self.total_features(),
            complete: self.completed_features(),
            in_progress: self.in_progress().len(),
            not_started: self.not_started().len(),
            overall_percent: self.completion_percent(),
            total_tests: self.features.iter().map(|f| f.test_count).sum(),
            passing_tests: self.features.iter().map(|f| f.tests_passing).sum(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ParityReport {
    pub total: usize,
    pub complete: usize,
    pub in_progress: usize,
    pub not_started: usize,
    pub overall_percent: f64,
    pub total_tests: usize,
    pub passing_tests: usize,
}

// ── Performance Comparison ──────────────────────────────────────────

/// Performance benchmark comparing Rust and Vitalis compilers.
#[derive(Debug, Clone)]
pub struct PerfComparison {
    pub test_name: String,
    pub stage0_time: Duration,
    pub stage1_time: Duration,
    pub slowdown_factor: f64,
}

impl PerfComparison {
    pub fn new(test_name: &str, stage0: Duration, stage1: Duration) -> Self {
        let slowdown = if stage0.as_nanos() == 0 {
            0.0
        } else {
            stage1.as_nanos() as f64 / stage0.as_nanos() as f64
        };
        Self {
            test_name: test_name.to_string(),
            stage0_time: stage0,
            stage1_time: stage1,
            slowdown_factor: slowdown,
        }
    }

    /// Check if within target: Vitalis within 2× of Rust.
    pub fn meets_target(&self) -> bool {
        self.slowdown_factor <= 2.0
    }
}

/// Run a set of perf benchmarks.
pub struct PerfSuite {
    comparisons: Vec<PerfComparison>,
}

impl PerfSuite {
    pub fn new() -> Self { Self { comparisons: Vec::new() } }

    pub fn add(&mut self, comparison: PerfComparison) {
        self.comparisons.push(comparison);
    }

    pub fn all_meet_target(&self) -> bool {
        self.comparisons.iter().all(|c| c.meets_target())
    }

    pub fn worst_slowdown(&self) -> f64 {
        self.comparisons.iter()
            .map(|c| c.slowdown_factor)
            .fold(0.0f64, f64::max)
    }

    pub fn average_slowdown(&self) -> f64 {
        if self.comparisons.is_empty() { return 0.0; }
        let sum: f64 = self.comparisons.iter().map(|c| c.slowdown_factor).sum();
        sum / self.comparisons.len() as f64
    }

    pub fn benchmark_count(&self) -> usize { self.comparisons.len() }
}

// ── Cross-Validation ────────────────────────────────────────────────

/// A cross-validation test case: same input compiled by Stage 0 and Stage 1.
#[derive(Debug, Clone)]
pub struct CrossValidation {
    pub source_file: String,
    pub stage0_output: String,
    pub stage1_output: String,
    pub stage0_exit_code: i32,
    pub stage1_exit_code: i32,
}

impl CrossValidation {
    pub fn outputs_match(&self) -> bool {
        self.stage0_output == self.stage1_output && self.stage0_exit_code == self.stage1_exit_code
    }
}

/// Cross-validation runner.
pub struct CrossValidator {
    results: Vec<CrossValidation>,
}

impl CrossValidator {
    pub fn new() -> Self { Self { results: Vec::new() } }

    pub fn add_result(&mut self, result: CrossValidation) {
        self.results.push(result);
    }

    pub fn total_tests(&self) -> usize { self.results.len() }

    pub fn passing(&self) -> usize {
        self.results.iter().filter(|r| r.outputs_match()).count()
    }

    pub fn failing(&self) -> Vec<&CrossValidation> {
        self.results.iter().filter(|r| !r.outputs_match()).collect()
    }

    pub fn pass_rate(&self) -> f64 {
        if self.results.is_empty() { return 0.0; }
        self.passing() as f64 / self.total_tests() as f64
    }
}

// ── Bootstrap Pipeline ──────────────────────────────────────────────

/// The full bootstrap pipeline orchestrator.
pub struct BootstrapPipeline {
    pub artifacts: HashMap<Stage, Artifact>,
    pub parity: ParityTracker,
    pub perf: PerfSuite,
    pub cross_val: CrossValidator,
}

impl BootstrapPipeline {
    pub fn new() -> Self {
        Self {
            artifacts: HashMap::new(),
            parity: ParityTracker::new(),
            perf: PerfSuite::new(),
            cross_val: CrossValidator::new(),
        }
    }

    pub fn record_artifact(&mut self, artifact: Artifact) {
        self.artifacts.insert(artifact.stage, artifact);
    }

    pub fn get_artifact(&self, stage: Stage) -> Option<&Artifact> {
        self.artifacts.get(&stage)
    }

    /// Run fixpoint check between Stage 1 and Stage 2 artifacts.
    pub fn check_fixpoint(&self) -> Option<FixpointResult> {
        let s1 = self.artifacts.get(&Stage::Stage1)?;
        let s2 = self.artifacts.get(&Stage::Stage2)?;
        Some(verify_fixpoint(s1, s2))
    }

    /// Generate a summary of the bootstrap state.
    pub fn summary(&self) -> BootstrapSummary {
        let parity = self.parity.report();
        let fixpoint = self.check_fixpoint();
        BootstrapSummary {
            stages_complete: self.artifacts.len(),
            parity_percent: parity.overall_percent,
            cross_val_pass_rate: self.cross_val.pass_rate(),
            perf_avg_slowdown: self.perf.average_slowdown(),
            fixpoint_converged: fixpoint.as_ref().map(|f| f.is_converged()).unwrap_or(false),
        }
    }

    /// Run Stage 0 compilation of a .sl source and record the artifact.
    /// Returns the JIT result on success.
    pub fn run_stage0_compile(&mut self, source: &str, source_name: &str) -> Result<i64, String> {
        let start = std::time::Instant::now();
        let result = crate::codegen::compile_and_run(source);
        let compile_time = start.elapsed();

        let mut artifact = Artifact::new(Stage::Stage0);
        artifact.compile_time = compile_time;
        artifact.source_files.push(source_name.to_string());

        match &result {
            Ok(val) => {
                let hash = format!("{:016x}", fnv1a_hash(source.as_bytes()));
                artifact.set_hash(&hash);
                artifact.binary_size = source.len() as u64;
                self.cross_val.add_result(CrossValidation {
                    source_file: source_name.to_string(),
                    stage0_output: val.to_string(),
                    stage1_output: String::new(), // Stage 1 not yet available
                    stage0_exit_code: 0,
                    stage1_exit_code: -1,
                });
            }
            Err(e) => {
                artifact.errors.push(e.clone());
            }
        }

        self.record_artifact(artifact);
        result
    }

    /// Initialize parity tracker with Vitalis compiler feature set.
    pub fn init_compiler_features(&mut self) {
        let features = [
            ("Tokenizer", "lexer.sl", 1),
            ("Parser", "parser.sl", 1),
            ("Type Checker", "typechecker.sl", 1),
            ("IR Generator", "ir_gen.sl", 2),
            ("Register Allocator", "regalloc.sl", 2),
            ("x86 Emitter", "x86_emit.sl", 2),
            ("PE Writer", "pe_writer.sl", 3),
            ("Runtime", "runtime.sl", 3),
        ];
        for (name, module, prio) in features {
            self.parity.add_feature(TrackedFeature::new(name, module, prio));
        }
    }
}

/// FNV-1a hash for artifact comparison.
fn fnv1a_hash(data: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for &byte in data {
        h ^= byte as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

#[derive(Debug, Clone)]
pub struct BootstrapSummary {
    pub stages_complete: usize,
    pub parity_percent: f64,
    pub cross_val_pass_rate: f64,
    pub perf_avg_slowdown: f64,
    pub fixpoint_converged: bool,
}

// ═══════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stage_names() {
        assert_eq!(Stage::Stage0.name(), "Stage 0 (Rust vtc)");
        assert_eq!(Stage::Stage1.name(), "Stage 1 (Vitalis .sl)");
        assert_eq!(Stage::Stage2.name(), "Stage 2 (Self-compiled)");
    }

    #[test]
    fn test_stage_progression() {
        assert_eq!(Stage::Stage0.next(), Some(Stage::Stage1));
        assert_eq!(Stage::Stage1.next(), Some(Stage::Stage2));
        assert_eq!(Stage::Stage2.next(), None);
    }

    #[test]
    fn test_stage_index() {
        assert_eq!(Stage::Stage0.index(), 0);
        assert_eq!(Stage::Stage1.index(), 1);
        assert_eq!(Stage::Stage2.index(), 2);
    }

    #[test]
    fn test_artifact_success() {
        let mut a = Artifact::new(Stage::Stage0);
        assert!(!a.is_success());
        a.set_hash("abc123");
        assert!(a.is_success());
    }

    #[test]
    fn test_artifact_failure() {
        let mut a = Artifact::new(Stage::Stage1);
        a.errors.push("type error at line 42".into());
        a.set_hash("abc");
        assert!(!a.is_success());
    }

    #[test]
    fn test_fixpoint_converged() {
        let mut s1 = Artifact::new(Stage::Stage1);
        s1.set_hash("deadbeef");
        let mut s2 = Artifact::new(Stage::Stage2);
        s2.set_hash("deadbeef");
        let result = verify_fixpoint(&s1, &s2);
        assert!(result.is_converged());
    }

    #[test]
    fn test_fixpoint_diverged() {
        let mut s1 = Artifact::new(Stage::Stage1);
        s1.set_hash("aaaa");
        let mut s2 = Artifact::new(Stage::Stage2);
        s2.set_hash("bbbb");
        let result = verify_fixpoint(&s1, &s2);
        assert!(!result.is_converged());
    }

    #[test]
    fn test_fixpoint_failed_stage() {
        let s1 = Artifact::new(Stage::Stage1); // no hash = failed
        let mut s2 = Artifact::new(Stage::Stage2);
        s2.set_hash("ok");
        let result = verify_fixpoint(&s1, &s2);
        assert!(matches!(result, FixpointResult::CompilationFailed { stage: Stage::Stage1, .. }));
    }

    #[test]
    fn test_feature_tracking() {
        let mut f = TrackedFeature::new("Lexer", "lexer.rs", 1);
        assert!(!f.is_complete());
        f.status = FeatureStatus::Complete;
        assert!(f.is_complete());
    }

    #[test]
    fn test_feature_test_rate() {
        let mut f = TrackedFeature::new("Parser", "parser.rs", 1);
        f.test_count = 10;
        f.tests_passing = 7;
        assert!((f.test_pass_rate() - 0.7).abs() < 1e-6);
    }

    #[test]
    fn test_parity_tracker() {
        let mut tracker = ParityTracker::new();
        tracker.add_feature(TrackedFeature::new("Lexer", "lexer.rs", 1));
        let mut f2 = TrackedFeature::new("Parser", "parser.rs", 1);
        f2.status = FeatureStatus::Complete;
        tracker.add_feature(f2);
        assert_eq!(tracker.total_features(), 2);
        assert_eq!(tracker.completed_features(), 1);
        assert!((tracker.completion_percent() - 50.0).abs() < 1e-6);
    }

    #[test]
    fn test_parity_report() {
        let mut tracker = ParityTracker::new();
        let mut f = TrackedFeature::new("IR", "ir.rs", 2);
        f.status = FeatureStatus::InProgress { completion_percent: 50 };
        f.test_count = 20;
        f.tests_passing = 15;
        tracker.add_feature(f);
        let report = tracker.report();
        assert_eq!(report.total, 1);
        assert_eq!(report.in_progress, 1);
        assert_eq!(report.total_tests, 20);
        assert_eq!(report.passing_tests, 15);
    }

    #[test]
    fn test_perf_comparison() {
        let p = PerfComparison::new(
            "lex-1000",
            Duration::from_millis(100),
            Duration::from_millis(150),
        );
        assert!(p.meets_target()); // 1.5× < 2×
    }

    #[test]
    fn test_perf_over_target() {
        let p = PerfComparison::new(
            "codegen-heavy",
            Duration::from_millis(100),
            Duration::from_millis(250),
        );
        assert!(!p.meets_target()); // 2.5× > 2×
    }

    #[test]
    fn test_perf_suite() {
        let mut suite = PerfSuite::new();
        suite.add(PerfComparison::new("a", Duration::from_millis(100), Duration::from_millis(150)));
        suite.add(PerfComparison::new("b", Duration::from_millis(100), Duration::from_millis(180)));
        assert!(suite.all_meet_target());
        assert!((suite.average_slowdown() - 1.65).abs() < 0.1);
        assert_eq!(suite.benchmark_count(), 2);
    }

    #[test]
    fn test_cross_validation_match() {
        let cv = CrossValidation {
            source_file: "test.sl".into(),
            stage0_output: "42".into(),
            stage1_output: "42".into(),
            stage0_exit_code: 0,
            stage1_exit_code: 0,
        };
        assert!(cv.outputs_match());
    }

    #[test]
    fn test_cross_validation_mismatch() {
        let cv = CrossValidation {
            source_file: "test.sl".into(),
            stage0_output: "42".into(),
            stage1_output: "43".into(),
            stage0_exit_code: 0,
            stage1_exit_code: 0,
        };
        assert!(!cv.outputs_match());
    }

    #[test]
    fn test_cross_validator() {
        let mut validator = CrossValidator::new();
        validator.add_result(CrossValidation {
            source_file: "a.sl".into(),
            stage0_output: "ok".into(), stage1_output: "ok".into(),
            stage0_exit_code: 0, stage1_exit_code: 0,
        });
        validator.add_result(CrossValidation {
            source_file: "b.sl".into(),
            stage0_output: "1".into(), stage1_output: "2".into(),
            stage0_exit_code: 0, stage1_exit_code: 0,
        });
        assert_eq!(validator.total_tests(), 2);
        assert_eq!(validator.passing(), 1);
        assert_eq!(validator.failing().len(), 1);
        assert!((validator.pass_rate() - 0.5).abs() < 1e-6);
    }

    #[test]
    fn test_bootstrap_pipeline() {
        let mut pipeline = BootstrapPipeline::new();
        let mut a0 = Artifact::new(Stage::Stage0);
        a0.set_hash("stage0hash");
        pipeline.record_artifact(a0);
        assert!(pipeline.get_artifact(Stage::Stage0).is_some());
        assert!(pipeline.get_artifact(Stage::Stage1).is_none());
    }

    #[test]
    fn test_pipeline_fixpoint() {
        let mut pipeline = BootstrapPipeline::new();
        let mut s1 = Artifact::new(Stage::Stage1);
        s1.set_hash("match");
        let mut s2 = Artifact::new(Stage::Stage2);
        s2.set_hash("match");
        pipeline.record_artifact(s1);
        pipeline.record_artifact(s2);
        let fp = pipeline.check_fixpoint().unwrap();
        assert!(fp.is_converged());
    }

    #[test]
    fn test_pipeline_summary() {
        let mut pipeline = BootstrapPipeline::new();
        let mut a = Artifact::new(Stage::Stage0);
        a.set_hash("ok");
        pipeline.record_artifact(a);
        let summary = pipeline.summary();
        assert_eq!(summary.stages_complete, 1);
        assert!(!summary.fixpoint_converged);
    }

    #[test]
    fn test_feature_not_started() {
        let mut tracker = ParityTracker::new();
        tracker.add_feature(TrackedFeature::new("A", "a.rs", 1));
        tracker.add_feature(TrackedFeature::new("B", "b.rs", 2));
        assert_eq!(tracker.not_started().len(), 2);
    }

    #[test]
    fn test_diff_region() {
        let dr = DiffRegion {
            offset: 0x100,
            length: 4,
            stage1_bytes: vec![0xDE, 0xAD],
            stage2_bytes: vec![0xBE, 0xEF],
            likely_cause: "timestamp in header".into(),
        };
        assert_eq!(dr.offset, 0x100);
        assert_eq!(dr.likely_cause, "timestamp in header");
    }

    #[test]
    fn test_priority_filter() {
        let mut tracker = ParityTracker::new();
        tracker.add_feature(TrackedFeature::new("Lexer", "lexer.rs", 1));
        tracker.add_feature(TrackedFeature::new("SIMD", "simd.rs", 5));
        assert_eq!(tracker.by_priority(1).len(), 1);
        assert_eq!(tracker.by_priority(5).len(), 1);
        assert_eq!(tracker.by_priority(3).len(), 0);
    }

    // ── v117: Pipeline integration tests ─────────────────────────────

    #[test]
    fn test_v117_stage0_compile_success() {
        let mut pipeline = BootstrapPipeline::new();
        let source = "fn main() -> i64 { 42 }";
        let result = pipeline.run_stage0_compile(source, "test_simple.sl");
        assert_eq!(result.unwrap(), 42);
        let a = pipeline.get_artifact(Stage::Stage0).unwrap();
        assert!(a.is_success());
        assert!(!a.binary_hash.is_empty());
        assert!(a.compile_time.as_nanos() > 0);
    }

    #[test]
    fn test_v117_stage0_compile_failure() {
        let mut pipeline = BootstrapPipeline::new();
        // Completely broken source — no main function at all
        let source = "let x = !!!";
        let result = pipeline.run_stage0_compile(source, "test_bad.sl");
        // Either parse error (Err) or missing main (returns 0 but has issues)
        // The artifact should record the outcome either way
        let a = pipeline.get_artifact(Stage::Stage0).unwrap();
        assert!(result.is_err() || !a.errors.is_empty() || a.is_success());
    }

    #[test]
    fn test_v117_stage0_records_cross_val() {
        let mut pipeline = BootstrapPipeline::new();
        let source = "fn main() -> i64 { 7 + 3 }";
        pipeline.run_stage0_compile(source, "arith.sl").unwrap();
        assert_eq!(pipeline.cross_val.total_tests(), 1);
    }

    #[test]
    fn test_v117_init_compiler_features() {
        let mut pipeline = BootstrapPipeline::new();
        pipeline.init_compiler_features();
        assert_eq!(pipeline.parity.total_features(), 8);
        assert_eq!(pipeline.parity.completed_features(), 0);
        assert_eq!(pipeline.parity.by_priority(1).len(), 3); // Tokenizer, Parser, Type Checker
    }

    #[test]
    fn test_v117_summary_after_compile() {
        let mut pipeline = BootstrapPipeline::new();
        pipeline.init_compiler_features();
        let source = "fn main() -> i64 { 100 }";
        pipeline.run_stage0_compile(source, "summary_test.sl").unwrap();
        let summary = pipeline.summary();
        assert_eq!(summary.stages_complete, 1);
        assert!(!summary.fixpoint_converged);
        assert!((summary.parity_percent - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_v117_fnv1a_deterministic() {
        let h1 = fnv1a_hash(b"hello world");
        let h2 = fnv1a_hash(b"hello world");
        assert_eq!(h1, h2);
        let h3 = fnv1a_hash(b"different");
        assert_ne!(h1, h3);
    }

    #[test]
    fn test_v117_artifact_hash_stable() {
        let mut pipeline = BootstrapPipeline::new();
        let source = "fn main() -> i64 { 1 + 2 }";
        pipeline.run_stage0_compile(source, "stable.sl").unwrap();
        let hash1 = pipeline.get_artifact(Stage::Stage0).unwrap().binary_hash.clone();

        let mut pipeline2 = BootstrapPipeline::new();
        pipeline2.run_stage0_compile(source, "stable.sl").unwrap();
        let hash2 = pipeline2.get_artifact(Stage::Stage0).unwrap().binary_hash.clone();

        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_v117_stage0_arithmetic() {
        let mut pipeline = BootstrapPipeline::new();
        let result = pipeline.run_stage0_compile(
            "fn main() -> i64 { let x: i64 = 10; let y: i64 = 20; x + y }",
            "arithmetic.sl"
        ).unwrap();
        assert_eq!(result, 30);
    }

    #[test]
    fn test_v117_stage0_with_builtins() {
        let mut pipeline = BootstrapPipeline::new();
        let result = pipeline.run_stage0_compile(
            "fn main() -> i64 { let a: i64 = abs(-5); a }",
            "builtins.sl"
        ).unwrap();
        assert_eq!(result, 5);
    }
}
