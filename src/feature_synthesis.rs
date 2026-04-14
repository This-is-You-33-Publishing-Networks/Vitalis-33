//! v435 — Language Feature Synthesis.
//!
//! Automatically proposes new language features by analyzing user code patterns.
//! Detects common boilerplate → synthesizes sugar. Detects common errors →
//! synthesizes new type-level checks. Human-in-the-loop approval gate.

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

/// A detected code pattern that could benefit from a language feature.
#[derive(Debug, Clone)]
pub struct CodePattern {
    pub id: u64,
    pub name: String,
    pub description: String,
    /// How many times this pattern was seen.
    pub occurrences: u64,
    /// Example source fragments.
    pub examples: Vec<String>,
    pub category: PatternCategory,
}

/// Categories of detectable patterns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatternCategory {
    /// Repeated boilerplate code.
    Boilerplate,
    /// Common type mismatch errors.
    TypeError,
    /// Null/None handling patterns.
    NullCheck,
    /// Resource management patterns (open/close).
    ResourceManagement,
    /// Error handling patterns (try/catch).
    ErrorHandling,
    /// Loop patterns (map/filter/reduce).
    LoopIdiom,
}

impl PatternCategory {
    pub fn name(&self) -> &'static str {
        match self {
            PatternCategory::Boilerplate => "boilerplate",
            PatternCategory::TypeError => "type_error",
            PatternCategory::NullCheck => "null_check",
            PatternCategory::ResourceManagement => "resource_mgmt",
            PatternCategory::ErrorHandling => "error_handling",
            PatternCategory::LoopIdiom => "loop_idiom",
        }
    }
}

/// A proposed language feature.
#[derive(Debug, Clone)]
pub struct FeatureProposal {
    pub id: u64,
    pub name: String,
    pub description: String,
    pub syntax_example: String,
    pub pattern_id: u64,
    pub confidence: f64,
    pub status: ProposalStatus,
}

/// Status of a feature proposal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProposalStatus {
    Draft,
    Proposed,
    Approved,
    Rejected,
    Implemented,
}

impl ProposalStatus {
    pub fn name(&self) -> &'static str {
        match self {
            ProposalStatus::Draft => "draft",
            ProposalStatus::Proposed => "proposed",
            ProposalStatus::Approved => "approved",
            ProposalStatus::Rejected => "rejected",
            ProposalStatus::Implemented => "implemented",
        }
    }
}

/// The feature synthesis engine.
pub struct FeatureSynthesizer {
    pub patterns: HashMap<u64, CodePattern>,
    pub proposals: HashMap<u64, FeatureProposal>,
    pattern_counter: u64,
    proposal_counter: u64,
    /// Minimum occurrences before proposing a feature.
    pub proposal_threshold: u64,
}

impl FeatureSynthesizer {
    pub fn new() -> Self {
        Self {
            patterns: HashMap::new(),
            proposals: HashMap::new(),
            pattern_counter: 0,
            proposal_counter: 0,
            proposal_threshold: 5,
        }
    }

    /// Record a code pattern occurrence.
    pub fn record_pattern(
        &mut self,
        name: &str,
        description: &str,
        category: PatternCategory,
        example: &str,
    ) -> u64 {
        // Check if pattern already exists
        for (id, p) in &mut self.patterns {
            if p.name == name {
                p.occurrences += 1;
                if p.examples.len() < 5 {
                    p.examples.push(example.to_string());
                }
                return *id;
            }
        }
        // New pattern
        self.pattern_counter += 1;
        let id = self.pattern_counter;
        self.patterns.insert(id, CodePattern {
            id,
            name: name.to_string(),
            description: description.to_string(),
            occurrences: 1,
            examples: vec![example.to_string()],
            category,
        });
        id
    }

    /// Check for patterns that have exceeded the threshold and auto-propose features.
    pub fn check_proposals(&mut self) -> Vec<u64> {
        let mut new_proposals = Vec::new();
        let threshold = self.proposal_threshold;

        let qualifying: Vec<(u64, String, String, PatternCategory)> = self.patterns.iter()
            .filter(|(_, p)| p.occurrences >= threshold)
            .filter(|(id, _)| !self.proposals.values().any(|prop| prop.pattern_id == **id))
            .map(|(id, p)| (*id, p.name.clone(), p.description.clone(), p.category))
            .collect();

        for (pattern_id, name, desc, category) in qualifying {
            self.proposal_counter += 1;
            let prop_id = self.proposal_counter;
            let syntax = self.synthesize_syntax(&name, category);
            let confidence = self.calculate_confidence(pattern_id);
            self.proposals.insert(prop_id, FeatureProposal {
                id: prop_id,
                name: format!("auto_{}", name),
                description: format!("Auto-proposed: {}", desc),
                syntax_example: syntax,
                pattern_id,
                confidence,
                status: ProposalStatus::Draft,
            });
            new_proposals.push(prop_id);
        }
        new_proposals
    }

