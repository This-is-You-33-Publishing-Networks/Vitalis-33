//! Compiler Introspection — Vitalis v626
//!
//! Self-aware compiler that can inspect its own compilation process:
//! - Pipeline stage timing and resource tracking
//! - Pass effectiveness measurement
//! - Code quality metrics after each pass
//! - Compilation bottleneck identification
//! - Self-diagnostic capability

use std::collections::HashMap;

// ── Pipeline Stage Tracking ──────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CompilerStage {
    Lexing,
    Parsing,
    TypeChecking,
    BorrowChecking,
    IrGeneration,
    Optimization,
    CodeGeneration,
    Linking,
    Custom(String),
}

impl CompilerStage {
    pub fn label(&self) -> &str {
        match self {
            CompilerStage::Lexing => "lexing",
            CompilerStage::Parsing => "parsing",
            CompilerStage::TypeChecking => "type_checking",
            CompilerStage::BorrowChecking => "borrow_checking",
            CompilerStage::IrGeneration => "ir_generation",
            CompilerStage::Optimization => "optimization",
            CompilerStage::CodeGeneration => "code_generation",
            CompilerStage::Linking => "linking",
            CompilerStage::Custom(s) => s,
        }
    }
}

/// Recorded metrics for a single compilation pass.
#[derive(Debug, Clone)]
pub struct StageMetrics {
    pub stage: CompilerStage,
    pub duration_us: u64,
    pub instructions_before: usize,
    pub instructions_after: usize,
    pub errors_found: usize,
    pub warnings_found: usize,
    pub memory_bytes: usize,
}

impl StageMetrics {
    pub fn instruction_delta(&self) -> i64 {
        self.instructions_after as i64 - self.instructions_before as i64
    }

    pub fn reduction_ratio(&self) -> f64 {
        if self.instructions_before == 0 { return 0.0; }
        1.0 - (self.instructions_after as f64 / self.instructions_before as f64)
    }

    pub fn instructions_per_us(&self) -> f64 {
        if self.duration_us == 0 { return 0.0; }
        self.instructions_before as f64 / self.duration_us as f64
    }
}

// ── Compilation Profile ──────────────────────────────────────────────

/// Full compilation profile tracking all stages.
pub struct CompilationProfile {
    stages: Vec<StageMetrics>,
    pass_history: HashMap<String, Vec<StageMetrics>>,
}

impl CompilationProfile {
    pub fn new() -> Self {
        Self { stages: Vec::new(), pass_history: HashMap::new() }
    }

    pub fn record(&mut self, metrics: StageMetrics) {
        let key = metrics.stage.label().to_string();
        self.pass_history.entry(key).or_default().push(metrics.clone());
        self.stages.push(metrics);
    }

    pub fn total_duration_us(&self) -> u64 {
        self.stages.iter().map(|s| s.duration_us).sum()
    }

    pub fn total_errors(&self) -> usize {
        self.stages.iter().map(|s| s.errors_found).sum()
    }

    pub fn total_warnings(&self) -> usize {
        self.stages.iter().map(|s| s.warnings_found).sum()
    }

    pub fn total_memory_bytes(&self) -> usize {
        self.stages.iter().map(|s| s.memory_bytes).max().unwrap_or(0)
    }

    pub fn stage_count(&self) -> usize { self.stages.len() }

    /// Find the bottleneck stage (longest duration).
    pub fn bottleneck(&self) -> Option<&StageMetrics> {
        self.stages.iter().max_by_key(|s| s.duration_us)
    }

    /// Identify stages contributing > threshold% of total time.
    pub fn hot_stages(&self, threshold_pct: f64) -> Vec<&StageMetrics> {
        let total = self.total_duration_us() as f64;
        if total < 1.0 { return vec![]; }
        self.stages.iter()
            .filter(|s| (s.duration_us as f64 / total * 100.0) >= threshold_pct)
            .collect()
    }

