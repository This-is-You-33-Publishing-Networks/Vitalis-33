//! Supply-chain security primitives for package verification and policy gates.
//!
//! This module provides deterministic signature checks, provenance validation,
//! vulnerability risk scoring, and install policy decisions.

use crate::package_manager::SemVer;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Severity {
    Low,
    Medium,
    High,
    Critical,
}

impl Severity {
    fn weight(&self) -> f64 {
        match self {
            Severity::Low => 1.0,
            Severity::Medium => 2.5,
            Severity::High => 4.5,
            Severity::Critical => 7.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecurityAdvisory {
    pub id: String,
    pub severity: Severity,
    pub affected_package: String,
    pub fixed_in: Option<SemVer>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageProvenance {
    pub builder_id: String,
    pub source_repo: String,
    pub source_revision: String,
    pub attestation_hash: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PackageSecurityRecord {
    pub name: String,
    pub version: SemVer,
    pub checksum: String,
    pub signature: String,
    pub signature_valid: bool,
    pub provenance: Option<PackageProvenance>,
    pub advisories: Vec<SecurityAdvisory>,
    pub maintainer_trust: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SecurityPolicy {
    pub require_signature: bool,
    pub require_provenance: bool,
    pub max_risk_score: f64,
    pub min_maintainer_trust: f64,
    pub deny_advisory_ids: Vec<String>,
}

impl Default for SecurityPolicy {
    fn default() -> Self {
        Self {
            require_signature: true,
            require_provenance: true,
            max_risk_score: 5.0,
            min_maintainer_trust: 0.6,
            deny_advisory_ids: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PolicyDecision {
    pub allow: bool,
    pub risk_score: f64,
    pub reasons: Vec<String>,
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for &b in bytes {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

pub fn compute_package_signature(name: &str, version: &SemVer, checksum: &str) -> String {
    let payload = format!("{}@{}:{}", name, version, checksum);
    format!("sig:{:016x}", fnv1a64(payload.as_bytes()))
}

pub fn verify_signature(name: &str, version: &SemVer, checksum: &str, signature: &str) -> bool {
    compute_package_signature(name, version, checksum) == signature
}

pub fn advisory_applies(advisory: &SecurityAdvisory, package: &str, version: &SemVer) -> bool {
    if advisory.affected_package != package {
        return false;
    }
    match &advisory.fixed_in {
        Some(fixed) => version < fixed,
        None => true,
    }
}

pub fn compute_risk_score(record: &PackageSecurityRecord) -> f64 {
    let mut score = 0.0;

    if !record.signature_valid {
        score += 5.0;
    }
    if record.provenance.is_none() {
        score += 2.0;
    }

    for advisory in &record.advisories {
        if advisory_applies(advisory, &record.name, &record.version) {
            score += advisory.severity.weight();
        }
    }

    let trust_penalty = (1.0 - record.maintainer_trust).max(0.0) * 3.0;
    score + trust_penalty
}

pub fn evaluate_policy(record: &PackageSecurityRecord, policy: &SecurityPolicy) -> PolicyDecision {
    let mut reasons = Vec::new();

    if policy.require_signature && !record.signature_valid {
        reasons.push("signature verification failed".to_string());
    }
    if policy.require_provenance && record.provenance.is_none() {
        reasons.push("provenance attestation missing".to_string());
    }
    if record.maintainer_trust < policy.min_maintainer_trust {
        reasons.push(format!(
            "maintainer trust {} below minimum {}",
            record.maintainer_trust, policy.min_maintainer_trust
        ));
    }

    for deny in &policy.deny_advisory_ids {
        if record.advisories.iter().any(|a| a.id == *deny) {
            reasons.push(format!("deny-listed advisory present: {}", deny));
        }
    }

    let risk_score = compute_risk_score(record);
    if risk_score > policy.max_risk_score {
        reasons.push(format!(
            "risk score {} exceeds policy maximum {}",
            risk_score, policy.max_risk_score
        ));
    }

    PolicyDecision {
        allow: reasons.is_empty(),
        risk_score,
        reasons,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signature_round_trip() {
        let version = SemVer::new(1, 0, 0);
        let checksum = "sha256:abc";
        let sig = compute_package_signature("math", &version, checksum);
        assert!(verify_signature("math", &version, checksum, &sig));
        assert!(!verify_signature("math", &version, "sha256:def", &sig));
    }

    #[test]
    fn test_advisory_applies_with_fix_version() {
        let advisory = SecurityAdvisory {
            id: "CVE-1000".to_string(),
            severity: Severity::High,
            affected_package: "math".to_string(),
            fixed_in: Some(SemVer::new(1, 2, 0)),
        };
        assert!(advisory_applies(&advisory, "math", &SemVer::new(1, 1, 9)));
        assert!(!advisory_applies(&advisory, "math", &SemVer::new(1, 2, 0)));
    }

    #[test]
    fn test_risk_score_penalizes_missing_controls() {
        let record = PackageSecurityRecord {
            name: "math".to_string(),
            version: SemVer::new(1, 0, 0),
            checksum: "sha256:x".to_string(),
            signature: "bad".to_string(),
            signature_valid: false,
            provenance: None,
            advisories: vec![SecurityAdvisory {
                id: "CVE-2000".to_string(),
                severity: Severity::Critical,
                affected_package: "math".to_string(),
                fixed_in: None,
            }],
            maintainer_trust: 0.2,
        };

        let risk = compute_risk_score(&record);
        assert!(risk >= 14.0);
    }

    #[test]
    fn test_policy_accepts_low_risk_record() {
        let version = SemVer::new(1, 4, 0);
        let checksum = "sha256:good";
        let signature = compute_package_signature("core", &version, checksum);

        let record = PackageSecurityRecord {
            name: "core".to_string(),
            version,
            checksum: checksum.to_string(),
            signature,
            signature_valid: true,
            provenance: Some(PackageProvenance {
                builder_id: "gha-linux-amd64".to_string(),
                source_repo: "https://example.invalid/core".to_string(),
                source_revision: "abc123".to_string(),
                attestation_hash: "sha256:att".to_string(),
            }),
            advisories: Vec::new(),
            maintainer_trust: 0.9,
        };

        let decision = evaluate_policy(&record, &SecurityPolicy::default());
        assert!(decision.allow, "{:#?}", decision.reasons);
        assert!(decision.risk_score <= 5.0);
    }

    #[test]
    fn test_policy_rejects_deny_list_advisory() {
        let version = SemVer::new(2, 0, 0);
        let checksum = "sha256:ok";
        let signature = compute_package_signature("net", &version, checksum);
        let advisory = SecurityAdvisory {
            id: "GHSA-XYZ".to_string(),
            severity: Severity::Medium,
            affected_package: "net".to_string(),
            fixed_in: None,
        };

        let record = PackageSecurityRecord {
            name: "net".to_string(),
            version,
            checksum: checksum.to_string(),
            signature,
            signature_valid: true,
            provenance: Some(PackageProvenance {
                builder_id: "gha".to_string(),
                source_repo: "https://example.invalid/net".to_string(),
                source_revision: "def456".to_string(),
                attestation_hash: "sha256:att2".to_string(),
            }),
            advisories: vec![advisory],
            maintainer_trust: 0.95,
        };

        let mut policy = SecurityPolicy::default();
        policy.deny_advisory_ids.push("GHSA-XYZ".to_string());
        let decision = evaluate_policy(&record, &policy);

        assert!(!decision.allow);
        assert!(decision.reasons.iter().any(|r| r.contains("deny-listed advisory")));
    }
}
