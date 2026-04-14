//! Evolution Safety Rails — policy enforcement, violation detection, resource budgets,
//! sandbox constraints, and rollback management for the self-evolution pipeline.
//!
//! # Architecture
//!
//! ```text
//! engine.rs (evolve)
//!     │
//!     ├── pre_mutation_check()   → Policy gate (before mutation)
//!     ├── validate_mutation()    → Structural safety (after mutation, before compile)
//!     ├── post_compile_check()   → Budget & resource enforcement (after compile)
//!     └── on_violation()         → Violation log + auto-rollback trigger
//!
//! SafetyGovernor
//!     ├── MutationPolicy         → Allowed files, kinds, edit budget
//!     ├── ResourceBudget         → CPU time, memory, code size limits
//!     ├── StructuralConstraints  → Type-safety, effect-safety invariants
//!     ├── ViolationLog           → Bounded ring buffer of detected violations
//!     └── SnapshotChain          → Ordered rollback checkpoints
//! ```
//!
//! # Safety Philosophy
//!
//! Evolution CANNOT:
//! - Remove type checking or safety features
//! - Modify core compiler pipeline modules (lexer, parser, codegen)
//! - Exceed resource budgets (CPU, memory, code size)
//! - Produce code that fails type checking
//! - Remove existing tests
//!
//! Evolution CAN:
//! - Mutate `@evolvable` function bodies
//! - Reorder and combine optimization passes
//! - Adjust numeric constants and operator choices
//! - Add new test cases

use std::collections::BTreeSet;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicI64, Ordering};

// ─── Global violation counter (for FFI) ──────────────────────────────────

static TOTAL_VIOLATIONS: AtomicI64 = AtomicI64::new(0);
static TOTAL_ROLLBACKS: AtomicI64 = AtomicI64::new(0);
static TOTAL_BLOCKED: AtomicI64 = AtomicI64::new(0);

// ─── Core Types ──────────────────────────────────────────────────────────

/// A single violation detected by the safety rails.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Violation {
    pub kind: ViolationKind,
    pub severity: Severity,
    pub message: String,
    pub function_name: String,
    pub cycle: u64,
}

/// Classification of safety violations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViolationKind {
    /// Mutation targets a forbidden file/module.
    ForbiddenTarget,
    /// Edit count exceeds policy budget.
    BudgetExceeded,
    /// Mutation kind not permitted by policy.
    DisallowedMutationKind,
    /// Compiled output exceeds code size limit.
    CodeSizeExceeded,
    /// Compilation time exceeds CPU budget.
    CpuTimeExceeded,
    /// Fitness regression beyond threshold.
    FitnessRegression,
    /// Mutation would remove or weaken type safety checks.
    TypeSafetyErosion,
    /// Mutation would remove existing test cases.
    TestRemoval,
    /// Mutation affects structural invariants (e.g., entry point).
    StructuralInvariantBroken,
    /// Resource limit (memory) exceeded during evaluation.
    MemoryExceeded,
}

/// Severity levels for violations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    /// Informational — logged but not blocking.
    Info,
    /// Warning — logged and counted, evolution continues.
    Warning,
    /// Error — mutation is rejected and rolled back.
    Error,
    /// Critical — mutation rejected, evolution paused until review.
    Critical,
}

/// What action the safety governor took in response.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SafetyAction {
    /// Mutation allowed to proceed.
    Allow,
    /// Mutation blocked before compilation.
    Block,
    /// Mutation rolled back after compilation/evaluation.
    Rollback,
    /// Evolution paused entirely — requires manual review.
    Pause,
}

// ─── Mutation Policy ─────────────────────────────────────────────────────

/// Policy governing what mutations are allowed.
#[derive(Debug, Clone)]
pub struct MutationPolicy {
    /// Files/modules that CANNOT be mutated (e.g., "src/codegen.rs").
    pub forbidden_paths: BTreeSet<String>,
    /// Maximum number of edits per evolution cycle.
    pub max_edits: usize,
    /// Maximum number of edits per single function.
    pub max_edits_per_function: usize,
    /// Allowed mutation kinds (if empty, all kinds are allowed).
    pub allowed_mutation_kinds: BTreeSet<String>,
    /// Minimum fitness threshold — reject if fitness drops below this.
    pub min_fitness_threshold: f64,
    /// Maximum fitness regression percentage before auto-rollback.
    pub max_regression_pct: f64,
    /// Whether removing test functions is permitted.
    pub allow_test_removal: bool,
}

impl Default for MutationPolicy {
    fn default() -> Self {
        let mut forbidden = BTreeSet::new();
        // Core compiler pipeline is NEVER mutable by evolution
        for path in &[
            "src/lexer.rs", "src/parser.rs", "src/ast.rs", "src/types.rs",
            "src/ir.rs", "src/codegen.rs", "src/aot.rs", "src/main.rs", "src/lib.rs",
        ] {
            forbidden.insert(path.to_string());
        }
        Self {
            forbidden_paths: forbidden,
            max_edits: 50,
            max_edits_per_function: 10,
            allowed_mutation_kinds: BTreeSet::new(), // empty = all allowed
            min_fitness_threshold: 0.0,
            max_regression_pct: 20.0,
            allow_test_removal: false,
        }
    }
}

