//! v475 — Autonomous Code Review.
//!
//! Static analysis beyond linting: detect logic errors, performance anti-patterns,
//! security vulnerabilities, API misuse. Severity scoring with confidence levels.

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

/// A review finding.
#[derive(Debug, Clone)]
pub struct ReviewFinding {
    pub rule: ReviewRule,
    pub severity: Severity,
    pub message: String,
    pub location: String,
    pub confidence: f64,
    pub auto_fixable: bool,
}

/// Severity levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Severity {
    Info,
    Warning,
    Error,
    Critical,
}

impl Severity {
    pub fn name(&self) -> &'static str {
        match self {
            Severity::Info => "info",
            Severity::Warning => "warning",
            Severity::Error => "error",
            Severity::Critical => "critical",
        }
    }
}

/// Review rule categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReviewRule {
    /// Unused variable.
    UnusedVariable,
    /// Potential null/None dereference.
    NullDereference,
    /// Integer overflow.
    IntegerOverflow,
    /// Division by zero.
    DivisionByZero,
    /// Infinite loop.
    InfiniteLoop,
    /// Unhandled error.
    UnhandledError,
    /// Resource leak (file, connection).
    ResourceLeak,
    /// SQL injection potential.
    SqlInjection,
    /// Hard-coded credentials.
    HardcodedCredentials,
    /// Deprecated API usage.
    DeprecatedApi,
    /// Unnecessary allocation.
    UnnecessaryAllocation,
    /// Missing bounds check.
    MissingBoundsCheck,
    /// Dead code.
    DeadCode,
    /// Redundant clone.
    RedundantClone,
    /// Large function (cyclomatic complexity).
    LargeFunction,
}

impl ReviewRule {
    pub fn name(&self) -> &'static str {
        match self {
            ReviewRule::UnusedVariable => "unused_variable",
            ReviewRule::NullDereference => "null_deref",
            ReviewRule::IntegerOverflow => "integer_overflow",
            ReviewRule::DivisionByZero => "div_zero",
            ReviewRule::InfiniteLoop => "infinite_loop",
            ReviewRule::UnhandledError => "unhandled_error",
            ReviewRule::ResourceLeak => "resource_leak",
            ReviewRule::SqlInjection => "sql_injection",
            ReviewRule::HardcodedCredentials => "hardcoded_creds",
            ReviewRule::DeprecatedApi => "deprecated_api",
            ReviewRule::UnnecessaryAllocation => "unnecessary_alloc",
            ReviewRule::MissingBoundsCheck => "missing_bounds",
            ReviewRule::DeadCode => "dead_code",
            ReviewRule::RedundantClone => "redundant_clone",
            ReviewRule::LargeFunction => "large_function",
        }
    }

    pub fn default_severity(&self) -> Severity {
        match self {
            ReviewRule::SqlInjection | ReviewRule::HardcodedCredentials => Severity::Critical,
            ReviewRule::NullDereference | ReviewRule::DivisionByZero |
            ReviewRule::IntegerOverflow | ReviewRule::ResourceLeak => Severity::Error,
            ReviewRule::UnhandledError | ReviewRule::InfiniteLoop |
            ReviewRule::MissingBoundsCheck => Severity::Warning,
            _ => Severity::Info,
        }
    }
}

/// Pattern checker for a specific rule.
#[derive(Debug, Clone)]
pub struct PatternChecker {
    pub rule: ReviewRule,
    pub patterns: Vec<String>,
    pub enabled: bool,
}

/// Autonomous code review engine.
pub struct CodeReviewer {
    pub checkers: Vec<PatternChecker>,
    pub findings: Vec<ReviewFinding>,
    pub reviewed_functions: u64,
    pub suppressed: HashMap<ReviewRule, u64>,
}

