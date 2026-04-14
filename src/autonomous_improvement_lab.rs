//! Autonomous Improvement Lab — reproducible trial execution, A/B comparison,
//! fitness measurement, statistical significance testing, and report generation.
//!
//! # Architecture
//!
//! ```text
//! ImprovementLab
//!   ├── TrialManifest       → Reproducible trial definition (seed, objective, budget)
//!   ├── TrialExecutor       → Execute trials: compile → run → measure → record
//!   ├── TrialResult         → Per-trial outcome (compile time, runtime, fitness, correctness)
//!   ├── ABComparison        → Statistical comparison of baseline vs candidate
//!   ├── ImprovementReport   → Aggregated report with confidence intervals
//!   └── TrialHistory        → Bounded log of all executed trials
//! ```
//!
//! The lab integrates with:
//! - `evolution_safety_rails::SafetyGovernor` for safety enforcement
//! - `evolution::EvolutionRegistry` for function variant management
//! - `reward_model` patterns for fitness evaluation

use std::collections::VecDeque;

// ─── Core Types ──────────────────────────────────────────────────────────

/// Improvement objective that a trial optimizes for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Objective {
    /// Minimize compile time.
    MinCompileTime,
    /// Minimize runtime.
    MinRuntime,
    /// Maximize fitness score.
    MaxFitness,
    /// Minimize code size.
    MinCodeSize,
    /// Multi-objective: Pareto-optimal across all dimensions.
    Pareto,
}

/// Status of a trial.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrialStatus {
    Pending,
    Running,
    Passed,
    Failed,
    Rejected,
}

/// A reproducible trial manifest defining what to test and how.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrialManifest {
    pub id: String,
    pub seed: u64,
    pub objective: String,
    pub budget: u64,
    /// Function being tested.
    pub function_name: String,
    /// Source code of the candidate variant.
    pub candidate_source: String,
    /// Number of repetitions for statistical stability.
    pub repetitions: usize,
}

/// Result of executing a single trial.
#[derive(Debug, Clone, PartialEq)]
pub struct TrialResult {
    pub id: String,
    pub compile_time_ms: f64,
    pub runtime_ms: f64,
    pub correctness: bool,
    /// Fitness score computed from measurements.
    pub fitness: f64,
    /// Output of the execution (for correctness checking).
    pub output: i64,
    /// Code size in bytes.
    pub code_size: usize,
    pub status: TrialStatus,
    pub error: Option<String>,
}

/// Aggregated measurements from multiple repetitions of a trial.
#[derive(Debug, Clone)]
pub struct TrialAggregate {
    pub id: String,
    pub function_name: String,
    pub repetitions: usize,
    pub mean_compile_ms: f64,
    pub mean_runtime_ms: f64,
    pub mean_fitness: f64,
    pub stddev_fitness: f64,
    pub pass_rate: f64,
    pub best_fitness: f64,
    pub worst_fitness: f64,
}

// ─── A/B Comparison ──────────────────────────────────────────────────────

/// Statistical comparison between baseline and candidate variants.
#[derive(Debug, Clone)]
pub struct ABComparison {
    pub function_name: String,
    pub baseline: TrialAggregate,
    pub candidate: TrialAggregate,
    /// Improvement ratio: (candidate - baseline) / baseline.
    pub improvement_ratio: f64,
    /// Whether improvement is statistically significant (p < 0.05 via Welch's t-test).
    pub significant: bool,
    /// t-statistic from Welch's t-test.
    pub t_statistic: f64,
    /// Recommendation: Accept, Reject, or Inconclusive.
    pub recommendation: Recommendation,
}

/// Recommendation from an A/B comparison.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Recommendation {
    /// Candidate is significantly better — accept the mutation.
    Accept,
    /// Candidate is significantly worse — reject the mutation.
    Reject,
    /// Not enough evidence — need more trials.
    Inconclusive,
}

impl ABComparison {
    pub fn to_json(&self) -> String {
        format!(
            concat!(
                "{{\"function\":\"{}\",\"improvement\":{:.4},",
                "\"significant\":{},\"t_stat\":{:.4},\"recommendation\":\"{}\"}}",
            ),
            self.function_name, self.improvement_ratio,
            self.significant, self.t_statistic,
            match self.recommendation {
                Recommendation::Accept => "accept",
                Recommendation::Reject => "reject",
                Recommendation::Inconclusive => "inconclusive",
            }
        )
    }
}

