//! API Compatibility — Tooling (v353)
//!
//! Semver diff, breaking change detection, and API surface analysis.

use std::sync::atomic::{AtomicI64, Ordering};

static BREAKING_CHANGES: AtomicI64 = AtomicI64::new(0);
static ADDITIONS: AtomicI64 = AtomicI64::new(0);
static DEPRECATIONS: AtomicI64 = AtomicI64::new(0);
static API_SURFACE: AtomicI64 = AtomicI64::new(0);

/// Compare two semver versions. Returns: 1=compatible, 0=breaking, -1=same.
#[unsafe(no_mangle)]
pub extern "C" fn slang_api_semver_diff(major_old: i64, major_new: i64) -> i64 {
    if major_old == major_new { -1 }
    else if major_new > major_old && major_old > 0 { 0 } // major bump = breaking
    else { 1 } // minor/patch = compatible
}

/// Record a breaking change. Returns total count.
#[unsafe(no_mangle)]
pub extern "C" fn slang_api_breaking_change() -> i64 {
    BREAKING_CHANGES.fetch_add(1, Ordering::SeqCst) + 1
}

/// Record an API addition. Returns total count.
#[unsafe(no_mangle)]
pub extern "C" fn slang_api_addition() -> i64 {
    ADDITIONS.fetch_add(1, Ordering::SeqCst) + 1
}

/// Record a deprecation. Returns total count.
#[unsafe(no_mangle)]
pub extern "C" fn slang_api_deprecation() -> i64 {
    DEPRECATIONS.fetch_add(1, Ordering::SeqCst) + 1
}

/// Set API surface size. Returns previous size.
#[unsafe(no_mangle)]
pub extern "C" fn slang_api_surface(size: i64) -> i64 {
    API_SURFACE.swap(size.max(0), Ordering::SeqCst)
}

/// Get summary: breaking count.
#[unsafe(no_mangle)]
pub extern "C" fn slang_api_breaking_count() -> i64 {
    BREAKING_CHANGES.load(Ordering::SeqCst)
}

/// Get summary: addition count.
#[unsafe(no_mangle)]
pub extern "C" fn slang_api_addition_count() -> i64 {
    ADDITIONS.load(Ordering::SeqCst)
}

/// Get summary: deprecation count.
#[unsafe(no_mangle)]
pub extern "C" fn slang_api_deprecation_count() -> i64 {
    DEPRECATIONS.load(Ordering::SeqCst)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_semver_same() { assert_eq!(slang_api_semver_diff(1, 1), -1); }
    #[test] fn test_semver_breaking() { assert_eq!(slang_api_semver_diff(1, 2), 0); }
    #[test] fn test_semver_compat() { assert_eq!(slang_api_semver_diff(0, 1), 1); }
    #[test] fn test_semver_downgrade() { assert_eq!(slang_api_semver_diff(2, 1), 1); }
    #[test] fn test_breaking_change() { assert!(slang_api_breaking_change() >= 1); }
    #[test] fn test_addition() { assert!(slang_api_addition() >= 1); }
    #[test] fn test_deprecation() { assert!(slang_api_deprecation() >= 1); }
    #[test] fn test_surface() { let prev = slang_api_surface(100); assert!(prev >= 0); assert_eq!(API_SURFACE.load(Ordering::SeqCst), 100); }
    #[test] fn test_breaking_count() {
        let before = slang_api_breaking_count();
        slang_api_breaking_change();
        assert_eq!(slang_api_breaking_count(), before + 1);
    }
    #[test] fn test_addition_count() {
        let before = slang_api_addition_count();
        slang_api_addition();
        assert_eq!(slang_api_addition_count(), before + 1);
    }
    #[test] fn test_deprecation_count() {
        let before = slang_api_deprecation_count();
        slang_api_deprecation();
        assert_eq!(slang_api_deprecation_count(), before + 1);
    }
    #[test] fn test_surface_clamp_negative() { slang_api_surface(-10); assert_eq!(API_SURFACE.load(Ordering::SeqCst), 0); }
    #[test] fn test_semver_zero() { assert_eq!(slang_api_semver_diff(0, 0), -1); }
    #[test] fn test_semver_large() { assert_eq!(slang_api_semver_diff(100, 101), 0); }
    #[test] fn test_multiple_breaking() {
        let before = slang_api_breaking_count();
        for _ in 0..5 { slang_api_breaking_change(); }
        assert_eq!(slang_api_breaking_count(), before + 5);
    }
    #[test] fn test_multiple_additions() {
        let before = slang_api_addition_count();
        for _ in 0..5 { slang_api_addition(); }
        assert_eq!(slang_api_addition_count(), before + 5);
    }
    #[test] fn test_surface_returns_previous() {
        slang_api_surface(50);
        assert_eq!(slang_api_surface(100), 50);
    }
    #[test] fn test_surface_zero() {
        slang_api_surface(0);
        assert_eq!(API_SURFACE.load(Ordering::SeqCst), 0);
    }
    #[test] fn test_surface_large() {
        slang_api_surface(1_000_000);
        assert_eq!(API_SURFACE.load(Ordering::SeqCst), 1_000_000);
    }
    #[test] fn test_monotonic() {
        let a = slang_api_breaking_change();
        let b = slang_api_breaking_change();
        assert!(b > a);
    }
}
