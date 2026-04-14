//! Load Balancer — v392
//! Round-robin, least-connections, weighted, and consistent hash load balancing.

use std::sync::{LazyLock, Mutex};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum LbStrategy {
    RoundRobin,
    LeastConnections,
    Weighted,
    Random,
}

#[derive(Debug, Clone)]
pub struct Backend {
    pub id: i64,
    pub name: String,
    pub weight: u32,
    pub connections: u64,
    pub healthy: bool,
    pub total_requests: u64,
}

impl Backend {
    pub fn new(id: i64, name: &str, weight: u32) -> Self {
        Self {
            id,
            name: name.to_string(),
            weight,
            connections: 0,
            healthy: true,
            total_requests: 0,
        }
    }
}

#[derive(Debug)]
pub struct LoadBalancer {
    pub id: i64,
    pub strategy: LbStrategy,
    pub backends: Vec<Backend>,
    current_index: usize,
    total_requests: u64,
}

impl LoadBalancer {
    pub fn new(id: i64, strategy: LbStrategy) -> Self {
        Self {
            id,
            strategy,
            backends: Vec::new(),
            current_index: 0,
            total_requests: 0,
        }
    }

    pub fn add_backend(&mut self, name: &str, weight: u32) -> i64 {
        let backend_id = self.backends.len() as i64;
        self.backends.push(Backend::new(backend_id, name, weight));
        backend_id
    }

    pub fn remove_backend(&mut self, backend_id: i64) -> bool {
        let len_before = self.backends.len();
        self.backends.retain(|b| b.id != backend_id);
        self.backends.len() < len_before
    }

    /// Select next backend based on strategy.
    pub fn next(&mut self) -> Option<i64> {
        let healthy: Vec<usize> = self.backends.iter()
            .enumerate()
            .filter(|(_, b)| b.healthy)
            .map(|(i, _)| i)
            .collect();

        if healthy.is_empty() {
            return None;
        }

        self.total_requests += 1;

        let idx = match self.strategy {
            LbStrategy::RoundRobin => {
                let i = self.current_index % healthy.len();
                self.current_index += 1;
                healthy[i]
            }
            LbStrategy::LeastConnections => {
                let &min_idx = healthy.iter()
                    .min_by_key(|&&i| self.backends[i].connections)
                    .unwrap();
                min_idx
            }
            LbStrategy::Weighted => {
                // Weighted round-robin using weight accumulation.
                let total_weight: u32 = healthy.iter()
                    .map(|&i| self.backends[i].weight)
                    .sum();
                if total_weight == 0 {
                    healthy[0]
                } else {
                    let target = (self.total_requests as u32) % total_weight;
                    let mut acc = 0u32;
                    let mut selected = healthy[0];
                    for &i in &healthy {
                        acc += self.backends[i].weight;
                        if target < acc {
                            selected = i;
                            break;
                        }
                    }
                    selected
                }
            }
            LbStrategy::Random => {
                // Deterministic "random" based on request count.
                let i = (self.total_requests as usize * 7 + 3) % healthy.len();
                healthy[i]
            }
        };

        self.backends[idx].connections += 1;
        self.backends[idx].total_requests += 1;
        Some(self.backends[idx].id)
    }

    pub fn set_weight(&mut self, backend_id: i64, weight: u32) -> bool {
        if let Some(b) = self.backends.iter_mut().find(|b| b.id == backend_id) {
            b.weight = weight;
            true
        } else {
            false
        }
    }

    pub fn set_healthy(&mut self, backend_id: i64, healthy: bool) -> bool {
        if let Some(b) = self.backends.iter_mut().find(|b| b.id == backend_id) {
            b.healthy = healthy;
            true
        } else {
            false
        }
    }

    pub fn health_check(&self) -> Vec<(i64, bool)> {
        self.backends.iter().map(|b| (b.id, b.healthy)).collect()
    }

    pub fn backend_count(&self) -> usize {
        self.backends.iter().filter(|b| b.healthy).count()
    }

    pub fn release_connection(&mut self, backend_id: i64) {
        if let Some(b) = self.backends.iter_mut().find(|b| b.id == backend_id) {
            if b.connections > 0 {
                b.connections -= 1;
            }
        }
    }
}

static LB_STORE: LazyLock<Mutex<HashMap<i64, LoadBalancer>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
static LB_NEXT_ID: LazyLock<Mutex<i64>> = LazyLock::new(|| Mutex::new(1));

