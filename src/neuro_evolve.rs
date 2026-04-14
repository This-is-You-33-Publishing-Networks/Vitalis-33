//! Neuro-Evolution & Architecture Search (v243–v248)
//!
//! NEAT v2, self-organizing maps, neural architecture search,
//! spike-based RL, curiosity-driven exploration, meta-learning.

use std::sync::atomic::{AtomicI64, Ordering};

// ── v243: NEAT v2 ────────────────────────────────────────────────────────────

static NEAT_GENERATION: AtomicI64 = AtomicI64::new(0);
static NEAT_SPECIES: AtomicI64 = AtomicI64::new(0);

/// NEAT genome crossover. Returns offspring fitness ×1000.
#[unsafe(no_mangle)]
pub extern "C" fn slang_neat_crossover(parent_a: f64, parent_b: f64) -> i64 {
    let offspring = (parent_a + parent_b) / 2.0 * 1.05; // slight hybrid vigor
    (offspring * 1000.0).round() as i64
}

/// NEAT structural mutation. Returns 1 (add node), 2 (add connection), 0 (weight only).
#[unsafe(no_mangle)]
pub extern "C" fn slang_neat_mutate(current_nodes: i64, max_nodes: i64) -> i64 {
    if current_nodes < max_nodes / 2 { 1 }
    else if current_nodes < max_nodes { 2 }
    else { 0 }
}

/// NEAT speciation: compute compatibility distance. Returns distance ×1000.
#[unsafe(no_mangle)]
pub extern "C" fn slang_neat_speciate(excess: i64, disjoint: i64, weight_diff: f64) -> i64 {
    let c1 = 1.0; let c2 = 1.0; let c3 = 0.4;
    let dist = c1 * excess as f64 + c2 * disjoint as f64 + c3 * weight_diff;
    (dist * 1000.0).round() as i64
}

/// Advance NEAT generation. Returns generation number.
#[unsafe(no_mangle)]
pub extern "C" fn slang_neat_generation() -> i64 {
    NEAT_GENERATION.fetch_add(1, Ordering::SeqCst) + 1
}

// ── v244: Self-Organizing Maps ───────────────────────────────────────────────

/// SOM: find best matching unit (BMU). Returns BMU index.
#[unsafe(no_mangle)]
pub extern "C" fn slang_som_bmu(input_x: f64, input_y: f64, grid_size: i64) -> i64 {
    // Simple grid mapping
    let gx = (input_x * grid_size as f64).clamp(0.0, (grid_size - 1) as f64).round() as i64;
    let gy = (input_y * grid_size as f64).clamp(0.0, (grid_size - 1) as f64).round() as i64;
    gy * grid_size + gx
}

/// SOM: compute neighbourhood radius. Returns radius.
#[unsafe(no_mangle)]
pub extern "C" fn slang_som_radius(initial_radius: f64, iteration: i64, time_constant: i64) -> i64 {
    let radius = initial_radius * (-iteration as f64 / time_constant.max(1) as f64).exp();
    radius.ceil() as i64
}

/// SOM: learning rate decay. Returns learning rate ×1000.
#[unsafe(no_mangle)]
pub extern "C" fn slang_som_learning_rate(initial: f64, iteration: i64, max_iter: i64) -> i64 {
    let lr = initial * (1.0 - iteration as f64 / max_iter.max(1) as f64);
    (lr.max(0.0) * 1000.0).round() as i64
}

/// SOM: quantization error. Returns error ×1000.
#[unsafe(no_mangle)]
pub extern "C" fn slang_som_quant_error(dist_to_bmu: f64) -> i64 {
    (dist_to_bmu * 1000.0).round() as i64
}

// ── v245: Neural Architecture Search ─────────────────────────────────────────

static NAS_BEST_SCORE: AtomicI64 = AtomicI64::new(0);