// ─── Improvement Lab ─────────────────────────────────────────────────────

/// The autonomous improvement lab — orchestrates trials, comparisons, and reports.
pub struct ImprovementLab {
    /// All pending trial manifests.
    pending: Vec<TrialManifest>,
    /// History of completed trial results (bounded ring buffer).
    history: VecDeque<TrialResult>,
    max_history: usize,
    /// Completed A/B comparisons.
    comparisons: Vec<ABComparison>,
    /// Trial counter for ID generation.
    trial_counter: u64,
    /// Current cycle.
    cycle: u64,
    /// Minimum repetitions for statistical significance.
    min_repetitions: usize,
    /// Significance threshold (default: 0.05).
    significance_level: f64,
}

impl ImprovementLab {
    pub fn new() -> Self {
        Self {
            pending: Vec::new(),
            history: VecDeque::with_capacity(1000),
            max_history: 1000,
            comparisons: Vec::new(),
            trial_counter: 0,
            cycle: 0,
            min_repetitions: 5,
            significance_level: 0.05,
        }
    }

    /// Create a lab with custom settings.
    pub fn with_config(max_history: usize, min_repetitions: usize, significance_level: f64) -> Self {
        Self {
            max_history,
            min_repetitions,
            significance_level,
            ..Self::new()
        }
    }

    // ─── Trial Scheduling ────────────────────────────────────────────

    /// Schedule a set of trials for a function variant.
    pub fn schedule_trials(
        &mut self,
        base_seed: u64,
        count: usize,
        objective: &str,
        function_name: &str,
        candidate_source: &str,
    ) -> Vec<TrialManifest> {
        let manifests: Vec<TrialManifest> = (0..count)
            .map(|i| {
                self.trial_counter += 1;
                TrialManifest {
                    id: format!("trial-{}-{}", self.cycle, self.trial_counter),
                    seed: base_seed.wrapping_add(i as u64),
                    objective: objective.to_string(),
                    budget: 1000,
                    function_name: function_name.to_string(),
                    candidate_source: candidate_source.to_string(),
                    repetitions: self.min_repetitions,
                }
            })
            .collect();

        self.pending.extend(manifests.clone());
        manifests
    }

    /// Get pending trial count.
    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    /// Drain all pending manifests.
    pub fn drain_pending(&mut self) -> Vec<TrialManifest> {
        std::mem::take(&mut self.pending)
    }

    // ─── Trial Execution ─────────────────────────────────────────────

    /// Execute a trial manifest and record the result.
    ///
    /// The `evaluate_fn` callback performs actual compilation and execution,
    /// returning (compile_time_ms, runtime_ms, output, code_size).
    pub fn execute_trial<F>(&mut self, manifest: &TrialManifest, evaluate_fn: F) -> TrialResult
    where
        F: FnOnce(&str, u64) -> Result<(f64, f64, i64, usize), String>,
    {
        match evaluate_fn(&manifest.candidate_source, manifest.seed) {
            Ok((compile_ms, runtime_ms, output, code_size)) => {
                let fitness = compute_fitness(compile_ms, runtime_ms, code_size, &manifest.objective);
                let result = TrialResult {
                    id: manifest.id.clone(),
                    compile_time_ms: compile_ms,
                    runtime_ms: runtime_ms,
                    correctness: true,
                    fitness,
                    output,
                    code_size,
                    status: TrialStatus::Passed,
                    error: None,
                };
                self.record_result(result.clone());
                result
            }
            Err(e) => {
                let result = TrialResult {
                    id: manifest.id.clone(),
                    compile_time_ms: 0.0,
                    runtime_ms: 0.0,
                    correctness: false,
                    fitness: 0.0,
                    output: 0,
                    code_size: 0,
                    status: TrialStatus::Failed,
                    error: Some(e),
                };
                self.record_result(result.clone());
                result
            }
        }
    }

