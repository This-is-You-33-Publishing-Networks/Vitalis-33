//! Concept Extraction Engine — Vitalis v602
//!
//! Extracts high-level programming concepts from code structure:
//! - Pattern detection (retry loops, backoff, caching, state machines)
//! - Idiom recognition (builder pattern, observer, strategy, etc.)
//! - Algorithmic intent (sorting, searching, traversal, accumulation)
//! - Concurrency patterns (producer-consumer, fan-out, pipeline)
//! - Error handling patterns (guard clauses, result chains, recovery)
//! - Feature vector generation for ML-based code understanding

use std::collections::HashMap;

// ── Concept Types ────────────────────────────────────────────────────

/// A recognized high-level concept in code.
#[derive(Debug, Clone, PartialEq)]
pub struct Concept {
    pub kind: ConceptKind,
    pub confidence: f64,
    pub span_start: usize,
    pub span_end: usize,
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ConceptKind {
    RetryLoop,
    ExponentialBackoff,
    CachePattern,
    BuilderPattern,
    ObserverPattern,
    StrategyPattern,
    StateMachine,
    ProducerConsumer,
    MapReduce,
    GuardClause,
    ResultChain,
    ResourceAcquisition,  // RAII
    Accumulator,
    SearchAlgorithm,
    SortAlgorithm,
    TreeTraversal,
    GraphTraversal,
    DivideAndConquer,
    Memoization,
    LazyEvaluation,
    Pipeline,
    FanOut,
    Singleton,
    FactoryMethod,
}

impl ConceptKind {
    pub fn label(&self) -> &'static str {
        match self {
            ConceptKind::RetryLoop => "retry_loop",
            ConceptKind::ExponentialBackoff => "exponential_backoff",
            ConceptKind::CachePattern => "cache_pattern",
            ConceptKind::BuilderPattern => "builder_pattern",
            ConceptKind::ObserverPattern => "observer_pattern",
            ConceptKind::StrategyPattern => "strategy_pattern",
            ConceptKind::StateMachine => "state_machine",
            ConceptKind::ProducerConsumer => "producer_consumer",
            ConceptKind::MapReduce => "map_reduce",
            ConceptKind::GuardClause => "guard_clause",
            ConceptKind::ResultChain => "result_chain",
            ConceptKind::ResourceAcquisition => "raii",
            ConceptKind::Accumulator => "accumulator",
            ConceptKind::SearchAlgorithm => "search",
            ConceptKind::SortAlgorithm => "sort",
            ConceptKind::TreeTraversal => "tree_traversal",
            ConceptKind::GraphTraversal => "graph_traversal",
            ConceptKind::DivideAndConquer => "divide_and_conquer",
            ConceptKind::Memoization => "memoization",
            ConceptKind::LazyEvaluation => "lazy_eval",
            ConceptKind::Pipeline => "pipeline",
            ConceptKind::FanOut => "fan_out",
            ConceptKind::Singleton => "singleton",
            ConceptKind::FactoryMethod => "factory_method",
        }
    }
}

// ── Code Feature Vector ──────────────────────────────────────────────

/// Quantitative features extracted from a code block for ML analysis.
#[derive(Debug, Clone)]
pub struct CodeFeatures {
    pub loop_count: usize,
    pub branch_count: usize,
    pub call_count: usize,
    pub recursion_depth: usize,
    pub variable_count: usize,
    pub mutation_count: usize,
    pub return_count: usize,
    pub error_handling_count: usize,
    pub closure_count: usize,
    pub async_count: usize,
    pub match_arms: usize,
    pub nesting_depth: usize,
    pub line_count: usize,
    pub cyclomatic_complexity: usize,
}

impl CodeFeatures {
    pub fn new() -> Self {
        Self {
            loop_count: 0, branch_count: 0, call_count: 0,
            recursion_depth: 0, variable_count: 0, mutation_count: 0,
            return_count: 0, error_handling_count: 0, closure_count: 0,
            async_count: 0, match_arms: 0, nesting_depth: 0,
            line_count: 0, cyclomatic_complexity: 0,
        }
    }