impl MutationPolicy {
    /// Create a restrictive policy for production environments.
    pub fn strict() -> Self {
        let mut policy = Self::default();
        policy.max_edits = 10;
        policy.max_edits_per_function = 3;
        policy.max_regression_pct = 5.0;
        policy
    }

    /// Create a permissive policy for experimentation.
    pub fn permissive() -> Self {
        Self {
            forbidden_paths: BTreeSet::new(),
            max_edits: 500,
            max_edits_per_function: 100,
            allowed_mutation_kinds: BTreeSet::new(),
            min_fitness_threshold: 0.0,
            max_regression_pct: 50.0,
            allow_test_removal: false,
        }
    }
}

// ─── Resource Budget ─────────────────────────────────────────────────────

/// Resource limits for a single evolution cycle.
#[derive(Debug, Clone)]
pub struct ResourceBudget {
    /// Maximum CPU time (ms) for compilation + evaluation per mutation.
    pub max_compile_ms: f64,
    /// Maximum memory (bytes) the evolved code may allocate.
    pub max_memory_bytes: usize,
    /// Maximum compiled code size (bytes).
    pub max_code_size_bytes: usize,
    /// Maximum total mutations per cycle.
    pub max_mutations_per_cycle: usize,
    /// Maximum consecutive failures before pausing.
    pub max_consecutive_failures: usize,
}

impl Default for ResourceBudget {
    fn default() -> Self {
        Self {
            max_compile_ms: 5000.0,
            max_memory_bytes: 256 * 1024 * 1024, // 256 MB
            max_code_size_bytes: 1024 * 1024,     // 1 MB
            max_mutations_per_cycle: 100,
            max_consecutive_failures: 10,
        }
    }
}

// ─── Structural Constraints ──────────────────────────────────────────────

/// Invariants that must hold after any mutation.
#[derive(Debug, Clone)]
pub struct StructuralConstraints {
    /// Functions that must exist (never deleted by evolution).
    pub required_functions: BTreeSet<String>,
    /// Minimum test count — evolution cannot reduce below this.
    pub min_test_count: usize,
    /// Patterns that must NOT appear in evolved code (e.g., `unsafe`).
    pub forbidden_patterns: Vec<String>,
    /// Patterns that MUST appear in evolved code (e.g., type annotations).
    pub required_patterns: Vec<String>,
}

impl Default for StructuralConstraints {
    fn default() -> Self {
        Self {
            required_functions: BTreeSet::new(),
            min_test_count: 0,
            forbidden_patterns: vec![
                "std::process::Command".to_string(),
                "std::fs::remove".to_string(),
                "std::env::set_var".to_string(),
            ],
            required_patterns: Vec::new(),
        }
    }
}

// ─── Snapshot Chain ──────────────────────────────────────────────────────

/// A snapshot of function state before mutation, enabling rollback.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snapshot {
    /// Unique revision identifier.
    pub revision: String,
    /// Cycle number when snapshot was taken.
    pub cycle: u64,
    /// Function name this snapshot covers.
    pub function_name: String,
    /// Source code of the function before mutation.
    pub source_before: String,
    /// Fitness score before mutation.
    pub fitness_before_x1000: i64,
    /// Files involved in the mutation.
    pub changed_files: Vec<String>,
}

/// Ordered chain of snapshots for rollback history.
#[derive(Debug, Clone)]
pub struct SnapshotChain {
    /// Maximum number of snapshots to retain.
    max_snapshots: usize,
    /// Ordered snapshots (newest at back).
    snapshots: VecDeque<Snapshot>,
}

impl SnapshotChain {
    pub fn new(max_snapshots: usize) -> Self {
        Self {
            max_snapshots: max_snapshots.max(1),
            snapshots: VecDeque::with_capacity(max_snapshots.min(1024)),
        }
    }

    /// Push a new snapshot. Evicts oldest if at capacity.
    pub fn push(&mut self, snapshot: Snapshot) {
        if self.snapshots.len() >= self.max_snapshots {
            self.snapshots.pop_front();
        }
        self.snapshots.push_back(snapshot);
    }

    /// Get the most recent snapshot for a function.
    pub fn latest_for(&self, function_name: &str) -> Option<&Snapshot> {
        self.snapshots.iter().rev().find(|s| s.function_name == function_name)
    }

    /// Get all snapshots for a function, newest first.
    pub fn history_for(&self, function_name: &str) -> Vec<&Snapshot> {
        self.snapshots.iter().rev().filter(|s| s.function_name == function_name).collect()
    }

    /// Number of snapshots in the chain.
    pub fn len(&self) -> usize {
        self.snapshots.len()
    }

    /// Whether the chain is empty.
    pub fn is_empty(&self) -> bool {
        self.snapshots.is_empty()
    }

    /// Clear all snapshots.
    pub fn clear(&mut self) {
        self.snapshots.clear();
    }
}

// ─── Safety Governor ─────────────────────────────────────────────────────

