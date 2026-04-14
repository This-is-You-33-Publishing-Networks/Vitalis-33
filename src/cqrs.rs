//! CQRS — Command Query Responsibility Segregation (v336)
//!
//! Separates write commands from read queries with event projection.

use std::sync::{LazyLock, Mutex};
use std::collections::HashMap;

struct CqrsState {
    commands: Vec<(i64, i64)>, // (aggregate_id, payload)
    projections: HashMap<i64, i64>, // aggregate_id → projected state
    command_count: i64,
    query_count: i64,
}

static CQRS: LazyLock<Mutex<CqrsState>> = LazyLock::new(|| Mutex::new(CqrsState {
    commands: Vec::new(), projections: HashMap::new(), command_count: 0, query_count: 0,
}));

/// Issue a command. Returns command sequence number.
#[unsafe(no_mangle)]
pub extern "C" fn slang_cqrs_command(aggregate_id: i64, payload: i64) -> i64 {
    let mut s = CQRS.lock().unwrap();
    s.commands.push((aggregate_id, payload));
    let prev = s.projections.get(&aggregate_id).copied().unwrap_or(0);
    s.projections.insert(aggregate_id, prev.wrapping_add(payload));
    s.command_count += 1;
    s.command_count
}

/// Query projected state for an aggregate. Returns projected value.
#[unsafe(no_mangle)]
pub extern "C" fn slang_cqrs_query(aggregate_id: i64) -> i64 {
    let mut s = CQRS.lock().unwrap();
    s.query_count += 1;
    s.projections.get(&aggregate_id).copied().unwrap_or(0)
}

/// Return total command count.
#[unsafe(no_mangle)]
pub extern "C" fn slang_cqrs_command_count() -> i64 {
    CQRS.lock().unwrap().command_count
}

/// Return total query count.
#[unsafe(no_mangle)]
pub extern "C" fn slang_cqrs_query_count() -> i64 {
    CQRS.lock().unwrap().query_count
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_command() { let n = slang_cqrs_command(1, 10); assert!(n >= 1); }
    #[test] fn test_query_empty() { assert_eq!(slang_cqrs_query(99999), 0); }
    #[test] fn test_command_then_query() {
        slang_cqrs_command(1000, 42);
        assert_eq!(slang_cqrs_query(1000), 42);
    }
    #[test] fn test_accumulation() {
        slang_cqrs_command(2000, 10);
        slang_cqrs_command(2000, 20);
        assert_eq!(slang_cqrs_query(2000), 30);
    }
    #[test] fn test_command_count() { let _ = slang_cqrs_command(3000, 1); assert!(slang_cqrs_command_count() >= 1); }
    #[test] fn test_query_count() { let _ = slang_cqrs_query(4000); assert!(slang_cqrs_query_count() >= 1); }
    #[test] fn test_multiple_aggregates() {
        slang_cqrs_command(5001, 10);
        slang_cqrs_command(5002, 20);
        assert_eq!(slang_cqrs_query(5001), 10);
        assert_eq!(slang_cqrs_query(5002), 20);
    }
    #[test] fn test_negative_payload() {
        slang_cqrs_command(6000, 100);
        slang_cqrs_command(6000, -30);
        assert_eq!(slang_cqrs_query(6000), 70);
    }
    #[test] fn test_zero_payload() {
        slang_cqrs_command(7000, 0);
        assert_eq!(slang_cqrs_query(7000), 0);
    }
    #[test] fn test_command_returns_seq() {
        let a = slang_cqrs_command(8000, 1);
        let b = slang_cqrs_command(8000, 2);
        assert!(b > a);
    }
    #[test] fn test_separate_aggregates_isolated() {
        slang_cqrs_command(9001, 100);
        assert_eq!(slang_cqrs_query(9002), 0);
    }
    #[test] fn test_large_payload() {
        slang_cqrs_command(10000, i64::MAX / 2);
        assert_eq!(slang_cqrs_query(10000), i64::MAX / 2);
    }
    #[test] fn test_many_commands() {
        for i in 0..50 { slang_cqrs_command(11000, 1); }
        assert_eq!(slang_cqrs_query(11000), 50);
    }
    #[test] fn test_query_idempotent() {
        slang_cqrs_command(12000, 5);
        let a = slang_cqrs_query(12000);
        let b = slang_cqrs_query(12000);
        assert_eq!(a, b);
    }
    #[test] fn test_overwrite_behavior() {
        slang_cqrs_command(13000, 10);
        slang_cqrs_command(13000, -10);
        assert_eq!(slang_cqrs_query(13000), 0);
    }
    #[test] fn test_wrapping_add() {
        slang_cqrs_command(14000, i64::MAX);
        slang_cqrs_command(14000, 1);
        // wrapping_add should handle overflow
        assert_eq!(slang_cqrs_query(14000), i64::MIN);
    }
    #[test] fn test_command_count_increments() {
        let before = slang_cqrs_command_count();
        slang_cqrs_command(15000, 1);
        assert_eq!(slang_cqrs_command_count(), before + 1);
    }
    #[test] fn test_query_count_increments() {
        let before = slang_cqrs_query_count();
        slang_cqrs_query(16000);
        assert_eq!(slang_cqrs_query_count(), before + 1);
    }
    #[test] fn test_distinct_ids() {
        for i in 17000..17010 { slang_cqrs_command(i, i); }
        for i in 17000..17010 { assert_eq!(slang_cqrs_query(i), i); }
    }
    #[test] fn test_negative_aggregate_id() {
        slang_cqrs_command(-1, 99);
        assert_eq!(slang_cqrs_query(-1), 99);
    }
}