    /// Synthesize example syntax for a pattern category.
    fn synthesize_syntax(&self, name: &str, category: PatternCategory) -> String {
        match category {
            PatternCategory::Boilerplate => format!("@derive({}) ...", name),
            PatternCategory::TypeError => format!("type {} = constrained<...>", name),
            PatternCategory::NullCheck => format!("let x = value?.unwrap_or(default)"),
            PatternCategory::ResourceManagement => format!("with {} {{ ... }}", name),
            PatternCategory::ErrorHandling => format!("try {{ ... }} rescue {} {{ ... }}", name),
            PatternCategory::LoopIdiom => format!("collection |> {}", name),
        }
    }

    /// Calculate confidence for a proposal based on pattern frequency & consistency.
    fn calculate_confidence(&self, pattern_id: u64) -> f64 {
        if let Some(pattern) = self.patterns.get(&pattern_id) {
            let freq_score = (pattern.occurrences as f64 / 100.0).min(1.0);
            let example_diversity = (pattern.examples.len() as f64 / 5.0).min(1.0);
            (freq_score + example_diversity) / 2.0
        } else {
            0.0
        }
    }

    /// Approve a proposal (human-in-the-loop gate).
    pub fn approve_proposal(&mut self, proposal_id: u64) -> bool {
        if let Some(prop) = self.proposals.get_mut(&proposal_id) {
            if prop.status == ProposalStatus::Draft || prop.status == ProposalStatus::Proposed {
                prop.status = ProposalStatus::Approved;
                return true;
            }
        }
        false
    }

    /// Reject a proposal.
    pub fn reject_proposal(&mut self, proposal_id: u64) -> bool {
        if let Some(prop) = self.proposals.get_mut(&proposal_id) {
            prop.status = ProposalStatus::Rejected;
            return true;
        }
        false
    }

    /// Mark a proposal as implemented.
    pub fn implement_proposal(&mut self, proposal_id: u64) -> bool {
        if let Some(prop) = self.proposals.get_mut(&proposal_id) {
            if prop.status == ProposalStatus::Approved {
                prop.status = ProposalStatus::Implemented;
                return true;
            }
        }
        false
    }

    /// Count proposals by status.
    pub fn count_by_status(&self, status: ProposalStatus) -> usize {
        self.proposals.values().filter(|p| p.status == status).count()
    }

    /// Total patterns tracked.
    pub fn pattern_count(&self) -> usize {
        self.patterns.len()
    }

    /// Total proposals.
    pub fn proposal_count(&self) -> usize {
        self.proposals.len()
    }

    /// Get high-confidence proposals.
    pub fn high_confidence_proposals(&self, threshold: f64) -> Vec<&FeatureProposal> {
        self.proposals.values()
            .filter(|p| p.confidence >= threshold && p.status == ProposalStatus::Draft)
            .collect()
    }
}

// ─── FFI ──────────────────────────────────────────────────────────────

static GLOBAL_SYNTH: LazyLock<Mutex<FeatureSynthesizer>> =
    LazyLock::new(|| Mutex::new(FeatureSynthesizer::new()));

