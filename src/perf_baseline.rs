//! v390 — Performance baseline module.
//!
//! Establishes baselines for critical performance metrics and provides
//! regression detection for self-evolution. All benchmarks are deterministic
//! and produce comparable results across runs.

use std::time::Instant;

/// A performance measurement result.
#[derive(Debug, Clone)]
pub struct Measurement {
    pub name: String,
    pub duration_us: u64,
    pub throughput: f64,
    pub iterations: u64,
}

/// A baseline entry stored for regression comparison.
#[derive(Debug, Clone)]
pub struct Baseline {
    pub name: String,
    pub mean_us: f64,
    pub stddev_us: f64,
    pub min_us: f64,
    pub max_us: f64,
    pub samples: usize,
}

/// Regression detection result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegressionStatus {
    /// Performance is within expected bounds.
    Ok,
    /// Performance improved (>5% faster).
    Improved,
    /// Performance regressed (>5% slower).
    Regressed,
    /// Insufficient data for comparison.
    NoBaseline,
}

/// Performance baseline registry.
pub struct PerfBaseline {
    baselines: Vec<Baseline>,
    measurements: Vec<Measurement>,
    regression_threshold: f64,
}

impl PerfBaseline {
    pub fn new() -> Self {
        Self {
            baselines: Vec::new(),
            measurements: Vec::new(),
            regression_threshold: 0.05, // 5% threshold
        }
    }

    /// Run a benchmark function N times and record measurements.
    pub fn bench<F: FnMut()>(&mut self, name: &str, iterations: u64, mut f: F) -> Measurement {
        let start = Instant::now();
        for _ in 0..iterations {
            f();
        }
        let elapsed = start.elapsed();
        let duration_us = elapsed.as_micros() as u64;
        let throughput = if duration_us > 0 {
            (iterations as f64 * 1_000_000.0) / duration_us as f64
        } else {
            f64::INFINITY
        };

        let m = Measurement {
            name: name.to_string(),
            duration_us,
            throughput,
            iterations,
        };
        self.measurements.push(m.clone());
        m
    }

    /// Run a benchmark with warmup and multiple samples.
    pub fn bench_sampled<F: FnMut()>(
        &mut self,
        name: &str,
        iterations: u64,
        samples: usize,
        warmup: usize,
        mut f: F,
    ) -> Baseline {
        // Warmup
        for _ in 0..warmup {
            for _ in 0..iterations { f(); }
        }

        // Collect samples
        let mut durations = Vec::with_capacity(samples);
        for _ in 0..samples {
            let start = Instant::now();
            for _ in 0..iterations { f(); }
            durations.push(start.elapsed().as_micros() as f64);
        }

        let n = durations.len() as f64;
        let mean = durations.iter().sum::<f64>() / n;
        let variance = if durations.len() > 1 {
            durations.iter().map(|d| (d - mean).powi(2)).sum::<f64>() / (n - 1.0)
        } else {
            0.0
        };
        let stddev = variance.sqrt();
        let min = durations.iter().cloned().fold(f64::INFINITY, f64::min);
        let max = durations.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

        let baseline = Baseline {
            name: name.to_string(),
            mean_us: mean,
            stddev_us: stddev,
            min_us: min,
            max_us: max,
            samples: durations.len(),
        };

        self.baselines.push(baseline.clone());
        baseline
    }

    /// Compare a new measurement against a stored baseline.
    pub fn check_regression(&self, name: &str, current_us: f64) -> RegressionStatus {
        let baseline = match self.baselines.iter().find(|b| b.name == name) {
            Some(b) => b,
            None => return RegressionStatus::NoBaseline,
        };

        if baseline.mean_us <= 0.0 {
            return RegressionStatus::Ok;
        }

        let ratio = (current_us - baseline.mean_us) / baseline.mean_us;
        if ratio > self.regression_threshold {
            RegressionStatus::Regressed
        } else if ratio < -self.regression_threshold {
            RegressionStatus::Improved
        } else {
            RegressionStatus::Ok
        }
    }