    /// Get average duration for a named stage across all runs.
    pub fn average_duration(&self, stage_name: &str) -> f64 {
        if let Some(history) = self.pass_history.get(stage_name) {
            if history.is_empty() { return 0.0; }
            let sum: u64 = history.iter().map(|h| h.duration_us).sum();
            sum as f64 / history.len() as f64
        } else {
            0.0
        }
    }

    /// Compute pass effectiveness: ratio of work done per time unit.
    pub fn pass_effectiveness(&self) -> Vec<(String, f64)> {
        let mut result = Vec::new();
        for (name, history) in &self.pass_history {
            let avg_reduction: f64 = if history.is_empty() { 0.0 } else {
                history.iter().map(|h| h.reduction_ratio()).sum::<f64>() / history.len() as f64
            };
            let avg_time: f64 = if history.is_empty() { 0.0 } else {
                history.iter().map(|h| h.duration_us as f64).sum::<f64>() / history.len() as f64
            };
            let effectiveness = if avg_time < 1.0 { avg_reduction } else { avg_reduction / avg_time * 1000.0 };
            result.push((name.clone(), effectiveness));
        }
        result.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        result
    }

    /// Self-diagnostic: identify potential issues in the compilation pipeline.
    pub fn diagnose(&self) -> Vec<String> {
        let mut issues = Vec::new();
        let total = self.total_duration_us();

        // Check for stages that dominate compilation time
        for s in &self.stages {
            if total > 0 && s.duration_us as f64 / total as f64 > 0.5 {
                issues.push(format!(
                    "BOTTLENECK: {} takes {:.1}% of total compilation time",
                    s.stage.label(),
                    s.duration_us as f64 / total as f64 * 100.0
                ));
            }
        }

        // Check for passes that increase code size
        for s in &self.stages {
            if s.instruction_delta() > 0 {
                issues.push(format!(
                    "SIZE_INCREASE: {} added {} instructions",
                    s.stage.label(), s.instruction_delta()
                ));
            }
        }

        // Check for high memory usage
        let peak_mem = self.total_memory_bytes();
        if peak_mem > 500 * 1024 * 1024 {
            issues.push(format!("HIGH_MEMORY: peak memory {}MB", peak_mem / (1024 * 1024)));
        }

        issues
    }
}

impl Default for CompilationProfile {
    fn default() -> Self { Self::new() }
}

