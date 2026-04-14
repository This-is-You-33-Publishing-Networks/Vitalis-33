//! Compilation Learning — Vitalis v629
//!
//! Machine learning models that improve compilation over time:
//! - Online learning from compilation outcomes
//! - Feature extraction from code for pass selection
//! - Multi-armed bandit for optimization pass ordering
//! - Performance regression detection
//! - Compilation strategy evolution

// ── Multi-Armed Bandit ───────────────────────────────────────────────

/// Thompson Sampling bandit for pass selection.
pub struct ThompsonBandit {
    arms: Vec<BanditArm>,
}

struct BanditArm {
    name: String,
    successes: f64,  // alpha
    failures: f64,   // beta
}

impl ThompsonBandit {
    pub fn new(arm_names: &[&str]) -> Self {
        Self {
            arms: arm_names.iter().map(|n| BanditArm {
                name: n.to_string(), successes: 1.0, failures: 1.0,
            }).collect(),
        }
    }

    /// Select an arm using Thompson Sampling (deterministic approximation).
    pub fn select(&self) -> usize {
        self.arms.iter().enumerate()
            .max_by(|(_, a), (_, b)| {
                let score_a = a.successes / (a.successes + a.failures);
                let score_b = b.successes / (b.successes + b.failures);
                score_a.partial_cmp(&score_b).unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(i, _)| i)
            .unwrap_or(0)
    }

    pub fn update(&mut self, arm: usize, reward: bool) {
        if arm >= self.arms.len() { return; }
        if reward {
            self.arms[arm].successes += 1.0;
        } else {
            self.arms[arm].failures += 1.0;
        }
    }

    pub fn arm_score(&self, arm: usize) -> f64 {
        if arm >= self.arms.len() { return 0.0; }
        let a = &self.arms[arm];
        a.successes / (a.successes + a.failures)
    }

    pub fn arm_count(&self) -> usize { self.arms.len() }

    pub fn best_arm(&self) -> (usize, &str) {
        let idx = self.select();
        (idx, &self.arms[idx].name)
    }

    pub fn arm_name(&self, idx: usize) -> &str {
        if idx < self.arms.len() { &self.arms[idx].name } else { "" }
    }
}

// ── Exponential Moving Average Tracker ───────────────────────────────

/// Tracks a metric with exponential decay.
pub struct EmaTracker {
    value: f64,
    alpha: f64,
    count: u64,
}

impl EmaTracker {
    pub fn new(alpha: f64) -> Self {
        Self { value: 0.0, alpha: alpha.clamp(0.01, 1.0), count: 0 }
    }

    pub fn update(&mut self, observation: f64) {
        if self.count == 0 {
            self.value = observation;
        } else {
            self.value = self.alpha * observation + (1.0 - self.alpha) * self.value;
        }
        self.count += 1;
    }

    pub fn value(&self) -> f64 { self.value }
    pub fn count(&self) -> u64 { self.count }
}

// ── Performance Regression Detector ──────────────────────────────────

/// Detect performance regressions using CUSUM (cumulative sum).
pub struct RegressionDetector {
    baseline: f64,
    threshold: f64,
    cusum_pos: f64,
    cusum_neg: f64,
    observations: u64,
    drift: f64,
}

impl RegressionDetector {
    pub fn new(baseline: f64, threshold: f64) -> Self {
        Self {
            baseline, threshold,
            cusum_pos: 0.0, cusum_neg: 0.0,
            observations: 0, drift: 0.0,
        }
    }

    /// Observe a new measurement. Returns true if regression detected.
    pub fn observe(&mut self, value: f64) -> bool {
        self.observations += 1;
        let deviation = value - self.baseline;
        self.drift = deviation;

        // CUSUM
        self.cusum_pos = (self.cusum_pos + deviation).max(0.0);
        self.cusum_neg = (self.cusum_neg - deviation).max(0.0);

        self.cusum_pos > self.threshold || self.cusum_neg > self.threshold
    }

    pub fn reset(&mut self) {
        self.cusum_pos = 0.0;
        self.cusum_neg = 0.0;
        self.drift = 0.0;
    }

    pub fn current_drift(&self) -> f64 { self.drift }
    pub fn observation_count(&self) -> u64 { self.observations }

    pub fn update_baseline(&mut self, new_baseline: f64) {
        self.baseline = new_baseline;
        self.reset();
    }
}

// ── Compilation Strategy ─────────────────────────────────────────────

/// A named compilation strategy with its history.
#[derive(Debug, Clone)]
pub struct CompilationStrategy {
    pub name: String,
    pub pass_sequence: Vec<String>,
    pub total_time_saved: f64,
    pub total_applications: u32,
    pub success_rate: f64,
}

impl CompilationStrategy {
    pub fn new(name: &str, passes: Vec<String>) -> Self {
        Self {
            name: name.to_string(),
            pass_sequence: passes,
            total_time_saved: 0.0,
            total_applications: 0,
            success_rate: 0.5,
        }
    }

    pub fn record_outcome(&mut self, time_saved: f64, success: bool) {
        self.total_applications += 1;
        self.total_time_saved += time_saved;
        let alpha = 0.1;
        self.success_rate = alpha * (if success { 1.0 } else { 0.0 }) + (1.0 - alpha) * self.success_rate;
    }

    pub fn avg_time_saved(&self) -> f64 {
        if self.total_applications == 0 { 0.0 }
        else { self.total_time_saved / self.total_applications as f64 }
    }