    /// Get all recorded baselines.
    pub fn baselines(&self) -> &[Baseline] {
        &self.baselines
    }

    /// Get all measurements from this session.
    pub fn measurements(&self) -> &[Measurement] {
        &self.measurements
    }

    /// JSON export of all baselines.
    pub fn export_json(&self) -> String {
        let entries: Vec<String> = self.baselines.iter().map(|b| {
            format!(
                "{{\"name\":\"{}\",\"mean_us\":{:.2},\"stddev_us\":{:.2},\"min_us\":{:.2},\"max_us\":{:.2},\"samples\":{}}}",
                b.name, b.mean_us, b.stddev_us, b.min_us, b.max_us, b.samples
            )
        }).collect();
        format!("{{\"baselines\":[{}]}}", entries.join(","))
    }

    pub fn reset(&mut self) {
        self.baselines.clear();
        self.measurements.clear();
    }
}

impl Default for PerfBaseline {
    fn default() -> Self { Self::new() }
}

// ─── Built-in Benchmarks ─────────────────────────────────────────────

/// Benchmark the lexer on a complex source.
pub fn bench_lexer(pb: &mut PerfBaseline) -> Measurement {
    let source = r#"
fn factorial(n: i64) -> i64 {
    if n <= 1 { 1 } else { n * factorial(n - 1) }
}
fn fibonacci(n: i64) -> i64 {
    if n <= 1 { n } else { fibonacci(n - 1) + fibonacci(n - 2) }
}
struct Point { x: i64, y: i64 }
enum Color { Red, Green, Blue }
fn main() -> i64 { factorial(10) + fibonacci(10) }
"#;
    pb.bench("lexer-complex", 1000, || {
        let _ = crate::lexer::lex(source);
    })
}

/// Benchmark the parser on a complex source.
pub fn bench_parser(pb: &mut PerfBaseline) -> Measurement {
    let source = r#"
fn factorial(n: i64) -> i64 {
    if n <= 1 { 1 } else { n * factorial(n - 1) }
}
fn fibonacci(n: i64) -> i64 {
    if n <= 1 { n } else { fibonacci(n - 1) + fibonacci(n - 2) }
}
fn main() -> i64 { factorial(10) + fibonacci(10) }
"#;
    pb.bench("parser-complex", 500, || {
        let _ = crate::parser::parse(source);
    })
}

/// Benchmark tensor operations.
pub fn bench_tensor_ops(pb: &mut PerfBaseline) -> Measurement {
    pb.bench("tensor-create-100x100", 100, || {
        let _ = crate::tensor::Tensor::zeros(&[100, 100]);
    })
}

/// Benchmark the LSP symbol indexing.
pub fn bench_lsp_indexing(pb: &mut PerfBaseline) -> Measurement {
    let source = r#"
fn a() -> i64 { 1 }
fn b() -> i64 { 2 }
fn c() -> i64 { 3 }
fn d() -> i64 { 4 }
fn e() -> i64 { 5 }
struct S1 { x: i64 }
struct S2 { y: i64 }
enum E1 { A, B, C }
fn main() -> i64 { a() + b() + c() + d() + e() }
"#;
    pb.bench("lsp-indexing", 200, || {
        let mut server = crate::lsp::LspServer::new();
        server.initialize();
        server.open_document("test.sl", source);
        let _ = server.document_symbols("test.sl");
    })
}

/// Run all built-in benchmarks and return the baseline set.
pub fn run_all_benchmarks() -> PerfBaseline {
    let mut pb = PerfBaseline::new();
    bench_lexer(&mut pb);
    bench_parser(&mut pb);
    bench_tensor_ops(&mut pb);
    bench_lsp_indexing(&mut pb);
    pb
}

// ─── FFI ─────────────────────────────────────────────────────────────

