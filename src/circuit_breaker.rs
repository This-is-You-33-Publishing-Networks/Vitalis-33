//! Circuit Breaker — v391
//! Circuit breaker pattern with closed/open/half-open states and failure thresholds.

use std::sync::{LazyLock, Mutex};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

#[derive(Debug)]
pub struct CircuitBreaker {
    pub id: i64,
    pub state: CircuitState,
    pub failure_count: u64,
    pub success_count: u64,
    pub failure_threshold: u64,
    pub success_threshold: u64,
    pub total_calls: u64,
    pub total_failures: u64,
    pub total_successes: u64,
    pub half_open_max_calls: u64,
    pub half_open_calls: u64,
}

impl CircuitBreaker {
    pub fn new(id: i64, failure_threshold: u64, success_threshold: u64) -> Self {
        Self {
            id,
            state: CircuitState::Closed,
            failure_count: 0,
            success_count: 0,
            failure_threshold,
            success_threshold,
            total_calls: 0,
            total_failures: 0,
            total_successes: 0,
            half_open_max_calls: success_threshold,
            half_open_calls: 0,
        }
    }

    /// Check if a call is allowed.
    pub fn allow_request(&self) -> bool {
        match self.state {
            CircuitState::Closed => true,
            CircuitState::Open => false,
            CircuitState::HalfOpen => self.half_open_calls < self.half_open_max_calls,
        }
    }

    /// Record a successful call.
    pub fn record_success(&mut self) {
        self.total_calls += 1;
        self.total_successes += 1;
        match self.state {
            CircuitState::Closed => {
                self.failure_count = 0;
                self.success_count += 1;
            }
            CircuitState::HalfOpen => {
                self.success_count += 1;
                self.half_open_calls += 1;
                if self.success_count >= self.success_threshold {
                    self.state = CircuitState::Closed;
                    self.failure_count = 0;
                    self.success_count = 0;
                    self.half_open_calls = 0;
                }
            }
            CircuitState::Open => {} // Shouldn't happen
        }
    }

    /// Record a failed call.
    pub fn record_failure(&mut self) {
        self.total_calls += 1;
        self.total_failures += 1;
        match self.state {
            CircuitState::Closed => {
                self.failure_count += 1;
                if self.failure_count >= self.failure_threshold {
                    self.state = CircuitState::Open;
                    self.failure_count = 0;
                    self.success_count = 0;
                }
            }
            CircuitState::HalfOpen => {
                self.state = CircuitState::Open;
                self.failure_count = 0;
                self.success_count = 0;
                self.half_open_calls = 0;
            }
            CircuitState::Open => {}
        }
    }

    /// Attempt to transition to half-open for probing.
    pub fn try_half_open(&mut self) -> bool {
        if self.state == CircuitState::Open {
            self.state = CircuitState::HalfOpen;
            self.half_open_calls = 0;
            self.success_count = 0;
            true
        } else {
            false
        }
    }

    /// Reset to closed state.
    pub fn reset(&mut self) {
        self.state = CircuitState::Closed;
        self.failure_count = 0;
        self.success_count = 0;
        self.half_open_calls = 0;
    }

    pub fn is_open(&self) -> bool {
        self.state == CircuitState::Open
    }

    pub fn is_closed(&self) -> bool {
        self.state == CircuitState::Closed
    }

    pub fn failure_rate(&self) -> f64 {
        if self.total_calls == 0 {
            0.0
        } else {
            self.total_failures as f64 / self.total_calls as f64
        }
    }
}

static CB_STORE: LazyLock<Mutex<HashMap<i64, CircuitBreaker>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
static CB_NEXT_ID: LazyLock<Mutex<i64>> = LazyLock::new(|| Mutex::new(1));

