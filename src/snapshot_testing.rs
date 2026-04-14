//! Snapshot Testing — Testing Framework (v349)
//!
//! Snapshot capture, comparison, and update for regression testing.

use std::sync::{LazyLock, Mutex};
use std::collections::HashMap;

struct Snapshot {
    value: i64,
    version: i64,
}

static SNAPSHOTS: LazyLock<Mutex<HashMap<i64, Snapshot>>> = LazyLock::new(|| Mutex::new(HashMap::new()));
static MATCH_COUNT: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(0);
static MISMATCH_COUNT: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(0);

/// Capture a snapshot. Returns version number.
#[unsafe(no_mangle)]
pub extern "C" fn slang_snapshot_capture(key: i64, value: i64) -> i64 {
    let mut snaps = SNAPSHOTS.lock().unwrap();
    let version = snaps.get(&key).map(|s| s.version + 1).unwrap_or(1);
    snaps.insert(key, Snapshot { value, version });
    version
}

/// Compare current value against snapshot. Returns 1 if match, 0 if mismatch.
#[unsafe(no_mangle)]
pub extern "C" fn slang_snapshot_compare(key: i64, value: i64) -> i64 {
    let snaps = SNAPSHOTS.lock().unwrap();
    match snaps.get(&key) {
        Some(s) if s.value == value => {
            MATCH_COUNT.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            1
        }
        Some(_) => {
            MISMATCH_COUNT.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            0
        }
        None => -1,
    }
}

/// Update a snapshot with a new value. Returns new version.
#[unsafe(no_mangle)]
pub extern "C" fn slang_snapshot_update(key: i64, value: i64) -> i64 {
    slang_snapshot_capture(key, value)
}

/// Get snapshot version. Returns -1 if not found.
#[unsafe(no_mangle)]
pub extern "C" fn slang_snapshot_version(key: i64) -> i64 {
    SNAPSHOTS.lock().unwrap().get(&key).map(|s| s.version).unwrap_or(-1)
}

/// Get total match count.
#[unsafe(no_mangle)]
pub extern "C" fn slang_snapshot_match_count() -> i64 {
    MATCH_COUNT.load(std::sync::atomic::Ordering::SeqCst)
}

/// Get total mismatch count.
#[unsafe(no_mangle)]
pub extern "C" fn slang_snapshot_mismatch_count() -> i64 {
    MISMATCH_COUNT.load(std::sync::atomic::Ordering::SeqCst)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_capture() { assert_eq!(slang_snapshot_capture(100000, 42), 1); }
    #[test] fn test_compare_match() {
        slang_snapshot_capture(100001, 42);
        assert_eq!(slang_snapshot_compare(100001, 42), 1);
    }
    #[test] fn test_compare_mismatch() {
        slang_snapshot_capture(100002, 42);
        assert_eq!(slang_snapshot_compare(100002, 99), 0);
    }
    #[test] fn test_compare_missing() { assert_eq!(slang_snapshot_compare(999999, 0), -1); }
    #[test] fn test_version() {
        slang_snapshot_capture(100003, 1);
        assert_eq!(slang_snapshot_version(100003), 1);
    }
    #[test] fn test_version_increments() {
        slang_snapshot_capture(100004, 1);
        slang_snapshot_capture(100004, 2);
        assert_eq!(slang_snapshot_version(100004), 2);
    }
    #[test] fn test_update() {
        slang_snapshot_capture(100005, 1);
        slang_snapshot_update(100005, 2);
        assert_eq!(slang_snapshot_compare(100005, 2), 1);
    }
    #[test] fn test_version_missing() { assert_eq!(slang_snapshot_version(888888), -1); }
    #[test] fn test_match_count() {
        let before = slang_snapshot_match_count();
        slang_snapshot_capture(100006, 42);
        slang_snapshot_compare(100006, 42);
        assert_eq!(slang_snapshot_match_count(), before + 1);
    }
    #[test] fn test_mismatch_count() {
        let before = slang_snapshot_mismatch_count();
        slang_snapshot_capture(100007, 42);
        slang_snapshot_compare(100007, 99);
        assert_eq!(slang_snapshot_mismatch_count(), before + 1);
    }
    #[test] fn test_capture_overwrites() {
        slang_snapshot_capture(100008, 1);
        slang_snapshot_capture(100008, 2);
        assert_eq!(slang_snapshot_compare(100008, 2), 1);
    }
    #[test] fn test_update_version() {
        slang_snapshot_capture(100009, 1);
        let v = slang_snapshot_update(100009, 2);
        assert_eq!(v, 2);
    }
    #[test] fn test_many_snapshots() {
        for i in 200000..200020 { slang_snapshot_capture(i, i * 10); }
        for i in 200000..200020 { assert_eq!(slang_snapshot_compare(i, i * 10), 1); }
    }
    #[test] fn test_zero_value() {
        slang_snapshot_capture(100010, 0);
        assert_eq!(slang_snapshot_compare(100010, 0), 1);
    }
    #[test] fn test_negative_value() {
        slang_snapshot_capture(100011, -42);
        assert_eq!(slang_snapshot_compare(100011, -42), 1);
    }
    #[test] fn test_negative_key() {
        slang_snapshot_capture(-1, 99);
        assert_eq!(slang_snapshot_compare(-1, 99), 1);
    }
    #[test] fn test_large_value() {
        slang_snapshot_capture(100012, i64::MAX);
        assert_eq!(slang_snapshot_compare(100012, i64::MAX), 1);
    }
    #[test] fn test_capture_returns_version() {
        let v1 = slang_snapshot_capture(100013, 1);
        let v2 = slang_snapshot_capture(100013, 2);
        let v3 = slang_snapshot_capture(100013, 3);
        assert_eq!(v1, 1);
        assert_eq!(v2, 2);
        assert_eq!(v3, 3);
    }
    #[test] fn test_independent_keys() {
        slang_snapshot_capture(100014, 10);
        slang_snapshot_capture(100015, 20);
        assert_eq!(slang_snapshot_compare(100014, 20), 0);
    }
    #[test] fn test_update_new_key() {
        let v = slang_snapshot_update(100016, 42);
        assert_eq!(v, 1);
    }
}
