//! Chaos Engineering — Testing Framework (v341)
//!
//! Fault injection, latency simulation, error rates, and partition testing.

use std::sync::atomic::{AtomicI64, Ordering};

static FAULT_COUNT: AtomicI64 = AtomicI64::new(0);
static LATENCY_SUM: AtomicI64 = AtomicI64::new(0);
static ERROR_RATE_BPS: AtomicI64 = AtomicI64::new(0); // basis points (0-10000)
static PARTITION_ACTIVE: AtomicI64 = AtomicI64::new(0);

/// Inject a fault. Returns total fault count.
#[unsafe(no_mangle)]
pub extern "C" fn slang_chaos_inject_fault() -> i64 {
    FAULT_COUNT.fetch_add(1, Ordering::SeqCst) + 1
}

/// Add latency (ms). Returns total accumulated latency.
#[unsafe(no_mangle)]
pub extern "C" fn slang_chaos_inject_latency(ms: i64) -> i64 {
    let clamped = ms.max(0);
    LATENCY_SUM.fetch_add(clamped, Ordering::SeqCst) + clamped
}

/// Set error rate in basis points (0-10000). Returns previous rate.
#[unsafe(no_mangle)]
pub extern "C" fn slang_chaos_error_rate(rate_bps: i64) -> i64 {
    let clamped = rate_bps.clamp(0, 10000);
    ERROR_RATE_BPS.swap(clamped, Ordering::SeqCst)
}

/// Simulate network partition. toggle=1 to enable, 0 to disable. Returns state.
#[unsafe(no_mangle)]
pub extern "C" fn slang_chaos_partition(toggle: i64) -> i64 {
    let val = if toggle != 0 { 1 } else { 0 };
    PARTITION_ACTIVE.store(val, Ordering::SeqCst);
    val
}

/// Get total fault count.
#[unsafe(no_mangle)]
pub extern "C" fn slang_chaos_fault_count() -> i64 {
    FAULT_COUNT.load(Ordering::SeqCst)
}

/// Get accumulated latency.
#[unsafe(no_mangle)]
pub extern "C" fn slang_chaos_total_latency() -> i64 {
    LATENCY_SUM.load(Ordering::SeqCst)
}

/// Check if partition is active.
#[unsafe(no_mangle)]
pub extern "C" fn slang_chaos_is_partitioned() -> i64 {
    PARTITION_ACTIVE.load(Ordering::SeqCst)
}

/// Get current error rate in basis points.
#[unsafe(no_mangle)]
pub extern "C" fn slang_chaos_current_error_rate() -> i64 {
    ERROR_RATE_BPS.load(Ordering::SeqCst)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_inject_fault() { let c = slang_chaos_inject_fault(); assert!(c >= 1); }
    #[test] fn test_fault_count() {
        let before = slang_chaos_fault_count();
        slang_chaos_inject_fault();
        assert_eq!(slang_chaos_fault_count(), before + 1);
    }
    #[test] fn test_latency() {
        let before = slang_chaos_total_latency();
        let after = slang_chaos_inject_latency(100);
        assert_eq!(after, before + 100);
    }
    #[test] fn test_latency_negative_clamped() {
        let before = slang_chaos_total_latency();
        slang_chaos_inject_latency(-50);
        assert_eq!(slang_chaos_total_latency(), before); // negative clamped to 0
    }
    #[test] fn test_error_rate() {
        slang_chaos_error_rate(5000); // 50%
        assert_eq!(slang_chaos_current_error_rate(), 5000);
    }
    #[test] fn test_error_rate_clamp_high() {
        slang_chaos_error_rate(20000);
        assert_eq!(slang_chaos_current_error_rate(), 10000);
    }
    #[test] fn test_error_rate_clamp_low() {
        slang_chaos_error_rate(-100);
        assert_eq!(slang_chaos_current_error_rate(), 0);
    }
    #[test] fn test_partition_enable() {
        slang_chaos_partition(1);
        assert_eq!(slang_chaos_is_partitioned(), 1);
    }
    #[test] fn test_partition_disable() {
        slang_chaos_partition(0);
        assert_eq!(slang_chaos_is_partitioned(), 0);
    }
    #[test] fn test_partition_toggle() {
        slang_chaos_partition(1);
        assert_eq!(slang_chaos_is_partitioned(), 1);
        slang_chaos_partition(0);
        assert_eq!(slang_chaos_is_partitioned(), 0);
    }
    #[test] fn test_error_rate_returns_previous() {
        slang_chaos_error_rate(1000);
        let prev = slang_chaos_error_rate(2000);
        assert_eq!(prev, 1000);
    }
    #[test] fn test_multiple_faults() {
        let before = slang_chaos_fault_count();
        for _ in 0..10 { slang_chaos_inject_fault(); }
        assert_eq!(slang_chaos_fault_count(), before + 10);
    }
    #[test] fn test_accumulated_latency() {
        let before = slang_chaos_total_latency();
        slang_chaos_inject_latency(10);
        slang_chaos_inject_latency(20);
        slang_chaos_inject_latency(30);
        assert_eq!(slang_chaos_total_latency(), before + 60);
    }
    #[test] fn test_zero_latency() {
        let before = slang_chaos_total_latency();
        slang_chaos_inject_latency(0);
        assert_eq!(slang_chaos_total_latency(), before);
    }
    #[test] fn test_zero_error_rate() {
        slang_chaos_error_rate(0);
        assert_eq!(slang_chaos_current_error_rate(), 0);
    }
    #[test] fn test_partition_nonzero() {
        slang_chaos_partition(42); // any nonzero
        assert_eq!(slang_chaos_is_partitioned(), 1);
    }
    #[test] fn test_partition_negative() {
        slang_chaos_partition(-1);
        assert_eq!(slang_chaos_is_partitioned(), 1);
    }
    #[test] fn test_large_latency() {
        let before = slang_chaos_total_latency();
        slang_chaos_inject_latency(1_000_000);
        assert!(slang_chaos_total_latency() >= before + 1_000_000);
    }
    #[test] fn test_error_rate_exactly_10000() {
        slang_chaos_error_rate(10000);
        assert_eq!(slang_chaos_current_error_rate(), 10000);
    }
    #[test] fn test_fault_monotonic() {
        let a = slang_chaos_inject_fault();
        let b = slang_chaos_inject_fault();
        assert!(b > a);
    }
}
