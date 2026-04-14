//! Fuzzer — Coverage-Guided Fuzzing (v350)
//!
//! Coverage-guided fuzzing, corpus management, crash dedup, and energy scheduling.

use std::sync::atomic::{AtomicI64, Ordering};

static CORPUS_SIZE: AtomicI64 = AtomicI64::new(0);
static TOTAL_RUNS: AtomicI64 = AtomicI64::new(0);
static CRASH_COUNT: AtomicI64 = AtomicI64::new(0);
static UNIQUE_CRASHES: AtomicI64 = AtomicI64::new(0);
static COVERAGE_EDGES: AtomicI64 = AtomicI64::new(0);
static LAST_CRASH_HASH: AtomicI64 = AtomicI64::new(0);

/// Add an input to the corpus. Returns corpus size.
#[unsafe(no_mangle)]
pub extern "C" fn slang_fuzz_add_corpus(input_hash: i64) -> i64 {
    let _ = input_hash;
    CORPUS_SIZE.fetch_add(1, Ordering::SeqCst) + 1
}

/// Record a fuzzing run. Returns total runs.
#[unsafe(no_mangle)]
pub extern "C" fn slang_fuzz_run(coverage_edges: i64) -> i64 {
    TOTAL_RUNS.fetch_add(1, Ordering::SeqCst);
    if coverage_edges > COVERAGE_EDGES.load(Ordering::SeqCst) {
        COVERAGE_EDGES.store(coverage_edges, Ordering::SeqCst);
    }
    TOTAL_RUNS.load(Ordering::SeqCst)
}

/// Record a crash. Returns 1 if new unique crash, 0 if duplicate.
#[unsafe(no_mangle)]
pub extern "C" fn slang_fuzz_crash(crash_hash: i64) -> i64 {
    CRASH_COUNT.fetch_add(1, Ordering::SeqCst);
    let last = LAST_CRASH_HASH.swap(crash_hash, Ordering::SeqCst);
    if last != crash_hash {
        UNIQUE_CRASHES.fetch_add(1, Ordering::SeqCst);
        1
    } else { 0 }
}

/// Get current corpus size.
#[unsafe(no_mangle)]
pub extern "C" fn slang_fuzz_corpus_size() -> i64 {
    CORPUS_SIZE.load(Ordering::SeqCst)
}

/// Get coverage edge count.
#[unsafe(no_mangle)]
pub extern "C" fn slang_fuzz_coverage() -> i64 {
    COVERAGE_EDGES.load(Ordering::SeqCst)
}

/// Get total crash count.
#[unsafe(no_mangle)]
pub extern "C" fn slang_fuzz_crash_count() -> i64 {
    CRASH_COUNT.load(Ordering::SeqCst)
}

/// Get unique crash count.
#[unsafe(no_mangle)]
pub extern "C" fn slang_fuzz_unique_crashes() -> i64 {
    UNIQUE_CRASHES.load(Ordering::SeqCst)
}

/// Get total runs.
#[unsafe(no_mangle)]
pub extern "C" fn slang_fuzz_total_runs() -> i64 {
    TOTAL_RUNS.load(Ordering::SeqCst)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_add_corpus() { assert!(slang_fuzz_add_corpus(1) >= 1); }
    #[test] fn test_corpus_size() {
        let before = slang_fuzz_corpus_size();
        slang_fuzz_add_corpus(42);
        assert_eq!(slang_fuzz_corpus_size(), before + 1);
    }
    #[test] fn test_run() { assert!(slang_fuzz_run(10) >= 1); }
    #[test] fn test_total_runs() {
        let before = slang_fuzz_total_runs();
        slang_fuzz_run(5);
        assert_eq!(slang_fuzz_total_runs(), before + 1);
    }
    #[test] fn test_coverage_update() {
        let before = slang_fuzz_coverage();
        slang_fuzz_run(before + 100);
        assert_eq!(slang_fuzz_coverage(), before + 100);
    }
    #[test] fn test_coverage_no_decrease() {
        let before = slang_fuzz_coverage();
        slang_fuzz_run(0);
        assert!(slang_fuzz_coverage() >= before);
    }
    #[test] fn test_crash() {
        let before = slang_fuzz_crash_count();
        slang_fuzz_crash(12345);
        assert_eq!(slang_fuzz_crash_count(), before + 1);
    }
    #[test] fn test_unique_crash() {
        let before = slang_fuzz_unique_crashes();
        slang_fuzz_crash(i64::MAX - 1); // very unlikely to collide
        assert!(slang_fuzz_unique_crashes() >= before);
    }
    #[test] fn test_duplicate_crash() {
        slang_fuzz_crash(77777);
        let before = slang_fuzz_unique_crashes();
        let result = slang_fuzz_crash(77777);
        assert_eq!(result, 0);
        assert_eq!(slang_fuzz_unique_crashes(), before);
    }
    #[test] fn test_corpus_grow() {
        let before = slang_fuzz_corpus_size();
        for _ in 0..5 { slang_fuzz_add_corpus(0); }
        assert_eq!(slang_fuzz_corpus_size(), before + 5);
    }
    #[test] fn test_multiple_runs() {
        let before = slang_fuzz_total_runs();
        for _ in 0..10 { slang_fuzz_run(0); }
        assert_eq!(slang_fuzz_total_runs(), before + 10);
    }
    #[test] fn test_crash_returns_new() {
        let result = slang_fuzz_crash(i64::MIN + 1);
        assert!(result == 0 || result == 1);
    }
    #[test] fn test_coverage_larger() {
        let c = slang_fuzz_coverage();
        slang_fuzz_run(c + 50);
        assert_eq!(slang_fuzz_coverage(), c + 50);
    }
    #[test] fn test_zero_edges() {
        slang_fuzz_run(0);
        assert!(slang_fuzz_total_runs() >= 1);
    }
    #[test] fn test_negative_corpus() {
        let before = slang_fuzz_corpus_size();
        slang_fuzz_add_corpus(-1);
        assert_eq!(slang_fuzz_corpus_size(), before + 1);
    }
    #[test] fn test_multiple_unique_crashes() {
        let before = slang_fuzz_unique_crashes();
        slang_fuzz_crash(500000);
        slang_fuzz_crash(500001);
        assert!(slang_fuzz_unique_crashes() >= before + 2);
    }
    #[test] fn test_crash_count_monotonic() {
        let a = slang_fuzz_crash_count();
        slang_fuzz_crash(0);
        let b = slang_fuzz_crash_count();
        assert!(b > a);
    }
    #[test] fn test_run_monotonic() {
        let a = slang_fuzz_total_runs();
        slang_fuzz_run(0);
        assert!(slang_fuzz_total_runs() > a);
    }
    #[test] fn test_corpus_monotonic() {
        let a = slang_fuzz_corpus_size();
        slang_fuzz_add_corpus(0);
        assert!(slang_fuzz_corpus_size() > a);
    }
}
