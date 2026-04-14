//! Migration — Code Migration Engine (v354)
//!
//! Version transforms, migration planning, and rollback support.

use std::sync::{LazyLock, Mutex};
use std::collections::HashMap;

struct Migration {
    from_version: i64,
    to_version: i64,
    transforms_applied: i64,
    rolled_back: bool,
}

static MIGRATIONS: LazyLock<Mutex<HashMap<i64, Migration>>> = LazyLock::new(|| Mutex::new(HashMap::new()));
static MIG_COUNTER: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(0);

/// Create a migration plan. Returns migration ID.
#[unsafe(no_mangle)]
pub extern "C" fn slang_migration_create(from_version: i64, to_version: i64) -> i64 {
    let id = MIG_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    MIGRATIONS.lock().unwrap().insert(id, Migration {
        from_version, to_version, transforms_applied: 0, rolled_back: false,
    });
    id
}

/// Apply a transform step. Returns total transforms applied.
#[unsafe(no_mangle)]
pub extern "C" fn slang_migration_transform(migration_id: i64) -> i64 {
    let mut migs = MIGRATIONS.lock().unwrap();
    if let Some(m) = migs.get_mut(&migration_id) {
        if m.rolled_back { return -1; }
        m.transforms_applied += 1;
        m.transforms_applied
    } else { -1 }
}

/// Rollback a migration. Returns 1 on success.
#[unsafe(no_mangle)]
pub extern "C" fn slang_migration_rollback(migration_id: i64) -> i64 {
    let mut migs = MIGRATIONS.lock().unwrap();
    if let Some(m) = migs.get_mut(&migration_id) {
        m.rolled_back = true;
        m.transforms_applied = 0;
        1
    } else { -1 }
}

/// Get migration progress. Returns transforms applied.
#[unsafe(no_mangle)]
pub extern "C" fn slang_migration_progress(migration_id: i64) -> i64 {
    MIGRATIONS.lock().unwrap().get(&migration_id).map(|m| m.transforms_applied).unwrap_or(-1)
}

/// Get version delta (to - from).
#[unsafe(no_mangle)]
pub extern "C" fn slang_migration_delta(migration_id: i64) -> i64 {
    MIGRATIONS.lock().unwrap().get(&migration_id).map(|m| m.to_version - m.from_version).unwrap_or(-1)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_create() { let id = slang_migration_create(1, 2); assert!(id >= 0); }
    #[test] fn test_transform() {
        let id = slang_migration_create(1, 2);
        assert_eq!(slang_migration_transform(id), 1);
    }
    #[test] fn test_multiple_transforms() {
        let id = slang_migration_create(1, 5);
        for i in 1..=5 { assert_eq!(slang_migration_transform(id), i); }
    }
    #[test] fn test_rollback() {
        let id = slang_migration_create(1, 2);
        slang_migration_transform(id);
        assert_eq!(slang_migration_rollback(id), 1);
    }
    #[test] fn test_progress_after_rollback() {
        let id = slang_migration_create(1, 2);
        slang_migration_transform(id);
        slang_migration_rollback(id);
        assert_eq!(slang_migration_progress(id), 0);
    }
    #[test] fn test_transform_after_rollback() {
        let id = slang_migration_create(1, 2);
        slang_migration_rollback(id);
        assert_eq!(slang_migration_transform(id), -1);
    }
    #[test] fn test_delta() {
        let id = slang_migration_create(10, 20);
        assert_eq!(slang_migration_delta(id), 10);
    }
    #[test] fn test_delta_negative() {
        let id = slang_migration_create(20, 10);
        assert_eq!(slang_migration_delta(id), -10);
    }
    #[test] fn test_progress() {
        let id = slang_migration_create(1, 2);
        slang_migration_transform(id);
        slang_migration_transform(id);
        assert_eq!(slang_migration_progress(id), 2);
    }
    #[test] fn test_progress_empty() {
        let id = slang_migration_create(1, 2);
        assert_eq!(slang_migration_progress(id), 0);
    }
    #[test] fn test_invalid() { assert_eq!(slang_migration_transform(999), -1); }
    #[test] fn test_rollback_invalid() { assert_eq!(slang_migration_rollback(999), -1); }
    #[test] fn test_progress_invalid() { assert_eq!(slang_migration_progress(999), -1); }
    #[test] fn test_delta_invalid() { assert_eq!(slang_migration_delta(999), -1); }
    #[test] fn test_zero_delta() {
        let id = slang_migration_create(5, 5);
        assert_eq!(slang_migration_delta(id), 0);
    }
    #[test] fn test_multiple_migrations() {
        let a = slang_migration_create(1, 2);
        let b = slang_migration_create(3, 4);
        slang_migration_transform(a);
        assert_eq!(slang_migration_progress(a), 1);
        assert_eq!(slang_migration_progress(b), 0);
    }
    #[test] fn test_large_delta() {
        let id = slang_migration_create(0, 1000);
        assert_eq!(slang_migration_delta(id), 1000);
    }
    #[test] fn test_rollback_idempotent() {
        let id = slang_migration_create(1, 2);
        slang_migration_rollback(id);
        assert_eq!(slang_migration_rollback(id), 1); // still works
    }
    #[test] fn test_many_transforms() {
        let id = slang_migration_create(1, 100);
        for _ in 0..50 { slang_migration_transform(id); }
        assert_eq!(slang_migration_progress(id), 50);
    }
    #[test] fn test_negative_versions() {
        let id = slang_migration_create(-5, -2);
        assert_eq!(slang_migration_delta(id), 3);
    }
}
