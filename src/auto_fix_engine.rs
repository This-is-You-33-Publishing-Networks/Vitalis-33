//! Auto Fix Engine — Vitalis v654
//!
//! Automatic code repair via pattern-based and search-based strategies:
//! - Fix templates for common bug patterns
//! - Mutation-based repair (GenProg-inspired)
//! - Fix validation via test suite re-execution model
//! - Patch ranking by minimality and safety
//! - Multi-edit composite fix synthesis

use std::collections::HashMap;

// ── Fix Templates ────────────────────────────────────────────────────

/// Category of automatic fix.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FixKind {
    OffByOne,
    NullCheck,
    BoundsCheck,
    TypeCast,
    MissingReturn,
    SwappedArgs,
    WrongOperator,
    MissingBreak,
    ResourceClose,
    InitBeforeUse,
    LockGuard,
    ErrorPropagation,
}

impl FixKind {
    pub fn description(&self) -> &'static str {
        match self {
            Self::OffByOne => "Adjust loop bound by ±1",
            Self::NullCheck => "Add null/None check before dereference",
            Self::BoundsCheck => "Add bounds check before array access",
            Self::TypeCast => "Insert explicit type conversion",
            Self::MissingReturn => "Add missing return statement",
            Self::SwappedArgs => "Swap argument order in function call",
            Self::WrongOperator => "Replace comparison/arithmetic operator",
            Self::MissingBreak => "Add break statement to loop/match",
            Self::ResourceClose => "Add resource cleanup in all exit paths",
            Self::InitBeforeUse => "Move initialization before first use",
            Self::LockGuard => "Replace manual lock/unlock with guard pattern",
            Self::ErrorPropagation => "Add error propagation (? operator)",
        }
    }

    pub fn confidence(&self) -> f64 {
        match self {
            Self::OffByOne => 0.75,
            Self::NullCheck => 0.90,
            Self::BoundsCheck => 0.85,
            Self::TypeCast => 0.70,
            Self::MissingReturn => 0.95,
            Self::SwappedArgs => 0.60,
            Self::WrongOperator => 0.65,
            Self::MissingBreak => 0.80,
            Self::ResourceClose => 0.85,
            Self::InitBeforeUse => 0.90,
            Self::LockGuard => 0.80,
            Self::ErrorPropagation => 0.88,
        }
    }
}

// ── Patch Representation ─────────────────────────────────────────────

/// A single edit operation.
#[derive(Debug, Clone)]
pub struct Edit {
    pub location: u32,     // Line number
    pub kind: EditOp,
}

#[derive(Debug, Clone)]
pub enum EditOp {
    Replace { old: String, new: String },
    InsertBefore(String),
    InsertAfter(String),
    Delete,
}

/// A complete patch (set of edits).
#[derive(Debug, Clone)]
pub struct Patch {
    pub fix_kind: FixKind,
    pub edits: Vec<Edit>,
    pub confidence: f64,
    pub passes_tests: Option<bool>,
}

impl Patch {
    pub fn new(fix_kind: FixKind, edits: Vec<Edit>) -> Self {
        Self { fix_kind, edits, confidence: fix_kind.confidence(), passes_tests: None }
    }

    pub fn edit_count(&self) -> usize { self.edits.len() }

    /// Minimality score: fewer edits = higher score.
    pub fn minimality_score(&self) -> f64 {
        1.0 / (self.edits.len() as f64 + 1.0)
    }

    /// Overall patch score combining confidence and minimality.
    pub fn score(&self) -> f64 {
        let test_bonus = match self.passes_tests {
            Some(true) => 0.3,
            Some(false) => -0.5,
            None => 0.0,
        };
        (self.confidence * 0.6 + self.minimality_score() * 0.4 + test_bonus).clamp(0.0, 1.0)
    }
}

// ── Mutation Operators ───────────────────────────────────────────────

/// GenProg-style mutation operator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MutationOp {
    InsertStatement,
    DeleteStatement,
    SwapStatements,
    ReplaceCondition,
    ChangeBoundary,    // < → <=, > → >=
    NegateCondition,
    AddGuardClause,
}