/// The central safety enforcement engine for evolution.
///
/// Integrates policy, budget, constraints, violation tracking, and snapshots
/// into a single entry point that the evolution engine calls at each stage.
pub struct SafetyGovernor {
    pub policy: MutationPolicy,
    pub budget: ResourceBudget,
    pub constraints: StructuralConstraints,
    pub snapshots: SnapshotChain,
    violations: VecDeque<Violation>,
    max_violations: usize,
    consecutive_failures: usize,
    total_mutations_this_cycle: usize,
    cycle: u64,
    paused: bool,
}

impl SafetyGovernor {
    /// Create a governor with default policy, budget, and constraints.
    pub fn new() -> Self {
        Self {
            policy: MutationPolicy::default(),
            budget: ResourceBudget::default(),
            constraints: StructuralConstraints::default(),
            snapshots: SnapshotChain::new(200),
            violations: VecDeque::with_capacity(200),
            max_violations: 500,
            consecutive_failures: 0,
            total_mutations_this_cycle: 0,
            cycle: 0,
            paused: false,
        }
    }

    /// Create a governor with custom policy.
    pub fn with_policy(policy: MutationPolicy) -> Self {
        let mut gov = Self::new();
        gov.policy = policy;
        gov
    }

    /// Create a governor with strict production settings.
    pub fn strict() -> Self {
        Self::with_policy(MutationPolicy::strict())
    }

    // ─── Pre-Mutation Gate ───────────────────────────────────────────

    /// Check whether a mutation should be allowed BEFORE it is applied.
    /// Returns `SafetyAction::Allow` if the mutation may proceed.
    pub fn pre_mutation_check(
        &mut self,
        function_name: &str,
        target_path: &str,
        mutation_kind: &str,
        edit_count: usize,
    ) -> SafetyAction {
        if self.paused {
            return SafetyAction::Pause;
        }

        // Check cycle mutation budget
        if self.total_mutations_this_cycle >= self.budget.max_mutations_per_cycle {
            self.record_violation(Violation {
                kind: ViolationKind::BudgetExceeded,
                severity: Severity::Error,
                message: format!(
                    "Cycle mutation limit reached ({}/{})",
                    self.total_mutations_this_cycle, self.budget.max_mutations_per_cycle
                ),
                function_name: function_name.to_string(),
                cycle: self.cycle,
            });
            return SafetyAction::Block;
        }

        // Check forbidden paths
        if self.policy.forbidden_paths.contains(target_path) {
            self.record_violation(Violation {
                kind: ViolationKind::ForbiddenTarget,
                severity: Severity::Error,
                message: format!("Target path '{}' is forbidden by policy", target_path),
                function_name: function_name.to_string(),
                cycle: self.cycle,
            });
            TOTAL_BLOCKED.fetch_add(1, Ordering::Relaxed);
            return SafetyAction::Block;
        }

        // Check edit budget per function
        if edit_count > self.policy.max_edits_per_function {
            self.record_violation(Violation {
                kind: ViolationKind::BudgetExceeded,
                severity: Severity::Error,
                message: format!(
                    "Edit count {} exceeds per-function limit {}",
                    edit_count, self.policy.max_edits_per_function
                ),
                function_name: function_name.to_string(),
                cycle: self.cycle,
            });
            return SafetyAction::Block;
        }

        // Check allowed mutation kinds (empty = all allowed)
        if !self.policy.allowed_mutation_kinds.is_empty()
            && !self.policy.allowed_mutation_kinds.contains(mutation_kind)
        {
            self.record_violation(Violation {
                kind: ViolationKind::DisallowedMutationKind,
                severity: Severity::Warning,
                message: format!("Mutation kind '{}' not in allowed set", mutation_kind),
                function_name: function_name.to_string(),
                cycle: self.cycle,
            });
            return SafetyAction::Block;
        }

        self.total_mutations_this_cycle += 1;
        SafetyAction::Allow
    }

    // ─── Post-Mutation Validation ────────────────────────────────────

    /// Validate mutated source code AFTER mutation but BEFORE compilation.
    /// Checks structural constraints (forbidden patterns, required functions, etc.).
    pub fn validate_mutation(
        &mut self,
        function_name: &str,
        mutated_source: &str,
        original_source: &str,
    ) -> SafetyAction {
        // Check for forbidden patterns in mutated code
        for pattern in &self.constraints.forbidden_patterns {
            if mutated_source.contains(pattern.as_str()) {
                self.record_violation(Violation {
                    kind: ViolationKind::StructuralInvariantBroken,
                    severity: Severity::Critical,
                    message: format!("Forbidden pattern '{}' found in mutated code", pattern),
                    function_name: function_name.to_string(),
                    cycle: self.cycle,
                });
                return SafetyAction::Block;
            }
        }

        // Check required patterns are still present
        for pattern in &self.constraints.required_patterns {
            if !mutated_source.contains(pattern.as_str()) {
                self.record_violation(Violation {
                    kind: ViolationKind::StructuralInvariantBroken,
                    severity: Severity::Error,
                    message: format!("Required pattern '{}' missing from mutated code", pattern),
                    function_name: function_name.to_string(),
                    cycle: self.cycle,
                });
                return SafetyAction::Block;
            }
        }

        // Check test removal
        if !self.policy.allow_test_removal {
            let orig_test_count = count_test_annotations(original_source);
            let new_test_count = count_test_annotations(mutated_source);
            if new_test_count < orig_test_count {
                self.record_violation(Violation {
                    kind: ViolationKind::TestRemoval,
                    severity: Severity::Critical,
                    message: format!(
                        "Mutation removed {} test(s) ({} → {})",
                        orig_test_count - new_test_count, orig_test_count, new_test_count
                    ),
                    function_name: function_name.to_string(),
                    cycle: self.cycle,
                });
                return SafetyAction::Block;
            }
        }

        // Check that required functions still exist (simple heuristic)
        for req_fn in &self.constraints.required_functions {
            let sig = format!("fn {}", req_fn);
            if original_source.contains(&sig) && !mutated_source.contains(&sig) {
                self.record_violation(Violation {
                    kind: ViolationKind::TypeSafetyErosion,
                    severity: Severity::Critical,
                    message: format!("Required function '{}' was removed by mutation", req_fn),
                    function_name: function_name.to_string(),
                    cycle: self.cycle,
                });
                return SafetyAction::Block;
            }
        }

        SafetyAction::Allow
    }

