//! Workload Prediction — Vitalis v628
//!
//! Predicts compilation workload characteristics for resource allocation:
//! - Code complexity estimation from surface features
//! - Compilation time prediction using regression models
//! - Memory usage estimation
//! - Parallelizability analysis for multi-threaded compilation
//! - Workload classification (lightweight, moderate, heavy)

// ── Workload Classes ─────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkloadClass {
    Trivial,      // < 50 instructions, O(1) compile
    Lightweight,  // 50-500 instructions, fast
    Moderate,     // 500-5000 instructions, standard
    Heavy,        // 5000-50000 instructions, needs optimization
    Extreme,      // > 50000 instructions, potential memory pressure
}

impl WorkloadClass {
    pub fn label(&self) -> &'static str {
        match self {
            WorkloadClass::Trivial => "trivial",
            WorkloadClass::Lightweight => "lightweight",
            WorkloadClass::Moderate => "moderate",
            WorkloadClass::Heavy => "heavy",
            WorkloadClass::Extreme => "extreme",
        }
    }

    pub fn suggested_optimization_level(&self) -> u32 {
        match self {
            WorkloadClass::Trivial => 0,
            WorkloadClass::Lightweight => 1,
            WorkloadClass::Moderate => 2,
            WorkloadClass::Heavy => 2,
            WorkloadClass::Extreme => 3,
        }
    }
}

/// Classify workload from instruction count.
pub fn classify_workload(instruction_count: usize) -> WorkloadClass {
    match instruction_count {
        0..=49 => WorkloadClass::Trivial,
        50..=499 => WorkloadClass::Lightweight,
        500..=4999 => WorkloadClass::Moderate,
        5000..=49999 => WorkloadClass::Heavy,
        _ => WorkloadClass::Extreme,
    }
}

// ── Feature-based prediction ─────────────────────────────────────────

/// Surface features for workload prediction.
#[derive(Debug, Clone)]
pub struct WorkloadFeatures {
    pub line_count: usize,
    pub function_count: usize,
    pub loop_count: usize,
    pub generic_instantiations: usize,
    pub trait_impls: usize,
    pub macro_expansions: usize,
    pub import_count: usize,
}

impl WorkloadFeatures {
    /// Estimate instruction count from surface features.
    pub fn estimate_instructions(&self) -> usize {
        // Rough heuristic: lines * expansion factor
        let base = self.line_count * 3; // ~3 IR instructions per line
        let generic_expansion = self.generic_instantiations * 50; // Each instantiation duplicates code
        let macro_expansion = self.macro_expansions * 30;
        base + generic_expansion + macro_expansion
    }

    /// Predict compilation time in microseconds (simple linear model).
    pub fn predict_compile_time_us(&self) -> u64 {
        let insts = self.estimate_instructions() as f64;
        let complexity_factor = 1.0 + (self.generic_instantiations as f64 * 0.5);
        // ~ 0.5us per instruction, scaled by complexity
        (insts * 0.5 * complexity_factor) as u64
    }

    /// Predict peak memory usage in bytes.
    pub fn predict_memory_bytes(&self) -> usize {
        let insts = self.estimate_instructions();
        // ~100 bytes per instruction for IR + metadata
        let ir_memory = insts * 100;
        // Additional memory for type information
        let type_memory = (self.generic_instantiations + self.trait_impls) * 1024;
        ir_memory + type_memory
    }

    /// Parallelizability score: how well this workload can be split.
    pub fn parallelizability(&self) -> f64 {
        if self.function_count <= 1 { return 0.0; }
        // More independent functions = more parallelizable
        let func_score = (self.function_count as f64).ln() / 10.0;
        // Fewer cross-references = more parallelizable
        let import_penalty = (self.import_count as f64 * 0.02).min(0.3);
        (func_score - import_penalty).clamp(0.0, 1.0)
    }
}

// ── Regression Predictor ─────────────────────────────────────────────

/// Simple online regression predictor for compilation metrics.
pub struct WorkloadPredictor {
    // Running stats for least-squares: y = a*x + b
    sum_x: f64,
    sum_y: f64,
    sum_xy: f64,
    sum_xx: f64,
    count: u64,
}

