//! v90 Ecosystem stability release helpers: API stabilization review,
//! soak-test summaries, and compatibility matrices.

use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApiState {
    Stable,
    Deprecated { replacement: Option<String> },
    Experimental,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApiSurfaceItem {
    pub symbol: String,
    pub state: ApiState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StabilizationReview {
    pub stable_count: usize,
    pub deprecated_count: usize,
    pub experimental_count: usize,
    pub missing_replacements: Vec<String>,
}

pub fn review_api_stability(items: &[ApiSurfaceItem]) -> StabilizationReview {
    let mut stable_count = 0usize;
    let mut deprecated_count = 0usize;
    let mut experimental_count = 0usize;
    let mut missing_replacements = Vec::new();

    for item in items {
        match &item.state {
            ApiState::Stable => stable_count += 1,
            ApiState::Deprecated { replacement } => {
                deprecated_count += 1;
                if replacement.is_none() {
                    missing_replacements.push(item.symbol.clone());
                }
            }
            ApiState::Experimental => experimental_count += 1,
        }
    }

    missing_replacements.sort();

    StabilizationReview {
        stable_count,
        deprecated_count,
        experimental_count,
        missing_replacements,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SoakRun {
    pub scenario: String,
    pub iterations: u64,
    pub failures: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SoakSummary {
    pub total_iterations: u64,
    pub total_failures: u64,
    pub failure_rate: f64,
    pub flaky_scenarios: Vec<String>,
}

pub fn summarize_soak(runs: &[SoakRun], flaky_threshold: f64) -> SoakSummary {
    let total_iterations = runs.iter().map(|r| r.iterations).sum::<u64>();
    let total_failures = runs.iter().map(|r| r.failures).sum::<u64>();
    let failure_rate = if total_iterations == 0 {
        0.0
    } else {
        total_failures as f64 / total_iterations as f64
    };

    let mut flaky_scenarios = Vec::new();
    for run in runs {
        if run.iterations == 0 {
            continue;
        }
        let rate = run.failures as f64 / run.iterations as f64;
        if rate > flaky_threshold {
            flaky_scenarios.push(run.scenario.clone());
        }
    }
    flaky_scenarios.sort();

    SoakSummary {
        total_iterations,
        total_failures,
        failure_rate,
        flaky_scenarios,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompatibilityMatrix {
    pub targets: Vec<String>,
    pub features: Vec<String>,
    pub support: BTreeMap<(String, String), bool>,
}

impl CompatibilityMatrix {
    pub fn new(targets: &[&str], features: &[&str]) -> Self {
        let targets = targets.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        let features = features.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        Self {
            targets,
            features,
            support: BTreeMap::new(),
        }
    }

    pub fn set(&mut self, target: &str, feature: &str, supported: bool) {
        self.support
            .insert((target.to_string(), feature.to_string()), supported);
    }

    pub fn is_supported(&self, target: &str, feature: &str) -> bool {
        self.support
            .get(&(target.to_string(), feature.to_string()))
            .copied()
            .unwrap_or(false)
    }

    pub fn unsupported_pairs(&self) -> Vec<(String, String)> {
        let mut pairs = BTreeSet::new();
        for t in &self.targets {
            for f in &self.features {
                if !self.is_supported(t, f) {
                    pairs.insert((t.clone(), f.clone()));
                }
            }
        }
        pairs.into_iter().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_review_api_stability() {
        let review = review_api_stability(&[
            ApiSurfaceItem {
                symbol: "pkg::install".to_string(),
                state: ApiState::Stable,
            },
            ApiSurfaceItem {
                symbol: "pkg::legacy_resolve".to_string(),
                state: ApiState::Deprecated {
                    replacement: Some("pkg::resolve_v2".to_string()),
                },
            },
            ApiSurfaceItem {
                symbol: "pkg::legacy_lock".to_string(),
                state: ApiState::Deprecated { replacement: None },
            },
            ApiSurfaceItem {
                symbol: "pkg::experimental_sat".to_string(),
                state: ApiState::Experimental,
            },
        ]);

        assert_eq!(review.stable_count, 1);
        assert_eq!(review.deprecated_count, 2);
        assert_eq!(review.experimental_count, 1);
        assert_eq!(review.missing_replacements, vec!["pkg::legacy_lock".to_string()]);
    }

    #[test]
    fn test_summarize_soak() {
        let summary = summarize_soak(
            &[
                SoakRun {
                    scenario: "registry-install".to_string(),
                    iterations: 10_000,
                    failures: 1,
                },
                SoakRun {
                    scenario: "workspace-upgrade".to_string(),
                    iterations: 5_000,
                    failures: 120,
                },
            ],
            0.01,
        );

        assert_eq!(summary.total_iterations, 15_000);
        assert_eq!(summary.total_failures, 121);
        assert!(summary.failure_rate > 0.0);
        assert_eq!(summary.flaky_scenarios, vec!["workspace-upgrade".to_string()]);
    }

    #[test]
    fn test_compatibility_matrix() {
        let mut m = CompatibilityMatrix::new(
            &["x86_64-windows-msvc", "x86_64-unknown-linux-gnu"],
            &["jit", "aot", "wasm"],
        );

        m.set("x86_64-windows-msvc", "jit", true);
        m.set("x86_64-windows-msvc", "aot", true);
        m.set("x86_64-windows-msvc", "wasm", true);
        m.set("x86_64-unknown-linux-gnu", "jit", true);
        m.set("x86_64-unknown-linux-gnu", "aot", true);
        m.set("x86_64-unknown-linux-gnu", "wasm", false);

        assert!(m.is_supported("x86_64-windows-msvc", "jit"));
        assert!(!m.is_supported("x86_64-unknown-linux-gnu", "wasm"));

        let unsupported = m.unsupported_pairs();
        assert!(unsupported.contains(&(
            "x86_64-unknown-linux-gnu".to_string(),
            "wasm".to_string()
        )));
    }
}