/// NAS: evaluate architecture candidate. Returns score ×1000.
#[unsafe(no_mangle)]
pub extern "C" fn slang_neuro_nas_evaluate(accuracy: f64, params_m: f64, latency_ms: f64) -> i64 {
    let score = accuracy * 0.6 + (1.0 / (1.0 + params_m)) * 0.2 + (1.0 / (1.0 + latency_ms)) * 0.2;
    let s = (score * 1000.0).round() as i64;
    NAS_BEST_SCORE.fetch_max(s, Ordering::SeqCst);
    s
}

/// NAS: sample architecture from search space. Returns architecture hash.
#[unsafe(no_mangle)]
pub extern "C" fn slang_neuro_nas_sample(n_cells: i64, n_ops: i64) -> i64 {
    n_cells * 1000 + n_ops
}

/// NAS: prune architecture. Returns remaining parameters.
#[unsafe(no_mangle)]
pub extern "C" fn slang_neuro_nas_prune(total_params: i64, prune_ratio: f64) -> i64 {
    (total_params as f64 * (1.0 - prune_ratio.clamp(0.0, 0.99))).round() as i64
}

/// NAS: get best score found.
#[unsafe(no_mangle)]
pub extern "C" fn slang_neuro_nas_best_score() -> i64 {
    NAS_BEST_SCORE.load(Ordering::SeqCst)
}

// ── v246: Spike-Based Reinforcement Learning ─────────────────────────────────

/// Spike R-STDP: reward-modulated plasticity. Returns weight change ×1000.
#[unsafe(no_mangle)]
pub extern "C" fn slang_spike_rl_rstdp(pre_post_dt: f64, reward: f64) -> i64 {
    let stdp = if pre_post_dt > 0.0 { (-pre_post_dt / 20.0).exp() }
               else { -(-pre_post_dt / 20.0).exp() * 0.5 };
    (stdp * reward * 1000.0).round() as i64
}

/// Spike TD error. Returns TD error ×1000.
#[unsafe(no_mangle)]
pub extern "C" fn slang_spike_rl_td(reward: f64, gamma: f64, v_next: f64, v_curr: f64) -> i64 {
    let td = reward + gamma * v_next - v_curr;
    (td * 1000.0).round() as i64
}

/// Spike policy spike rate (softmax). Returns rate ×1000.
#[unsafe(no_mangle)]
pub extern "C" fn slang_spike_rl_policy(q_value: f64, temperature: f64) -> i64 {
    let rate = 1.0 / (1.0 + (-q_value / temperature.max(0.01)).exp());
    (rate * 1000.0).round() as i64
}

/// Spike reward prediction. Returns predicted reward ×1000.
#[unsafe(no_mangle)]
pub extern "C" fn slang_spike_rl_predict_reward(spike_rate: f64, weight: f64) -> i64 {
    (spike_rate * weight * 1000.0).round() as i64
}

// ── v247: Curiosity-Driven Exploration ───────────────────────────────────────

/// Intrinsic curiosity reward. Returns curiosity signal ×1000.
#[unsafe(no_mangle)]
pub extern "C" fn slang_curiosity_reward(prediction_error: f64) -> i64 {
    let reward = prediction_error.abs().min(10.0);
    (reward * 1000.0).round() as i64
}

/// Information gain estimate. Returns info gain (bits) ×1000.
#[unsafe(no_mangle)]
pub extern "C" fn slang_curiosity_info_gain(prior_entropy: f64, posterior_entropy: f64) -> i64 {
    let gain = (prior_entropy - posterior_entropy).max(0.0);
    (gain * 1000.0).round() as i64
}

/// Novelty score based on distance to k-nearest. Returns score ×1000.
#[unsafe(no_mangle)]
pub extern "C" fn slang_curiosity_novelty(avg_knn_dist: f64) -> i64 {
    (avg_knn_dist * 1000.0).min(10000.0) as i64
}