    /// Execute multiple repetitions of a trial for statistical stability.
    pub fn execute_repeated<F>(
        &mut self,
        manifest: &TrialManifest,
        mut evaluate_fn: F,
    ) -> Vec<TrialResult>
    where
        F: FnMut(&str, u64) -> Result<(f64, f64, i64, usize), String>,
    {
        let reps = manifest.repetitions.max(1);
        let mut results = Vec::with_capacity(reps);
        for i in 0..reps {
            let seed = manifest.seed.wrapping_add(i as u64 * 7919); // Prime spacing
            match evaluate_fn(&manifest.candidate_source, seed) {
                Ok((compile_ms, runtime_ms, output, code_size)) => {
                    let fitness = compute_fitness(compile_ms, runtime_ms, code_size, &manifest.objective);
                    let result = TrialResult {
                        id: format!("{}-rep{}", manifest.id, i),
                        compile_time_ms: compile_ms,
                        runtime_ms: runtime_ms,
                        correctness: true,
                        fitness,
                        output,
                        code_size,
                        status: TrialStatus::Passed,
                        error: None,
                    };
                    self.record_result(result.clone());
                    results.push(result);
                }
                Err(e) => {
                    let result = TrialResult {
                        id: format!("{}-rep{}", manifest.id, i),
                        compile_time_ms: 0.0,
                        runtime_ms: 0.0,
                        correctness: false,
                        fitness: 0.0,
                        output: 0,
                        code_size: 0,
                        status: TrialStatus::Failed,
                        error: Some(e),
                    };
                    self.record_result(result.clone());
                    results.push(result);
                }
            }
        }
        results
    }

    /// Record a trial result into history.
    fn record_result(&mut self, result: TrialResult) {
        if self.history.len() >= self.max_history {
            self.history.pop_front();
        }
        self.history.push_back(result);
    }

    // ─── Aggregation ─────────────────────────────────────────────────

    /// Aggregate multiple trial results for the same function.
    pub fn aggregate(&self, function_name: &str, results: &[TrialResult]) -> TrialAggregate {
        let passed: Vec<&TrialResult> = results.iter().filter(|r| r.status == TrialStatus::Passed).collect();
        let n = passed.len();
        if n == 0 {
            return TrialAggregate {
                id: format!("agg-{}", function_name),
                function_name: function_name.to_string(),
                repetitions: results.len(),
                mean_compile_ms: 0.0,
                mean_runtime_ms: 0.0,
                mean_fitness: 0.0,
                stddev_fitness: 0.0,
                pass_rate: 0.0,
                best_fitness: 0.0,
                worst_fitness: 0.0,
            };
        }

        let nf = n as f64;
        let mean_compile = passed.iter().map(|r| r.compile_time_ms).sum::<f64>() / nf;
        let mean_runtime = passed.iter().map(|r| r.runtime_ms).sum::<f64>() / nf;
        let mean_fitness = passed.iter().map(|r| r.fitness).sum::<f64>() / nf;

        // Standard deviation of fitness
        let variance = if n > 1 {
            passed.iter().map(|r| (r.fitness - mean_fitness).powi(2)).sum::<f64>() / (nf - 1.0)
        } else {
            0.0
        };
        let stddev = variance.sqrt();

        let best = passed.iter().map(|r| r.fitness).fold(f64::NEG_INFINITY, f64::max);
        let worst = passed.iter().map(|r| r.fitness).fold(f64::INFINITY, f64::min);

        TrialAggregate {
            id: format!("agg-{}", function_name),
            function_name: function_name.to_string(),
            repetitions: results.len(),
            mean_compile_ms: mean_compile,
            mean_runtime_ms: mean_runtime,
            mean_fitness,
            stddev_fitness: stddev,
            pass_rate: n as f64 / results.len() as f64,
            best_fitness: best,
            worst_fitness: worst,
        }
    }

    // ─── A/B Comparison ──────────────────────────────────────────────