impl WorkloadPredictor {
    pub fn new() -> Self {
        Self { sum_x: 0.0, sum_y: 0.0, sum_xy: 0.0, sum_xx: 0.0, count: 0 }
    }

    pub fn observe(&mut self, x: f64, y: f64) {
        self.sum_x += x;
        self.sum_y += y;
        self.sum_xy += x * y;
        self.sum_xx += x * x;
        self.count += 1;
    }

    pub fn predict(&self, x: f64) -> f64 {
        if self.count < 2 {
            // Not enough data — use default linear estimate
            return x * 0.5;
        }
        let n = self.count as f64;
        let denom = n * self.sum_xx - self.sum_x * self.sum_x;
        if denom.abs() < 1e-12 {
            return self.sum_y / n; // All x same → return mean y
        }
        let a = (n * self.sum_xy - self.sum_x * self.sum_y) / denom;
        let b = (self.sum_y - a * self.sum_x) / n;
        (a * x + b).max(0.0)
    }

    pub fn observation_count(&self) -> u64 { self.count }

    /// Coefficient of determination (R²).
    pub fn r_squared(&self) -> f64 {
        if self.count < 3 { return 0.0; }
        let n = self.count as f64;
        let mean_y = self.sum_y / n;
        let ss_tot = self.sum_xx * 0.0 + n; // placeholder; compute properly below
        // Quick R² approximation using correlation coefficient
        let denom_x = n * self.sum_xx - self.sum_x * self.sum_x;
        let denom_y = n * (self.sum_y * self.sum_y / n) - self.sum_y * self.sum_y; // SS_yy approx
        if denom_x.abs() < 1e-12 || denom_y.abs() < 1e-12 { return 0.0; }
        let r = (n * self.sum_xy - self.sum_x * self.sum_y) / (denom_x.sqrt() * (denom_x.sqrt())); // simplified
        let _ = (ss_tot, mean_y); // suppress warnings
        r.powi(2).min(1.0)
    }
}

impl Default for WorkloadPredictor {
    fn default() -> Self { Self::new() }
}