    // ─── Post-Compile Enforcement ────────────────────────────────────

    /// Enforce resource budgets AFTER compilation.
    pub fn post_compile_check(
        &mut self,
        function_name: &str,
        compile_time_ms: f64,
        code_size_bytes: usize,
    ) -> SafetyAction {
        if compile_time_ms > self.budget.max_compile_ms {
            self.record_violation(Violation {
                kind: ViolationKind::CpuTimeExceeded,
                severity: Severity::Error,
                message: format!(
                    "Compile time {:.1}ms exceeds budget {:.1}ms",
                    compile_time_ms, self.budget.max_compile_ms
                ),
                function_name: function_name.to_string(),
                cycle: self.cycle,
            });
            return SafetyAction::Rollback;
        }

        if code_size_bytes > self.budget.max_code_size_bytes {
            self.record_violation(Violation {
                kind: ViolationKind::CodeSizeExceeded,
                severity: Severity::Error,
                message: format!(
                    "Code size {} bytes exceeds limit {} bytes",
                    code_size_bytes, self.budget.max_code_size_bytes
                ),
                function_name: function_name.to_string(),
                cycle: self.cycle,
            });
            return SafetyAction::Rollback;
        }

        SafetyAction::Allow
    }

    // ─── Fitness Gate ────────────────────────────────────────────────

    /// Check fitness after evaluation. Triggers rollback if regression too large.
    pub fn fitness_check(
        &mut self,
        function_name: &str,
        prev_fitness: f64,
        new_fitness: f64,
    ) -> SafetyAction {
        // Absolute minimum threshold
        if new_fitness < self.policy.min_fitness_threshold {
            self.record_violation(Violation {
                kind: ViolationKind::FitnessRegression,
                severity: Severity::Error,
                message: format!(
                    "Fitness {:.4} below minimum threshold {:.4}",
                    new_fitness, self.policy.min_fitness_threshold
                ),
                function_name: function_name.to_string(),
                cycle: self.cycle,
            });
            self.on_failure();
            return SafetyAction::Rollback;
        }

        // Regression percentage check
        if prev_fitness > 0.0 {
            let regression_pct = ((prev_fitness - new_fitness) / prev_fitness) * 100.0;
            if regression_pct > self.policy.max_regression_pct {
                self.record_violation(Violation {
                    kind: ViolationKind::FitnessRegression,
                    severity: Severity::Error,
                    message: format!(
                        "Fitness regressed {:.1}% ({:.4} → {:.4}), max allowed {:.1}%",
                        regression_pct, prev_fitness, new_fitness, self.policy.max_regression_pct
                    ),
                    function_name: function_name.to_string(),
                    cycle: self.cycle,
                });
                self.on_failure();
                return SafetyAction::Rollback;
            }
        }

        self.consecutive_failures = 0;
        SafetyAction::Allow
    }

    // ─── Snapshot Management ─────────────────────────────────────────

    /// Take a snapshot before mutation for potential rollback.
    pub fn take_snapshot(
        &mut self,
        function_name: &str,
        source: &str,
        fitness: f64,
        changed_files: &[String],
    ) {
        let revision = format!("c{}_f{}", self.cycle, function_name);
        self.snapshots.push(Snapshot {
            revision,
            cycle: self.cycle,
            function_name: function_name.to_string(),
            source_before: source.to_string(),
            fitness_before_x1000: (fitness * 1000.0) as i64,
            changed_files: {
                let mut files = changed_files.to_vec();
                files.sort();
                files
            },
        });
    }

    /// Get the source code to rollback to for a function.
    pub fn get_rollback_source(&self, function_name: &str) -> Option<&str> {
        self.snapshots.latest_for(function_name).map(|s| s.source_before.as_str())
    }

    /// Check if rollback is possible (all changed files still exist).
    pub fn rollback_possible(&self, function_name: &str, current_files: &[String]) -> bool {
        match self.snapshots.latest_for(function_name) {
            Some(snap) => {
                let cur: BTreeSet<_> = current_files.iter().cloned().collect();
                snap.changed_files.iter().all(|f| cur.contains(f))
            }
            None => false,
        }
    }

    // ─── Cycle Management ────────────────────────────────────────────