    /// Compare baseline results against candidate results using Welch's t-test.
    pub fn compare_ab(
        &mut self,
        function_name: &str,
        baseline_results: &[TrialResult],
        candidate_results: &[TrialResult],
    ) -> ABComparison {
        let baseline_agg = self.aggregate(function_name, baseline_results);
        let candidate_agg = self.aggregate(function_name, candidate_results);

        let improvement = if baseline_agg.mean_fitness.abs() > 1e-10 {
            (candidate_agg.mean_fitness - baseline_agg.mean_fitness) / baseline_agg.mean_fitness.abs()
        } else {
            candidate_agg.mean_fitness
        };

        // Welch's t-test
        let baseline_fitness: Vec<f64> = baseline_results.iter()
            .filter(|r| r.status == TrialStatus::Passed)
            .map(|r| r.fitness)
            .collect();
        let candidate_fitness: Vec<f64> = candidate_results.iter()
            .filter(|r| r.status == TrialStatus::Passed)
            .map(|r| r.fitness)
            .collect();

        let (t_stat, significant) = welch_t_test(&baseline_fitness, &candidate_fitness, self.significance_level);

        let recommendation = if !significant {
            Recommendation::Inconclusive
        } else if improvement > 0.0 {
            Recommendation::Accept
        } else {
            Recommendation::Reject
        };

        let comparison = ABComparison {
            function_name: function_name.to_string(),
            baseline: baseline_agg,
            candidate: candidate_agg,
            improvement_ratio: improvement,
            significant,
            t_statistic: t_stat,
            recommendation,
        };

        self.comparisons.push(comparison.clone());
        comparison
    }

    // ─── Reporting ───────────────────────────────────────────────────

    /// Generate a JSON report of all comparisons.
    pub fn report_json(&self) -> String {
        let comparisons: Vec<String> = self.comparisons.iter().map(|c| c.to_json()).collect();
        format!(
            "{{\"cycle\":{},\"trials_executed\":{},\"comparisons\":[{}]}}",
            self.cycle,
            self.history.len(),
            comparisons.join(","),
        )
    }

    /// Get the number of completed trials.
    pub fn trials_completed(&self) -> usize {
        self.history.len()
    }

    /// Get all comparison results.
    pub fn comparisons(&self) -> &[ABComparison] {
        &self.comparisons
    }

    /// Advance to next cycle.
    pub fn begin_cycle(&mut self) {
        self.cycle += 1;
    }

    /// Get trial results for a specific function.
    pub fn results_for_function(&self, function_name: &str) -> Vec<&TrialResult> {
        self.history.iter()
            .filter(|r| r.id.contains(function_name))
            .collect()
    }

    /// Clear all state.
    pub fn reset(&mut self) {
        self.pending.clear();
        self.history.clear();
        self.comparisons.clear();
        self.trial_counter = 0;
        self.cycle = 0;
    }
}

// ─── Statistical Helpers ─────────────────────────────────────────────────

/// Compute fitness score from measurements, weighted by objective.
fn compute_fitness(compile_ms: f64, runtime_ms: f64, code_size: usize, objective: &str) -> f64 {
    match objective {
        "min-compile" => 1.0 / (1.0 + compile_ms),
        "min-runtime" => 1.0 / (1.0 + runtime_ms),
        "min-codesize" => 1.0 / (1.0 + code_size as f64),
        "pareto" => {
            // Geometric mean of individual fitness dimensions
            let f_compile = 1.0 / (1.0 + compile_ms);
            let f_runtime = 1.0 / (1.0 + runtime_ms);
            let f_size = 1.0 / (1.0 + code_size as f64);
            (f_compile * f_runtime * f_size).cbrt()
        }
        _ => {
            // Default: maximize fitness = inverse of combined cost
            1.0 / (1.0 + compile_ms + runtime_ms)
        }
    }
}