    /// Convert to a fixed-size f64 vector for ML consumption.
    pub fn to_vector(&self) -> Vec<f64> {
        vec![
            self.loop_count as f64,
            self.branch_count as f64,
            self.call_count as f64,
            self.recursion_depth as f64,
            self.variable_count as f64,
            self.mutation_count as f64,
            self.return_count as f64,
            self.error_handling_count as f64,
            self.closure_count as f64,
            self.async_count as f64,
            self.match_arms as f64,
            self.nesting_depth as f64,
            self.line_count as f64,
            self.cyclomatic_complexity as f64,
        ]
    }

    /// Cosine similarity between two feature vectors.
    pub fn similarity(&self, other: &CodeFeatures) -> f64 {
        let a = self.to_vector();
        let b = other.to_vector();
        let dot: f64 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let mag_a: f64 = a.iter().map(|x| x * x).sum::<f64>().sqrt();
        let mag_b: f64 = b.iter().map(|x| x * x).sum::<f64>().sqrt();
        if mag_a < 1e-15 || mag_b < 1e-15 { return 0.0; }
        (dot / (mag_a * mag_b)).clamp(0.0, 1.0)
    }
}

impl Default for CodeFeatures {
    fn default() -> Self { Self::new() }
}

// ── Concept Detector ─────────────────────────────────────────────────

/// Pattern-based concept detector operating on structural features.
pub struct ConceptDetector {
    rules: Vec<DetectionRule>,
}

struct DetectionRule {
    kind: ConceptKind,
    detector: Box<dyn Fn(&CodeFeatures, &[String]) -> Option<(f64, Vec<String>)>>,
}