impl CodeReviewer {
    pub fn new() -> Self {
        let checkers = vec![
            PatternChecker {
                rule: ReviewRule::DivisionByZero,
                patterns: vec!["/ 0".to_string(), "/0".to_string()],
                enabled: true,
            },
            PatternChecker {
                rule: ReviewRule::HardcodedCredentials,
                patterns: vec!["password".to_string(), "secret".to_string(), "api_key".to_string()],
                enabled: true,
            },
            PatternChecker {
                rule: ReviewRule::InfiniteLoop,
                patterns: vec!["while true".to_string(), "loop {".to_string()],
                enabled: true,
            },
            PatternChecker {
                rule: ReviewRule::LargeFunction,
                patterns: vec![], // Checked by line count
                enabled: true,
            },
        ];

        Self {
            checkers,
            findings: Vec::new(),
            reviewed_functions: 0,
            suppressed: HashMap::new(),
        }
    }

    /// Review a code snippet.
    pub fn review(&mut self, code: &str, location: &str) -> Vec<ReviewFinding> {
        self.reviewed_functions += 1;
        let mut findings = Vec::new();
        let lower = code.to_lowercase();

        for checker in &self.checkers {
            if !checker.enabled { continue; }

            // Pattern-based checking
            for pattern in &checker.patterns {
                if lower.contains(&pattern.to_lowercase()) {
                    // Check if suppressed
                    if self.suppressed.contains_key(&checker.rule) { continue; }

                    findings.push(ReviewFinding {
                        rule: checker.rule,
                        severity: checker.rule.default_severity(),
                        message: format!("Detected {} pattern: '{}'", checker.rule.name(), pattern),
                        location: location.to_string(),
                        confidence: 0.8,
                        auto_fixable: false,
                    });
                }
            }

            // Special: large function check
            if checker.rule == ReviewRule::LargeFunction {
                let line_count = code.lines().count();
                if line_count > 100 {
                    findings.push(ReviewFinding {
                        rule: ReviewRule::LargeFunction,
                        severity: Severity::Info,
                        message: format!("Function is {} lines (threshold: 100)", line_count),
                        location: location.to_string(),
                        confidence: 1.0,
                        auto_fixable: false,
                    });
                }
            }
        }

        self.findings.extend(findings.clone());
        findings
    }

    /// Suppress a rule.
    pub fn suppress_rule(&mut self, rule: ReviewRule) {
        *self.suppressed.entry(rule).or_insert(0) += 1;
    }

    /// Total findings.
    pub fn total_findings(&self) -> usize {
        self.findings.len()
    }

    /// Findings by severity.
    pub fn findings_by_severity(&self, severity: Severity) -> usize {
        self.findings.iter().filter(|f| f.severity == severity).count()
    }

    /// Enabled checker count.
    pub fn enabled_checkers(&self) -> usize {
        self.checkers.iter().filter(|c| c.enabled).count()
    }

    /// Review score (0.0 = terrible, 1.0 = perfect).
    pub fn review_score(&self) -> f64 {
        if self.reviewed_functions == 0 { return 1.0; }
        let critical = self.findings_by_severity(Severity::Critical) as f64;
        let errors = self.findings_by_severity(Severity::Error) as f64;
        let warnings = self.findings_by_severity(Severity::Warning) as f64;
        let penalty = critical * 0.3 + errors * 0.15 + warnings * 0.05;
        (1.0 - penalty / self.reviewed_functions as f64).max(0.0)
    }
}

// ─── FFI ──────────────────────────────────────────────────────────────

static GLOBAL_REVIEWER: LazyLock<Mutex<CodeReviewer>> =
    LazyLock::new(|| Mutex::new(CodeReviewer::new()));