fn cb_alloc() -> i64 {
    let mut next = CB_NEXT_ID.lock().unwrap();
    let id = *next;
    *next += 1;
    id
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_cb_create(failure_threshold: i64, success_threshold: i64) -> i64 {
    let id = cb_alloc();
    let cb = CircuitBreaker::new(id, failure_threshold.max(1) as u64, success_threshold.max(1) as u64);
    CB_STORE.lock().unwrap().insert(id, cb);
    id
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_cb_call(id: i64) -> i64 {
    let store = CB_STORE.lock().unwrap();
    if let Some(cb) = store.get(&id) {
        if cb.allow_request() { 1 } else { 0 }
    } else {
        -1
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_cb_state(id: i64) -> i64 {
    let store = CB_STORE.lock().unwrap();
    if let Some(cb) = store.get(&id) {
        match cb.state {
            CircuitState::Closed => 0,
            CircuitState::Open => 1,
            CircuitState::HalfOpen => 2,
        }
    } else {
        -1
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_cb_record_success(id: i64) -> i64 {
    let mut store = CB_STORE.lock().unwrap();
    if let Some(cb) = store.get_mut(&id) {
        cb.record_success();
        1
    } else {
        -1
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_cb_record_failure(id: i64) -> i64 {
    let mut store = CB_STORE.lock().unwrap();
    if let Some(cb) = store.get_mut(&id) {
        cb.record_failure();
        1
    } else {
        -1
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_cb_reset(id: i64) -> i64 {
    let mut store = CB_STORE.lock().unwrap();
    if let Some(cb) = store.get_mut(&id) {
        cb.reset();
        1
    } else {
        -1
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_cb_failure_count(id: i64) -> i64 {
    let store = CB_STORE.lock().unwrap();
    if let Some(cb) = store.get(&id) {
        cb.total_failures as i64
    } else {
        -1
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_cb_is_open(id: i64) -> i64 {
    let store = CB_STORE.lock().unwrap();
    if let Some(cb) = store.get(&id) {
        if cb.is_open() { 1 } else { 0 }
    } else {
        -1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_circuit_closed() {
        let cb = CircuitBreaker::new(1, 3, 2);
        assert!(cb.is_closed());
        assert!(cb.allow_request());
    }

    #[test]
    fn test_success_stays_closed() {
        let mut cb = CircuitBreaker::new(1, 3, 2);
        cb.record_success();
        cb.record_success();
        assert!(cb.is_closed());
    }

    #[test]
    fn test_failures_open_circuit() {
        let mut cb = CircuitBreaker::new(1, 3, 2);
        cb.record_failure();
        cb.record_failure();
        cb.record_failure();
        assert!(cb.is_open());
    }

    #[test]
    fn test_open_blocks_requests() {
        let mut cb = CircuitBreaker::new(1, 3, 2);
        cb.record_failure();
        cb.record_failure();
        cb.record_failure();
        assert!(!cb.allow_request());
    }

    #[test]
    fn test_half_open_transition() {
        let mut cb = CircuitBreaker::new(1, 3, 2);
        cb.record_failure();
        cb.record_failure();
        cb.record_failure();
        assert!(cb.try_half_open());
        assert_eq!(cb.state, CircuitState::HalfOpen);
    }

    #[test]
    fn test_half_open_allows_requests() {
        let mut cb = CircuitBreaker::new(1, 3, 2);
        cb.record_failure();
        cb.record_failure();
        cb.record_failure();
        cb.try_half_open();
        assert!(cb.allow_request());
    }

    #[test]
    fn test_half_open_success_closes() {
        let mut cb = CircuitBreaker::new(1, 3, 2);
        cb.record_failure();
        cb.record_failure();
        cb.record_failure();
        cb.try_half_open();
        cb.record_success();
        cb.record_success();
        assert!(cb.is_closed());
    }

    #[test]
    fn test_half_open_failure_opens() {
        let mut cb = CircuitBreaker::new(1, 3, 2);
        cb.record_failure();
        cb.record_failure();
        cb.record_failure();
        cb.try_half_open();
        cb.record_failure();
        assert!(cb.is_open());
    }

    #[test]
    fn test_reset() {
        let mut cb = CircuitBreaker::new(1, 3, 2);
        cb.record_failure();
        cb.record_failure();
        cb.record_failure();
        cb.reset();
        assert!(cb.is_closed());
    }

    #[test]
    fn test_try_half_open_from_closed() {
        let mut cb = CircuitBreaker::new(1, 3, 2);
        assert!(!cb.try_half_open()); // Can't go half-open from closed
    }

    #[test]
    fn test_failure_rate() {
        let mut cb = CircuitBreaker::new(1, 10, 2);
        cb.record_success();
        cb.record_success();
        cb.record_failure();
        cb.record_failure();
        assert!((cb.failure_rate() - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_failure_rate_zero() {
        let cb = CircuitBreaker::new(1, 3, 2);
        assert_eq!(cb.failure_rate(), 0.0);
    }

    #[test]
    fn test_total_counts() {
        let mut cb = CircuitBreaker::new(1, 10, 2);
        cb.record_success();
        cb.record_failure();
        cb.record_success();
        assert_eq!(cb.total_calls, 3);
        assert_eq!(cb.total_successes, 2);
        assert_eq!(cb.total_failures, 1);
    }

    #[test]
    fn test_success_resets_failure_count() {
        let mut cb = CircuitBreaker::new(1, 3, 2);
        cb.record_failure();
        cb.record_failure();
        cb.record_success(); // resets failure_count
        assert!(cb.is_closed());
        assert_eq!(cb.failure_count, 0);
    }

    #[test]
    fn test_threshold_boundary() {
        let mut cb = CircuitBreaker::new(1, 2, 1);
        cb.record_failure();
        assert!(cb.is_closed()); // 1 < threshold 2
        cb.record_failure();
        assert!(cb.is_open()); // 2 >= threshold 2
    }

    #[test]
    fn test_half_open_max_calls() {
        let mut cb = CircuitBreaker::new(1, 3, 3);
        cb.record_failure();
        cb.record_failure();
        cb.record_failure();
        cb.try_half_open();
        assert!(cb.allow_request()); // 0 < 3
        cb.record_success();
        assert!(cb.allow_request()); // 1 < 3
        cb.record_success();
        assert!(cb.allow_request()); // 2 < 3
        cb.record_success(); // closes circuit
        assert!(cb.is_closed());
    }

    #[test]
    fn test_mixed_operations() {
        let mut cb = CircuitBreaker::new(1, 5, 3);
        for _ in 0..4 { cb.record_failure(); }
        cb.record_success(); // resets failure count
        assert!(cb.is_closed());
    }

    #[test]
    fn test_consecutive_opens() {
        let mut cb = CircuitBreaker::new(1, 2, 1);
        cb.record_failure();
        cb.record_failure();
        assert!(cb.is_open());
        cb.try_half_open();
        cb.record_failure();
        assert!(cb.is_open());
    }

    #[test]
    fn test_single_failure_threshold() {
        let mut cb = CircuitBreaker::new(1, 1, 1);
        cb.record_failure();
        assert!(cb.is_open());
    }
}
