//! v99 Fixpoint candidate validation over benchmark corpus.

#[derive(Debug, Clone, PartialEq)]
pub struct BenchmarkCase {
    pub name: String,
    pub baseline_ms: f64,
    pub candidate_ms: f64,
    pub correct: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FixpointReport {
    pub speedup_ratio: f64,
    pub correctness_ok: bool,
    pub regressed_cases: Vec<String>,
}

pub fn evaluate_fixpoint(cases: &[BenchmarkCase], regression_tolerance: f64) -> FixpointReport {
    if cases.is_empty() {
        return FixpointReport { speedup_ratio: 1.0, correctness_ok: true, regressed_cases: Vec::new() };
    }

    let base = cases.iter().map(|c| c.baseline_ms).sum::<f64>();
    let cand = cases.iter().map(|c| c.candidate_ms).sum::<f64>();
    let mut regressed = Vec::new();

    for c in cases {
        if c.candidate_ms > c.baseline_ms * (1.0 + regression_tolerance) {
            regressed.push(c.name.clone());
        }
    }

    let correctness_ok = cases.iter().all(|c| c.correct);
    FixpointReport {
        speedup_ratio: if cand > 0.0 { base / cand } else { 1.0 },
        correctness_ok,
        regressed_cases: regressed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fixpoint_report_detects_regression() {
        let cases = vec![
            BenchmarkCase { name: "a".into(), baseline_ms: 10.0, candidate_ms: 9.0, correct: true },
            BenchmarkCase { name: "b".into(), baseline_ms: 10.0, candidate_ms: 12.0, correct: true },
        ];
        let r = evaluate_fixpoint(&cases, 0.1);
        assert_eq!(r.regressed_cases, vec!["b".to_string()]);
        assert!(r.correctness_ok);
    }

    #[test]
    fn test_fixpoint_report_correctness_gate() {
        let cases = vec![BenchmarkCase { name: "x".into(), baseline_ms: 3.0, candidate_ms: 2.9, correct: false }];
        let r = evaluate_fixpoint(&cases, 0.1);
        assert!(!r.correctness_ok);
    }
}