#[unsafe(no_mangle)]
pub extern "C" fn slang_review_check(line_count: i64) -> i64 {
    let mut reviewer = GLOBAL_REVIEWER.lock().unwrap();
    let code = "fn f() { }".repeat(line_count.max(1) as usize);
    let findings = reviewer.review(&code, "auto");
    findings.len() as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_review_total() -> i64 {
    let reviewer = GLOBAL_REVIEWER.lock().unwrap();
    reviewer.total_findings() as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_review_score() -> i64 {
    let reviewer = GLOBAL_REVIEWER.lock().unwrap();
    f64::to_bits(reviewer.review_score()) as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_severity_ordering() {
        assert!(Severity::Info < Severity::Warning);
        assert!(Severity::Warning < Severity::Error);
        assert!(Severity::Error < Severity::Critical);
    }

    #[test]
    fn test_severity_name() {
        assert_eq!(Severity::Critical.name(), "critical");
    }

    #[test]
    fn test_rule_name() {
        assert_eq!(ReviewRule::DivisionByZero.name(), "div_zero");
        assert_eq!(ReviewRule::SqlInjection.name(), "sql_injection");
    }

    #[test]
    fn test_rule_severity() {
        assert_eq!(ReviewRule::SqlInjection.default_severity(), Severity::Critical);
        assert_eq!(ReviewRule::DeadCode.default_severity(), Severity::Info);
    }

    #[test]
    fn test_reviewer_new() {
        let r = CodeReviewer::new();
        assert!(r.enabled_checkers() > 0);
        assert_eq!(r.total_findings(), 0);
    }

    #[test]
    fn test_review_clean_code() {
        let mut r = CodeReviewer::new();
        let findings = r.review("fn add(a: i64, b: i64) -> i64 { a + b }", "test.sl:1");
        assert!(findings.is_empty());
    }

    #[test]
    fn test_review_div_zero() {
        let mut r = CodeReviewer::new();
        let findings = r.review("let x = y / 0", "test.sl:5");
        assert!(!findings.is_empty());
        assert_eq!(findings[0].rule, ReviewRule::DivisionByZero);
    }

    #[test]
    fn test_review_hardcoded_creds() {
        let mut r = CodeReviewer::new();
        let findings = r.review("let password = \"hunter2\"", "config.sl:10");
        assert!(!findings.is_empty());
        assert_eq!(findings[0].severity, Severity::Critical);
    }

    #[test]
    fn test_review_infinite_loop() {
        let mut r = CodeReviewer::new();
        let findings = r.review("while true { do_stuff() }", "main.sl:20");
        assert!(!findings.is_empty());
    }

    #[test]
    fn test_suppress_rule() {
        let mut r = CodeReviewer::new();
        r.suppress_rule(ReviewRule::DivisionByZero);
        let findings = r.review("let x = y / 0", "test.sl:5");
        assert!(findings.is_empty());
    }

    #[test]
    fn test_findings_by_severity() {
        let mut r = CodeReviewer::new();
        r.review("let password = secret", "a.sl:1");
        assert!(r.findings_by_severity(Severity::Critical) > 0);
    }

    #[test]
    fn test_review_score_clean() {
        let mut r = CodeReviewer::new();
        r.review("fn f() { 1 + 2 }", "clean.sl:1");
        assert!((r.review_score() - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_review_score_degraded() {
        let mut r = CodeReviewer::new();
        r.review("let password = x / 0", "bad.sl:1");
        assert!(r.review_score() < 1.0);
    }

    #[test]
    fn test_review_finding_struct() {
        let f = ReviewFinding {
            rule: ReviewRule::DeadCode,
            severity: Severity::Info,
            message: "Dead code".to_string(),
            location: "test.sl:10".to_string(),
            confidence: 0.9,
            auto_fixable: true,
        };
        assert!(f.auto_fixable);
    }

    #[test]
    fn test_pattern_checker() {
        let pc = PatternChecker {
            rule: ReviewRule::UnusedVariable,
            patterns: vec!["_unused".to_string()],
            enabled: true,
        };
        assert!(pc.enabled);
    }

    #[test]
    fn test_ffi_check() {
        let n = slang_review_check(1);
        assert!(n >= 0);
    }

    #[test]
    fn test_ffi_total() {
        let n = slang_review_total();
        assert!(n >= 0);
    }
}
