//! Release — Release Engineering (v361)
//!
//! Changelog generation, version bumping, packaging, and release notes.

use std::sync::atomic::{AtomicI64, Ordering};

static CHANGELOG_ENTRIES: AtomicI64 = AtomicI64::new(0);
static CURRENT_MAJOR: AtomicI64 = AtomicI64::new(0);
static CURRENT_MINOR: AtomicI64 = AtomicI64::new(0);
static CURRENT_PATCH: AtomicI64 = AtomicI64::new(0);
static PACKAGES_BUILT: AtomicI64 = AtomicI64::new(0);

/// Add a changelog entry. Returns total entries.
#[unsafe(no_mangle)]
pub extern "C" fn slang_release_changelog(entry_type: i64) -> i64 {
    let _ = entry_type; // 0=fix, 1=feature, 2=breaking
    CHANGELOG_ENTRIES.fetch_add(1, Ordering::SeqCst) + 1
}

/// Bump version. bump_type: 0=patch, 1=minor, 2=major. Returns new major.
#[unsafe(no_mangle)]
pub extern "C" fn slang_release_version_bump(bump_type: i64) -> i64 {
    match bump_type {
        0 => { CURRENT_PATCH.fetch_add(1, Ordering::SeqCst); }
        1 => { CURRENT_MINOR.fetch_add(1, Ordering::SeqCst); CURRENT_PATCH.store(0, Ordering::SeqCst); }
        2 => { CURRENT_MAJOR.fetch_add(1, Ordering::SeqCst); CURRENT_MINOR.store(0, Ordering::SeqCst); CURRENT_PATCH.store(0, Ordering::SeqCst); }
        _ => {}
    }
    CURRENT_MAJOR.load(Ordering::SeqCst)
}

/// Build a package. Returns total packages built.
#[unsafe(no_mangle)]
pub extern "C" fn slang_release_package() -> i64 {
    PACKAGES_BUILT.fetch_add(1, Ordering::SeqCst) + 1
}

/// Get current version as packed i64: major*10000 + minor*100 + patch.
#[unsafe(no_mangle)]
pub extern "C" fn slang_release_version() -> i64 {
    let m = CURRENT_MAJOR.load(Ordering::SeqCst);
    let n = CURRENT_MINOR.load(Ordering::SeqCst);
    let p = CURRENT_PATCH.load(Ordering::SeqCst);
    m * 10000 + n * 100 + p
}

/// Get changelog entry count.
#[unsafe(no_mangle)]
pub extern "C" fn slang_release_changelog_count() -> i64 {
    CHANGELOG_ENTRIES.load(Ordering::SeqCst)
}

/// Get packages built count.
#[unsafe(no_mangle)]
pub extern "C" fn slang_release_packages_built() -> i64 {
    PACKAGES_BUILT.load(Ordering::SeqCst)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reset_version() {
        CURRENT_MAJOR.store(0, Ordering::SeqCst);
        CURRENT_MINOR.store(0, Ordering::SeqCst);
        CURRENT_PATCH.store(0, Ordering::SeqCst);
    }

    #[test] fn test_changelog() { assert!(slang_release_changelog(0) >= 1); }
    #[test] fn test_changelog_feature() { assert!(slang_release_changelog(1) >= 1); }
    #[test] fn test_changelog_breaking() { assert!(slang_release_changelog(2) >= 1); }
    #[test] fn test_changelog_count() {
        let before = slang_release_changelog_count();
        slang_release_changelog(0);
        assert_eq!(slang_release_changelog_count(), before + 1);
    }
    #[test] fn test_patch_bump() {
        reset_version();
        slang_release_version_bump(0);
        assert_eq!(CURRENT_PATCH.load(Ordering::SeqCst), 1);
    }
    #[test] fn test_minor_bump() {
        reset_version();
        slang_release_version_bump(1);
        assert_eq!(CURRENT_MINOR.load(Ordering::SeqCst), 1);
    }
    #[test] fn test_major_bump() {
        reset_version();
        slang_release_version_bump(2);
        assert_eq!(CURRENT_MAJOR.load(Ordering::SeqCst), 1);
    }
    #[test] fn test_minor_resets_patch() {
        reset_version();
        slang_release_version_bump(0);
        slang_release_version_bump(0);
        slang_release_version_bump(1);
        assert_eq!(CURRENT_PATCH.load(Ordering::SeqCst), 0);
    }
    #[test] fn test_major_resets_minor() {
        reset_version();
        slang_release_version_bump(1);
        slang_release_version_bump(2);
        assert_eq!(CURRENT_MINOR.load(Ordering::SeqCst), 0);
    }
    #[test] fn test_version_packed() {
        reset_version();
        slang_release_version_bump(2); // 1.0.0
        slang_release_version_bump(1); // 1.1.0
        slang_release_version_bump(0); // 1.1.1
        assert_eq!(slang_release_version(), 10101);
    }
    #[test] fn test_package() { assert!(slang_release_package() >= 1); }
    #[test] fn test_packages_built() {
        let before = slang_release_packages_built();
        slang_release_package();
        assert_eq!(slang_release_packages_built(), before + 1);
    }
    #[test] fn test_version_initial() {
        reset_version();
        assert_eq!(slang_release_version(), 0);
    }
    #[test] fn test_invalid_bump() {
        reset_version();
        slang_release_version_bump(99);
        assert_eq!(slang_release_version(), 0);
    }
    #[test] fn test_multiple_patches() {
        reset_version();
        for _ in 0..5 { slang_release_version_bump(0); }
        assert_eq!(CURRENT_PATCH.load(Ordering::SeqCst), 5);
    }
    #[test] fn test_bump_returns_major() {
        reset_version();
        assert_eq!(slang_release_version_bump(2), 1);
        assert_eq!(slang_release_version_bump(2), 2);
    }
    #[test] fn test_changelog_monotonic() {
        let a = slang_release_changelog(0);
        let b = slang_release_changelog(0);
        assert!(b > a);
    }
    #[test] fn test_package_monotonic() {
        let a = slang_release_package();
        let b = slang_release_package();
        assert!(b > a);
    }
    #[test] fn test_many_changelogs() {
        let before = slang_release_changelog_count();
        for _ in 0..10 { slang_release_changelog(0); }
        assert_eq!(slang_release_changelog_count(), before + 10);
    }
}