    /// Start a new evolution cycle. Resets per-cycle counters.
    pub fn begin_cycle(&mut self) {
        self.cycle += 1;
        self.total_mutations_this_cycle = 0;
    }

    /// End the current cycle. Returns summary stats.
    pub fn end_cycle(&self) -> CycleSafetySummary {
        CycleSafetySummary {
            cycle: self.cycle,
            mutations_attempted: self.total_mutations_this_cycle,
            violations_this_session: self.violations.len(),
            consecutive_failures: self.consecutive_failures,
            paused: self.paused,
            snapshots_held: self.snapshots.len(),
        }
    }

    /// Whether evolution is currently paused due to safety concerns.
    pub fn is_paused(&self) -> bool {
        self.paused
    }

    /// Resume evolution after manual review.
    pub fn resume(&mut self) {
        self.paused = false;
        self.consecutive_failures = 0;
    }

    /// Current cycle number.
    pub fn current_cycle(&self) -> u64 {
        self.cycle
    }

    // ─── Violation Access ────────────────────────────────────────────

    /// Get all recorded violations.
    pub fn violations(&self) -> &VecDeque<Violation> {
        &self.violations
    }

    /// Get violations at or above a given severity.
    pub fn violations_at_severity(&self, min_severity: Severity) -> Vec<&Violation> {
        self.violations.iter().filter(|v| v.severity >= min_severity).collect()
    }

    /// Number of violations recorded.
    pub fn violation_count(&self) -> usize {
        self.violations.len()
    }

    /// Clear all recorded violations.
    pub fn clear_violations(&mut self) {
        self.violations.clear();
    }

    // ─── Internal Helpers ────────────────────────────────────────────

    fn record_violation(&mut self, violation: Violation) {
        TOTAL_VIOLATIONS.fetch_add(1, Ordering::Relaxed);
        if self.violations.len() >= self.max_violations {
            self.violations.pop_front();
        }
        self.violations.push_back(violation);
    }

    fn on_failure(&mut self) {
        self.consecutive_failures += 1;
        if self.consecutive_failures >= self.budget.max_consecutive_failures {
            self.paused = true;
        }
    }
}

// ─── Cycle Summary ───────────────────────────────────────────────────────

/// Summary of safety metrics for a completed cycle.
#[derive(Debug, Clone)]
pub struct CycleSafetySummary {
    pub cycle: u64,
    pub mutations_attempted: usize,
    pub violations_this_session: usize,
    pub consecutive_failures: usize,
    pub paused: bool,
    pub snapshots_held: usize,
}

impl CycleSafetySummary {
    pub fn to_json(&self) -> String {
        format!(
            "{{\"cycle\":{},\"mutations\":{},\"violations\":{},\"consecutive_failures\":{},\"paused\":{},\"snapshots\":{}}}",
            self.cycle, self.mutations_attempted, self.violations_this_session,
            self.consecutive_failures, self.paused, self.snapshots_held,
        )
    }
}

// ─── Helper Functions ────────────────────────────────────────────────────

/// Check if a mutation is allowed by policy (standalone convenience function).
pub fn allow_mutation(path: &str, edit_count: usize, policy: &MutationPolicy) -> bool {
    edit_count <= policy.max_edits && !policy.forbidden_paths.contains(path)
}

/// Build a snapshot from revision and file list (legacy compat).
pub fn build_snapshot(revision: &str, files: &[String]) -> Snapshot {
    let mut changed = files.to_vec();
    changed.sort();
    Snapshot {
        revision: revision.to_string(),
        cycle: 0,
        function_name: String::new(),
        source_before: String::new(),
        fitness_before_x1000: 0,
        changed_files: changed,
    }
}

/// Check if rollback is possible given current files (legacy compat).
pub fn rollback_possible(snapshot: &Snapshot, current_files: &[String]) -> bool {
    let cur: BTreeSet<_> = current_files.iter().cloned().collect();
    snapshot.changed_files.iter().all(|f| cur.contains(f))
}

/// Count `#[test]` annotations in source code.
fn count_test_annotations(source: &str) -> usize {
    source.matches("#[test]").count()
}

// ─── FFI Exports ─────────────────────────────────────────────────────────

/// Get total violations recorded across all governors.
#[unsafe(no_mangle)]
pub extern "C" fn slang_safety_total_violations() -> i64 {
    TOTAL_VIOLATIONS.load(Ordering::Relaxed)
}

/// Get total rollbacks triggered.
#[unsafe(no_mangle)]
pub extern "C" fn slang_safety_total_rollbacks() -> i64 {
    TOTAL_ROLLBACKS.load(Ordering::Relaxed)
}

/// Get total mutations blocked.
#[unsafe(no_mangle)]
pub extern "C" fn slang_safety_total_blocked() -> i64 {
    TOTAL_BLOCKED.load(Ordering::Relaxed)
}

/// Check if a mutation path is allowed. Returns 1 if allowed, 0 if forbidden.
#[unsafe(no_mangle)]
pub extern "C" fn slang_safety_check_path(path_ptr: *const u8, path_len: i64) -> i64 {
    if path_ptr.is_null() || path_len <= 0 {
        return 0;
    }
    let len = path_len as usize;
    let slice = unsafe { std::slice::from_raw_parts(path_ptr, len) };
    let path = match std::str::from_utf8(slice) {
        Ok(s) => s,
        Err(_) => return 0,
    };
    let policy = MutationPolicy::default();
    if policy.forbidden_paths.contains(path) { 0 } else { 1 }
}