impl ConceptDetector {
    pub fn new() -> Self {
        let mut rules = Vec::new();

        // Retry loop: loop with error handling and bounded iteration
        rules.push(DetectionRule {
            kind: ConceptKind::RetryLoop,
            detector: Box::new(|features, tokens| {
                let has_loop = features.loop_count > 0;
                let has_error = features.error_handling_count > 0;
                let has_counter = tokens.iter().any(|t| t.contains("retry") || t.contains("attempt") || t.contains("tries"));
                if has_loop && has_error {
                    let conf = if has_counter { 0.95 } else { 0.6 };
                    Some((conf, vec!["loop+error_handling".into()]))
                } else {
                    None
                }
            }),
        });

        // Exponential backoff: retry + multiplication/shift in sleep/delay
        rules.push(DetectionRule {
            kind: ConceptKind::ExponentialBackoff,
            detector: Box::new(|features, tokens| {
                let has_loop = features.loop_count > 0;
                let has_delay = tokens.iter().any(|t| t.contains("sleep") || t.contains("delay") || t.contains("wait"));
                let has_multiply = tokens.iter().any(|t| t.contains("*") || t.contains("<<") || t.contains("backoff"));
                if has_loop && has_delay && has_multiply {
                    Some((0.9, vec!["loop+delay+multiply".into()]))
                } else {
                    None
                }
            }),
        });

        // Cache pattern: HashMap + lookup before compute
        rules.push(DetectionRule {
            kind: ConceptKind::CachePattern,
            detector: Box::new(|features, tokens| {
                let has_map = tokens.iter().any(|t| t.contains("cache") || t.contains("memo") || t.contains("HashMap"));
                let has_lookup = tokens.iter().any(|t| t.contains("get") || t.contains("contains") || t.contains("lookup"));
                if has_map && has_lookup && features.branch_count > 0 {
                    Some((0.85, vec!["map+lookup+branch".into()]))
                } else {
                    None
                }
            }),
        });

        // Guard clause: early return/continue at start of function
        rules.push(DetectionRule {
            kind: ConceptKind::GuardClause,
            detector: Box::new(|features, tokens| {
                let has_early_return = features.return_count > 1;
                let has_check = features.branch_count > 0 && features.nesting_depth <= 2;
                let has_guard_indicator = tokens.iter().any(|t| {
                    t.contains("return") || t.contains("continue") || t.contains("break")
                });
                if has_early_return && has_check && has_guard_indicator {
                    Some((0.8, vec!["early_return+shallow_branch".into()]))
                } else {
                    None
                }
            }),
        });

        // Accumulator: loop with running total
        rules.push(DetectionRule {
            kind: ConceptKind::Accumulator,
            detector: Box::new(|features, tokens| {
                let has_loop = features.loop_count > 0;
                let has_mutation = features.mutation_count > 0;
                let has_accum = tokens.iter().any(|t| {
                    t.contains("sum") || t.contains("total") || t.contains("count") || t.contains("+=") || t.contains("acc")
                });
                if has_loop && has_mutation && has_accum {
                    Some((0.85, vec!["loop+mutation+accumulator_var".into()]))
                } else {
                    None
                }
            }),
        });

        // State machine: match on enum/state with transitions
        rules.push(DetectionRule {
            kind: ConceptKind::StateMachine,
            detector: Box::new(|features, tokens| {
                let has_match = features.match_arms >= 3;
                let has_loop = features.loop_count > 0;
                let has_state = tokens.iter().any(|t| t.contains("state") || t.contains("State") || t.contains("transition"));
                if has_match && has_loop && has_state {
                    Some((0.9, vec!["match_arms≥3+loop+state_var".into()]))
                } else {
                    None
                }
            }),
        });

        // Memoization: cache + recursive calls
        rules.push(DetectionRule {
            kind: ConceptKind::Memoization,
            detector: Box::new(|features, tokens| {
                let has_cache = tokens.iter().any(|t| t.contains("cache") || t.contains("memo") || t.contains("dp"));
                let has_recursion = features.recursion_depth > 0 || features.call_count > 2;
                if has_cache && has_recursion {
                    Some((0.88, vec!["cache+recursion".into()]))
                } else {
                    None
                }
            }),
        });

        // Pipeline: chain of function calls / pipe operator
        rules.push(DetectionRule {
            kind: ConceptKind::Pipeline,
            detector: Box::new(|features, tokens| {
                let has_chain = features.call_count >= 3;
                let has_pipe = tokens.iter().any(|t| t.contains("|>") || t.contains(".map") || t.contains(".filter") || t.contains("pipe"));
                if has_chain && has_pipe {
                    Some((0.87, vec!["call_chain+pipe_operators".into()]))
                } else {
                    None
                }
            }),
        });

        // Result chain: error propagation
        rules.push(DetectionRule {
            kind: ConceptKind::ResultChain,
            detector: Box::new(|features, tokens| {
                let has_error = features.error_handling_count >= 2;
                let has_question = tokens.iter().any(|t| t.contains("?") || t.contains("try") || t.contains("Result"));
                if has_error && has_question {
                    Some((0.82, vec!["error_handling+propagation".into()]))
                } else {
                    None
                }
            }),
        });

        // Divide and conquer: recursion with split
        rules.push(DetectionRule {
            kind: ConceptKind::DivideAndConquer,
            detector: Box::new(|features, tokens| {
                let has_recursion = features.recursion_depth > 0;
                let has_split = tokens.iter().any(|t| {
                    t.contains("mid") || t.contains("split") || t.contains("half") || t.contains("/ 2") || t.contains(">> 1")
                });
                if has_recursion && has_split && features.branch_count > 0 {
                    Some((0.85, vec!["recursion+split+branch".into()]))
                } else {
                    None
                }
            }),
        });

        Self { rules }
    }

    /// Detect concepts from features and token hints.
    pub fn detect(&self, features: &CodeFeatures, tokens: &[String]) -> Vec<Concept> {
        let mut concepts = Vec::new();
        for rule in &self.rules {
            if let Some((confidence, evidence)) = (rule.detector)(features, tokens) {
                concepts.push(Concept {
                    kind: rule.kind.clone(),
                    confidence,
                    span_start: 0,
                    span_end: 0,
                    evidence,
                });
            }
        }
        // Sort by confidence descending
        concepts.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap_or(std::cmp::Ordering::Equal));
        concepts
    }
}