/// Exploration bonus decay. Returns bonus ×1000.
#[unsafe(no_mangle)]
pub extern "C" fn slang_curiosity_decay(initial_bonus: f64, visit_count: i64) -> i64 {
    let bonus = initial_bonus / (1.0 + visit_count as f64).sqrt();
    (bonus * 1000.0).round() as i64
}

// ── v248: Meta-Learning ──────────────────────────────────────────────────────

/// MAML inner loop update. Returns adapted parameter ×1000.
#[unsafe(no_mangle)]
pub extern "C" fn slang_meta_maml_adapt(param: f64, grad: f64, alpha: f64) -> i64 {
    let adapted = param - alpha * grad;
    (adapted * 1000.0).round() as i64
}

/// Reptile meta-update. Returns interpolated parameter ×1000.
#[unsafe(no_mangle)]
pub extern "C" fn slang_meta_reptile(theta: f64, phi: f64, epsilon: f64) -> i64 {
    let updated = theta + epsilon * (phi - theta);
    (updated * 1000.0).round() as i64
}

/// Task similarity for meta-learning. Returns similarity (0-1000).
#[unsafe(no_mangle)]
pub extern "C" fn slang_meta_task_similarity(grad_dot: f64, grad_norm_a: f64, grad_norm_b: f64) -> i64 {
    let denom = grad_norm_a * grad_norm_b;
    if denom <= 0.0 { return 0; }
    let sim = (grad_dot / denom).clamp(-1.0, 1.0);
    ((sim + 1.0) * 500.0) as i64
}

/// Meta learning progress. Returns convergence rate ×1000.
#[unsafe(no_mangle)]
pub extern "C" fn slang_meta_convergence(loss_start: f64, loss_end: f64, n_steps: i64) -> i64 {
    if n_steps <= 0 { return 0; }
    let rate = (loss_start - loss_end) / (n_steps as f64 * loss_start.max(0.001));
    (rate * 1000.0).round() as i64
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_neat_crossover() {
        let f = slang_neat_crossover(0.8, 0.6);
        assert!(f > 700); // (0.8+0.6)/2 * 1.05 = 0.735 → 735
    }

    #[test]
    fn test_neat_speciate() {
        let d = slang_neat_speciate(3, 2, 0.5);
        assert_eq!(d, 5200); // 3+2+0.4*0.5=5.2 → 5200
    }

    #[test]
    fn test_som_bmu() {
        let bmu = slang_som_bmu(0.5, 0.5, 10);
        assert_eq!(bmu, 55); // 5*10+5
    }

    #[test]
    fn test_neuro_nas_evaluate() {
        let s = slang_neuro_nas_evaluate(0.95, 10.0, 5.0);
        assert!(s > 0);
    }

    #[test]
    fn test_spike_rl_td() {
        let td = slang_spike_rl_td(1.0, 0.99, 0.5, 0.3);
        assert!(td > 0); // 1.0 + 0.99*0.5 - 0.3 = 1.195 → 1195
    }

    #[test]
    fn test_curiosity_reward() {
        let r = slang_curiosity_reward(2.5);
        assert_eq!(r, 2500);
    }

    #[test]
    fn test_meta_maml() {
        let p = slang_meta_maml_adapt(1.0, 0.5, 0.01);
        assert_eq!(p, 995); // (1.0 - 0.01*0.5)*1000
    }

    #[test]
    fn test_meta_reptile() {
        let p = slang_meta_reptile(1.0, 2.0, 0.1);
        assert_eq!(p, 1100); // 1.0 + 0.1*(2.0-1.0) = 1.1 → 1100
    }

    #[test]
    fn test_spike_rl_policy() {
        let r = slang_spike_rl_policy(0.0, 1.0);
        assert_eq!(r, 500); // sigmoid(0) = 0.5 → 500
    }

    #[test]
    fn test_curiosity_decay() {
        let b = slang_curiosity_decay(10.0, 99);
        assert_eq!(b, 1000); // 10.0/sqrt(100) = 1.0 → 1000
    }
}
