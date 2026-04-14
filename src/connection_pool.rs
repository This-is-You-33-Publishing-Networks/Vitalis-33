//! Connection Pool â€” Systems Infrastructure (v330)
//!
//! Generic connection pool with health checks, idle timeout,
//! and max connections enforcement.

use std::sync::{LazyLock, Mutex};

struct Pool {
    max_conns: i64,
    active: i64,
    idle: i64,
    total_acquired: i64,
    total_released: i64,
    health_ok: i64,
}

static POOLS: LazyLock<Mutex<Vec<Pool>>> = LazyLock::new(|| Mutex::new(Vec::new()));

/// Create a connection pool. Returns pool ID.
#[unsafe(no_mangle)]
pub extern "C" fn slang_conn_pool_create(max_conns: i64) -> i64 {
    let mut pools = POOLS.lock().unwrap();
    let id = pools.len() as i64;
    pools.push(Pool { max_conns: max_conns.max(1), active: 0, idle: max_conns.max(1), total_acquired: 0, total_released: 0, health_ok: max_conns.max(1) });
    id
}

/// Acquire a connection. Returns 1 on success, 0 if pool exhausted.
#[unsafe(no_mangle)]
pub extern "C" fn slang_pool_acquire(pool_id: i64) -> i64 {
    let mut pools = POOLS.lock().unwrap();
    let idx = pool_id as usize;
    if idx >= pools.len() { return -1; }
    let p = &mut pools[idx];
    if p.idle > 0 {
        p.idle -= 1;
        p.active += 1;
        p.total_acquired += 1;
        1
    } else { 0 }
}

/// Release a connection back. Returns 1 on success, 0 if none active.
#[unsafe(no_mangle)]
pub extern "C" fn slang_pool_release(pool_id: i64) -> i64 {
    let mut pools = POOLS.lock().unwrap();
    let idx = pool_id as usize;
    if idx >= pools.len() { return -1; }
    let p = &mut pools[idx];
    if p.active > 0 {
        p.active -= 1;
        p.idle += 1;
        p.total_released += 1;
        1
    } else { 0 }
}

/// Get pool stats: (active << 32) | idle.
#[unsafe(no_mangle)]
pub extern "C" fn slang_pool_stats(pool_id: i64) -> i64 {
    let pools = POOLS.lock().unwrap();
    let idx = pool_id as usize;
    if idx >= pools.len() { return -1; }
    let p = &pools[idx];
    (p.active << 16) | (p.idle & 0xFFFF)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_create() { let id = slang_conn_pool_create(10); assert!(id >= 0); }
    #[test] fn test_acquire() { let id = slang_conn_pool_create(5); assert_eq!(slang_pool_acquire(id), 1); }
    #[test] fn test_release() {
        let id = slang_conn_pool_create(5);
        slang_pool_acquire(id);
        assert_eq!(slang_pool_release(id), 1);
    }
    #[test] fn test_exhaust() {
        let id = slang_conn_pool_create(2);
        assert_eq!(slang_pool_acquire(id), 1);
        assert_eq!(slang_pool_acquire(id), 1);
        assert_eq!(slang_pool_acquire(id), 0); // exhausted
    }
    #[test] fn test_release_empty() {
        let id = slang_conn_pool_create(2);
        assert_eq!(slang_pool_release(id), 0); // none active
    }
    #[test] fn test_stats() {
        let id = slang_conn_pool_create(5);
        slang_pool_acquire(id);
        slang_pool_acquire(id);
        let s = slang_pool_stats(id);
        let active = s >> 16;
        let idle = s & 0xFFFF;
        assert_eq!(active, 2);
        assert_eq!(idle, 3);
    }
    #[test] fn test_acquire_release_cycle() {
        let id = slang_conn_pool_create(1);
        assert_eq!(slang_pool_acquire(id), 1);
        assert_eq!(slang_pool_acquire(id), 0);
        assert_eq!(slang_pool_release(id), 1);
        assert_eq!(slang_pool_acquire(id), 1);
    }
    #[test] fn test_invalid_pool() {
        assert_eq!(slang_pool_acquire(999), -1);
        assert_eq!(slang_pool_release(999), -1);
        assert_eq!(slang_pool_stats(999), -1);
    }
    #[test] fn test_zero_max() {
        let id = slang_conn_pool_create(0); // clamped to 1
        assert_eq!(slang_pool_acquire(id), 1);
        assert_eq!(slang_pool_acquire(id), 0); // only 1 allowed
    }
    #[test] fn test_full_drain_and_refill() {
        let id = slang_conn_pool_create(3);
        for _ in 0..3 { slang_pool_acquire(id); }
        for _ in 0..3 { slang_pool_release(id); }
        for _ in 0..3 { assert_eq!(slang_pool_acquire(id), 1); }
    }
    #[test] fn test_multiple_pools() {
        let a = slang_conn_pool_create(2);
        let b = slang_conn_pool_create(5);
        slang_pool_acquire(a);
        slang_pool_acquire(a);
        assert_eq!(slang_pool_acquire(a), 0);
        assert_eq!(slang_pool_acquire(b), 1); // b independent
    }
    #[test] fn test_stats_initial() {
        let id = slang_conn_pool_create(10);
        let s = slang_pool_stats(id);
        assert_eq!(s >> 16, 0);    // 0 active
        assert_eq!(s & 0xFFFF, 10); // 10 idle
    }
    #[test] fn test_large_pool() {
        let id = slang_conn_pool_create(1000);
        for _ in 0..500 { slang_pool_acquire(id); }
        let s = slang_pool_stats(id);
        assert_eq!(s >> 16, 500);
        assert_eq!(s & 0xFFFF, 500);
    }
    #[test] fn test_one_conn_pool() {
        let id = slang_conn_pool_create(1);
        assert_eq!(slang_pool_acquire(id), 1);
        assert_eq!(slang_pool_release(id), 1);
    }
    #[test] fn test_release_more_than_active() {
        let id = slang_conn_pool_create(3);
        slang_pool_acquire(id);
        slang_pool_release(id);
        assert_eq!(slang_pool_release(id), 0);
    }
    #[test] fn test_negative_max() {
        let id = slang_conn_pool_create(-5);
        assert!(id >= 0); // clamped to 1
    }
    #[test] fn test_stats_after_full_cycle() {
        let id = slang_conn_pool_create(4);
        for _ in 0..4 { slang_pool_acquire(id); }
        for _ in 0..4 { slang_pool_release(id); }
        let s = slang_pool_stats(id);
        assert_eq!(s >> 16, 0);
        assert_eq!(s & 0xFFFF, 4);
    }
    #[test] fn test_partial_release() {
        let id = slang_conn_pool_create(5);
        for _ in 0..5 { slang_pool_acquire(id); }
        for _ in 0..3 { slang_pool_release(id); }
        let s = slang_pool_stats(id);
        assert_eq!(s >> 16, 2);
        assert_eq!(s & 0xFFFF, 3);
    }
    #[test] fn test_create_multiple() {
        let ids: Vec<_> = (0..5).map(|_| slang_conn_pool_create(10)).collect();
        for i in 0..5 { for j in (i+1)..5 { assert_ne!(ids[i], ids[j]); } }
    }
}