impl MutationOp {
    /// Probability of producing a valid fix.
    pub fn success_probability(&self) -> f64 {
        match self {
            Self::InsertStatement => 0.15,
            Self::DeleteStatement => 0.10,
            Self::SwapStatements => 0.08,
            Self::ReplaceCondition => 0.25,
            Self::ChangeBoundary => 0.30,
            Self::NegateCondition => 0.20,
            Self::AddGuardClause => 0.35,
        }
    }
}

// ── Auto Fix Engine ──────────────────────────────────────────────────

/// The main auto-fix engine.
pub struct AutoFixEngine {
    patches: Vec<Patch>,
    fix_history: HashMap<FixKind, u32>,
}

impl AutoFixEngine {
    pub fn new() -> Self {
        Self { patches: Vec::new(), fix_history: HashMap::new() }
    }

    /// Generate a candidate fix.
    pub fn generate_fix(&mut self, kind: FixKind, edits: Vec<Edit>) -> usize {
        let patch = Patch::new(kind, edits);
        let idx = self.patches.len();
        self.patches.push(patch);
        *self.fix_history.entry(kind).or_insert(0) += 1;
        idx
    }

    /// Mark a patch as passing or failing tests.
    pub fn set_test_result(&mut self, patch_idx: usize, passes: bool) {
        if let Some(p) = self.patches.get_mut(patch_idx) {
            p.passes_tests = Some(passes);
        }
    }

    /// Rank all patches by score.
    pub fn rank_patches(&self) -> Vec<(usize, f64)> {
        let mut ranked: Vec<(usize, f64)> = self.patches.iter().enumerate()
            .map(|(i, p)| (i, p.score()))
            .collect();
        ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        ranked
    }

    /// Get the best patch that passes tests.
    pub fn best_passing_patch(&self) -> Option<&Patch> {
        let ranked = self.rank_patches();
        ranked.iter()
            .filter_map(|&(i, _)| {
                let p = &self.patches[i];
                if p.passes_tests == Some(true) { Some(p) } else { None }
            })
            .next()
    }

    pub fn patch_count(&self) -> usize { self.patches.len() }

    /// Success rate across all tested patches.
    pub fn success_rate(&self) -> f64 {
        let tested: Vec<&Patch> = self.patches.iter()
            .filter(|p| p.passes_tests.is_some())
            .collect();
        if tested.is_empty() { return 0.0; }
        let passing = tested.iter().filter(|p| p.passes_tests == Some(true)).count();
        passing as f64 / tested.len() as f64
    }
}

impl Default for AutoFixEngine {
    fn default() -> Self { Self::new() }
}

