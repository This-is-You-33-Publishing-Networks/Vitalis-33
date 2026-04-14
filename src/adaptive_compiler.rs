//! v480 — Adaptive Compilation.
//!
//! Compiler that adapts to user coding style. Track which features are used,
//! optimize hot paths in frequently-used patterns. Per-project compilation profiles.

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

/// A tracked feature usage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FeatureUsage {
    Functions,
    Closures,
    Generics,
    PatternMatching,
    Pipes,
    AsyncAwait,
    Traits,
    Macros,
    Iterators,
    ErrorHandling,
    StructLiterals,
    EnumVariants,
}

impl FeatureUsage {
    pub fn name(&self) -> &'static str {
        match self {
            FeatureUsage::Functions => "functions",
            FeatureUsage::Closures => "closures",
            FeatureUsage::Generics => "generics",
            FeatureUsage::PatternMatching => "pattern_matching",
            FeatureUsage::Pipes => "pipes",
            FeatureUsage::AsyncAwait => "async_await",
            FeatureUsage::Traits => "traits",
            FeatureUsage::Macros => "macros",
            FeatureUsage::Iterators => "iterators",
            FeatureUsage::ErrorHandling => "error_handling",
            FeatureUsage::StructLiterals => "struct_literals",
            FeatureUsage::EnumVariants => "enum_variants",
        }
    }

    pub fn all() -> &'static [FeatureUsage] {
        &[
            FeatureUsage::Functions, FeatureUsage::Closures,
            FeatureUsage::Generics, FeatureUsage::PatternMatching,
            FeatureUsage::Pipes, FeatureUsage::AsyncAwait,
            FeatureUsage::Traits, FeatureUsage::Macros,
            FeatureUsage::Iterators, FeatureUsage::ErrorHandling,
            FeatureUsage::StructLiterals, FeatureUsage::EnumVariants,
        ]
    }
}

/// Compilation profile derived from usage patterns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompProfile {
    /// Functional style: optimize closures, pipes, iterators.
    Functional,
    /// OOP style: optimize traits, structs, methods.
    ObjectOriented,
    /// Concurrent: optimize async, channels, tasks.
    Concurrent,
    /// Numerical: optimize math ops, arrays.
    Numerical,
    /// General purpose.
    General,
}

impl CompProfile {
    pub fn name(&self) -> &'static str {
        match self {
            CompProfile::Functional => "functional",
            CompProfile::ObjectOriented => "oop",
            CompProfile::Concurrent => "concurrent",
            CompProfile::Numerical => "numerical",
            CompProfile::General => "general",
        }
    }
}

/// Adaptive compiler engine.
pub struct AdaptiveCompiler {
    pub usage_counts: HashMap<FeatureUsage, u64>,
    pub total_compilations: u64,
    pub profile: CompProfile,
    pub profile_history: Vec<(u64, CompProfile)>,
    /// Hot paths: function name → call count.
    pub hot_paths: HashMap<String, u64>,
}

impl AdaptiveCompiler {
    pub fn new() -> Self {
        Self {
            usage_counts: HashMap::new(),
            total_compilations: 0,
            profile: CompProfile::General,
            profile_history: Vec::new(),
            hot_paths: HashMap::new(),
        }
    }

    /// Record feature usage.
    pub fn record_usage(&mut self, feature: FeatureUsage, count: u64) {
        *self.usage_counts.entry(feature).or_insert(0) += count;
    }

    /// Record a compilation.
    pub fn record_compilation(&mut self) {
        self.total_compilations += 1;
        // Re-evaluate profile every 10 compilations
        if self.total_compilations % 10 == 0 {
            self.update_profile();
        }
    }

    /// Record a hot path.
    pub fn record_hot_path(&mut self, func_name: &str) {
        *self.hot_paths.entry(func_name.to_string()).or_insert(0) += 1;
    }

    /// Determine the compilation profile from usage patterns.
    pub fn update_profile(&mut self) {
        let functional_score = self.score_for(&[
            FeatureUsage::Closures, FeatureUsage::Pipes, FeatureUsage::Iterators,
        ]);
        let oop_score = self.score_for(&[
            FeatureUsage::Traits, FeatureUsage::StructLiterals, FeatureUsage::EnumVariants,
        ]);
        let concurrent_score = self.score_for(&[FeatureUsage::AsyncAwait]);
        let numerical_score = self.score_for(&[FeatureUsage::Functions]);

        let scores = [
            (functional_score, CompProfile::Functional),
            (oop_score, CompProfile::ObjectOriented),
            (concurrent_score * 3.0, CompProfile::Concurrent), // Weight async higher
            (numerical_score * 0.5, CompProfile::Numerical),
        ];

        let best = scores.iter()
            .max_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(_, p)| *p)
            .unwrap_or(CompProfile::General);