/// Welch's t-test for unequal variances.
/// Returns (t_statistic, is_significant at the given alpha level).
fn welch_t_test(a: &[f64], b: &[f64], alpha: f64) -> (f64, bool) {
    let n_a = a.len();
    let n_b = b.len();
    if n_a < 2 || n_b < 2 {
        return (0.0, false);
    }

    let mean_a = a.iter().sum::<f64>() / n_a as f64;
    let mean_b = b.iter().sum::<f64>() / n_b as f64;

    let var_a = a.iter().map(|x| (x - mean_a).powi(2)).sum::<f64>() / (n_a - 1) as f64;
    let var_b = b.iter().map(|x| (x - mean_b).powi(2)).sum::<f64>() / (n_b - 1) as f64;

    let se = (var_a / n_a as f64 + var_b / n_b as f64).sqrt();
    if se < 1e-15 {
        return (0.0, false);
    }

    let t = (mean_a - mean_b) / se;

    // Welch-Satterthwaite degrees of freedom
    let num = (var_a / n_a as f64 + var_b / n_b as f64).powi(2);
    let denom = (var_a / n_a as f64).powi(2) / (n_a - 1) as f64
        + (var_b / n_b as f64).powi(2) / (n_b - 1) as f64;
    let df = if denom > 1e-15 { num / denom } else { 2.0 };

    // Approximate critical t-value for two-sided test using conservative bound:
    // For df >= 2 and alpha = 0.05, t_crit ≈ 2.0 (very conservative).
    // For df >= 30, t_crit ≈ 1.96. We use a simple lookup.
    let t_crit = approximate_t_critical(df, alpha);

    (t, t.abs() > t_crit)
}

/// Approximate t-critical value for two-sided test.
/// Uses a small lookup table with interpolation for common alpha values.
fn approximate_t_critical(df: f64, alpha: f64) -> f64 {
    // t-distribution critical values for alpha/2 (two-sided)
    if alpha >= 0.10 {
        // alpha=0.10: approximate critical values
        if df < 3.0 { 2.920 }
        else if df < 5.0 { 2.132 }
        else if df < 10.0 { 1.833 }
        else if df < 30.0 { 1.699 }
        else { 1.645 }
    } else if alpha >= 0.05 {
        // alpha=0.05: standard critical values
        if df < 3.0 { 4.303 }
        else if df < 5.0 { 2.776 }
        else if df < 10.0 { 2.262 }
        else if df < 30.0 { 2.042 }
        else { 1.960 }
    } else {
        // alpha=0.01
        if df < 3.0 { 9.925 }
        else if df < 5.0 { 4.604 }
        else if df < 10.0 { 3.250 }
        else if df < 30.0 { 2.750 }
        else { 2.576 }
    }
}

// ─── Legacy Compatibility ────────────────────────────────────────────────

/// Schedule trials (standalone, legacy compat).
pub fn schedule_trials(base_seed: u64, count: usize, objective: &str) -> Vec<TrialManifest> {
    (0..count)
        .map(|i| TrialManifest {
            id: format!("trial-{}", i),
            seed: base_seed + i as u64,
            objective: objective.to_string(),
            budget: 1000,
            function_name: String::new(),
            candidate_source: String::new(),
            repetitions: 1,
        })
        .collect()
}

/// Compute reproducibility hash for a trial manifest.
pub fn reproducibility_hash(manifest: &TrialManifest) -> String {
    let raw = format!("{}:{}:{}:{}", manifest.id, manifest.seed, manifest.objective, manifest.budget);
    let mut h: u64 = 1469598103934665603;
    for b in raw.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(1099511628211);
    }
    format!("trial:{:016x}", h)
}

/// Aggregate objective metrics across results (legacy compat).
pub fn aggregate_objectives(results: &[TrialResult]) -> (f64, f64, f64) {
    if results.is_empty() {
        return (0.0, 0.0, 0.0);
    }
    let n = results.len() as f64;
    let c = results.iter().map(|r| r.compile_time_ms).sum::<f64>() / n;
    let r = results.iter().map(|r| r.runtime_ms).sum::<f64>() / n;
    let ok = results.iter().filter(|r| r.correctness).count() as f64 / n;
    (c, r, ok)
}

// ─── FFI ─────────────────────────────────────────────────────────────────

/// Compute fitness from compile time, runtime, and code size. Returns fitness × 10000.
#[unsafe(no_mangle)]
pub extern "C" fn slang_lab_fitness(compile_ms: f64, runtime_ms: f64, code_size: i64) -> i64 {
    let f = compute_fitness(compile_ms, runtime_ms, code_size.max(0) as usize, "pareto");
    (f * 10000.0).round() as i64
}

