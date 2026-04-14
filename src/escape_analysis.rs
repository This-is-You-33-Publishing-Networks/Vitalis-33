//! Escape Analysis — Compiler Core Hardening (v301)
//!
//! Analyzes pointer/reference escape through calls, returns, and closures.
//! Promotes heap allocations to stack when references don't escape the
//! defining scope. Tracks analysis results for optimization statistics.

use std::sync::{LazyLock, Mutex, atomic::{AtomicI64, Ordering}};

// ── Analysis state ───────────────────────────────────────────────────────────

/// Analysis record: (scope_id, escapes: bool as i64, promoted: bool as i64)
static ESCAPE_RECORDS: LazyLock<Mutex<Vec<(i64, i64, i64)>>> =
    LazyLock::new(|| Mutex::new(Vec::new()));

static STACK_PROMOTED_COUNT: AtomicI64 = AtomicI64::new(0);

/// Analyze an allocation at the given scope_id.
/// escape_kind: 0=no-escape (local), 1=escape-via-return, 2=escape-via-closure,
/// 3=escape-via-call-arg, 4=escape-via-store.
/// Returns 1 if allocation can be stack-promoted (kind==0), 0 otherwise.
#[unsafe(no_mangle)]
pub extern "C" fn slang_escape_analyze(scope_id: i64, escape_kind: i64) -> i64 {
    let escapes = if escape_kind == 0 { 0 } else { 1 };
    let promoted = if escape_kind == 0 { 1 } else { 0 };
    if promoted == 1 {
        STACK_PROMOTED_COUNT.fetch_add(1, Ordering::SeqCst);
    }
    let mut recs = ESCAPE_RECORDS.lock().unwrap();
    recs.push((scope_id, escapes, promoted));
    promoted
}

/// Return the number of allocations promoted from heap to stack.
#[unsafe(no_mangle)]
pub extern "C" fn slang_escape_stack_promoted() -> i64 {
    STACK_PROMOTED_COUNT.load(Ordering::SeqCst)
}

/// Return total number of escape analysis records.
#[unsafe(no_mangle)]
pub extern "C" fn slang_escape_summary() -> i64 {
    ESCAPE_RECORDS.lock().unwrap().len() as i64
}

/// Clear all escape analysis state. Returns count of records cleared.
#[unsafe(no_mangle)]
pub extern "C" fn slang_escape_clear() -> i64 {
    let mut recs = ESCAPE_RECORDS.lock().unwrap();
    let n = recs.len() as i64;
    recs.clear();
    STACK_PROMOTED_COUNT.store(0, Ordering::SeqCst);
    n
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn reset() { slang_escape_clear(); }

    #[test]
    fn test_no_escape_promotes() {
        reset();
        assert_eq!(slang_escape_analyze(1, 0), 1); // no-escape → promoted
    }

    #[test]
    fn test_escape_via_return() {
        reset();
        assert_eq!(slang_escape_analyze(1, 1), 0); // escapes → not promoted
    }

    #[test]
    fn test_escape_via_closure() {
        reset();
        assert_eq!(slang_escape_analyze(1, 2), 0);
    }

    #[test]
    fn test_escape_via_call_arg() {
        reset();
        assert_eq!(slang_escape_analyze(1, 3), 0);
    }

    #[test]
    fn test_escape_via_store() {
        reset();
        assert_eq!(slang_escape_analyze(1, 4), 0);
    }

    #[test]
    fn test_stack_promoted_count() {
        reset();
        slang_escape_analyze(1, 0);
        slang_escape_analyze(2, 0);
        slang_escape_analyze(3, 1);
        assert_eq!(slang_escape_stack_promoted(), 2);
    }

    #[test]
    fn test_summary_count() {
        reset();
        slang_escape_analyze(1, 0);
        slang_escape_analyze(2, 1);
        slang_escape_analyze(3, 2);
        assert_eq!(slang_escape_summary(), 3);
    }

    #[test]
    fn test_clear_returns_count() {
        reset();
        slang_escape_analyze(1, 0);
        slang_escape_analyze(2, 0);
        let cleared = slang_escape_clear();
        assert_eq!(cleared, 2);
        assert_eq!(slang_escape_summary(), 0);
        assert_eq!(slang_escape_stack_promoted(), 0);
    }

    #[test]
    fn test_multiple_scopes() {
        reset();
        for i in 0..10 {
            slang_escape_analyze(i, 0); // all local
        }
        assert_eq!(slang_escape_stack_promoted(), 10);
        assert_eq!(slang_escape_summary(), 10);
    }

    #[test]
    fn test_mixed_escape_kinds() {
        reset();
        slang_escape_analyze(1, 0); // promoted
        slang_escape_analyze(2, 1); // not
        slang_escape_analyze(3, 0); // promoted
        slang_escape_analyze(4, 2); // not
        slang_escape_analyze(5, 0); // promoted
        assert_eq!(slang_escape_stack_promoted(), 3);
        assert_eq!(slang_escape_summary(), 5);
    }

    #[test]
    fn test_empty_summary() {
        reset();
        assert_eq!(slang_escape_summary(), 0);
    }

    #[test]
    fn test_clear_idempotent() {
        reset();
        assert_eq!(slang_escape_clear(), 0);
        assert_eq!(slang_escape_clear(), 0);
    }

    #[test]
    fn test_large_batch() {
        reset();
        for i in 0..100 {
            slang_escape_analyze(i, if i % 3 == 0 { 0 } else { 1 });
        }
        assert_eq!(slang_escape_stack_promoted(), 34); // 0,3,6,...,99
        assert_eq!(slang_escape_summary(), 100);
    }

    #[test]
    fn test_same_scope_multiple_allocs() {
        reset();
        slang_escape_analyze(1, 0);
        slang_escape_analyze(1, 0);
        slang_escape_analyze(1, 1);
        assert_eq!(slang_escape_stack_promoted(), 2);
        assert_eq!(slang_escape_summary(), 3);
    }

    #[test]
    fn test_negative_scope_id() {
        reset();
        assert_eq!(slang_escape_analyze(-1, 0), 1);
        assert_eq!(slang_escape_summary(), 1);
    }

    #[test]
    fn test_unknown_escape_kind() {
        reset();
        assert_eq!(slang_escape_analyze(1, 99), 0); // unknown = escapes
    }

    #[test]
    fn test_zero_scope() {
        reset();
        assert_eq!(slang_escape_analyze(0, 0), 1);
    }

    #[test]
    fn test_promoted_after_clear() {
        reset();
        slang_escape_analyze(1, 0);
        assert_eq!(slang_escape_stack_promoted(), 1);
        slang_escape_clear();
        slang_escape_analyze(2, 1);
        assert_eq!(slang_escape_stack_promoted(), 0);
    }

    #[test]
    fn test_all_escape_kinds() {
        reset();
        for kind in 0..=4 {
            slang_escape_analyze(kind, kind);
        }
        assert_eq!(slang_escape_stack_promoted(), 1); // only kind=0
        assert_eq!(slang_escape_summary(), 5);
    }

    #[test]
    fn test_summary_after_partial_clear() {
        reset();
        slang_escape_analyze(1, 0);
        slang_escape_analyze(2, 0);
        slang_escape_clear();
        slang_escape_analyze(3, 0);
        assert_eq!(slang_escape_summary(), 1);
    }
}
