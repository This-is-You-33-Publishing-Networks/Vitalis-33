//! CSP — Content Security Policy (v343)
//!
//! Security header generation, directive management, nonce generation.

use std::sync::{LazyLock, Mutex};
use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, Ordering};

struct Policy {
    directives: HashMap<i64, Vec<i64>>, // directive_id → source_ids
    nonce: i64,
    report_only: bool,
}

static POLICIES: LazyLock<Mutex<HashMap<i64, Policy>>> = LazyLock::new(|| Mutex::new(HashMap::new()));
static POLICY_COUNTER: AtomicI64 = AtomicI64::new(0);
static NONCE_COUNTER: AtomicI64 = AtomicI64::new(1000);

/// Create a CSP policy. Returns policy ID.
#[unsafe(no_mangle)]
pub extern "C" fn slang_csp_create() -> i64 {
    let id = POLICY_COUNTER.fetch_add(1, Ordering::SeqCst);
    let nonce = NONCE_COUNTER.fetch_add(7, Ordering::SeqCst); // deterministic
    POLICIES.lock().unwrap().insert(id, Policy {
        directives: HashMap::new(), nonce, report_only: false,
    });
    id
}

/// Add a directive source. Returns source count for directive.
#[unsafe(no_mangle)]
pub extern "C" fn slang_csp_add_directive(policy_id: i64, directive_id: i64, source_id: i64) -> i64 {
    let mut policies = POLICIES.lock().unwrap();
    if let Some(p) = policies.get_mut(&policy_id) {
        let sources = p.directives.entry(directive_id).or_insert_with(Vec::new);
        sources.push(source_id);
        sources.len() as i64
    } else { -1 }
}

/// Get the nonce for a policy.
#[unsafe(no_mangle)]
pub extern "C" fn slang_csp_nonce(policy_id: i64) -> i64 {
    POLICIES.lock().unwrap().get(&policy_id).map(|p| p.nonce).unwrap_or(-1)
}

/// Get directive count for a policy.
#[unsafe(no_mangle)]
pub extern "C" fn slang_csp_directive_count(policy_id: i64) -> i64 {
    POLICIES.lock().unwrap().get(&policy_id).map(|p| p.directives.len() as i64).unwrap_or(-1)
}

/// Set report-only mode. Returns 1 on success.
#[unsafe(no_mangle)]
pub extern "C" fn slang_csp_report_only(policy_id: i64, enabled: i64) -> i64 {
    let mut policies = POLICIES.lock().unwrap();
    if let Some(p) = policies.get_mut(&policy_id) {
        p.report_only = enabled != 0;
        1
    } else { -1 }
}

/// Check if a source is allowed for a directive. Returns 1 if allowed.
#[unsafe(no_mangle)]
pub extern "C" fn slang_csp_check(policy_id: i64, directive_id: i64, source_id: i64) -> i64 {
    let policies = POLICIES.lock().unwrap();
    if let Some(p) = policies.get(&policy_id) {
        if let Some(sources) = p.directives.get(&directive_id) {
            if sources.contains(&source_id) { 1 } else { 0 }
        } else { 0 }
    } else { -1 }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_create() { let id = slang_csp_create(); assert!(id >= 0); }
    #[test] fn test_nonce() { let id = slang_csp_create(); assert!(slang_csp_nonce(id) > 0); }
    #[test] fn test_nonce_unique() {
        let a = slang_csp_create();
        let b = slang_csp_create();
        assert_ne!(slang_csp_nonce(a), slang_csp_nonce(b));
    }
    #[test] fn test_add_directive() {
        let id = slang_csp_create();
        assert_eq!(slang_csp_add_directive(id, 1, 100), 1);
    }
    #[test] fn test_directive_count() {
        let id = slang_csp_create();
        slang_csp_add_directive(id, 1, 100);
        slang_csp_add_directive(id, 2, 200);
        assert_eq!(slang_csp_directive_count(id), 2);
    }
    #[test] fn test_check_allowed() {
        let id = slang_csp_create();
        slang_csp_add_directive(id, 1, 100);
        assert_eq!(slang_csp_check(id, 1, 100), 1);
    }
    #[test] fn test_check_denied() {
        let id = slang_csp_create();
        slang_csp_add_directive(id, 1, 100);
        assert_eq!(slang_csp_check(id, 1, 999), 0);
    }
    #[test] fn test_check_no_directive() {
        let id = slang_csp_create();
        assert_eq!(slang_csp_check(id, 99, 1), 0);
    }
    #[test] fn test_report_only() {
        let id = slang_csp_create();
        assert_eq!(slang_csp_report_only(id, 1), 1);
    }
    #[test] fn test_invalid_policy() { assert_eq!(slang_csp_nonce(999), -1); }
    #[test] fn test_add_multiple_sources() {
        let id = slang_csp_create();
        slang_csp_add_directive(id, 1, 10);
        assert_eq!(slang_csp_add_directive(id, 1, 20), 2);
    }
    #[test] fn test_check_second_source() {
        let id = slang_csp_create();
        slang_csp_add_directive(id, 1, 10);
        slang_csp_add_directive(id, 1, 20);
        assert_eq!(slang_csp_check(id, 1, 20), 1);
    }
    #[test] fn test_directive_count_empty() {
        let id = slang_csp_create();
        assert_eq!(slang_csp_directive_count(id), 0);
    }
    #[test] fn test_report_only_disable() {
        let id = slang_csp_create();
        slang_csp_report_only(id, 1);
        slang_csp_report_only(id, 0);
        let p = POLICIES.lock().unwrap();
        assert!(!p.get(&id).unwrap().report_only);
    }
    #[test] fn test_check_invalid_policy() { assert_eq!(slang_csp_check(999, 1, 1), -1); }
    #[test] fn test_add_directive_invalid() { assert_eq!(slang_csp_add_directive(999, 1, 1), -1); }
    #[test] fn test_report_only_invalid() { assert_eq!(slang_csp_report_only(999, 1), -1); }
    #[test] fn test_many_directives() {
        let id = slang_csp_create();
        for i in 0..10 { slang_csp_add_directive(id, i, i * 10); }
        assert_eq!(slang_csp_directive_count(id), 10);
    }
    #[test] fn test_negative_ids() {
        let id = slang_csp_create();
        slang_csp_add_directive(id, -1, -10);
        assert_eq!(slang_csp_check(id, -1, -10), 1);
    }
    #[test] fn test_multiple_policies() {
        let a = slang_csp_create();
        let b = slang_csp_create();
        slang_csp_add_directive(a, 1, 100);
        assert_eq!(slang_csp_check(b, 1, 100), 0); // different policy
    }
}