/// Run Welch's t-test on two arrays. Returns 1 if significant at α=0.05, 0 otherwise.
/// Arrays are encoded as: [n_a, a_0..a_{n_a-1}, n_b, b_0..b_{n_b-1}]
#[unsafe(no_mangle)]
pub extern "C" fn slang_lab_t_test_significant(
    mean_a_x1000: i64, var_a_x1000: i64, n_a: i64,
    mean_b_x1000: i64, var_b_x1000: i64, n_b: i64,
) -> i64 {
    if n_a < 2 || n_b < 2 { return 0; }
    let mean_a = mean_a_x1000 as f64 / 1000.0;
    let mean_b = mean_b_x1000 as f64 / 1000.0;
    let var_a = (var_a_x1000 as f64 / 1000.0).max(0.0);
    let var_b = (var_b_x1000 as f64 / 1000.0).max(0.0);
    let n_a_f = n_a as f64;
    let n_b_f = n_b as f64;

    let se = (var_a / n_a_f + var_b / n_b_f).sqrt();
    if se < 1e-15 { return 0; }
    let t = (mean_a - mean_b).abs() / se;

    let num = (var_a / n_a_f + var_b / n_b_f).powi(2);
    let denom = (var_a / n_a_f).powi(2) / (n_a_f - 1.0)
        + (var_b / n_b_f).powi(2) / (n_b_f - 1.0);
    let df = if denom > 1e-15 { num / denom } else { 2.0 };

    let t_crit = approximate_t_critical(df, 0.05);
    if t > t_crit { 1 } else { 0 }
}