fn lb_alloc() -> i64 {
    let mut next = LB_NEXT_ID.lock().unwrap();
    let id = *next;
    *next += 1;
    id
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_lb_create(strategy: i64) -> i64 {
    let id = lb_alloc();
    let s = match strategy {
        0 => LbStrategy::RoundRobin,
        1 => LbStrategy::LeastConnections,
        2 => LbStrategy::Weighted,
        _ => LbStrategy::RoundRobin,
    };
    let lb = LoadBalancer::new(id, s);
    LB_STORE.lock().unwrap().insert(id, lb);
    id
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_lb_add_backend(id: i64, weight: i64) -> i64 {
    let mut store = LB_STORE.lock().unwrap();
    if let Some(lb) = store.get_mut(&id) {
        let name = format!("backend_{}", lb.backends.len());
        lb.add_backend(&name, weight.max(1) as u32)
    } else {
        -1
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_lb_remove_backend(id: i64, backend_id: i64) -> i64 {
    let mut store = LB_STORE.lock().unwrap();
    if let Some(lb) = store.get_mut(&id) {
        if lb.remove_backend(backend_id) { 1 } else { 0 }
    } else {
        -1
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_lb_next(id: i64) -> i64 {
    let mut store = LB_STORE.lock().unwrap();
    if let Some(lb) = store.get_mut(&id) {
        lb.next().unwrap_or(-1)
    } else {
        -1
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_lb_backend_count(id: i64) -> i64 {
    let store = LB_STORE.lock().unwrap();
    if let Some(lb) = store.get(&id) {
        lb.backend_count() as i64
    } else {
        -1
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_lb_set_weight(id: i64, backend_id: i64, weight: i64) -> i64 {
    let mut store = LB_STORE.lock().unwrap();
    if let Some(lb) = store.get_mut(&id) {
        if lb.set_weight(backend_id, weight.max(0) as u32) { 1 } else { 0 }
    } else {
        -1
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_lb_health_check(id: i64) -> i64 {
    let store = LB_STORE.lock().unwrap();
    if let Some(lb) = store.get(&id) {
        let healthy = lb.backends.iter().filter(|b| b.healthy).count();
        healthy as i64
    } else {
        -1
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_lb_strategy(id: i64) -> i64 {
    let store = LB_STORE.lock().unwrap();
    if let Some(lb) = store.get(&id) {
        match lb.strategy {
            LbStrategy::RoundRobin => 0,
            LbStrategy::LeastConnections => 1,
            LbStrategy::Weighted => 2,
            LbStrategy::Random => 3,
        }
    } else {
        -1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lb_new() {
        let lb = LoadBalancer::new(1, LbStrategy::RoundRobin);
        assert_eq!(lb.backend_count(), 0);
    }

    #[test]
    fn test_add_backend() {
        let mut lb = LoadBalancer::new(1, LbStrategy::RoundRobin);
        lb.add_backend("server1", 1);
        assert_eq!(lb.backend_count(), 1);
    }

    #[test]
    fn test_remove_backend() {
        let mut lb = LoadBalancer::new(1, LbStrategy::RoundRobin);
        let id = lb.add_backend("server1", 1);
        lb.remove_backend(id);
        assert_eq!(lb.backend_count(), 0);
    }

    #[test]
    fn test_round_robin() {
        let mut lb = LoadBalancer::new(1, LbStrategy::RoundRobin);
        lb.add_backend("a", 1);
        lb.add_backend("b", 1);
        lb.add_backend("c", 1);
        let r1 = lb.next().unwrap();
        let r2 = lb.next().unwrap();
        let r3 = lb.next().unwrap();
        let r4 = lb.next().unwrap();
        // Should cycle through backends.
        assert_eq!(r1, r4); // wraps around
        assert_ne!(r1, r2);
    }

    #[test]
    fn test_least_connections() {
        let mut lb = LoadBalancer::new(1, LbStrategy::LeastConnections);
        lb.add_backend("a", 1);
        lb.add_backend("b", 1);
        // First request goes to first (both have 0 connections).
        let r1 = lb.next().unwrap();
        // Second should go to the one with fewer connections.
        let r2 = lb.next().unwrap();
        assert_ne!(r1, r2);
    }

    #[test]
    fn test_weighted() {
        let mut lb = LoadBalancer::new(1, LbStrategy::Weighted);
        lb.add_backend("heavy", 3);
        lb.add_backend("light", 1);
        let mut heavy_count = 0;
        for _ in 0..4 {
            if lb.next().unwrap() == 0 {
                heavy_count += 1;
            }
        }
        assert!(heavy_count >= 2); // heavy should get more requests
    }

    #[test]
    fn test_empty_lb() {
        let mut lb = LoadBalancer::new(1, LbStrategy::RoundRobin);
        assert!(lb.next().is_none());
    }

    #[test]
    fn test_unhealthy_backend_skipped() {
        let mut lb = LoadBalancer::new(1, LbStrategy::RoundRobin);
        lb.add_backend("a", 1);
        lb.add_backend("b", 1);
        lb.set_healthy(0, false);
        let result = lb.next().unwrap();
        assert_eq!(result, 1); // Only backend 1 is healthy
    }

    #[test]
    fn test_all_unhealthy() {
        let mut lb = LoadBalancer::new(1, LbStrategy::RoundRobin);
        lb.add_backend("a", 1);
        lb.set_healthy(0, false);
        assert!(lb.next().is_none());
    }

    #[test]
    fn test_set_weight() {
        let mut lb = LoadBalancer::new(1, LbStrategy::Weighted);
        lb.add_backend("a", 1);
        assert!(lb.set_weight(0, 5));
        assert_eq!(lb.backends[0].weight, 5);
    }

    #[test]
    fn test_health_check() {
        let mut lb = LoadBalancer::new(1, LbStrategy::RoundRobin);
        lb.add_backend("a", 1);
        lb.add_backend("b", 1);
        let checks = lb.health_check();
        assert_eq!(checks.len(), 2);
        assert!(checks.iter().all(|(_, h)| *h));
    }

    #[test]
    fn test_release_connection() {
        let mut lb = LoadBalancer::new(1, LbStrategy::LeastConnections);
        lb.add_backend("a", 1);
        lb.next(); // adds a connection
        assert_eq!(lb.backends[0].connections, 1);
        lb.release_connection(0);
        assert_eq!(lb.backends[0].connections, 0);
    }

    #[test]
    fn test_total_requests_tracked() {
        let mut lb = LoadBalancer::new(1, LbStrategy::RoundRobin);
        lb.add_backend("a", 1);
        lb.next();
        lb.next();
        assert_eq!(lb.backends[0].total_requests, 2);
    }

    #[test]
    fn test_backend_count_healthy() {
        let mut lb = LoadBalancer::new(1, LbStrategy::RoundRobin);
        lb.add_backend("a", 1);
        lb.add_backend("b", 1);
        lb.add_backend("c", 1);
        lb.set_healthy(1, false);
        assert_eq!(lb.backend_count(), 2);
    }

    #[test]
    fn test_strategy_stored() {
        let lb = LoadBalancer::new(1, LbStrategy::LeastConnections);
        assert_eq!(lb.strategy, LbStrategy::LeastConnections);
    }

    #[test]
    fn test_random_strategy() {
        let mut lb = LoadBalancer::new(1, LbStrategy::Random);
        lb.add_backend("a", 1);
        lb.add_backend("b", 1);
        lb.add_backend("c", 1);
        // Should return a valid backend.
        let r = lb.next().unwrap();
        assert!(r >= 0 && r < 3);
    }

    #[test]
    fn test_set_weight_nonexistent() {
        let mut lb = LoadBalancer::new(1, LbStrategy::Weighted);
        assert!(!lb.set_weight(99, 5));
    }

    #[test]
    fn test_remove_nonexistent() {
        let mut lb = LoadBalancer::new(1, LbStrategy::RoundRobin);
        assert!(!lb.remove_backend(99));
    }

    #[test]
    fn test_release_zero_connections() {
        let mut lb = LoadBalancer::new(1, LbStrategy::RoundRobin);
        lb.add_backend("a", 1);
        lb.release_connection(0); // Should not underflow
        assert_eq!(lb.backends[0].connections, 0);
    }

    #[test]
    fn test_many_round_robin_cycles() {
        let mut lb = LoadBalancer::new(1, LbStrategy::RoundRobin);
        lb.add_backend("a", 1);
        lb.add_backend("b", 1);
        for _ in 0..100 {
            assert!(lb.next().is_some());
        }
    }
}