/// Reset all global safety counters.
#[unsafe(no_mangle)]
pub extern "C" fn slang_safety_reset_counters() -> i64 {
    TOTAL_VIOLATIONS.store(0, Ordering::Relaxed);
    TOTAL_ROLLBACKS.store(0, Ordering::Relaxed);
    TOTAL_BLOCKED.store(0, Ordering::Relaxed);
    1
}

// ─── Tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Policy Tests ─────────────────────────────────────────────────

    #[test]
    fn test_policy_blocks_forbidden() {
        let mut forbidden = BTreeSet::new();
        forbidden.insert("src/codegen.rs".to_string());
        let p = MutationPolicy {
            forbidden_paths: forbidden,
            ..MutationPolicy::default()
        };
        assert!(!allow_mutation("src/codegen.rs", 1, &p));
        assert!(allow_mutation("src/optimizer.rs", 3, &p));
    }

    #[test]
    fn test_policy_blocks_over_budget() {
        let p = MutationPolicy { max_edits: 5, ..MutationPolicy::default() };
        assert!(allow_mutation("src/foo.rs", 5, &p));
        assert!(!allow_mutation("src/foo.rs", 6, &p));
    }

    #[test]
    fn test_default_policy_forbids_core() {
        let p = MutationPolicy::default();
        assert!(!allow_mutation("src/codegen.rs", 1, &p));
        assert!(!allow_mutation("src/parser.rs", 1, &p));
        assert!(!allow_mutation("src/lexer.rs", 1, &p));
        assert!(!allow_mutation("src/types.rs", 1, &p));
        assert!(!allow_mutation("src/ir.rs", 1, &p));
        assert!(!allow_mutation("src/ast.rs", 1, &p));
        assert!(allow_mutation("src/evolution.rs", 1, &p));
    }

    #[test]
    fn test_strict_policy() {
        let p = MutationPolicy::strict();
        assert_eq!(p.max_edits, 10);
        assert_eq!(p.max_edits_per_function, 3);
        assert!((p.max_regression_pct - 5.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_permissive_policy() {
        let p = MutationPolicy::permissive();
        assert_eq!(p.max_edits, 500);
        assert!(p.forbidden_paths.is_empty());
    }

    // ── Snapshot Tests ───────────────────────────────────────────────

    #[test]
    fn test_rollback_snapshot() {
        let snap = build_snapshot("r1", &["a.rs".to_string(), "b.rs".to_string()]);
        assert!(rollback_possible(&snap, &["a.rs".to_string(), "b.rs".to_string(), "c.rs".to_string()]));
        assert!(!rollback_possible(&snap, &["a.rs".to_string()]));
    }

    #[test]
    fn test_snapshot_chain_push_and_latest() {
        let mut chain = SnapshotChain::new(3);
        assert!(chain.is_empty());

        chain.push(Snapshot {
            revision: "r1".into(), cycle: 1, function_name: "foo".into(),
            source_before: "v1".into(), fitness_before_x1000: 500,
            changed_files: vec!["a.rs".into()],
        });
        chain.push(Snapshot {
            revision: "r2".into(), cycle: 2, function_name: "foo".into(),
            source_before: "v2".into(), fitness_before_x1000: 700,
            changed_files: vec!["a.rs".into()],
        });

        assert_eq!(chain.len(), 2);
        let latest = chain.latest_for("foo").unwrap();
        assert_eq!(latest.source_before, "v2");
    }

    #[test]
    fn test_snapshot_chain_eviction() {
        let mut chain = SnapshotChain::new(2);
        for i in 0..5 {
            chain.push(Snapshot {
                revision: format!("r{}", i), cycle: i as u64,
                function_name: "bar".into(), source_before: format!("v{}", i),
                fitness_before_x1000: 100 * i as i64, changed_files: vec![],
            });
        }
        assert_eq!(chain.len(), 2);
        let latest = chain.latest_for("bar").unwrap();
        assert_eq!(latest.source_before, "v4");
    }

    #[test]
    fn test_snapshot_chain_multi_function() {
        let mut chain = SnapshotChain::new(10);
        chain.push(Snapshot {
            revision: "r1".into(), cycle: 1, function_name: "alpha".into(),
            source_before: "a1".into(), fitness_before_x1000: 100, changed_files: vec![],
        });
        chain.push(Snapshot {
            revision: "r2".into(), cycle: 2, function_name: "beta".into(),
            source_before: "b1".into(), fitness_before_x1000: 200, changed_files: vec![],
        });
        chain.push(Snapshot {
            revision: "r3".into(), cycle: 3, function_name: "alpha".into(),
            source_before: "a2".into(), fitness_before_x1000: 300, changed_files: vec![],
        });

        assert_eq!(chain.latest_for("alpha").unwrap().source_before, "a2");
        assert_eq!(chain.latest_for("beta").unwrap().source_before, "b1");
        assert_eq!(chain.history_for("alpha").len(), 2);
    }

    // ── Governor Pre-Mutation Tests ──────────────────────────────────

    #[test]
    fn test_governor_blocks_forbidden_path() {
        let mut gov = SafetyGovernor::new();
        gov.begin_cycle();
        let action = gov.pre_mutation_check("test_fn", "src/codegen.rs", "SwapOperator", 1);
        assert_eq!(action, SafetyAction::Block);
        assert_eq!(gov.violation_count(), 1);
        assert_eq!(gov.violations()[0].kind, ViolationKind::ForbiddenTarget);
    }

    #[test]
    fn test_governor_allows_valid_mutation() {
        let mut gov = SafetyGovernor::new();
        gov.begin_cycle();
        let action = gov.pre_mutation_check("test_fn", "src/evolution.rs", "SwapOperator", 1);
        assert_eq!(action, SafetyAction::Allow);
        assert_eq!(gov.violation_count(), 0);
    }

    #[test]
    fn test_governor_blocks_over_function_budget() {
        let mut gov = SafetyGovernor::new();
        gov.begin_cycle();
        let action = gov.pre_mutation_check("test_fn", "src/foo.rs", "SwapOperator", 999);
        assert_eq!(action, SafetyAction::Block);
        assert_eq!(gov.violations()[0].kind, ViolationKind::BudgetExceeded);
    }

    #[test]
    fn test_governor_blocks_disallowed_kind() {
        let mut gov = SafetyGovernor::new();
        gov.policy.allowed_mutation_kinds.insert("SwapOperator".to_string());
        gov.begin_cycle();
        let action = gov.pre_mutation_check("f", "src/foo.rs", "DeleteStatement", 1);
        assert_eq!(action, SafetyAction::Block);
    }

    #[test]
    fn test_governor_cycle_budget_limit() {
        let mut gov = SafetyGovernor::new();
        gov.budget.max_mutations_per_cycle = 2;
        gov.begin_cycle();
        assert_eq!(gov.pre_mutation_check("f1", "src/a.rs", "X", 1), SafetyAction::Allow);
        assert_eq!(gov.pre_mutation_check("f2", "src/b.rs", "X", 1), SafetyAction::Allow);
        assert_eq!(gov.pre_mutation_check("f3", "src/c.rs", "X", 1), SafetyAction::Block);
    }

    // ── Governor Validate Mutation Tests ─────────────────────────────

    #[test]
    fn test_validate_blocks_forbidden_pattern() {
        let mut gov = SafetyGovernor::new();
        gov.begin_cycle();
        let action = gov.validate_mutation(
            "test_fn",
            "fn test_fn() { std::process::Command::new(\"rm\"); }",
            "fn test_fn() { 42 }",
        );
        assert_eq!(action, SafetyAction::Block);
    }

    #[test]
    fn test_validate_blocks_test_removal() {
        let mut gov = SafetyGovernor::new();
        gov.begin_cycle();
        let original = "#[test]\nfn test_a() {}\n#[test]\nfn test_b() {}";
        let mutated = "#[test]\nfn test_a() {}";
        let action = gov.validate_mutation("f", mutated, original);
        assert_eq!(action, SafetyAction::Block);
        assert_eq!(gov.violations().back().unwrap().kind, ViolationKind::TestRemoval);
    }

    #[test]
    fn test_validate_allows_clean_mutation() {
        let mut gov = SafetyGovernor::new();
        gov.begin_cycle();
        let action = gov.validate_mutation("f", "fn f() { 43 }", "fn f() { 42 }");
        assert_eq!(action, SafetyAction::Allow);
    }

    #[test]
    fn test_validate_blocks_required_function_removal() {
        let mut gov = SafetyGovernor::new();
        gov.constraints.required_functions.insert("main".to_string());
        gov.begin_cycle();
        let action = gov.validate_mutation("main", "fn helper() {}", "fn main() {}");
        assert_eq!(action, SafetyAction::Block);
    }

    // ── Governor Post-Compile Tests ──────────────────────────────────

    #[test]
    fn test_post_compile_allows_within_budget() {
        let mut gov = SafetyGovernor::new();
        gov.begin_cycle();
        let action = gov.post_compile_check("f", 100.0, 1024);
        assert_eq!(action, SafetyAction::Allow);
    }

    #[test]
    fn test_post_compile_rollback_on_cpu_exceeded() {
        let mut gov = SafetyGovernor::new();
        gov.budget.max_compile_ms = 100.0;
        gov.begin_cycle();
        let action = gov.post_compile_check("f", 200.0, 100);
        assert_eq!(action, SafetyAction::Rollback);
    }

    #[test]
    fn test_post_compile_rollback_on_code_size() {
        let mut gov = SafetyGovernor::new();
        gov.budget.max_code_size_bytes = 500;
        gov.begin_cycle();
        let action = gov.post_compile_check("f", 10.0, 1000);
        assert_eq!(action, SafetyAction::Rollback);
    }

    // ── Fitness Gate Tests ───────────────────────────────────────────

    #[test]
    fn test_fitness_allows_improvement() {
        let mut gov = SafetyGovernor::new();
        gov.begin_cycle();
        let action = gov.fitness_check("f", 0.5, 0.8);
        assert_eq!(action, SafetyAction::Allow);
    }

    #[test]
    fn test_fitness_rollback_on_regression() {
        let mut gov = SafetyGovernor::new();
        gov.policy.max_regression_pct = 10.0;
        gov.begin_cycle();
        let action = gov.fitness_check("f", 1.0, 0.5);
        assert_eq!(action, SafetyAction::Rollback);
    }

    #[test]
    fn test_fitness_rollback_below_threshold() {
        let mut gov = SafetyGovernor::new();
        gov.policy.min_fitness_threshold = 0.3;
        gov.begin_cycle();
        let action = gov.fitness_check("f", 0.0, 0.1);
        assert_eq!(action, SafetyAction::Rollback);
    }

    // ── Pause & Resume Tests ─────────────────────────────────────────

    #[test]
    fn test_governor_pauses_on_consecutive_failures() {
        let mut gov = SafetyGovernor::new();
        gov.budget.max_consecutive_failures = 3;
        gov.policy.min_fitness_threshold = 0.5;
        gov.begin_cycle();

        gov.fitness_check("f", 0.0, 0.1);
        assert!(!gov.is_paused());
        gov.fitness_check("f", 0.0, 0.1);
        assert!(!gov.is_paused());
        gov.fitness_check("f", 0.0, 0.1);
        assert!(gov.is_paused());

        // Mutations blocked while paused
        let action = gov.pre_mutation_check("f", "src/foo.rs", "X", 1);
        assert_eq!(action, SafetyAction::Pause);

        // Resume
        gov.resume();
        assert!(!gov.is_paused());
        let action = gov.pre_mutation_check("f", "src/foo.rs", "X", 1);
        assert_eq!(action, SafetyAction::Allow);
    }

    // ── Cycle Summary Tests ──────────────────────────────────────────

    #[test]
    fn test_cycle_begin_end() {
        let mut gov = SafetyGovernor::new();
        gov.begin_cycle();
        gov.pre_mutation_check("f", "src/a.rs", "X", 1);
        let summary = gov.end_cycle();
        assert_eq!(summary.cycle, 1);
        assert_eq!(summary.mutations_attempted, 1);
        assert!(!summary.paused);
    }

    #[test]
    fn test_cycle_reset_counter() {
        let mut gov = SafetyGovernor::new();
        gov.budget.max_mutations_per_cycle = 1;

        gov.begin_cycle();
        assert_eq!(gov.pre_mutation_check("f", "src/a.rs", "X", 1), SafetyAction::Allow);
        assert_eq!(gov.pre_mutation_check("f", "src/b.rs", "X", 1), SafetyAction::Block);

        gov.begin_cycle();
        assert_eq!(gov.pre_mutation_check("f", "src/c.rs", "X", 1), SafetyAction::Allow);
    }

    // ── Snapshot Integration ─────────────────────────────────────────

    #[test]
    fn test_governor_snapshot_rollback() {
        let mut gov = SafetyGovernor::new();
        gov.begin_cycle();
        gov.take_snapshot("optimize", "fn optimize() { a + b }", 0.75, &["src/opt.rs".into()]);

        let src = gov.get_rollback_source("optimize").unwrap();
        assert_eq!(src, "fn optimize() { a + b }");
        assert!(gov.rollback_possible("optimize", &["src/opt.rs".into()]));
        assert!(!gov.rollback_possible("optimize", &["src/other.rs".into()]));
    }

    // ── Severity Filter ──────────────────────────────────────────────

    #[test]
    fn test_violations_at_severity() {
        let mut gov = SafetyGovernor::new();
        gov.begin_cycle();
        // Trigger a warning (disallowed kind)
        gov.policy.allowed_mutation_kinds.insert("X".into());
        gov.pre_mutation_check("f", "src/a.rs", "Y", 1);
        // Trigger an error (forbidden path)
        gov.pre_mutation_check("f", "src/codegen.rs", "X", 1);

        let errors = gov.violations_at_severity(Severity::Error);
        assert_eq!(errors.len(), 1);
        let warnings = gov.violations_at_severity(Severity::Warning);
        assert_eq!(warnings.len(), 2); // both warning and error are >= Warning
    }

    // ── FFI Tests ────────────────────────────────────────────────────

    #[test]
    fn test_ffi_counters() {
        slang_safety_reset_counters();
        assert_eq!(slang_safety_total_violations(), 0);
        assert_eq!(slang_safety_total_rollbacks(), 0);
        assert_eq!(slang_safety_total_blocked(), 0);
    }

    // ── Count Test Annotations ───────────────────────────────────────

    #[test]
    fn test_count_test_annotations() {
        let code = "#[test]\nfn a() {}\n#[test]\nfn b() {}\nfn c() {}";
        assert_eq!(count_test_annotations(code), 2);
        assert_eq!(count_test_annotations("fn main() {}"), 0);
    }

    // ── CycleSafetySummary JSON ──────────────────────────────────────

    #[test]
    fn test_cycle_summary_json() {
        let summary = CycleSafetySummary {
            cycle: 5, mutations_attempted: 3, violations_this_session: 1,
            consecutive_failures: 0, paused: false, snapshots_held: 2,
        };
        let json = summary.to_json();
        assert!(json.contains("\"cycle\":5"));
        assert!(json.contains("\"paused\":false"));
    }
}
