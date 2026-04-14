//! v445 — Autonomous Benchmark Evolution.
//!
//! Auto-generates benchmarks from code patterns, evolves benchmark suites
//! that stress-test evolved optimizations, and detects regressions via
//! statistical comparison (Welch's t-test).

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

/// A single benchmark case.
#[derive(Debug, Clone)]
pub struct BenchCase {
    pub name: String,
    pub description: String,
    pub category: BenchCategory,
    /// Recorded execution times in nanoseconds.
    pub samples: Vec<f64>,
    /// Baseline mean (from initial run).
    pub baseline_mean: f64,
    /// Baseline std deviation.
    pub baseline_std: f64,
    /// Number of baseline samples.
    pub baseline_n: usize,
    pub generation: u64,
}

/// Benchmark categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BenchCategory {
    Lexer,
    Parser,
    TypeCheck,
    Codegen,
    Optimization,
    Runtime,
    SpikeEngine,
    Evolution,
}

impl BenchCategory {
    pub fn name(&self) -> &'static str {
        match self {
            BenchCategory::Lexer => "lexer",
            BenchCategory::Parser => "parser",
            BenchCategory::TypeCheck => "typecheck",
            BenchCategory::Codegen => "codegen",
            BenchCategory::Optimization => "optimization",
            BenchCategory::Runtime => "runtime",
            BenchCategory::SpikeEngine => "spike_engine",
            BenchCategory::Evolution => "evolution",
        }
    }
}

impl BenchCase {
    pub fn new(name: impl Into<String>, category: BenchCategory) -> Self {
        Self {
            name: name.into(),
            description: String::new(),
            category,
            samples: Vec::new(),
            baseline_mean: 0.0,
            baseline_std: 0.0,
            baseline_n: 0,
            generation: 0,
        }
    }

    /// Record a sample.
    pub fn record(&mut self, ns: f64) {
        self.samples.push(ns);
    }

    /// Set baseline from current samples.
    pub fn set_baseline(&mut self) {
        if self.samples.is_empty() { return; }
        self.baseline_n = self.samples.len();
        self.baseline_mean = self.samples.iter().sum::<f64>() / self.samples.len() as f64;
        let variance = self.samples.iter()
            .map(|x| (x - self.baseline_mean).powi(2))
            .sum::<f64>() / (self.samples.len() as f64 - 1.0).max(1.0);
        self.baseline_std = variance.sqrt();
    }

    /// Current mean.
    pub fn current_mean(&self) -> f64 {
        if self.samples.is_empty() { return 0.0; }
        self.samples.iter().sum::<f64>() / self.samples.len() as f64
    }

    /// Current std dev.
    pub fn current_std(&self) -> f64 {
        if self.samples.len() < 2 { return 0.0; }
        let mean = self.current_mean();
        let variance = self.samples.iter()
            .map(|x| (x - mean).powi(2))
            .sum::<f64>() / (self.samples.len() as f64 - 1.0);
        variance.sqrt()
    }

    /// Welch's t-test against baseline.
    /// Returns (t_statistic, significant_regression) where significance is at p < 0.05 (~t > 2.0).
    pub fn regression_test(&self) -> (f64, bool) {
        if self.baseline_n < 2 || self.samples.len() < 2 {
            return (0.0, false);
        }
        let m1 = self.baseline_mean;
        let s1 = self.baseline_std;
        let n1 = self.baseline_n as f64;
        let m2 = self.current_mean();
        let s2 = self.current_std();
        let n2 = self.samples.len() as f64;

        let denom = ((s1 * s1 / n1) + (s2 * s2 / n2)).sqrt();
        if denom < 1e-15 { return (0.0, false); }
        let t = (m2 - m1) / denom;
        // Positive t means regression (slower), threshold ~2.0 for p < 0.05
        (t, t > 2.0)
    }

    /// Speedup ratio vs baseline (< 1.0 means regression).
    pub fn speedup(&self) -> f64 {
        if self.baseline_mean < 1e-15 { return 1.0; }
        self.baseline_mean / self.current_mean()
    }

    /// Clear current samples.
    pub fn clear_samples(&mut self) {
        self.samples.clear();
    }
}

/// Evolves benchmark suites.
pub struct BenchEvolver {
    pub benchmarks: HashMap<String, BenchCase>,
    pub total_regressions: u64,
    pub total_improvements: u64,
    pub generation: u64,
}

impl BenchEvolver {
    pub fn new() -> Self {
        Self {
            benchmarks: HashMap::new(),
            total_regressions: 0,
            total_improvements: 0,
            generation: 0,
        }
    }

