//! Performance Prophecy — Vitalis v631
//!
//! Predictive performance analysis engine:
//! - Asymptotic complexity estimation from code structure
//! - Cache behavior prediction (spatial/temporal locality)
//! - Branch prediction modeling
//! - Memory access pattern classification
//! - Performance bottleneck prophecy

// ── Complexity Estimation ────────────────────────────────────────────

/// Big-O complexity class.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Complexity {
    O1,           // Constant
    OLogN,        // Logarithmic
    ON,           // Linear
    ONLogN,       // Linearithmic
    ON2,          // Quadratic
    ON3,          // Cubic
    O2N,          // Exponential
    OFactorial,   // Factorial
}

impl Complexity {
    /// Human-readable notation.
    pub fn notation(&self) -> &'static str {
        match self {
            Self::O1 => "O(1)",
            Self::OLogN => "O(log n)",
            Self::ON => "O(n)",
            Self::ONLogN => "O(n log n)",
            Self::ON2 => "O(n²)",
            Self::ON3 => "O(n³)",
            Self::O2N => "O(2ⁿ)",
            Self::OFactorial => "O(n!)",
        }
    }

    /// Estimated operations for given n.
    pub fn estimate_ops(&self, n: u64) -> f64 {
        match self {
            Self::O1 => 1.0,
            Self::OLogN => (n as f64).ln().max(1.0),
            Self::ON => n as f64,
            Self::ONLogN => n as f64 * (n as f64).ln().max(1.0),
            Self::ON2 => (n as f64) * (n as f64),
            Self::ON3 => (n as f64).powi(3),
            Self::O2N => 2.0_f64.powi(n.min(62) as i32),
            Self::OFactorial => {
                let mut result = 1.0_f64;
                for i in 2..=n.min(20) { result *= i as f64; }
                result
            }
        }
    }

    /// Severity ranking (0 = best, 7 = worst).
    pub fn severity(&self) -> u8 {
        match self {
            Self::O1 => 0, Self::OLogN => 1, Self::ON => 2, Self::ONLogN => 3,
            Self::ON2 => 4, Self::ON3 => 5, Self::O2N => 6, Self::OFactorial => 7,
        }
    }
}

/// Estimate complexity from loop nesting depth.
pub fn estimate_from_loop_depth(max_nesting: u32, has_halving: bool) -> Complexity {
    match (max_nesting, has_halving) {
        (0, _) => Complexity::O1,
        (1, true) => Complexity::OLogN,
        (1, false) => Complexity::ON,
        (2, true) => Complexity::ONLogN,
        (2, false) => Complexity::ON2,
        (3, false) => Complexity::ON3,
        _ => Complexity::ON3,
    }
}

// ── Cache Behavior ───────────────────────────────────────────────────

/// Memory access pattern classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessPattern {
    Sequential,   // arr[0], arr[1], arr[2] — excellent spatial locality
    Strided,      // arr[0], arr[4], arr[8] — moderate spatial locality
    Random,       // arr[hash(x)], arr[hash(y)] — poor locality
    Columnar,     // Matrix column access — stride = row_width
    Diagonal,     // Matrix diagonal — stride = row_width + 1
}

impl AccessPattern {
    /// Estimated cache miss rate (0.0 = perfect, 1.0 = every access misses).
    pub fn estimated_miss_rate(&self, cache_line_bytes: u32, element_bytes: u32) -> f64 {
        let elements_per_line = (cache_line_bytes / element_bytes.max(1)).max(1) as f64;
        match self {
            Self::Sequential => 1.0 / elements_per_line,
            Self::Strided => (2.0 / elements_per_line).min(1.0),
            Self::Random => 0.9,
            Self::Columnar => 0.7,
            Self::Diagonal => 0.6,
        }
    }
}

/// Cache behavior prediction result.
#[derive(Debug, Clone)]
pub struct CachePrediction {
    pub pattern: AccessPattern,
    pub estimated_miss_rate: f64,
    pub l1_hit_pct: f64,
    pub l2_hit_pct: f64,
    pub advice: String,
}

impl CachePrediction {
    pub fn predict(pattern: AccessPattern, working_set_bytes: u64) -> Self {
        let miss_rate = pattern.estimated_miss_rate(64, 8);
        let l1_size = 32 * 1024u64;  // 32KB typical L1
        let l2_size = 256 * 1024u64; // 256KB typical L2

        let l1_hit = if working_set_bytes <= l1_size { 1.0 - miss_rate } else { (1.0 - miss_rate) * 0.6 };
        let l2_hit = if working_set_bytes <= l2_size { 1.0 - miss_rate * 0.3 } else { (1.0 - miss_rate) * 0.4 };

        let advice = match pattern {
            AccessPattern::Random => "Consider sorting data or using cache-oblivious algorithms".to_string(),
            AccessPattern::Columnar => "Transpose matrix to row-major for better locality".to_string(),
            AccessPattern::Strided => "Consider loop tiling to improve cache reuse".to_string(),
            _ => "Access pattern is cache-friendly".to_string(),
        };

        Self { pattern, estimated_miss_rate: miss_rate, l1_hit_pct: l1_hit, l2_hit_pct: l2_hit, advice }
    }
}