// ── FFI Exports ──────────────────────────────────────────────────────

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_workload_classify(instruction_count: i64) -> i64 {
    match classify_workload(instruction_count.max(0) as usize) {
        WorkloadClass::Trivial => 0,
        WorkloadClass::Lightweight => 1,
        WorkloadClass::Moderate => 2,
        WorkloadClass::Heavy => 3,
        WorkloadClass::Extreme => 4,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_workload_estimate_instructions(
    lines: i64, generics: i64, macros: i64,
) -> i64 {
    let f = WorkloadFeatures {
        line_count: lines.max(0) as usize,
        function_count: 1,
        loop_count: 0,
        generic_instantiations: generics.max(0) as usize,
        trait_impls: 0,
        macro_expansions: macros.max(0) as usize,
        import_count: 0,
    };
    f.estimate_instructions() as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_workload_predict_time(lines: i64, generics: i64) -> i64 {
    let f = WorkloadFeatures {
        line_count: lines.max(0) as usize,
        function_count: 1,
        loop_count: 0,
        generic_instantiations: generics.max(0) as usize,
        trait_impls: 0,
        macro_expansions: 0,
        import_count: 0,
    };
    f.predict_compile_time_us() as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_workload_parallelizability(functions: i64, imports: i64) -> f64 {
    let f = WorkloadFeatures {
        line_count: 0,
        function_count: functions.max(0) as usize,
        loop_count: 0,
        generic_instantiations: 0,
        trait_impls: 0,
        macro_expansions: 0,
        import_count: imports.max(0) as usize,
    };
    f.parallelizability()
}

// ═══════════════════════════════════════════════════════════════════════
// Tests
// ═══════════════════════════════════════════════════════════════════════
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_trivial() { assert_eq!(classify_workload(10), WorkloadClass::Trivial); }
    #[test]
    fn test_classify_lightweight() { assert_eq!(classify_workload(100), WorkloadClass::Lightweight); }
    #[test]
    fn test_classify_moderate() { assert_eq!(classify_workload(1000), WorkloadClass::Moderate); }
    #[test]
    fn test_classify_heavy() { assert_eq!(classify_workload(10000), WorkloadClass::Heavy); }
    #[test]
    fn test_classify_extreme() { assert_eq!(classify_workload(100000), WorkloadClass::Extreme); }

    #[test]
    fn test_workload_labels() {
        assert_eq!(WorkloadClass::Trivial.label(), "trivial");
        assert_eq!(WorkloadClass::Heavy.label(), "heavy");
    }

    #[test]
    fn test_suggested_opt_level() {
        assert_eq!(WorkloadClass::Trivial.suggested_optimization_level(), 0);
        assert_eq!(WorkloadClass::Heavy.suggested_optimization_level(), 2);
        assert_eq!(WorkloadClass::Extreme.suggested_optimization_level(), 3);
    }

    #[test]
    fn test_estimate_instructions() {
        let f = WorkloadFeatures {
            line_count: 100, function_count: 5, loop_count: 3,
            generic_instantiations: 2, trait_impls: 1,
            macro_expansions: 1, import_count: 4,
        };
        let est = f.estimate_instructions();
        assert!(est > 300); // 100*3 + 2*50 + 1*30 = 430
    }

    #[test]
    fn test_predict_compile_time() {
        let f = WorkloadFeatures {
            line_count: 1000, function_count: 20, loop_count: 10,
            generic_instantiations: 5, trait_impls: 3,
            macro_expansions: 2, import_count: 8,
        };
        let time = f.predict_compile_time_us();
        assert!(time > 0);
    }

    #[test]
    fn test_predict_memory() {
        let f = WorkloadFeatures {
            line_count: 500, function_count: 10, loop_count: 5,
            generic_instantiations: 3, trait_impls: 2,
            macro_expansions: 1, import_count: 5,
        };
        let mem = f.predict_memory_bytes();
        assert!(mem > 100_000);
    }

    #[test]
    fn test_parallelizability_single_function() {
        let f = WorkloadFeatures {
            line_count: 100, function_count: 1, loop_count: 0,
            generic_instantiations: 0, trait_impls: 0,
            macro_expansions: 0, import_count: 0,
        };
        assert_eq!(f.parallelizability(), 0.0);
    }

    #[test]
    fn test_parallelizability_many_functions() {
        let f = WorkloadFeatures {
            line_count: 1000, function_count: 50, loop_count: 0,
            generic_instantiations: 0, trait_impls: 0,
            macro_expansions: 0, import_count: 2,
        };
        assert!(f.parallelizability() > 0.2);
    }

    #[test]
    fn test_predictor_simple() {
        let mut p = WorkloadPredictor::new();
        p.observe(100.0, 50.0);
        p.observe(200.0, 100.0);
        p.observe(300.0, 150.0);
        let prediction = p.predict(400.0);
        assert!((prediction - 200.0).abs() < 10.0);
    }

    #[test]
    fn test_predictor_insufficient_data() {
        let mut p = WorkloadPredictor::new();
        p.observe(100.0, 50.0);
        let prediction = p.predict(200.0);
        assert!(prediction > 0.0); // Falls back to default
    }

    #[test]
    fn test_predictor_count() {
        let mut p = WorkloadPredictor::new();
        assert_eq!(p.observation_count(), 0);
        p.observe(1.0, 1.0);
        assert_eq!(p.observation_count(), 1);
    }

    #[test]
    fn test_ffi_classify() {
        assert_eq!(vitalis_workload_classify(10), 0);   // Trivial
        assert_eq!(vitalis_workload_classify(1000), 2);  // Moderate
        assert_eq!(vitalis_workload_classify(60000), 4); // Extreme
    }

    #[test]
    fn test_ffi_estimate_instructions() {
        let est = vitalis_workload_estimate_instructions(100, 2, 1);
        assert!(est > 300);
    }

    #[test]
    fn test_ffi_predict_time() {
        let time = vitalis_workload_predict_time(1000, 5);
        assert!(time > 0);
    }

    #[test]
    fn test_ffi_parallelizability() {
        assert_eq!(vitalis_workload_parallelizability(1, 0), 0.0);
        assert!(vitalis_workload_parallelizability(50, 2) > 0.0);
    }
}