// ─── Tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schedule_reproducible() {
        let a = schedule_trials(7, 3, "min-runtime");
        let b = schedule_trials(7, 3, "min-runtime");
        assert_eq!(a, b);
    }

    #[test]
    fn test_hash_stable() {
        let m = TrialManifest {
            id: "trial-0".into(), seed: 42, objective: "x".into(), budget: 100,
            function_name: String::new(), candidate_source: String::new(), repetitions: 1,
        };
        assert_eq!(reproducibility_hash(&m), reproducibility_hash(&m));
    }

    #[test]
    fn test_objective_aggregation() {
        let r = vec![
            TrialResult {
                id: "a".into(), compile_time_ms: 10.0, runtime_ms: 20.0,
                correctness: true, fitness: 0.5, output: 0, code_size: 100,
                status: TrialStatus::Passed, error: None,
            },
            TrialResult {
                id: "b".into(), compile_time_ms: 14.0, runtime_ms: 24.0,
                correctness: false, fitness: 0.0, output: 0, code_size: 0,
                status: TrialStatus::Failed, error: Some("error".into()),
            },
        ];
        let (c, rt, ok) = aggregate_objectives(&r);
        assert_eq!(c, 12.0);
        assert_eq!(rt, 22.0);
        assert_eq!(ok, 0.5);
    }

    #[test]
    fn test_lab_schedule_and_drain() {
        let mut lab = ImprovementLab::new();
        lab.begin_cycle();
        let manifests = lab.schedule_trials(42, 3, "min-runtime", "optimize", "fn optimize() {}");
        assert_eq!(manifests.len(), 3);
        assert_eq!(lab.pending_count(), 3);
        let drained = lab.drain_pending();
        assert_eq!(drained.len(), 3);
        assert_eq!(lab.pending_count(), 0);
    }

    #[test]
    fn test_lab_execute_trial_success() {
        let mut lab = ImprovementLab::new();
        lab.begin_cycle();
        let manifests = lab.schedule_trials(1, 1, "min-runtime", "func", "code");
        let manifest = &manifests[0];

        let result = lab.execute_trial(manifest, |_source, _seed| {
            Ok((10.0, 5.0, 42, 256))
        });

        assert_eq!(result.status, TrialStatus::Passed);
        assert!(result.correctness);
        assert_eq!(result.output, 42);
        assert!(result.fitness > 0.0);
        assert_eq!(lab.trials_completed(), 1);
    }

    #[test]
    fn test_lab_execute_trial_failure() {
        let mut lab = ImprovementLab::new();
        lab.begin_cycle();
        let manifests = lab.schedule_trials(1, 1, "min-runtime", "func", "bad code");
        let manifest = &manifests[0];

        let result = lab.execute_trial(manifest, |_source, _seed| {
            Err("compilation failed".to_string())
        });

        assert_eq!(result.status, TrialStatus::Failed);
        assert!(!result.correctness);
        assert!(result.error.is_some());
    }

    #[test]
    fn test_lab_execute_repeated() {
        let mut lab = ImprovementLab::new();
        lab.begin_cycle();
        let manifests = lab.schedule_trials(1, 1, "min-runtime", "func", "code");
        let mut manifest = manifests[0].clone();
        manifest.repetitions = 5;

        let mut call_count = 0u64;
        let results = lab.execute_repeated(&manifest, |_source, _seed| {
            call_count += 1;
            Ok((10.0 + call_count as f64, 5.0, 42, 256))
        });

        assert_eq!(results.len(), 5);
        assert!(results.iter().all(|r| r.status == TrialStatus::Passed));
    }

    #[test]
    fn test_lab_aggregate() {
        let lab = ImprovementLab::new();
        let results = vec![
            TrialResult {
                id: "t1".into(), compile_time_ms: 10.0, runtime_ms: 5.0,
                correctness: true, fitness: 0.8, output: 1, code_size: 100,
                status: TrialStatus::Passed, error: None,
            },
            TrialResult {
                id: "t2".into(), compile_time_ms: 12.0, runtime_ms: 6.0,
                correctness: true, fitness: 0.6, output: 1, code_size: 120,
                status: TrialStatus::Passed, error: None,
            },
            TrialResult {
                id: "t3".into(), compile_time_ms: 0.0, runtime_ms: 0.0,
                correctness: false, fitness: 0.0, output: 0, code_size: 0,
                status: TrialStatus::Failed, error: Some("err".into()),
            },
        ];

        let agg = lab.aggregate("func", &results);
        assert_eq!(agg.repetitions, 3);
        assert!((agg.mean_fitness - 0.7).abs() < 1e-10);
        assert!((agg.pass_rate - 2.0 / 3.0).abs() < 1e-10);
        assert!((agg.best_fitness - 0.8).abs() < 1e-10);
        assert!((agg.worst_fitness - 0.6).abs() < 1e-10);
        assert!(agg.stddev_fitness > 0.0);
    }

    #[test]
    fn test_lab_ab_comparison_significant_improvement() {
        let mut lab = ImprovementLab::new();
        // Baseline: consistently low fitness
        let baseline: Vec<TrialResult> = (0..10).map(|i| TrialResult {
            id: format!("base-{}", i), compile_time_ms: 10.0, runtime_ms: 5.0,
            correctness: true, fitness: 0.5 + (i as f64) * 0.001, output: 1,
            code_size: 100, status: TrialStatus::Passed, error: None,
        }).collect();
        // Candidate: consistently high fitness
        let candidate: Vec<TrialResult> = (0..10).map(|i| TrialResult {
            id: format!("cand-{}", i), compile_time_ms: 8.0, runtime_ms: 3.0,
            correctness: true, fitness: 0.9 + (i as f64) * 0.001, output: 1,
            code_size: 90, status: TrialStatus::Passed, error: None,
        }).collect();

        let comparison = lab.compare_ab("optimize", &baseline, &candidate);
        assert!(comparison.significant);
        assert!(comparison.improvement_ratio > 0.0);
        assert_eq!(comparison.recommendation, Recommendation::Accept);
    }

    #[test]
    fn test_lab_ab_comparison_no_improvement() {
        let mut lab = ImprovementLab::new();
        // Both groups have same fitness
        let make_results = |prefix: &str| -> Vec<TrialResult> {
            (0..5).map(|i| TrialResult {
                id: format!("{}-{}", prefix, i), compile_time_ms: 10.0, runtime_ms: 5.0,
                correctness: true, fitness: 0.5, output: 1,
                code_size: 100, status: TrialStatus::Passed, error: None,
            }).collect()
        };

        let comparison = lab.compare_ab("func", &make_results("base"), &make_results("cand"));
        assert!(!comparison.significant);
        assert_eq!(comparison.recommendation, Recommendation::Inconclusive);
    }

    #[test]
    fn test_welch_t_test_identical() {
        let a = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let b = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let (t, sig) = welch_t_test(&a, &b, 0.05);
        assert!(t.abs() < 1e-10);
        assert!(!sig);
    }

    #[test]
    fn test_welch_t_test_different() {
        let a = vec![1.0, 1.1, 0.9, 1.0, 1.05];
        let b = vec![5.0, 5.1, 4.9, 5.0, 5.05];
        let (t, sig) = welch_t_test(&a, &b, 0.05);
        assert!(t.abs() > 2.0);
        assert!(sig);
    }

    #[test]
    fn test_welch_t_test_insufficient_data() {
        let (_, sig) = welch_t_test(&[1.0], &[2.0], 0.05);
        assert!(!sig);
    }

    #[test]
    fn test_compute_fitness_objectives() {
        let f_compile = compute_fitness(100.0, 50.0, 1000, "min-compile");
        let f_runtime = compute_fitness(100.0, 50.0, 1000, "min-runtime");
        let f_size = compute_fitness(100.0, 50.0, 1000, "min-codesize");
        let f_pareto = compute_fitness(100.0, 50.0, 1000, "pareto");

        assert!(f_compile > 0.0 && f_compile < 1.0);
        assert!(f_runtime > 0.0 && f_runtime < 1.0);
        assert!(f_size > 0.0 && f_size < 1.0);
        assert!(f_pareto > 0.0 && f_pareto < 1.0);
    }

    #[test]
    fn test_fitness_inverse_relationship() {
        // Faster compile should give higher fitness for min-compile
        let f_fast = compute_fitness(10.0, 50.0, 1000, "min-compile");
        let f_slow = compute_fitness(1000.0, 50.0, 1000, "min-compile");
        assert!(f_fast > f_slow);
    }

    #[test]
    fn test_report_json() {
        let mut lab = ImprovementLab::new();
        lab.begin_cycle();
        let json = lab.report_json();
        assert!(json.contains("\"cycle\":1"));
        assert!(json.contains("\"trials_executed\":0"));
    }

    #[test]
    fn test_lab_reset() {
        let mut lab = ImprovementLab::new();
        lab.begin_cycle();
        lab.schedule_trials(1, 3, "x", "f", "code");
        lab.reset();
        assert_eq!(lab.pending_count(), 0);
        assert_eq!(lab.trials_completed(), 0);
    }

    #[test]
    fn test_lab_history_bounded() {
        let mut lab = ImprovementLab::with_config(5, 1, 0.05);
        for i in 0..10 {
            lab.record_result(TrialResult {
                id: format!("t{}", i), compile_time_ms: 1.0, runtime_ms: 1.0,
                correctness: true, fitness: 0.5, output: 0, code_size: 0,
                status: TrialStatus::Passed, error: None,
            });
        }
        assert_eq!(lab.trials_completed(), 5);
    }

    #[test]
    fn test_ab_comparison_json() {
        let comparison = ABComparison {
            function_name: "opt".into(),
            baseline: TrialAggregate {
                id: "b".into(), function_name: "opt".into(), repetitions: 5,
                mean_compile_ms: 10.0, mean_runtime_ms: 5.0, mean_fitness: 0.5,
                stddev_fitness: 0.1, pass_rate: 1.0, best_fitness: 0.6, worst_fitness: 0.4,
            },
            candidate: TrialAggregate {
                id: "c".into(), function_name: "opt".into(), repetitions: 5,
                mean_compile_ms: 8.0, mean_runtime_ms: 3.0, mean_fitness: 0.8,
                stddev_fitness: 0.05, pass_rate: 1.0, best_fitness: 0.85, worst_fitness: 0.75,
            },
            improvement_ratio: 0.6,
            significant: true,
            t_statistic: 3.5,
            recommendation: Recommendation::Accept,
        };
        let json = comparison.to_json();
        assert!(json.contains("\"recommendation\":\"accept\""));
        assert!(json.contains("\"significant\":true"));
    }

    #[test]
    fn test_ffi_lab_fitness() {
        let f = slang_lab_fitness(10.0, 5.0, 256);
        assert!(f > 0);
    }

    #[test]
    fn test_ffi_t_test() {
        // Same means → not significant
        let sig = slang_lab_t_test_significant(500, 10, 10, 500, 10, 10);
        assert_eq!(sig, 0);
    }
}