// ── Branch Prediction ────────────────────────────────────────────────

/// Branch prediction model.
#[derive(Debug, Clone, Copy)]
pub struct BranchProfile {
    pub total_branches: u64,
    pub taken_count: u64,
    pub not_taken_count: u64,
}

impl BranchProfile {
    pub fn new() -> Self {
        Self { total_branches: 0, taken_count: 0, not_taken_count: 0 }
    }

    pub fn record(&mut self, taken: bool) {
        self.total_branches += 1;
        if taken { self.taken_count += 1; } else { self.not_taken_count += 1; }
    }

    /// Bias: how predictable this branch is (0.5 = random, 1.0 = always same).
    pub fn bias(&self) -> f64 {
        if self.total_branches == 0 { return 0.5; }
        let taken_ratio = self.taken_count as f64 / self.total_branches as f64;
        taken_ratio.max(1.0 - taken_ratio)
    }

    /// Estimated misprediction rate (lower = better).
    pub fn misprediction_rate(&self) -> f64 {
        1.0 - self.bias()
    }

    /// Performance impact: mispredictions * pipeline_flush_penalty.
    pub fn estimated_penalty_cycles(&self, pipeline_depth: u32) -> f64 {
        self.misprediction_rate() * self.total_branches as f64 * pipeline_depth as f64
    }
}

impl Default for BranchProfile {
    fn default() -> Self { Self::new() }
}

// ── Performance Prophecy ─────────────────────────────────────────────

/// Performance bottleneck kind.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BottleneckKind {
    Compute,       // CPU-bound
    MemoryLatency, // Memory-bound (latency)
    MemoryBandwidth, // Memory-bound (bandwidth)
    BranchHeavy,   // Branch misprediction
    CacheThrash,   // Cache thrashing
    Synchronization, // Lock contention
}

/// Complete performance prophecy for a code region.
#[derive(Debug, Clone)]
pub struct PerformanceProphecy {
    pub region_name: String,
    pub complexity: Complexity,
    pub cache: CachePrediction,
    pub branch_bias: f64,
    pub predicted_bottleneck: BottleneckKind,
    pub optimization_suggestions: Vec<String>,
}

impl PerformanceProphecy {
    pub fn analyze(
        name: &str, complexity: Complexity, pattern: AccessPattern,
        working_set: u64, branch_bias: f64, compute_intensity: f64,
    ) -> Self {
        let cache = CachePrediction::predict(pattern, working_set);

        let bottleneck = if cache.estimated_miss_rate > 0.5 {
            BottleneckKind::MemoryLatency
        } else if branch_bias < 0.65 {
            BottleneckKind::BranchHeavy
        } else if compute_intensity > 10.0 {
            BottleneckKind::Compute
        } else {
            BottleneckKind::MemoryBandwidth
        };

        let mut suggestions = Vec::new();
        if complexity.severity() >= 4 {
            suggestions.push(format!("Complexity {} may be prohibitive for large inputs", complexity.notation()));
        }
        if cache.estimated_miss_rate > 0.3 {
            suggestions.push(cache.advice.clone());
        }
        if branch_bias < 0.7 {
            suggestions.push("Consider branchless algorithms or CMOV patterns".to_string());
        }

        Self {
            region_name: name.to_string(), complexity, cache,
            branch_bias, predicted_bottleneck: bottleneck, optimization_suggestions: suggestions,
        }
    }

    /// Overall performance score (0-100, higher = better predicted perf).
    pub fn score(&self) -> f64 {
        let complexity_score = match self.complexity.severity() {
            0..=1 => 30.0,
            2..=3 => 25.0,
            4 => 15.0,
            5 => 8.0,
            _ => 2.0,
        };
        let cache_score = (1.0 - self.cache.estimated_miss_rate) * 40.0;
        let branch_score = self.branch_bias * 30.0;
        (complexity_score + cache_score + branch_score).clamp(0.0, 100.0)
    }
}