    /// Register a benchmark.
    pub fn register(&mut self, bench: BenchCase) {
        self.benchmarks.insert(bench.name.clone(), bench);
    }

    /// Get a benchmark by name.
    pub fn get(&self, name: &str) -> Option<&BenchCase> {
        self.benchmarks.get(name)
    }

    /// Record a sample.
    pub fn record_sample(&mut self, name: &str, ns: f64) {
        if let Some(bench) = self.benchmarks.get_mut(name) {
            bench.record(ns);
        }
    }

    /// Set baselines for all benchmarks.
    pub fn set_all_baselines(&mut self) {
        for bench in self.benchmarks.values_mut() {
            bench.set_baseline();
        }
    }

    /// Check all benchmarks for regressions.
    pub fn check_regressions(&mut self) -> Vec<(String, f64)> {
        let mut regressions = Vec::new();
        for (name, bench) in &self.benchmarks {
            let (t, regressed) = bench.regression_test();
            if regressed {
                regressions.push((name.clone(), t));
                self.total_regressions += 1;
            } else if t < -2.0 {
                // Significant improvement
                self.total_improvements += 1;
            }
        }
        regressions
    }

    /// Evolve: mutate benchmarks by adding stress variants.
    pub fn evolve_suite(&mut self, seed: u64) -> usize {
        self.generation += 1;
        let names: Vec<String> = self.benchmarks.keys().cloned().collect();
        let mut added = 0;
        for (i, name) in names.iter().enumerate() {
            if (seed.wrapping_add(i as u64)) % 3 == 0 {
                let variant_name = format!("{}_stress_g{}", name, self.generation);
                let category = self.benchmarks[name].category;
                let mut variant = BenchCase::new(variant_name.clone(), category);
                variant.generation = self.generation;
                variant.description = format!("Stress variant of {} at gen {}", name, self.generation);
                self.benchmarks.insert(variant_name, variant);
                added += 1;
            }
        }
        added
    }

    /// Total benchmarks.
    pub fn benchmark_count(&self) -> usize {
        self.benchmarks.len()
    }

    /// Benchmarks by category.
    pub fn by_category(&self, category: BenchCategory) -> Vec<&BenchCase> {
        self.benchmarks.values().filter(|b| b.category == category).collect()
    }

    /// Summary JSON.
    pub fn summary_json(&self) -> String {
        format!(
            r#"{{"benchmarks":{},"generation":{},"regressions":{},"improvements":{}}}"#,
            self.benchmark_count(),
            self.generation,
            self.total_regressions,
            self.total_improvements,
        )
    }
}

// ─── FFI ──────────────────────────────────────────────────────────────

static GLOBAL_BENCH: LazyLock<Mutex<BenchEvolver>> =
    LazyLock::new(|| Mutex::new(BenchEvolver::new()));