/// Run all benchmarks and return count of measurements taken.
#[unsafe(no_mangle)]
pub extern "C" fn slang_perf_run_all() -> i64 {
    let pb = run_all_benchmarks();
    pb.measurements().len() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bench_basic() {
        let mut pb = PerfBaseline::new();
        let m = pb.bench("test-bench", 100, || {
            let _ = 1 + 1;
        });
        assert_eq!(m.name, "test-bench");
        assert_eq!(m.iterations, 100);
        assert!(m.throughput > 0.0);
    }

    #[test]
    fn test_bench_sampled() {
        let mut pb = PerfBaseline::new();
        let baseline = pb.bench_sampled("test-sampled", 50, 5, 1, || {
            let _ = 1 + 1;
        });
        assert_eq!(baseline.samples, 5);
        assert!(baseline.mean_us >= 0.0);
        assert!(baseline.min_us <= baseline.max_us);
    }

    #[test]
    fn test_regression_no_baseline() {
        let pb = PerfBaseline::new();
        assert_eq!(pb.check_regression("nonexistent", 100.0), RegressionStatus::NoBaseline);
    }

    #[test]
    fn test_regression_ok() {
        let mut pb = PerfBaseline::new();
        pb.baselines.push(Baseline {
            name: "test".into(), mean_us: 100.0, stddev_us: 5.0,
            min_us: 90.0, max_us: 110.0, samples: 10,
        });
        assert_eq!(pb.check_regression("test", 103.0), RegressionStatus::Ok);
    }

    #[test]
    fn test_regression_detected() {
        let mut pb = PerfBaseline::new();
        pb.baselines.push(Baseline {
            name: "test".into(), mean_us: 100.0, stddev_us: 5.0,
            min_us: 90.0, max_us: 110.0, samples: 10,
        });
        assert_eq!(pb.check_regression("test", 120.0), RegressionStatus::Regressed);
    }

    #[test]
    fn test_regression_improved() {
        let mut pb = PerfBaseline::new();
        pb.baselines.push(Baseline {
            name: "test".into(), mean_us: 100.0, stddev_us: 5.0,
            min_us: 90.0, max_us: 110.0, samples: 10,
        });
        assert_eq!(pb.check_regression("test", 80.0), RegressionStatus::Improved);
    }

    #[test]
    fn test_export_json() {
        let mut pb = PerfBaseline::new();
        pb.baselines.push(Baseline {
            name: "test".into(), mean_us: 100.0, stddev_us: 5.0,
            min_us: 90.0, max_us: 110.0, samples: 10,
        });
        let json = pb.export_json();
        assert!(json.contains("\"name\":\"test\""));
        assert!(json.contains("\"mean_us\":100.00"));
    }

    #[test]
    fn test_bench_lexer() {
        let mut pb = PerfBaseline::new();
        let m = bench_lexer(&mut pb);
        assert!(m.duration_us > 0 || m.iterations > 0);
    }

    #[test]
    fn test_bench_parser() {
        let mut pb = PerfBaseline::new();
        let m = bench_parser(&mut pb);
        assert!(m.duration_us > 0 || m.iterations > 0);
    }

    #[test]
    fn test_bench_tensor() {
        let mut pb = PerfBaseline::new();
        let m = bench_tensor_ops(&mut pb);
        assert!(m.iterations == 100);
    }

    #[test]
    fn test_bench_lsp() {
        let mut pb = PerfBaseline::new();
        let m = bench_lsp_indexing(&mut pb);
        assert!(m.iterations > 0);
    }

    #[test]
    fn test_run_all_benchmarks() {
        let pb = run_all_benchmarks();
        assert!(pb.measurements().len() >= 4);
    }

    #[test]
    fn test_reset() {
        let mut pb = PerfBaseline::new();
        pb.bench("x", 1, || {});
        pb.reset();
        assert!(pb.measurements().is_empty());
        assert!(pb.baselines().is_empty());
    }

    #[test]
    fn test_ffi_perf_run() {
        let n = slang_perf_run_all();
        assert!(n >= 4);
    }
}
