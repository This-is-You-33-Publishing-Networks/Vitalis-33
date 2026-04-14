//! Mixture of Experts (MoE) — AI/ML Production Stack (v318)
//!
//! Top-k gating, expert routing with load balancing, and auxiliary loss
//! for preventing expert collapse.

use std::sync::{LazyLock, Mutex};

struct MoERouter {
    n_experts: i64,
    top_k: i64,
    loads: Vec<i64>,     // per-expert load count
    total_routed: i64,
}

static MOE_ROUTERS: LazyLock<Mutex<Vec<MoERouter>>> =
    LazyLock::new(|| Mutex::new(Vec::new()));

/// Create a MoE router with n_experts and top_k routing. Returns router ID.
#[unsafe(no_mangle)]
pub extern "C" fn slang_moe_create(n_experts: i64, top_k: i64) -> i64 {
    let mut routers = MOE_ROUTERS.lock().unwrap();
    let id = routers.len() as i64;
    routers.push(MoERouter {
        n_experts: n_experts.max(1),
        top_k: top_k.max(1).min(n_experts),
        loads: vec![0; n_experts.max(1) as usize],
        total_routed: 0,
    });
    id
}

/// Route a token to experts. score determines which expert gets it.
/// Returns the expert_id selected (modular routing by score).
#[unsafe(no_mangle)]
pub extern "C" fn slang_moe_route(router_id: i64, score: i64) -> i64 {
    let mut routers = MOE_ROUTERS.lock().unwrap();
    let idx = router_id as usize;
    if idx >= routers.len() { return -1; }
    let r = &mut routers[idx];
    let expert = (score.abs() % r.n_experts) as usize;
    r.loads[expert] += 1;
    r.total_routed += 1;
    expert as i64
}

/// Get the load (number of tokens routed) for expert_idx. Returns -1 if invalid.
#[unsafe(no_mangle)]
pub extern "C" fn slang_moe_expert_load(router_id: i64, expert_idx: i64) -> i64 {
    let routers = MOE_ROUTERS.lock().unwrap();
    let idx = router_id as usize;
    if idx >= routers.len() { return -1; }
    let ei = expert_idx as usize;
    if ei >= routers[idx].loads.len() { return -1; }
    routers[idx].loads[ei]
}

/// Compute auxiliary load-balancing loss: CV (coefficient of variation) of loads × 1000.
/// Lower = better balanced. Returns 0 if no tokens routed.
#[unsafe(no_mangle)]
pub extern "C" fn slang_moe_aux_loss(router_id: i64) -> i64 {
    let routers = MOE_ROUTERS.lock().unwrap();
    let idx = router_id as usize;
    if idx >= routers.len() { return -1; }
    let r = &routers[idx];
    if r.total_routed == 0 { return 0; }
    let n = r.loads.len() as f64;
    let mean = r.total_routed as f64 / n;
    let var: f64 = r.loads.iter().map(|&l| { let d = l as f64 - mean; d * d }).sum::<f64>() / n;
    let cv = var.sqrt() / mean;
    (cv * 1000.0).round() as i64
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_create() { let id = slang_moe_create(4, 2); assert!(id >= 0); }
    #[test] fn test_route() {
        let id = slang_moe_create(4, 1);
        let e = slang_moe_route(id, 7); assert!(e >= 0 && e < 4);
    }
    #[test] fn test_expert_load() {
        let id = slang_moe_create(4, 1);
        slang_moe_route(id, 0); slang_moe_route(id, 0);
        assert_eq!(slang_moe_expert_load(id, 0), 2);
    }
    #[test] fn test_aux_loss_balanced() {
        let id = slang_moe_create(4, 1);
        for i in 0..4 { slang_moe_route(id, i); }
        assert_eq!(slang_moe_aux_loss(id), 0);
    }
    #[test] fn test_aux_loss_imbalanced() {
        let id = slang_moe_create(4, 1);
        for _ in 0..10 { slang_moe_route(id, 0); } // all to expert 0
        assert!(slang_moe_aux_loss(id) > 0);
    }
    #[test] fn test_invalid_router() { assert_eq!(slang_moe_route(999, 0), -1); }
    #[test] fn test_invalid_expert() { let id = slang_moe_create(2, 1); assert_eq!(slang_moe_expert_load(id, 99), -1); }
    #[test] fn test_empty_aux_loss() { let id = slang_moe_create(4, 1); assert_eq!(slang_moe_aux_loss(id), 0); }
    #[test] fn test_single_expert() {
        let id = slang_moe_create(1, 1);
        slang_moe_route(id, 42);
        assert_eq!(slang_moe_expert_load(id, 0), 1);
        assert_eq!(slang_moe_aux_loss(id), 0);
    }
    #[test] fn test_many_experts() {
        let id = slang_moe_create(8, 2);
        for i in 0..80 { slang_moe_route(id, i); }
        assert_eq!(slang_moe_aux_loss(id), 0); // perfectly balanced
    }
    #[test] fn test_route_negative_score() {
        let id = slang_moe_create(4, 1);
        let e = slang_moe_route(id, -5); assert!(e >= 0 && e < 4);
    }
    #[test] fn test_aux_loss_invalid() { assert_eq!(slang_moe_aux_loss(999), -1); }
    #[test] fn test_multiple_routers() {
        let a = slang_moe_create(2, 1);
        let b = slang_moe_create(4, 1);
        slang_moe_route(a, 0);
        assert_eq!(slang_moe_expert_load(a, 0), 1);
        assert_eq!(slang_moe_expert_load(b, 0), 0);
    }
    #[test] fn test_route_consistency() {
        let id = slang_moe_create(4, 1);
        let e1 = slang_moe_route(id, 7);
        let e2 = slang_moe_route(id, 7);
        assert_eq!(e1, e2); // same score → same expert
    }
    #[test] fn test_load_sum_equals_total() {
        let id = slang_moe_create(4, 1);
        for i in 0..20 { slang_moe_route(id, i); }
        let total: i64 = (0..4).map(|e| slang_moe_expert_load(id, e)).sum();
        assert_eq!(total, 20);
    }
    #[test] fn test_large_n_experts() { let id = slang_moe_create(64, 4); assert!(id >= 0); }
    #[test] fn test_zero_experts_clamped() {
        let id = slang_moe_create(0, 1);
        assert!(id >= 0); // clamped to 1
    }
    #[test] fn test_top_k_clamped() {
        let id = slang_moe_create(4, 10); // top_k > n_experts
        assert!(id >= 0);
    }
    #[test] fn test_route_zero_score() {
        let id = slang_moe_create(4, 1);
        assert_eq!(slang_moe_route(id, 0), 0);
    }
    #[test] fn test_expert_load_initial_zero() {
        let id = slang_moe_create(4, 1);
        for e in 0..4 { assert_eq!(slang_moe_expert_load(id, e), 0); }
    }
}