impl Default for ConceptDetector {
    fn default() -> Self { Self::new() }
}

// ── Concept Frequency Analysis ───────────────────────────────────────

/// Track concept frequencies across a codebase.
pub struct ConceptFrequencyTracker {
    counts: HashMap<ConceptKind, usize>,
    total: usize,
}

impl ConceptFrequencyTracker {
    pub fn new() -> Self {
        Self { counts: HashMap::new(), total: 0 }
    }

    pub fn record(&mut self, concept: &Concept) {
        *self.counts.entry(concept.kind.clone()).or_insert(0) += 1;
        self.total += 1;
    }

    pub fn frequency(&self, kind: &ConceptKind) -> f64 {
        if self.total == 0 { return 0.0; }
        *self.counts.get(kind).unwrap_or(&0) as f64 / self.total as f64
    }

    pub fn most_common(&self, n: usize) -> Vec<(ConceptKind, usize)> {
        let mut items: Vec<_> = self.counts.iter().map(|(k, &v)| (k.clone(), v)).collect();
        items.sort_by(|a, b| b.1.cmp(&a.1));
        items.truncate(n);
        items
    }

    pub fn total_concepts(&self) -> usize { self.total }
    pub fn unique_concepts(&self) -> usize { self.counts.len() }
}

impl Default for ConceptFrequencyTracker {
    fn default() -> Self { Self::new() }
}

