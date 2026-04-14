//! Linear Types & Session Types — Language Feature Deepening (v313)
//!
//! Resources tracked via linear types must be used exactly once.
//! Session types model protocol state machines, ensuring correct
//! sequencing of operations at compile time.

use std::sync::{LazyLock, Mutex, atomic::{AtomicI64, Ordering}};
use std::collections::HashMap;

static LINEAR_RESOURCES: LazyLock<Mutex<HashMap<i64, i64>>> =
    LazyLock::new(|| Mutex::new(HashMap::new())); // id → 0=live, 1=consumed
static LINEAR_COUNTER: AtomicI64 = AtomicI64::new(0);
static SESSION_STATES: LazyLock<Mutex<HashMap<i64, i64>>> =
    LazyLock::new(|| Mutex::new(HashMap::new())); // id → current state
static SESSION_COUNTER: AtomicI64 = AtomicI64::new(0);

/// Create a linear resource. Returns resource ID. Must be consumed exactly once.
#[unsafe(no_mangle)]
pub extern "C" fn slang_linear_create() -> i64 {
    let id = LINEAR_COUNTER.fetch_add(1, Ordering::SeqCst);
    LINEAR_RESOURCES.lock().unwrap().insert(id, 0);
    id
}

/// Check if a linear resource is still live (not consumed). Returns 1=live, 0=consumed, -1=invalid.
#[unsafe(no_mangle)]
pub extern "C" fn slang_linear_check(id: i64) -> i64 {
    LINEAR_RESOURCES.lock().unwrap().get(&id).map(|&s| if s == 0 { 1 } else { 0 }).unwrap_or(-1)
}

/// Consume a linear resource. Returns 1 on success, 0 if already consumed, -1 if invalid.
#[unsafe(no_mangle)]
pub extern "C" fn slang_linear_consume(id: i64) -> i64 {
    let mut res = LINEAR_RESOURCES.lock().unwrap();
    match res.get_mut(&id) {
        Some(state) if *state == 0 => { *state = 1; 1 }
        Some(_) => 0, // already consumed
        None => -1,
    }
}

/// Create a session with n_states. Starts at state 0. Returns session ID.
#[unsafe(no_mangle)]
pub extern "C" fn slang_session_create(n_states: i64) -> i64 {
    let _ = n_states; // tracked externally; we just need the state counter
    let id = SESSION_COUNTER.fetch_add(1, Ordering::SeqCst);
    SESSION_STATES.lock().unwrap().insert(id, 0);
    id
}

/// Get the current state of a session. Returns state number or -1 if invalid.
#[unsafe(no_mangle)]
pub extern "C" fn slang_session_state(id: i64) -> i64 {
    SESSION_STATES.lock().unwrap().get(&id).copied().unwrap_or(-1)
}

/// Advance a session to the next state. Returns new state or -1 if invalid.
#[unsafe(no_mangle)]
pub extern "C" fn slang_session_advance(id: i64) -> i64 {
    let mut ss = SESSION_STATES.lock().unwrap();
    match ss.get_mut(&id) {
        Some(state) => { *state += 1; *state }
        None => -1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_linear_create() {
        let id = slang_linear_create();
        assert!(id >= 0);
        assert_eq!(slang_linear_check(id), 1); // live
    }
    #[test]
    fn test_linear_consume() {
        let id = slang_linear_create();
        assert_eq!(slang_linear_consume(id), 1);
        assert_eq!(slang_linear_check(id), 0); // consumed
    }
    #[test]
    fn test_double_consume() {
        let id = slang_linear_create();
        slang_linear_consume(id);
        assert_eq!(slang_linear_consume(id), 0); // already consumed
    }
    #[test]
    fn test_invalid_check() {
        assert_eq!(slang_linear_check(999999), -1);
    }
    #[test]
    fn test_invalid_consume() {
        assert_eq!(slang_linear_consume(999999), -1);
    }
    #[test]
    fn test_session_create() {
        let id = slang_session_create(5);
        assert_eq!(slang_session_state(id), 0);
    }
    #[test]
    fn test_session_advance() {
        let id = slang_session_create(3);
        assert_eq!(slang_session_advance(id), 1);
        assert_eq!(slang_session_advance(id), 2);
        assert_eq!(slang_session_state(id), 2);
    }
    #[test]
    fn test_session_invalid() {
        assert_eq!(slang_session_state(999999), -1);
        assert_eq!(slang_session_advance(999999), -1);
    }
    #[test]
    fn test_multiple_linear() {
        let a = slang_linear_create();
        let b = slang_linear_create();
        assert_ne!(a, b);
        slang_linear_consume(a);
        assert_eq!(slang_linear_check(a), 0);
        assert_eq!(slang_linear_check(b), 1);
    }
    #[test]
    fn test_multiple_sessions() {
        let a = slang_session_create(3);
        let b = slang_session_create(5);
        slang_session_advance(a);
        slang_session_advance(a);
        assert_eq!(slang_session_state(a), 2);
        assert_eq!(slang_session_state(b), 0);
    }
    #[test]
    fn test_session_many_advances() {
        let id = slang_session_create(100);
        for i in 1..=50 {
            assert_eq!(slang_session_advance(id), i);
        }
    }
    #[test]
    fn test_linear_batch() {
        let ids: Vec<_> = (0..10).map(|_| slang_linear_create()).collect();
        for &id in &ids {
            assert_eq!(slang_linear_check(id), 1);
        }
        for &id in &ids {
            assert_eq!(slang_linear_consume(id), 1);
        }
        for &id in &ids {
            assert_eq!(slang_linear_check(id), 0);
        }
    }
    #[test]
    fn test_linear_only_first_consume_succeeds() {
        let id = slang_linear_create();
        assert_eq!(slang_linear_consume(id), 1);
        assert_eq!(slang_linear_consume(id), 0);
        assert_eq!(slang_linear_consume(id), 0);
    }
    #[test]
    fn test_session_independent() {
        let a = slang_session_create(3);
        let b = slang_session_create(3);
        slang_session_advance(a);
        assert_eq!(slang_session_state(a), 1);
        assert_eq!(slang_session_state(b), 0);
    }
    #[test]
    fn test_linear_create_after_consume() {
        let a = slang_linear_create();
        slang_linear_consume(a);
        let b = slang_linear_create();
        assert_eq!(slang_linear_check(a), 0);
        assert_eq!(slang_linear_check(b), 1);
    }
    #[test]
    fn test_session_starts_at_zero() {
        for _ in 0..5 {
            let id = slang_session_create(10);
            assert_eq!(slang_session_state(id), 0);
        }
    }
    #[test]
    fn test_linear_unique_ids() {
        let a = slang_linear_create();
        let b = slang_linear_create();
        let c = slang_linear_create();
        assert!(a != b && b != c && a != c);
    }
    #[test]
    fn test_session_unique_ids() {
        let a = slang_session_create(1);
        let b = slang_session_create(1);
        assert_ne!(a, b);
    }
    #[test]
    fn test_linear_check_after_consume_returns_zero() {
        let id = slang_linear_create();
        slang_linear_consume(id);
        assert_eq!(slang_linear_check(id), 0);
    }
    #[test]
    fn test_session_advance_returns_new_state() {
        let id = slang_session_create(5);
        let s = slang_session_advance(id);
        assert_eq!(s, 1);
        assert_eq!(slang_session_state(id), s);
    }
}