        if best != self.profile {
            self.profile_history.push((self.total_compilations, self.profile));
            self.profile = best;
        }
    }

    fn score_for(&self, features: &[FeatureUsage]) -> f64 {
        features.iter()
            .map(|f| *self.usage_counts.get(f).unwrap_or(&0) as f64)
            .sum()
    }

    /// Get top N hot paths.
    pub fn top_hot_paths(&self, n: usize) -> Vec<(&str, u64)> {
        let mut paths: Vec<_> = self.hot_paths.iter().map(|(k, v)| (k.as_str(), *v)).collect();
        paths.sort_by(|a, b| b.1.cmp(&a.1));
        paths.truncate(n);
        paths
    }

    /// Feature diversity score (0.0 = one feature only, 1.0 = all features equally used).
    pub fn diversity_score(&self) -> f64 {
        let total: f64 = self.usage_counts.values().map(|v| *v as f64).sum();
        if total < 1.0 { return 0.0; }
        let n = self.usage_counts.len() as f64;
        if n <= 1.0 { return 0.0; }

        let entropy: f64 = self.usage_counts.values()
            .map(|v| {
                let p = *v as f64 / total;
                if p > 0.0 { -p * p.ln() } else { 0.0 }
            })
            .sum();
        let max_entropy = n.ln();
        if max_entropy < 1e-15 { return 0.0; }
        entropy / max_entropy
    }

    /// Total features tracked.
    pub fn tracked_features(&self) -> usize {
        self.usage_counts.len()
    }
}

// ─── FFI ──────────────────────────────────────────────────────────────

static GLOBAL_ADAPTIVE: LazyLock<Mutex<AdaptiveCompiler>> =
    LazyLock::new(|| Mutex::new(AdaptiveCompiler::new()));

#[unsafe(no_mangle)]
pub extern "C" fn slang_adaptive_record(feature_id: i64) -> i64 {
    let mut ac = GLOBAL_ADAPTIVE.lock().unwrap();
    let features = FeatureUsage::all();
    let idx = (feature_id as usize) % features.len();
    ac.record_usage(features[idx], 1);
    ac.tracked_features() as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_adaptive_compile() -> i64 {
    let mut ac = GLOBAL_ADAPTIVE.lock().unwrap();
    ac.record_compilation();
    ac.total_compilations as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_adaptive_profile() -> i64 {
    let ac = GLOBAL_ADAPTIVE.lock().unwrap();
    match ac.profile {
        CompProfile::Functional => 1,
        CompProfile::ObjectOriented => 2,
        CompProfile::Concurrent => 3,
        CompProfile::Numerical => 4,
        CompProfile::General => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feature_name() {
        assert_eq!(FeatureUsage::Closures.name(), "closures");
    }

    #[test]
    fn test_feature_all() {
        assert_eq!(FeatureUsage::all().len(), 12);
    }

    #[test]
    fn test_profile_name() {
        assert_eq!(CompProfile::Functional.name(), "functional");
    }

    #[test]
    fn test_compiler_new() {
        let c = AdaptiveCompiler::new();
        assert_eq!(c.total_compilations, 0);
        assert_eq!(c.profile, CompProfile::General);
    }

    #[test]
    fn test_record_usage() {
        let mut c = AdaptiveCompiler::new();
        c.record_usage(FeatureUsage::Closures, 10);
        assert_eq!(*c.usage_counts.get(&FeatureUsage::Closures).unwrap(), 10);
    }

    #[test]
    fn test_update_profile_functional() {
        let mut c = AdaptiveCompiler::new();
        c.record_usage(FeatureUsage::Closures, 100);
        c.record_usage(FeatureUsage::Pipes, 50);
        c.record_usage(FeatureUsage::Iterators, 80);
        c.update_profile();
        assert_eq!(c.profile, CompProfile::Functional);
    }

    #[test]
    fn test_update_profile_oop() {
        let mut c = AdaptiveCompiler::new();
        c.record_usage(FeatureUsage::Traits, 100);
        c.record_usage(FeatureUsage::StructLiterals, 100);
        c.record_usage(FeatureUsage::EnumVariants, 50);
        c.update_profile();
        assert_eq!(c.profile, CompProfile::ObjectOriented);
    }

    #[test]
    fn test_hot_paths() {
        let mut c = AdaptiveCompiler::new();
        c.record_hot_path("main");
        c.record_hot_path("main");
        c.record_hot_path("helper");
        let top = c.top_hot_paths(1);
        assert_eq!(top[0].0, "main");
        assert_eq!(top[0].1, 2);
    }

    #[test]
    fn test_diversity_zero() {
        let c = AdaptiveCompiler::new();
        assert!((c.diversity_score() - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_diversity_one_feature() {
        let mut c = AdaptiveCompiler::new();
        c.record_usage(FeatureUsage::Functions, 100);
        assert!((c.diversity_score() - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_diversity_multiple() {
        let mut c = AdaptiveCompiler::new();
        c.record_usage(FeatureUsage::Functions, 10);
        c.record_usage(FeatureUsage::Closures, 10);
        c.record_usage(FeatureUsage::Traits, 10);
        assert!(c.diversity_score() > 0.9); // Nearly uniform
    }

    #[test]
    fn test_profile_history() {
        let mut c = AdaptiveCompiler::new();
        c.record_usage(FeatureUsage::AsyncAwait, 100);
        c.update_profile();
        assert_eq!(c.profile, CompProfile::Concurrent);
        assert_eq!(c.profile_history.len(), 1);
    }

    #[test]
    fn test_ffi_record() {
        let n = slang_adaptive_record(0);
        assert!(n >= 1);
    }

    #[test]
    fn test_ffi_compile() {
        let n = slang_adaptive_compile();
        assert!(n >= 1);
    }

    #[test]
    fn test_ffi_profile() {
        let p = slang_adaptive_profile();
        assert!(p >= 0 && p <= 4);
    }
}