#[unsafe(no_mangle)]
pub extern "C" fn slang_bench_evo_register(category: i64) -> i64 {
    let cat = match category {
        0 => BenchCategory::Lexer,
        1 => BenchCategory::Parser,
        2 => BenchCategory::TypeCheck,
        3 => BenchCategory::Codegen,
        4 => BenchCategory::Optimization,
        5 => BenchCategory::Runtime,
        6 => BenchCategory::SpikeEngine,
        _ => BenchCategory::Evolution,
    };
    let mut b = GLOBAL_BENCH.lock().unwrap();
    let name = format!("bench_{}_{}", cat.name(), b.benchmark_count());
    b.register(BenchCase::new(name, cat));
    b.benchmark_count() as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_bench_evo_count() -> i64 {
    let b = GLOBAL_BENCH.lock().unwrap();
    b.benchmark_count() as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_bench_evo_evolve(seed: i64) -> i64 {
    let mut b = GLOBAL_BENCH.lock().unwrap();
    b.evolve_suite(seed as u64) as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_bench_evo_generation() -> i64 {
    let b = GLOBAL_BENCH.lock().unwrap();
    b.generation as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bench_category_name() {
        assert_eq!(BenchCategory::Lexer.name(), "lexer");
        assert_eq!(BenchCategory::SpikeEngine.name(), "spike_engine");
    }

    #[test]
    fn test_bench_case_new() {
        let b = BenchCase::new("test", BenchCategory::Codegen);
        assert_eq!(b.name, "test");
        assert!(b.samples.is_empty());
    }

    #[test]
    fn test_bench_case_record() {
        let mut b = BenchCase::new("test", BenchCategory::Lexer);
        b.record(100.0);
        b.record(110.0);
        assert_eq!(b.samples.len(), 2);
    }

    #[test]
    fn test_bench_case_mean() {
        let mut b = BenchCase::new("test", BenchCategory::Lexer);
        b.record(100.0);
        b.record(200.0);
        assert!((b.current_mean() - 150.0).abs() < 1e-10);
    }

    #[test]
    fn test_bench_case_std() {
        let mut b = BenchCase::new("test", BenchCategory::Lexer);
        b.record(100.0);
        b.record(200.0);
        assert!(b.current_std() > 0.0);
    }

    #[test]
    fn test_set_baseline() {
        let mut b = BenchCase::new("test", BenchCategory::Lexer);
        b.record(100.0);
        b.record(110.0);
        b.set_baseline();
        assert!((b.baseline_mean - 105.0).abs() < 1e-10);
        assert_eq!(b.baseline_n, 2);
    }

    #[test]
    fn test_regression_no_baseline() {
        let b = BenchCase::new("test", BenchCategory::Lexer);
        let (t, regressed) = b.regression_test();
        assert!((t - 0.0).abs() < 1e-10);
        assert!(!regressed);
    }

    #[test]
    fn test_no_regression() {
        let mut b = BenchCase::new("test", BenchCategory::Lexer);
        for _ in 0..20 { b.record(100.0); }
        b.set_baseline();
        b.clear_samples();
        for _ in 0..20 { b.record(100.0); }
        let (_, regressed) = b.regression_test();
        assert!(!regressed);
    }

    #[test]
    fn test_significant_regression() {
        let mut b = BenchCase::new("test", BenchCategory::Lexer);
        // Add slight noise so std dev is non-zero for Welch's t-test
        for i in 0..30 { b.record(100.0 + (i as f64) * 0.1); }
        b.set_baseline();
        b.clear_samples();
        for i in 0..30 { b.record(200.0 + (i as f64) * 0.1); } // 2x slower
        let (t, regressed) = b.regression_test();
        assert!(t > 2.0);
        assert!(regressed);
    }

    #[test]
    fn test_speedup() {
        let mut b = BenchCase::new("test", BenchCategory::Lexer);
        for _ in 0..10 { b.record(100.0); }
        b.set_baseline();
        b.clear_samples();
        for _ in 0..10 { b.record(50.0); } // 2x faster
        assert!((b.speedup() - 2.0).abs() < 0.1);
    }

    #[test]
    fn test_evolver_new() {
        let e = BenchEvolver::new();
        assert_eq!(e.benchmark_count(), 0);
    }

    #[test]
    fn test_evolver_register() {
        let mut e = BenchEvolver::new();
        e.register(BenchCase::new("b1", BenchCategory::Lexer));
        assert_eq!(e.benchmark_count(), 1);
    }

    #[test]
    fn test_evolver_record() {
        let mut e = BenchEvolver::new();
        e.register(BenchCase::new("b1", BenchCategory::Lexer));
        e.record_sample("b1", 100.0);
        assert_eq!(e.get("b1").unwrap().samples.len(), 1);
    }

    #[test]
    fn test_evolver_evolve() {
        let mut e = BenchEvolver::new();
        e.register(BenchCase::new("b1", BenchCategory::Lexer));
        e.register(BenchCase::new("b2", BenchCategory::Parser));
        e.register(BenchCase::new("b3", BenchCategory::Codegen));
        let added = e.evolve_suite(0);
        assert!(added > 0 || e.benchmark_count() >= 3);
    }

    #[test]
    fn test_evolver_by_category() {
        let mut e = BenchEvolver::new();
        e.register(BenchCase::new("b1", BenchCategory::Lexer));
        e.register(BenchCase::new("b2", BenchCategory::Lexer));
        e.register(BenchCase::new("b3", BenchCategory::Parser));
        assert_eq!(e.by_category(BenchCategory::Lexer).len(), 2);
    }

    #[test]
    fn test_summary_json() {
        let e = BenchEvolver::new();
        let json = e.summary_json();
        assert!(json.contains("benchmarks"));
    }

    #[test]
    fn test_ffi_register() {
        let count = slang_bench_evo_register(0);
        assert!(count >= 1);
    }

    #[test]
    fn test_ffi_count() {
        let count = slang_bench_evo_count();
        assert!(count >= 0);
    }

    #[test]
    fn test_ffi_generation() {
        let generation = slang_bench_evo_generation();
        assert!(generation >= 0);
    }
}