// ── FFI Exports ──────────────────────────────────────────────────────

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_perf_complexity_ops(severity: i64, n: i64) -> f64 {
    let c = match severity {
        0 => Complexity::O1, 1 => Complexity::OLogN, 2 => Complexity::ON,
        3 => Complexity::ONLogN, 4 => Complexity::ON2, 5 => Complexity::ON3,
        6 => Complexity::O2N, _ => Complexity::OFactorial,
    };
    c.estimate_ops(n.max(1) as u64)
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_perf_cache_miss_rate(pattern_id: i64, cache_line: i64, elem_size: i64) -> f64 {
    let p = match pattern_id {
        0 => AccessPattern::Sequential, 1 => AccessPattern::Strided,
        2 => AccessPattern::Random, 3 => AccessPattern::Columnar,
        _ => AccessPattern::Diagonal,
    };
    p.estimated_miss_rate(cache_line.max(1) as u32, elem_size.max(1) as u32)
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_perf_branch_penalty(taken: i64, not_taken: i64, pipeline: i64) -> f64 {
    let mut bp = BranchProfile::new();
    for _ in 0..taken.max(0) { bp.record(true); }
    for _ in 0..not_taken.max(0) { bp.record(false); }
    bp.estimated_penalty_cycles(pipeline.max(1) as u32)
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_perf_prophecy_score(
    complexity_severity: i64, miss_rate: f64, branch_bias: f64,
) -> f64 {
    let complexity_score = match complexity_severity {
        0..=1 => 30.0, 2..=3 => 25.0, 4 => 15.0, 5 => 8.0, _ => 2.0,
    };
    let cache_score = (1.0 - miss_rate.clamp(0.0, 1.0)) * 40.0;
    let branch_score = branch_bias.clamp(0.0, 1.0) * 30.0;
    (complexity_score + cache_score + branch_score).clamp(0.0, 100.0)
}

// ═══════════════════════════════════════════════════════════════════════
// Tests
// ═══════════════════════════════════════════════════════════════════════
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_complexity_notation() {
        assert_eq!(Complexity::ON2.notation(), "O(n²)");
        assert_eq!(Complexity::ONLogN.notation(), "O(n log n)");
    }

    #[test]
    fn test_complexity_estimate_ops() {
        assert!((Complexity::O1.estimate_ops(1000) - 1.0).abs() < 1e-10);
        assert!((Complexity::ON.estimate_ops(100) - 100.0).abs() < 1e-10);
        assert!((Complexity::ON2.estimate_ops(10) - 100.0).abs() < 1e-10);
    }

    #[test]
    fn test_complexity_severity_ordering() {
        assert!(Complexity::O1.severity() < Complexity::ON.severity());
        assert!(Complexity::ON.severity() < Complexity::ON2.severity());
        assert!(Complexity::ON2.severity() < Complexity::O2N.severity());
    }

    #[test]
    fn test_estimate_from_loop_depth() {
        assert_eq!(estimate_from_loop_depth(0, false), Complexity::O1);
        assert_eq!(estimate_from_loop_depth(1, false), Complexity::ON);
        assert_eq!(estimate_from_loop_depth(1, true), Complexity::OLogN);
        assert_eq!(estimate_from_loop_depth(2, false), Complexity::ON2);
    }

    #[test]
    fn test_access_pattern_miss_rate() {
        let seq = AccessPattern::Sequential.estimated_miss_rate(64, 8);
        let rnd = AccessPattern::Random.estimated_miss_rate(64, 8);
        assert!(seq < rnd);
    }

    #[test]
    fn test_cache_prediction() {
        let pred = CachePrediction::predict(AccessPattern::Sequential, 16 * 1024);
        assert!(pred.l1_hit_pct > 0.8);
    }

    #[test]
    fn test_branch_profile_biased() {
        let mut bp = BranchProfile::new();
        for _ in 0..100 { bp.record(true); }
        assert!(bp.bias() > 0.99);
        assert!(bp.misprediction_rate() < 0.01);
    }

    #[test]
    fn test_branch_profile_unbiased() {
        let mut bp = BranchProfile::new();
        for _ in 0..50 { bp.record(true); }
        for _ in 0..50 { bp.record(false); }
        assert!((bp.bias() - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_branch_penalty() {
        let mut bp = BranchProfile::new();
        for _ in 0..50 { bp.record(true); }
        for _ in 0..50 { bp.record(false); }
        assert!(bp.estimated_penalty_cycles(20) > 0.0);
    }

    #[test]
    fn test_prophecy_good() {
        let p = PerformanceProphecy::analyze(
            "fast_loop", Complexity::ON, AccessPattern::Sequential, 8192, 0.95, 5.0,
        );
        assert!(p.score() > 60.0);
    }

    #[test]
    fn test_prophecy_bad() {
        let p = PerformanceProphecy::analyze(
            "slow_loop", Complexity::ON3, AccessPattern::Random, 1 << 20, 0.55, 1.0,
        );
        assert!(p.score() < 50.0);
        assert!(!p.optimization_suggestions.is_empty());
    }

    #[test]
    fn test_prophecy_bottleneck_memory() {
        let p = PerformanceProphecy::analyze(
            "mem_heavy", Complexity::ON, AccessPattern::Random, 1 << 20, 0.9, 1.0,
        );
        assert_eq!(p.predicted_bottleneck, BottleneckKind::MemoryLatency);
    }

    #[test]
    fn test_ffi_complexity_ops() {
        assert!((vitalis_perf_complexity_ops(2, 100) - 100.0).abs() < 1e-10);
    }

    #[test]
    fn test_ffi_cache_miss() {
        let seq = vitalis_perf_cache_miss_rate(0, 64, 8);
        let rnd = vitalis_perf_cache_miss_rate(2, 64, 8);
        assert!(seq < rnd);
    }

    #[test]
    fn test_ffi_branch_penalty() {
        let p = vitalis_perf_branch_penalty(50, 50, 20);
        assert!(p > 0.0);
    }

    #[test]
    fn test_ffi_prophecy_score() {
        let good = vitalis_perf_prophecy_score(1, 0.05, 0.95);
        let bad = vitalis_perf_prophecy_score(6, 0.9, 0.5);
        assert!(good > bad);
    }
}