// ── FFI Exports ──────────────────────────────────────────────────────

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_concept_detect_retry(loop_count: i64, error_handling: i64, has_retry_token: i64) -> f64 {
    if loop_count > 0 && error_handling > 0 {
        if has_retry_token != 0 { 0.95 } else { 0.6 }
    } else {
        0.0
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_concept_detect_accumulator(loop_count: i64, mutation_count: i64, has_sum_token: i64) -> f64 {
    if loop_count > 0 && mutation_count > 0 && has_sum_token != 0 { 0.85 } else { 0.0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_concept_detect_state_machine(match_arms: i64, loop_count: i64, has_state_token: i64) -> f64 {
    if match_arms >= 3 && loop_count > 0 && has_state_token != 0 { 0.9 } else { 0.0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_concept_feature_similarity(
    a: *const f64, b: *const f64, dim: i64,
) -> f64 {
    if a.is_null() || b.is_null() || dim <= 0 { return 0.0; }
    let a = unsafe { std::slice::from_raw_parts(a, dim as usize) };
    let b = unsafe { std::slice::from_raw_parts(b, dim as usize) };
    let dot: f64 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let mag_a: f64 = a.iter().map(|x| x * x).sum::<f64>().sqrt();
    let mag_b: f64 = b.iter().map(|x| x * x).sum::<f64>().sqrt();
    if mag_a < 1e-15 || mag_b < 1e-15 { 0.0 } else { (dot / (mag_a * mag_b)).clamp(0.0, 1.0) }
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_concept_frequency(occurrences: i64, total: i64) -> f64 {
    if total <= 0 { 0.0 } else { occurrences as f64 / total as f64 }
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_concept_count_unique(flags: *const i64, count: i64) -> i64 {
    if flags.is_null() || count <= 0 { return 0; }
    let f = unsafe { std::slice::from_raw_parts(flags, count as usize) };
    f.iter().filter(|&&x| x > 0).count() as i64
}

// ═══════════════════════════════════════════════════════════════════════
// Tests
// ═══════════════════════════════════════════════════════════════════════
#[cfg(test)]
mod tests {
    use super::*;

    // ── Code Features ────────────────────────────────────────────────

    #[test]
    fn test_code_features_default() {
        let f = CodeFeatures::new();
        assert_eq!(f.loop_count, 0);
        assert_eq!(f.to_vector().len(), 14);
    }

    #[test]
    fn test_code_features_vector() {
        let mut f = CodeFeatures::new();
        f.loop_count = 3;
        f.branch_count = 2;
        f.call_count = 5;
        let v = f.to_vector();
        assert_eq!(v[0], 3.0);
        assert_eq!(v[1], 2.0);
        assert_eq!(v[2], 5.0);
    }

    #[test]
    fn test_feature_similarity_identical() {
        let mut f = CodeFeatures::new();
        f.loop_count = 2;
        f.branch_count = 3;
        assert!((f.similarity(&f) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_feature_similarity_orthogonal() {
        let mut a = CodeFeatures::new();
        a.loop_count = 1;
        let mut b = CodeFeatures::new();
        b.branch_count = 1;
        let sim = a.similarity(&b);
        assert!(sim < 0.01);
    }

    #[test]
    fn test_feature_similarity_zero_vectors() {
        let a = CodeFeatures::new();
        let b = CodeFeatures::new();
        assert_eq!(a.similarity(&b), 0.0);
    }

    // ── Concept Detection ────────────────────────────────────────────

    #[test]
    fn test_detect_retry_loop() {
        let detector = ConceptDetector::new();
        let mut f = CodeFeatures::new();
        f.loop_count = 1;
        f.error_handling_count = 1;
        let tokens = vec!["retry".to_string(), "attempt".to_string()];
        let concepts = detector.detect(&f, &tokens);
        assert!(concepts.iter().any(|c| c.kind == ConceptKind::RetryLoop));
        let retry = concepts.iter().find(|c| c.kind == ConceptKind::RetryLoop).unwrap();
        assert!(retry.confidence >= 0.9);
    }

    #[test]
    fn test_detect_accumulator() {
        let detector = ConceptDetector::new();
        let mut f = CodeFeatures::new();
        f.loop_count = 1;
        f.mutation_count = 2;
        let tokens = vec!["sum".to_string(), "+=".to_string()];
        let concepts = detector.detect(&f, &tokens);
        assert!(concepts.iter().any(|c| c.kind == ConceptKind::Accumulator));
    }

    #[test]
    fn test_detect_state_machine() {
        let detector = ConceptDetector::new();
        let mut f = CodeFeatures::new();
        f.match_arms = 5;
        f.loop_count = 1;
        let tokens = vec!["state".to_string(), "transition".to_string()];
        let concepts = detector.detect(&f, &tokens);
        assert!(concepts.iter().any(|c| c.kind == ConceptKind::StateMachine));
    }

    #[test]
    fn test_detect_guard_clause() {
        let detector = ConceptDetector::new();
        let mut f = CodeFeatures::new();
        f.return_count = 3;
        f.branch_count = 2;
        f.nesting_depth = 1;
        let tokens = vec!["return".to_string()];
        let concepts = detector.detect(&f, &tokens);
        assert!(concepts.iter().any(|c| c.kind == ConceptKind::GuardClause));
    }

    #[test]
    fn test_detect_memoization() {
        let detector = ConceptDetector::new();
        let mut f = CodeFeatures::new();
        f.recursion_depth = 2;
        f.call_count = 4;
        let tokens = vec!["memo".to_string(), "cache".to_string()];
        let concepts = detector.detect(&f, &tokens);
        assert!(concepts.iter().any(|c| c.kind == ConceptKind::Memoization));
    }

    #[test]
    fn test_detect_pipeline() {
        let detector = ConceptDetector::new();
        let mut f = CodeFeatures::new();
        f.call_count = 5;
        let tokens = vec!["|>".to_string(), ".map".to_string(), ".filter".to_string()];
        let concepts = detector.detect(&f, &tokens);
        assert!(concepts.iter().any(|c| c.kind == ConceptKind::Pipeline));
    }

    #[test]
    fn test_detect_divide_and_conquer() {
        let detector = ConceptDetector::new();
        let mut f = CodeFeatures::new();
        f.recursion_depth = 3;
        f.branch_count = 1;
        let tokens = vec!["mid".to_string(), "/ 2".to_string()];
        let concepts = detector.detect(&f, &tokens);
        assert!(concepts.iter().any(|c| c.kind == ConceptKind::DivideAndConquer));
    }

    #[test]
    fn test_no_false_positives_empty() {
        let detector = ConceptDetector::new();
        let f = CodeFeatures::new();
        let concepts = detector.detect(&f, &[]);
        assert!(concepts.is_empty());
    }

    #[test]
    fn test_concepts_sorted_by_confidence() {
        let detector = ConceptDetector::new();
        let mut f = CodeFeatures::new();
        f.loop_count = 1;
        f.error_handling_count = 2;
        f.mutation_count = 1;
        f.return_count = 3;
        f.branch_count = 1;
        f.nesting_depth = 1;
        let tokens = vec!["retry".to_string(), "sum".to_string(), "return".to_string(), "?".to_string()];
        let concepts = detector.detect(&f, &tokens);
        for w in concepts.windows(2) {
            assert!(w[0].confidence >= w[1].confidence);
        }
    }

    // ── Frequency Tracker ────────────────────────────────────────────

    #[test]
    fn test_frequency_tracker() {
        let mut tracker = ConceptFrequencyTracker::new();
        tracker.record(&Concept { kind: ConceptKind::RetryLoop, confidence: 0.9, span_start: 0, span_end: 10, evidence: vec![] });
        tracker.record(&Concept { kind: ConceptKind::RetryLoop, confidence: 0.8, span_start: 20, span_end: 30, evidence: vec![] });
        tracker.record(&Concept { kind: ConceptKind::Accumulator, confidence: 0.7, span_start: 40, span_end: 50, evidence: vec![] });
        assert_eq!(tracker.total_concepts(), 3);
        assert_eq!(tracker.unique_concepts(), 2);
        assert!((tracker.frequency(&ConceptKind::RetryLoop) - 2.0 / 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_most_common() {
        let mut tracker = ConceptFrequencyTracker::new();
        for _ in 0..5 { tracker.record(&Concept { kind: ConceptKind::GuardClause, confidence: 0.8, span_start: 0, span_end: 1, evidence: vec![] }); }
        for _ in 0..3 { tracker.record(&Concept { kind: ConceptKind::Accumulator, confidence: 0.7, span_start: 0, span_end: 1, evidence: vec![] }); }
        let top = tracker.most_common(1);
        assert_eq!(top[0].0, ConceptKind::GuardClause);
        assert_eq!(top[0].1, 5);
    }

    // ── Concept Labels ───────────────────────────────────────────────

    #[test]
    fn test_concept_labels() {
        assert_eq!(ConceptKind::RetryLoop.label(), "retry_loop");
        assert_eq!(ConceptKind::ExponentialBackoff.label(), "exponential_backoff");
        assert_eq!(ConceptKind::Memoization.label(), "memoization");
        assert_eq!(ConceptKind::Pipeline.label(), "pipeline");
    }

    // ── FFI Smoke Tests ──────────────────────────────────────────────

    #[test]
    fn test_ffi_detect_retry() {
        assert!(vitalis_concept_detect_retry(1, 1, 1) > 0.9);
        assert_eq!(vitalis_concept_detect_retry(0, 1, 1), 0.0);
    }

    #[test]
    fn test_ffi_detect_accumulator() {
        assert!(vitalis_concept_detect_accumulator(1, 1, 1) > 0.8);
        assert_eq!(vitalis_concept_detect_accumulator(0, 0, 0), 0.0);
    }

    #[test]
    fn test_ffi_feature_similarity() {
        let a = [1.0, 0.0, 0.0];
        let b = [1.0, 0.0, 0.0];
        let sim = vitalis_concept_feature_similarity(a.as_ptr(), b.as_ptr(), 3);
        assert!((sim - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_ffi_frequency() {
        assert!((vitalis_concept_frequency(3, 10) - 0.3).abs() < 1e-10);
        assert_eq!(vitalis_concept_frequency(1, 0), 0.0);
    }

    #[test]
    fn test_ffi_count_unique() {
        let flags = [1i64, 0, 1, 1, 0, 0, 1];
        assert_eq!(vitalis_concept_count_unique(flags.as_ptr(), 7), 4);
    }
}