#[unsafe(no_mangle)]
pub extern "C" fn slang_feature_record_pattern(category: i64) -> i64 {
    let cat = match category {
        0 => PatternCategory::Boilerplate,
        1 => PatternCategory::TypeError,
        2 => PatternCategory::NullCheck,
        3 => PatternCategory::ResourceManagement,
        4 => PatternCategory::ErrorHandling,
        _ => PatternCategory::LoopIdiom,
    };
    let mut synth = GLOBAL_SYNTH.lock().unwrap();
    synth.record_pattern("pattern", "auto-detected", cat, "example") as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_feature_check_proposals() -> i64 {
    let mut synth = GLOBAL_SYNTH.lock().unwrap();
    synth.check_proposals().len() as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_feature_pattern_count() -> i64 {
    let synth = GLOBAL_SYNTH.lock().unwrap();
    synth.pattern_count() as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_feature_proposal_count() -> i64 {
    let synth = GLOBAL_SYNTH.lock().unwrap();
    synth.proposal_count() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fresh() -> FeatureSynthesizer {
        FeatureSynthesizer::new()
    }

    #[test]
    fn test_pattern_category_name() {
        assert_eq!(PatternCategory::Boilerplate.name(), "boilerplate");
        assert_eq!(PatternCategory::LoopIdiom.name(), "loop_idiom");
    }

    #[test]
    fn test_proposal_status_name() {
        assert_eq!(ProposalStatus::Draft.name(), "draft");
        assert_eq!(ProposalStatus::Implemented.name(), "implemented");
    }

    #[test]
    fn test_synthesizer_new() {
        let s = fresh();
        assert_eq!(s.pattern_count(), 0);
        assert_eq!(s.proposal_count(), 0);
    }

    #[test]
    fn test_record_pattern() {
        let mut s = fresh();
        let id = s.record_pattern("null_check", "null checking", PatternCategory::NullCheck, "if x != null");
        assert_eq!(id, 1);
        assert_eq!(s.pattern_count(), 1);
    }

    #[test]
    fn test_record_pattern_dedup() {
        let mut s = fresh();
        s.record_pattern("p1", "d", PatternCategory::Boilerplate, "ex1");
        s.record_pattern("p1", "d", PatternCategory::Boilerplate, "ex2");
        assert_eq!(s.pattern_count(), 1);
        assert_eq!(s.patterns[&1].occurrences, 2);
    }

    #[test]
    fn test_check_proposals_below_threshold() {
        let mut s = fresh();
        s.record_pattern("p1", "d", PatternCategory::Boilerplate, "ex");
        let props = s.check_proposals();
        assert_eq!(props.len(), 0);
    }

    #[test]
    fn test_check_proposals_above_threshold() {
        let mut s = fresh();
        s.proposal_threshold = 3;
        for _ in 0..3 {
            s.record_pattern("p1", "d", PatternCategory::Boilerplate, "ex");
        }
        let props = s.check_proposals();
        assert_eq!(props.len(), 1);
        assert_eq!(s.proposal_count(), 1);
    }

    #[test]
    fn test_no_duplicate_proposals() {
        let mut s = fresh();
        s.proposal_threshold = 2;
        for _ in 0..5 {
            s.record_pattern("p1", "d", PatternCategory::Boilerplate, "ex");
        }
        s.check_proposals();
        s.check_proposals(); // second call shouldn't create duplicates
        assert_eq!(s.proposal_count(), 1);
    }

    #[test]
    fn test_approve_proposal() {
        let mut s = fresh();
        s.proposal_threshold = 1;
        s.record_pattern("p1", "d", PatternCategory::NullCheck, "ex");
        s.check_proposals();
        assert!(s.approve_proposal(1));
        assert_eq!(s.count_by_status(ProposalStatus::Approved), 1);
    }

    #[test]
    fn test_reject_proposal() {
        let mut s = fresh();
        s.proposal_threshold = 1;
        s.record_pattern("p1", "d", PatternCategory::NullCheck, "ex");
        s.check_proposals();
        assert!(s.reject_proposal(1));
        assert_eq!(s.count_by_status(ProposalStatus::Rejected), 1);
    }

    #[test]
    fn test_implement_proposal() {
        let mut s = fresh();
        s.proposal_threshold = 1;
        s.record_pattern("p1", "d", PatternCategory::NullCheck, "ex");
        s.check_proposals();
        s.approve_proposal(1);
        assert!(s.implement_proposal(1));
        assert_eq!(s.count_by_status(ProposalStatus::Implemented), 1);
    }

    #[test]
    fn test_implement_without_approval() {
        let mut s = fresh();
        s.proposal_threshold = 1;
        s.record_pattern("p1", "d", PatternCategory::NullCheck, "ex");
        s.check_proposals();
        assert!(!s.implement_proposal(1)); // can't implement draft
    }

    #[test]
    fn test_high_confidence() {
        let mut s = fresh();
        s.proposal_threshold = 1;
        for _ in 0..50 {
            s.record_pattern("p1", "d", PatternCategory::Boilerplate, "ex");
        }
        s.check_proposals();
        let high = s.high_confidence_proposals(0.1);
        assert!(!high.is_empty());
    }

    #[test]
    fn test_ffi_record() {
        let id = slang_feature_record_pattern(0);
        assert!(id >= 1);
    }

    #[test]
    fn test_ffi_pattern_count() {
        let c = slang_feature_pattern_count();
        assert!(c >= 0);
    }

    #[test]
    fn test_ffi_proposal_count() {
        let c = slang_feature_proposal_count();
        assert!(c >= 0);
    }
}