// ── FFI Exports ──────────────────────────────────────────────────────

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_autofix_confidence(fix_kind_id: i64) -> f64 {
    let kind = match fix_kind_id {
        0 => FixKind::OffByOne, 1 => FixKind::NullCheck, 2 => FixKind::BoundsCheck,
        3 => FixKind::TypeCast, 4 => FixKind::MissingReturn, 5 => FixKind::SwappedArgs,
        6 => FixKind::WrongOperator, 7 => FixKind::MissingBreak, 8 => FixKind::ResourceClose,
        9 => FixKind::InitBeforeUse, 10 => FixKind::LockGuard, _ => FixKind::ErrorPropagation,
    };
    kind.confidence()
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_autofix_patch_score(confidence: f64, edit_count: i64, passes: i64) -> f64 {
    let minimality = 1.0 / (edit_count.max(1) as f64 + 1.0);
    let test_bonus = match passes {
        1 => 0.3,
        0 => -0.5,
        _ => 0.0,
    };
    (confidence * 0.6 + minimality * 0.4 + test_bonus).clamp(0.0, 1.0)
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_autofix_mutation_prob(op_id: i64) -> f64 {
    let op = match op_id {
        0 => MutationOp::InsertStatement, 1 => MutationOp::DeleteStatement,
        2 => MutationOp::SwapStatements, 3 => MutationOp::ReplaceCondition,
        4 => MutationOp::ChangeBoundary, 5 => MutationOp::NegateCondition,
        _ => MutationOp::AddGuardClause,
    };
    op.success_probability()
}

// ═══════════════════════════════════════════════════════════════════════
// Tests
// ═══════════════════════════════════════════════════════════════════════
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fix_kind_confidence() {
        assert!(FixKind::MissingReturn.confidence() > FixKind::SwappedArgs.confidence());
    }

    #[test]
    fn test_fix_kind_description() {
        assert!(!FixKind::OffByOne.description().is_empty());
        assert!(!FixKind::NullCheck.description().is_empty());
    }

    #[test]
    fn test_patch_minimality() {
        let p1 = Patch::new(FixKind::OffByOne, vec![Edit { location: 1, kind: EditOp::Delete }]);
        let p2 = Patch::new(FixKind::OffByOne, vec![
            Edit { location: 1, kind: EditOp::Delete },
            Edit { location: 2, kind: EditOp::Delete },
            Edit { location: 3, kind: EditOp::Delete },
        ]);
        assert!(p1.minimality_score() > p2.minimality_score());
    }

    #[test]
    fn test_patch_score_with_tests() {
        let mut p = Patch::new(FixKind::NullCheck, vec![Edit { location: 1, kind: EditOp::Delete }]);
        let score_before = p.score();
        p.passes_tests = Some(true);
        assert!(p.score() > score_before);
    }

    #[test]
    fn test_patch_score_failing_tests() {
        let mut p = Patch::new(FixKind::NullCheck, vec![Edit { location: 1, kind: EditOp::Delete }]);
        p.passes_tests = Some(false);
        let p2 = Patch::new(FixKind::NullCheck, vec![Edit { location: 1, kind: EditOp::Delete }]);
        assert!(p.score() < p2.score());
    }

    #[test]
    fn test_engine_generate_fix() {
        let mut engine = AutoFixEngine::new();
        let idx = engine.generate_fix(FixKind::OffByOne, vec![
            Edit { location: 10, kind: EditOp::Replace { old: "<".into(), new: "<=".into() } },
        ]);
        assert_eq!(idx, 0);
        assert_eq!(engine.patch_count(), 1);
    }

    #[test]
    fn test_engine_rank() {
        let mut engine = AutoFixEngine::new();
        engine.generate_fix(FixKind::SwappedArgs, vec![Edit { location: 1, kind: EditOp::Delete }]);
        engine.generate_fix(FixKind::MissingReturn, vec![Edit { location: 2, kind: EditOp::Delete }]);
        let ranked = engine.rank_patches();
        assert_eq!(ranked.len(), 2);
        assert!(ranked[0].1 >= ranked[1].1);
    }

    #[test]
    fn test_engine_best_passing() {
        let mut engine = AutoFixEngine::new();
        engine.generate_fix(FixKind::NullCheck, vec![Edit { location: 1, kind: EditOp::Delete }]);
        engine.set_test_result(0, true);
        assert!(engine.best_passing_patch().is_some());
    }

    #[test]
    fn test_engine_success_rate() {
        let mut engine = AutoFixEngine::new();
        engine.generate_fix(FixKind::OffByOne, vec![Edit { location: 1, kind: EditOp::Delete }]);
        engine.generate_fix(FixKind::NullCheck, vec![Edit { location: 2, kind: EditOp::Delete }]);
        engine.set_test_result(0, true);
        engine.set_test_result(1, false);
        assert!((engine.success_rate() - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_mutation_op_probability() {
        assert!(MutationOp::AddGuardClause.success_probability() > MutationOp::SwapStatements.success_probability());
    }

    #[test]
    fn test_ffi_confidence() {
        assert!(vitalis_autofix_confidence(1) > 0.8); // NullCheck
        assert!(vitalis_autofix_confidence(5) < 0.7); // SwappedArgs
    }

    #[test]
    fn test_ffi_patch_score() {
        let passing = vitalis_autofix_patch_score(0.9, 1, 1);
        let failing = vitalis_autofix_patch_score(0.9, 1, 0);
        assert!(passing > failing);
    }

    #[test]
    fn test_ffi_mutation_prob() {
        assert!(vitalis_autofix_mutation_prob(6) > 0.0);
    }

    #[test]
    fn test_edit_replace() {
        let edit = Edit { location: 5, kind: EditOp::Replace { old: "foo".into(), new: "bar".into() } };
        assert_eq!(edit.location, 5);
        if let EditOp::Replace { ref old, ref new } = edit.kind {
            assert_eq!(old, "foo");
            assert_eq!(new, "bar");
        }
    }
}
