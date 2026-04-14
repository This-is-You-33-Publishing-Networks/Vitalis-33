//! Event Sourcing — Systems Infrastructure (v333)
//!
//! Append-only event store with projections, snapshots, and replay.

use std::sync::{LazyLock, Mutex};

struct EventStore {
    events: Vec<(i64, i64)>,  // (event_type, payload)
    snapshot: Option<i64>,     // latest snapshot value
    snapshot_at: i64,          // event index of snapshot
}

static STORES: LazyLock<Mutex<Vec<EventStore>>> = LazyLock::new(|| Mutex::new(Vec::new()));

/// Create an event store. Returns store ID.
#[unsafe(no_mangle)]
pub extern "C" fn slang_event_store_create() -> i64 {
    let mut stores = STORES.lock().unwrap();
    let id = stores.len() as i64;
    stores.push(EventStore { events: Vec::new(), snapshot: None, snapshot_at: 0 });
    id
}

/// Append an event. Returns new event count.
#[unsafe(no_mangle)]
pub extern "C" fn slang_event_append(store_id: i64, event_type: i64, payload: i64) -> i64 {
    let mut stores = STORES.lock().unwrap();
    let idx = store_id as usize;
    if idx >= stores.len() { return -1; }
    stores[idx].events.push((event_type, payload));
    stores[idx].events.len() as i64
}

/// Replay all events: sum of all payloads (simplified projection).
#[unsafe(no_mangle)]
pub extern "C" fn slang_event_replay(store_id: i64) -> i64 {
    let stores = STORES.lock().unwrap();
    let idx = store_id as usize;
    if idx >= stores.len() { return -1; }
    stores[idx].events.iter().map(|(_, p)| p).sum()
}

/// Take a snapshot of current state. Returns event count at snapshot.
#[unsafe(no_mangle)]
pub extern "C" fn slang_event_snapshot(store_id: i64) -> i64 {
    let mut stores = STORES.lock().unwrap();
    let idx = store_id as usize;
    if idx >= stores.len() { return -1; }
    let s = &mut stores[idx];
    let state: i64 = s.events.iter().map(|(_, p)| p).sum();
    s.snapshot = Some(state);
    s.snapshot_at = s.events.len() as i64;
    s.snapshot_at
}

/// Project from last snapshot + new events. Returns projected state.
#[unsafe(no_mangle)]
pub extern "C" fn slang_event_project(store_id: i64) -> i64 {
    let stores = STORES.lock().unwrap();
    let idx = store_id as usize;
    if idx >= stores.len() { return -1; }
    let s = &stores[idx];
    let base = s.snapshot.unwrap_or(0);
    let new_events: i64 = s.events[s.snapshot_at as usize..].iter().map(|(_, p)| p).sum();
    base + new_events
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_create() { let id = slang_event_store_create(); assert!(id >= 0); }
    #[test] fn test_append() {
        let id = slang_event_store_create();
        assert_eq!(slang_event_append(id, 1, 100), 1);
        assert_eq!(slang_event_append(id, 2, 200), 2);
    }
    #[test] fn test_replay() {
        let id = slang_event_store_create();
        slang_event_append(id, 1, 10);
        slang_event_append(id, 2, 20);
        assert_eq!(slang_event_replay(id), 30);
    }
    #[test] fn test_snapshot() {
        let id = slang_event_store_create();
        slang_event_append(id, 1, 100);
        assert_eq!(slang_event_snapshot(id), 1);
    }
    #[test] fn test_project() {
        let id = slang_event_store_create();
        slang_event_append(id, 1, 100);
        slang_event_snapshot(id);
        slang_event_append(id, 2, 50);
        assert_eq!(slang_event_project(id), 150);
    }
    #[test] fn test_empty_replay() {
        let id = slang_event_store_create();
        assert_eq!(slang_event_replay(id), 0);
    }
    #[test] fn test_project_no_snapshot() {
        let id = slang_event_store_create();
        slang_event_append(id, 1, 42);
        assert_eq!(slang_event_project(id), 42);
    }
    #[test] fn test_invalid_store() {
        assert_eq!(slang_event_append(999, 1, 1), -1);
        assert_eq!(slang_event_replay(999), -1);
    }
    #[test] fn test_negative_payload() {
        let id = slang_event_store_create();
        slang_event_append(id, 1, -50);
        slang_event_append(id, 2, 100);
        assert_eq!(slang_event_replay(id), 50);
    }
    #[test] fn test_many_events() {
        let id = slang_event_store_create();
        for i in 1..=100 { slang_event_append(id, 1, i); }
        assert_eq!(slang_event_replay(id), 5050);
    }
    #[test] fn test_snapshot_project_multiple() {
        let id = slang_event_store_create();
        for i in 1..=5 { slang_event_append(id, 1, i); }
        slang_event_snapshot(id);
        for i in 6..=10 { slang_event_append(id, 1, i); }
        assert_eq!(slang_event_project(id), 55); // 15 + 40
    }
    #[test] fn test_multiple_stores() {
        let a = slang_event_store_create();
        let b = slang_event_store_create();
        slang_event_append(a, 1, 10);
        slang_event_append(b, 1, 20);
        assert_eq!(slang_event_replay(a), 10);
        assert_eq!(slang_event_replay(b), 20);
    }
    #[test] fn test_snapshot_empty() {
        let id = slang_event_store_create();
        assert_eq!(slang_event_snapshot(id), 0);
    }
    #[test] fn test_zero_payload() {
        let id = slang_event_store_create();
        slang_event_append(id, 1, 0);
        assert_eq!(slang_event_replay(id), 0);
    }
    #[test] fn test_replay_idempotent() {
        let id = slang_event_store_create();
        slang_event_append(id, 1, 42);
        assert_eq!(slang_event_replay(id), 42);
        assert_eq!(slang_event_replay(id), 42); // same result
    }
    #[test] fn test_append_returns_count() {
        let id = slang_event_store_create();
        for i in 1..=10 { assert_eq!(slang_event_append(id, 1, 1), i); }
    }
    #[test] fn test_project_equals_replay_no_snapshot() {
        let id = slang_event_store_create();
        slang_event_append(id, 1, 10);
        slang_event_append(id, 2, 20);
        assert_eq!(slang_event_project(id), slang_event_replay(id));
    }
    #[test] fn test_large_payload() {
        let id = slang_event_store_create();
        slang_event_append(id, 1, 1_000_000_000);
        assert_eq!(slang_event_replay(id), 1_000_000_000);
    }
    #[test] fn test_snapshot_twice() {
        let id = slang_event_store_create();
        slang_event_append(id, 1, 10);
        slang_event_snapshot(id);
        slang_event_append(id, 2, 20);
        slang_event_snapshot(id);
        slang_event_append(id, 3, 30);
        assert_eq!(slang_event_project(id), 60); // snapshot=30, new=30
    }
}