    pub fn score(&self) -> f64 {
        self.avg_time_saved() * self.success_rate
    }
}

// ── FFI Exports ──────────────────────────────────────────────────────

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_learning_bandit_score(successes: f64, failures: f64) -> f64 {
    let total = successes + failures;
    if total <= 0.0 { 0.5 } else { successes / total }
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_learning_ema_update(current: f64, observation: f64, alpha: f64) -> f64 {
    alpha.clamp(0.01, 1.0) * observation + (1.0 - alpha.clamp(0.01, 1.0)) * current
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_learning_cusum_detect(
    observations: *const f64, count: i64, baseline: f64, threshold: f64,
) -> i64 {
    if observations.is_null() || count <= 0 { return 0; }
    let obs = unsafe { std::slice::from_raw_parts(observations, count as usize) };
    let mut det = RegressionDetector::new(baseline, threshold);
    for &v in obs {
        if det.observe(v) { return 1; }
    }
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_learning_strategy_score(time_saved: f64, applications: i64, success_rate: f64) -> f64 {
    if applications <= 0 { return 0.0; }
    (time_saved / applications as f64) * success_rate
}

// ═══════════════════════════════════════════════════════════════════════
// Tests
// ═══════════════════════════════════════════════════════════════════════
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bandit_initial_scores() {
        let b = ThompsonBandit::new(&["A", "B", "C"]);
        assert_eq!(b.arm_count(), 3);
        assert!((b.arm_score(0) - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_bandit_update_and_select() {
        let mut b = ThompsonBandit::new(&["const_fold", "dce", "cse"]);
        for _ in 0..10 { b.update(0, true); }
        for _ in 0..10 { b.update(1, false); }
        let (best, name) = b.best_arm();
        assert_eq!(best, 0);
        assert_eq!(name, "const_fold");
    }

    #[test]
    fn test_bandit_convergence() {
        let mut b = ThompsonBandit::new(&["good", "bad"]);
        for _ in 0..100 { b.update(0, true); b.update(1, false); }
        assert!(b.arm_score(0) > 0.9);
        assert!(b.arm_score(1) < 0.1);
    }

    #[test]
    fn test_ema_tracker() {
        let mut t = EmaTracker::new(0.3);
        t.update(10.0);
        assert_eq!(t.value(), 10.0);
        t.update(20.0);
        assert!((t.value() - 13.0).abs() < 1e-10); // 0.3*20 + 0.7*10
    }

    #[test]
    fn test_ema_count() {
        let mut t = EmaTracker::new(0.5);
        assert_eq!(t.count(), 0);
        t.update(1.0);
        t.update(2.0);
        assert_eq!(t.count(), 2);
    }

    #[test]
    fn test_regression_detector_normal() {
        let mut det = RegressionDetector::new(100.0, 50.0);
        assert!(!det.observe(102.0));
        assert!(!det.observe(98.0));
        assert!(!det.observe(101.0));
    }

    #[test]
    fn test_regression_detector_regression() {
        let mut det = RegressionDetector::new(100.0, 30.0);
        // Sustained increase should trigger
        for _ in 0..10 {
            det.observe(120.0);
        }
        assert!(det.observe(120.0)); // Should detect regression
    }

    #[test]
    fn test_regression_detector_reset() {
        let mut det = RegressionDetector::new(100.0, 30.0);
        det.observe(150.0);
        det.reset();
        assert!(!det.observe(101.0)); // Reset clears accumulated sum
    }

    #[test]
    fn test_regression_update_baseline() {
        let mut det = RegressionDetector::new(100.0, 30.0);
        det.update_baseline(120.0);
        assert!(!det.observe(122.0)); // Normal around new baseline
    }

    #[test]
    fn test_strategy_new() {
        let s = CompilationStrategy::new("fast", vec!["dce".into(), "const_fold".into()]);
        assert_eq!(s.name, "fast");
        assert_eq!(s.pass_sequence.len(), 2);
    }

    #[test]
    fn test_strategy_record_outcome() {
        let mut s = CompilationStrategy::new("test", vec![]);
        s.record_outcome(10.0, true);
        s.record_outcome(20.0, true);
        assert!((s.avg_time_saved() - 15.0).abs() < 1e-10);
    }

    #[test]
    fn test_strategy_score() {
        let mut s = CompilationStrategy::new("test", vec![]);
        s.record_outcome(100.0, true);
        assert!(s.score() > 0.0);
    }

    #[test]
    fn test_ffi_bandit_score() {
        assert!((vitalis_learning_bandit_score(9.0, 1.0) - 0.9).abs() < 1e-10);
        assert!((vitalis_learning_bandit_score(0.0, 0.0) - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_ffi_ema_update() {
        let result = vitalis_learning_ema_update(10.0, 20.0, 0.3);
        assert!((result - 13.0).abs() < 1e-10);
    }

    #[test]
    fn test_ffi_cusum_detect_normal() {
        let obs = [101.0, 99.0, 102.0, 98.0, 100.0];
        assert_eq!(vitalis_learning_cusum_detect(obs.as_ptr(), 5, 100.0, 50.0), 0);
    }

    #[test]
    fn test_ffi_cusum_detect_regression() {
        let obs = [120.0; 10];
        assert_eq!(vitalis_learning_cusum_detect(obs.as_ptr(), 10, 100.0, 30.0), 1);
    }

    #[test]
    fn test_ffi_strategy_score() {
        assert!((vitalis_learning_strategy_score(100.0, 4, 0.8) - 20.0).abs() < 1e-10);
    }
}