// ── FFI Exports ──────────────────────────────────────────────────────

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_introspect_reduction_ratio(before: i64, after: i64) -> f64 {
    if before <= 0 { 0.0 } else { 1.0 - (after as f64 / before as f64) }
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_introspect_bottleneck_pct(
    durations: *const i64, count: i64, index: i64,
) -> f64 {
    if durations.is_null() || count <= 0 || index < 0 || index >= count { return 0.0; }
    let d = unsafe { std::slice::from_raw_parts(durations, count as usize) };
    let total: i64 = d.iter().sum();
    if total <= 0 { 0.0 } else { d[index as usize] as f64 / total as f64 * 100.0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_introspect_throughput(instructions: i64, duration_us: i64) -> f64 {
    if duration_us <= 0 { 0.0 } else { instructions as f64 / duration_us as f64 }
}

// ═══════════════════════════════════════════════════════════════════════
// Tests
// ═══════════════════════════════════════════════════════════════════════
#[cfg(test)]
mod tests {
    use super::*;

    fn mk_stage(stage: CompilerStage, dur: u64, before: usize, after: usize) -> StageMetrics {
        StageMetrics {
            stage, duration_us: dur,
            instructions_before: before, instructions_after: after,
            errors_found: 0, warnings_found: 0, memory_bytes: 1024,
        }
    }

    #[test]
    fn test_stage_metrics_delta() {
        let m = mk_stage(CompilerStage::Optimization, 100, 1000, 800);
        assert_eq!(m.instruction_delta(), -200);
    }

    #[test]
    fn test_stage_reduction_ratio() {
        let m = mk_stage(CompilerStage::Optimization, 100, 1000, 700);
        assert!((m.reduction_ratio() - 0.3).abs() < 1e-10);
    }

    #[test]
    fn test_stage_throughput() {
        let m = mk_stage(CompilerStage::Parsing, 500, 10000, 10000);
        assert!((m.instructions_per_us() - 20.0).abs() < 1e-10);
    }

    #[test]
    fn test_profile_total_duration() {
        let mut p = CompilationProfile::new();
        p.record(mk_stage(CompilerStage::Lexing, 100, 0, 1000));
        p.record(mk_stage(CompilerStage::Parsing, 200, 1000, 1000));
        assert_eq!(p.total_duration_us(), 300);
    }

    #[test]
    fn test_profile_bottleneck() {
        let mut p = CompilationProfile::new();
        p.record(mk_stage(CompilerStage::Lexing, 50, 0, 100));
        p.record(mk_stage(CompilerStage::Optimization, 500, 100, 80));
        p.record(mk_stage(CompilerStage::CodeGeneration, 100, 80, 80));
        let bn = p.bottleneck().unwrap();
        assert_eq!(bn.stage, CompilerStage::Optimization);
    }

    #[test]
    fn test_profile_hot_stages() {
        let mut p = CompilationProfile::new();
        p.record(mk_stage(CompilerStage::Lexing, 10, 0, 100));
        p.record(mk_stage(CompilerStage::Optimization, 90, 100, 50));
        let hot = p.hot_stages(50.0);
        assert_eq!(hot.len(), 1);
        assert_eq!(hot[0].stage, CompilerStage::Optimization);
    }

    #[test]
    fn test_profile_average_duration() {
        let mut p = CompilationProfile::new();
        p.record(mk_stage(CompilerStage::Lexing, 100, 0, 100));
        p.record(mk_stage(CompilerStage::Lexing, 200, 0, 100));
        assert!((p.average_duration("lexing") - 150.0).abs() < 1e-10);
    }

    #[test]
    fn test_profile_effectiveness() {
        let mut p = CompilationProfile::new();
        p.record(mk_stage(CompilerStage::Optimization, 100, 1000, 500));
        let eff = p.pass_effectiveness();
        assert!(!eff.is_empty());
        assert!(eff[0].1 > 0.0);
    }

    #[test]
    fn test_profile_diagnose_bottleneck() {
        let mut p = CompilationProfile::new();
        p.record(mk_stage(CompilerStage::Lexing, 10, 0, 100));
        p.record(mk_stage(CompilerStage::Optimization, 100, 100, 50));
        let issues = p.diagnose();
        assert!(issues.iter().any(|i| i.contains("BOTTLENECK")));
    }

    #[test]
    fn test_profile_diagnose_size_increase() {
        let mut p = CompilationProfile::new();
        p.record(mk_stage(CompilerStage::IrGeneration, 50, 100, 200));
        let issues = p.diagnose();
        assert!(issues.iter().any(|i| i.contains("SIZE_INCREASE")));
    }

    #[test]
    fn test_stage_labels() {
        assert_eq!(CompilerStage::Lexing.label(), "lexing");
        assert_eq!(CompilerStage::Custom("my_pass".into()).label(), "my_pass");
    }

    #[test]
    fn test_empty_profile() {
        let p = CompilationProfile::new();
        assert_eq!(p.total_duration_us(), 0);
        assert_eq!(p.stage_count(), 0);
        assert!(p.bottleneck().is_none());
    }

    #[test]
    fn test_ffi_reduction_ratio() {
        assert!((vitalis_introspect_reduction_ratio(1000, 700) - 0.3).abs() < 1e-10);
        assert_eq!(vitalis_introspect_reduction_ratio(0, 100), 0.0);
    }

    #[test]
    fn test_ffi_bottleneck_pct() {
        let durations = [10i64, 90];
        let pct = vitalis_introspect_bottleneck_pct(durations.as_ptr(), 2, 1);
        assert!((pct - 90.0).abs() < 1e-10);
    }

    #[test]
    fn test_ffi_throughput() {
        assert!((vitalis_introspect_throughput(10000, 500) - 20.0).abs() < 1e-10);
    }
}
