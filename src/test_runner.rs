//! Test Runner — Testing Framework (v348)
//!
//! Test discovery, parallel execution, coverage tracking, and result aggregation.

use std::sync::atomic::{AtomicI64, Ordering};

static DISCOVERED: AtomicI64 = AtomicI64::new(0);
static PASSED: AtomicI64 = AtomicI64::new(0);
static FAILED: AtomicI64 = AtomicI64::new(0);
static SKIPPED: AtomicI64 = AtomicI64::new(0);
static COVERAGE_LINES: AtomicI64 = AtomicI64::new(0);
static COVERAGE_TOTAL: AtomicI64 = AtomicI64::new(0);

/// Discover tests. Returns total discovered.
#[unsafe(no_mangle)]
pub extern "C" fn slang_test_discover(count: i64) -> i64 {
    let c = count.max(0);
    DISCOVERED.fetch_add(c, Ordering::SeqCst) + c
}

/// Record a passed test.
#[unsafe(no_mangle)]
pub extern "C" fn slang_test_pass() -> i64 {
    PASSED.fetch_add(1, Ordering::SeqCst) + 1
}

/// Record a failed test.
#[unsafe(no_mangle)]
pub extern "C" fn slang_test_fail() -> i64 {
    FAILED.fetch_add(1, Ordering::SeqCst) + 1
}

/// Record a skipped test.
#[unsafe(no_mangle)]
pub extern "C" fn slang_test_skip() -> i64 {
    SKIPPED.fetch_add(1, Ordering::SeqCst) + 1
}

/// Record coverage. Returns coverage percentage (0-100).
#[unsafe(no_mangle)]
pub extern "C" fn slang_test_coverage(covered: i64, total: i64) -> i64 {
    if total <= 0 { return 0; }
    let c = covered.max(0).min(total);
    COVERAGE_LINES.fetch_add(c, Ordering::SeqCst);
    COVERAGE_TOTAL.fetch_add(total, Ordering::SeqCst);
    (c * 100) / total
}

/// Get pass rate (0-100). Returns -1 if no tests.
#[unsafe(no_mangle)]
pub extern "C" fn slang_test_pass_rate() -> i64 {
    let p = PASSED.load(Ordering::SeqCst);
    let f = FAILED.load(Ordering::SeqCst);
    let total = p + f;
    if total == 0 { -1 } else { (p * 100) / total }
}

/// Get total test count (passed + failed + skipped).
#[unsafe(no_mangle)]
pub extern "C" fn slang_test_total() -> i64 {
    PASSED.load(Ordering::SeqCst) + FAILED.load(Ordering::SeqCst) + SKIPPED.load(Ordering::SeqCst)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_discover() { let c = slang_test_discover(10); assert!(c >= 10); }
    #[test] fn test_pass() { let c = slang_test_pass(); assert!(c >= 1); }
    #[test] fn test_fail() { let c = slang_test_fail(); assert!(c >= 1); }
    #[test] fn test_skip() { let c = slang_test_skip(); assert!(c >= 1); }
    #[test] fn test_coverage() { assert_eq!(slang_test_coverage(80, 100), 80); }
    #[test] fn test_coverage_zero_total() { assert_eq!(slang_test_coverage(0, 0), 0); }
    #[test] fn test_coverage_full() { assert_eq!(slang_test_coverage(100, 100), 100); }
    #[test] fn test_coverage_clamp() { assert_eq!(slang_test_coverage(200, 100), 100); }
    #[test] fn test_pass_rate() { let _ = slang_test_pass(); assert!(slang_test_pass_rate() >= 0); }
    #[test] fn test_total() { assert!(slang_test_total() > 0); }
    #[test] fn test_discover_negative() { let before = DISCOVERED.load(Ordering::SeqCst); slang_test_discover(-5); assert_eq!(DISCOVERED.load(Ordering::SeqCst), before); }
    #[test] fn test_discover_zero() { let before = DISCOVERED.load(Ordering::SeqCst); slang_test_discover(0); assert_eq!(DISCOVERED.load(Ordering::SeqCst), before); }
    #[test] fn test_pass_increments() {
        let before = PASSED.load(Ordering::SeqCst);
        slang_test_pass();
        assert_eq!(PASSED.load(Ordering::SeqCst), before + 1);
    }
    #[test] fn test_fail_increments() {
        let before = FAILED.load(Ordering::SeqCst);
        slang_test_fail();
        assert_eq!(FAILED.load(Ordering::SeqCst), before + 1);
    }
    #[test] fn test_skip_increments() {
        let before = SKIPPED.load(Ordering::SeqCst);
        slang_test_skip();
        assert_eq!(SKIPPED.load(Ordering::SeqCst), before + 1);
    }
    #[test] fn test_coverage_negative_covered() {
        assert_eq!(slang_test_coverage(-10, 100), 0);
    }
    #[test] fn test_coverage_partial() { assert_eq!(slang_test_coverage(33, 100), 33); }
    #[test] fn test_total_includes_all() {
        let before = slang_test_total();
        slang_test_pass();
        slang_test_fail();
        slang_test_skip();
        assert_eq!(slang_test_total(), before + 3);
    }
    #[test] fn test_discover_large() {
        let before = DISCOVERED.load(Ordering::SeqCst);
        slang_test_discover(1000);
        assert_eq!(DISCOVERED.load(Ordering::SeqCst), before + 1000);
    }
    #[test] fn test_coverage_negative_total() { assert_eq!(slang_test_coverage(10, -5), 0); }
}
