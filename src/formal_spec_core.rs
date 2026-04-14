//! v91 Formal Spec Core: executable invariants and property conformance checks.

use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpecDomain {
    Parser,
    TypeSystem,
    Ir,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Invariant {
    pub domain: SpecDomain,
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpecViolation {
    pub domain: SpecDomain,
    pub invariant: String,
    pub witness: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConformanceReport {
    pub checked: usize,
    pub violations: Vec<SpecViolation>,
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for &b in bytes {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

pub fn spec_digest(invariants: &[Invariant]) -> String {
    let mut canonical = invariants
        .iter()
        .map(|i| format!("{:?}|{}|{}", i.domain, i.name, i.description))
        .collect::<Vec<_>>();
    canonical.sort();
    format!("spec:{:016x}", fnv1a64(canonical.join("\n").as_bytes()))
}

pub fn default_invariants() -> Vec<Invariant> {
    vec![
        Invariant {
            domain: SpecDomain::Parser,
            name: "unique-top-level-symbols".to_string(),
            description: "Top-level symbols must be unique".to_string(),
        },
        Invariant {
            domain: SpecDomain::TypeSystem,
            name: "assignment-type-preservation".to_string(),
            description: "Assignment preserves source/destination type".to_string(),
        },
        Invariant {
            domain: SpecDomain::Ir,
            name: "branch-target-exists".to_string(),
            description: "Each branch target must point to a valid block".to_string(),
        },
    ]
}

pub fn check_parser_symbols(symbols: &[String]) -> Option<SpecViolation> {
    let mut seen = BTreeSet::new();
    for sym in symbols {
        if !seen.insert(sym.clone()) {
            return Some(SpecViolation {
                domain: SpecDomain::Parser,
                invariant: "unique-top-level-symbols".to_string(),
                witness: format!("duplicate symbol: {}", sym),
            });
        }
    }
    None
}

pub fn check_type_preservation(assignments: &[(String, String)]) -> Option<SpecViolation> {
    for (src, dst) in assignments {
        if src != dst {
            return Some(SpecViolation {
                domain: SpecDomain::TypeSystem,
                invariant: "assignment-type-preservation".to_string(),
                witness: format!("source={} destination={}", src, dst),
            });
        }
    }
    None
}

pub fn check_ir_branch_targets(block_count: usize, branch_targets: &[usize]) -> Option<SpecViolation> {
    for &target in branch_targets {
        if target >= block_count {
            return Some(SpecViolation {
                domain: SpecDomain::Ir,
                invariant: "branch-target-exists".to_string(),
                witness: format!("target {} out of bounds for {} blocks", target, block_count),
            });
        }
    }
    None
}

pub fn run_conformance_suite(
    parser_symbols: &[String],
    assignments: &[(String, String)],
    block_count: usize,
    branch_targets: &[usize],
) -> ConformanceReport {
    let mut violations = Vec::new();

    if let Some(v) = check_parser_symbols(parser_symbols) {
        violations.push(v);
    }
    if let Some(v) = check_type_preservation(assignments) {
        violations.push(v);
    }
    if let Some(v) = check_ir_branch_targets(block_count, branch_targets) {
        violations.push(v);
    }

    ConformanceReport {
        checked: 3,
        violations,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spec_digest_stable() {
        let inv = default_invariants();
        let d1 = spec_digest(&inv);
        let d2 = spec_digest(&inv);
        assert_eq!(d1, d2);
    }

    #[test]
    fn test_parser_duplicate_detection() {
        let symbols = vec!["main".to_string(), "util".to_string(), "main".to_string()];
        let v = check_parser_symbols(&symbols).unwrap();
        assert_eq!(v.domain, SpecDomain::Parser);
        assert!(v.witness.contains("duplicate"));
    }

    #[test]
    fn test_type_preservation_detection() {
        let pairs = vec![
            ("i64".to_string(), "i64".to_string()),
            ("f64".to_string(), "i64".to_string()),
        ];
        let v = check_type_preservation(&pairs).unwrap();
        assert_eq!(v.domain, SpecDomain::TypeSystem);
        assert!(v.witness.contains("source=f64"));
    }

    #[test]
    fn test_ir_branch_target_detection() {
        let v = check_ir_branch_targets(4, &[0, 3, 4]).unwrap();
        assert_eq!(v.domain, SpecDomain::Ir);
        assert!(v.witness.contains("out of bounds"));
    }

    #[test]
    fn test_conformance_suite_green() {
        let report = run_conformance_suite(
            &["main".to_string(), "helper".to_string()],
            &[("i64".to_string(), "i64".to_string())],
            3,
            &[0, 1, 2],
        );
        assert_eq!(report.checked, 3);
        assert!(report.violations.is_empty());
    }

    #[test]
    fn test_conformance_suite_reports_multiple_violations() {
        let report = run_conformance_suite(
            &["main".to_string(), "main".to_string()],
            &[("i64".to_string(), "f64".to_string())],
            2,
            &[0, 2],
        );
        assert_eq!(report.checked, 3);
        assert_eq!(report.violations.len(), 3);
    }

    #[test]
    fn test_property_sweep_branch_targets() {
        // Property-style sweep: all in-range target sets must pass.
        for blocks in 1usize..16 {
            let targets: Vec<usize> = (0..blocks).collect();
            assert!(check_ir_branch_targets(blocks, &targets).is_none());
        }
    }
}
