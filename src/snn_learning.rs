//! SNN Learning Engine (v213–v218)
//!
//! Surrogate gradient learning, BPTT for SNNs, evolutionary SNN optimization,
//! federated SNN learning, transfer learning, and online/continual learning.

use std::sync::atomic::{AtomicI64, Ordering};

// ── v213: Surrogate Gradient Learning ────────────────────────────────────────

/// Surrogate gradient forward pass: LIF neuron with smooth surrogate.
/// Returns spike count (neurons exceeding threshold after n_steps).
#[unsafe(no_mangle)]
pub extern "C" fn slang_snn_surrogate_forward(input: f64, threshold: f64, decay: f64, n_steps: i64) -> i64 {
    let mut v = 0.0_f64;
    let mut spikes = 0_i64;
    for _ in 0..n_steps {
        v = v * decay + input;
        if v >= threshold {
            spikes += 1;
            v = 0.0;
        }
    }
    spikes
}

/// Surrogate gradient backward: fast sigmoid surrogate dS/dV = 1/(1+k|V-thr|)^2.
/// Returns gradient as fixed-point ×1000.
#[unsafe(no_mangle)]
pub extern "C" fn slang_snn_surrogate_backward(voltage: f64, threshold: f64, k: f64) -> i64 {
    let diff = (voltage - threshold).abs();
    let grad = 1.0 / (1.0 + k * diff).powi(2);
    (grad * 1000.0).round() as i64
}

/// Surrogate sigmoid function: σ(x) = x / (2 * (1 + |x|)) + 0.5.
/// Returns value as fixed-point ×1000.
#[unsafe(no_mangle)]
pub extern "C" fn slang_snn_surrogate_sigmoid(x: f64) -> i64 {
    let result = x / (2.0 * (1.0 + x.abs())) + 0.5;
    (result * 1000.0).round() as i64
}

/// Compute surrogate loss: sum of (target_spikes - actual_spikes)^2 over n neurons.
/// Returns MSE loss as fixed-point ×1000.
#[unsafe(no_mangle)]
pub extern "C" fn slang_snn_surrogate_loss(target_rate: f64, actual_rate: f64, n: i64) -> i64 {
    let diff = target_rate - actual_rate;
    let mse = diff * diff / n.max(1) as f64;
    (mse * 1000.0).round() as i64
}

// ── v214: BPTT for SNNs ─────────────────────────────────────────────────────

/// Forward pass for BPTT-SNN: accumulate voltage over timesteps.
/// Returns total spike count.
#[unsafe(no_mangle)]
pub extern "C" fn slang_snn_bptt_forward(input: f64, threshold: f64, decay: f64, timesteps: i64) -> i64 {
    let mut v = 0.0_f64;
    let mut spikes = 0_i64;
    for _ in 0..timesteps {
        v = v * decay + input;
        if v >= threshold {
            spikes += 1;
            v -= threshold; // soft reset
        }
    }
    spikes
}

/// BPTT backward pass: compute gradient through time.
/// Returns accumulated gradient as fixed-point ×1000.
#[unsafe(no_mangle)]
pub extern "C" fn slang_snn_bptt_backward(loss_grad: f64, decay: f64, timesteps: i64) -> i64 {
    let mut grad = loss_grad;
    let mut total = 0.0_f64;
    for _ in 0..timesteps {
        total += grad;
        grad *= decay; // gradient decays through time
    }
    (total * 1000.0).round() as i64
}

/// Truncated BPTT: limit backprop to window_size steps.
/// Returns gradient as fixed-point ×1000.
#[unsafe(no_mangle)]
pub extern "C" fn slang_snn_bptt_truncate(loss_grad: f64, decay: f64, window: i64) -> i64 {
    let mut grad = loss_grad;
    let mut total = 0.0_f64;
    for _ in 0..window.min(100) {
        total += grad;
        grad *= decay;
    }
    (total * 1000.0).round() as i64
}

/// Compute BPTT gradient norm for gradient clipping.
/// Returns L2 norm of gradient vector (simulated) ×1000.
#[unsafe(no_mangle)]
pub extern "C" fn slang_snn_bptt_gradient(grad_sum: f64, n_params: i64) -> i64 {
    if n_params <= 0 { return 0; }
    let norm = (grad_sum * grad_sum / n_params as f64).sqrt();
    (norm * 1000.0).round() as i64
}

// ── v215: Evolutionary SNN Optimization ──────────────────────────────────────

static SNN_NAS_BEST: AtomicI64 = AtomicI64::new(0);

/// NAS search: evaluate an SNN architecture defined by (layers, neurons_per_layer, connectivity).
/// Returns fitness score (higher is better).
#[unsafe(no_mangle)]
pub extern "C" fn slang_snn_nas_search(layers: i64, neurons_per_layer: i64, connectivity: f64) -> i64 {
    // Fitness: balance accuracy (more neurons) with efficiency (fewer connections)
    let capacity = layers * neurons_per_layer;
    let efficiency = 1.0 - connectivity.clamp(0.0, 1.0);
    let fitness = (capacity as f64 * (0.5 + 0.5 * efficiency)) as i64;
    let prev_best = SNN_NAS_BEST.load(Ordering::SeqCst);
    if fitness > prev_best {
        SNN_NAS_BEST.store(fitness, Ordering::SeqCst);
    }
    fitness
}

/// Evaluate SNN architecture fitness on a task.
/// Returns accuracy score ×1000 (simulated).
#[unsafe(no_mangle)]
pub extern "C" fn slang_snn_nas_evaluate(layers: i64, neurons: i64) -> i64 {
    // Simulated accuracy: diminishing returns with more layers
    let base = 700_i64; // 70% baseline
    let gain = (layers.min(10) * 20).min(250);
    let neuron_bonus = (neurons.min(1000) / 10).min(50);
    base + gain + neuron_bonus
}

/// Mutate SNN architecture: perturb layer/neuron counts.
/// Returns mutated value = original + random_delta clamped to [1, max].
#[unsafe(no_mangle)]
pub extern "C" fn slang_snn_nas_mutate(value: i64, max_val: i64, seed: i64) -> i64 {
    let delta = ((seed * 6364136223846793005_i64.wrapping_add(1)) % 5) - 2; // -2..2
    (value + delta).clamp(1, max_val)
}

/// Get best fitness found so far.
#[unsafe(no_mangle)]
pub extern "C" fn slang_snn_nas_best() -> i64 {
    SNN_NAS_BEST.load(Ordering::SeqCst)
}

// ── v216: Federated SNN Learning ─────────────────────────────────────────────

static FED_SNN_ROUND: AtomicI64 = AtomicI64::new(0);
static FED_SNN_NODES: AtomicI64 = AtomicI64::new(0);

/// Aggregate model weights from n nodes using FedAvg: avg = sum / n.
/// Returns aggregated weight as fixed-point ×1000.
#[unsafe(no_mangle)]
pub extern "C" fn slang_snn_fed_aggregate(weight_sum: f64, n_nodes: i64) -> i64 {
    if n_nodes <= 0 { return 0; }
    let avg = weight_sum / n_nodes as f64;
    (avg * 1000.0).round() as i64
}

/// Share local model update to global: computes delta = local - global.
/// Returns delta as fixed-point ×1000.
#[unsafe(no_mangle)]
pub extern "C" fn slang_snn_fed_share(local_weight: f64, global_weight: f64) -> i64 {
    let delta = local_weight - global_weight;
    (delta * 1000.0).round() as i64
}

/// Advance federated learning round. Returns new round number.
#[unsafe(no_mangle)]
pub extern "C" fn slang_snn_fed_round() -> i64 {
    FED_SNN_ROUND.fetch_add(1, Ordering::SeqCst) + 1
}

/// Get/set number of federated nodes.
#[unsafe(no_mangle)]
pub extern "C" fn slang_snn_fed_node_count(n: i64) -> i64 {
    if n > 0 {
        FED_SNN_NODES.store(n, Ordering::SeqCst);
    }
    FED_SNN_NODES.load(Ordering::SeqCst)
}

// ── v217: Transfer Learning for SNNs ─────────────────────────────────────────

static FROZEN_LAYERS: AtomicI64 = AtomicI64::new(0);

/// Freeze first n layers for transfer learning. Returns frozen count.
#[unsafe(no_mangle)]
pub extern "C" fn slang_snn_transfer_freeze(n_layers: i64) -> i64 {
    FROZEN_LAYERS.store(n_layers, Ordering::SeqCst);
    n_layers
}

/// Fine-tune: compute gradient only for unfrozen layers.
/// Returns effective gradient (attenuated for deep layers) ×1000.
#[unsafe(no_mangle)]
pub extern "C" fn slang_snn_transfer_finetune(gradient: f64, layer: i64) -> i64 {
    let frozen = FROZEN_LAYERS.load(Ordering::SeqCst);
    if layer < frozen {
        0 // frozen layer, no gradient
    } else {
        (gradient * 1000.0).round() as i64
    }
}

/// Adapt learning rate for transfer: lr = base_lr * adaptation_factor.
/// Returns adapted LR ×1000000.
#[unsafe(no_mangle)]
pub extern "C" fn slang_snn_transfer_adapt(base_lr: f64, source_similarity: f64) -> i64 {
    let factor = source_similarity.clamp(0.1, 1.0);
    let adapted = base_lr * factor;
    (adapted * 1_000_000.0) as i64
}

/// Compute similarity between source and target domains.
/// Uses simple cosine-like metric. Returns similarity ×1000.
#[unsafe(no_mangle)]
pub extern "C" fn slang_snn_transfer_similarity(src_rate: f64, tgt_rate: f64) -> i64 {
    if src_rate == 0.0 && tgt_rate == 0.0 {
        return 1000; // both zero = identical
    }
    let max = src_rate.abs().max(tgt_rate.abs());
    if max == 0.0 { return 1000; }
    let diff = (src_rate - tgt_rate).abs() / max;
    ((1.0 - diff) * 1000.0).round() as i64
}

// ── v218: Online/Continual SNN Learning ──────────────────────────────────────

static CONTINUAL_TASK_COUNT: AtomicI64 = AtomicI64::new(0);

/// Online learning step: update weight with EWC regularization.
/// new_w = old_w - lr * (grad + lambda * fisher * (old_w - optimal_w)).
/// Returns new weight ×1000.
#[unsafe(no_mangle)]
pub extern "C" fn slang_snn_continual_learn(
    old_w: f64, grad: f64, fisher: f64, optimal_w: f64, lr: f64, lambda: f64,
) -> i64 {
    let ewc_penalty = lambda * fisher * (old_w - optimal_w);
    let new_w = old_w - lr * (grad + ewc_penalty);
    (new_w * 1000.0).round() as i64
}

/// Consolidate memory: compute importance (Fisher information) for a weight.
/// fisher = grad^2 averaged over n samples.
/// Returns fisher ×1000.
#[unsafe(no_mangle)]
pub extern "C" fn slang_snn_continual_consolidate(grad_sq_sum: f64, n_samples: i64) -> i64 {
    if n_samples <= 0 { return 0; }
    let fisher = grad_sq_sum / n_samples as f64;
    (fisher * 1000.0).round() as i64
}

/// Experience replay: select replay probability based on task age.
/// Older tasks get lower probability. Returns probability ×1000.
#[unsafe(no_mangle)]
pub extern "C" fn slang_snn_continual_replay(task_age: i64, max_age: i64) -> i64 {
    if max_age <= 0 { return 1000; }
    let ratio = 1.0 - (task_age as f64 / max_age as f64).min(1.0);
    let prob = (0.1 + 0.9 * ratio).min(1.0);
    (prob * 1000.0).round() as i64
}

/// Compute forgetting score: how much accuracy dropped on old task.
/// Returns forgetting metric ×1000 (higher = more forgotten).
#[unsafe(no_mangle)]
pub extern "C" fn slang_snn_continual_forget_score(original_acc: f64, current_acc: f64) -> i64 {
    let forgetting = (original_acc - current_acc).max(0.0);
    CONTINUAL_TASK_COUNT.fetch_add(1, Ordering::SeqCst);
    (forgetting * 1000.0).round() as i64
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // v213
    #[test]
    fn test_surrogate_forward() {
        let spikes = slang_snn_surrogate_forward(0.5, 1.0, 0.9, 100);
        assert!(spikes > 0);
    }

    #[test]
    fn test_surrogate_backward() {
        let g = slang_snn_surrogate_backward(0.5, 0.5, 1.0); // at threshold
        assert_eq!(g, 1000); // gradient = 1.0 at threshold
    }

    #[test]
    fn test_surrogate_sigmoid() {
        assert_eq!(slang_snn_surrogate_sigmoid(0.0), 500); // 0.5 at x=0
    }

    #[test]
    fn test_surrogate_loss() {
        let loss = slang_snn_surrogate_loss(10.0, 8.0, 1);
        assert_eq!(loss, 4000); // (10-8)^2 = 4
    }

    // v214
    #[test]
    fn test_bptt_forward() {
        let spikes = slang_snn_bptt_forward(0.5, 1.0, 0.9, 50);
        assert!(spikes > 0);
    }

    #[test]
    fn test_bptt_backward() {
        let grad = slang_snn_bptt_backward(1.0, 0.5, 10);
        assert!(grad > 0);
    }

    #[test]
    fn test_bptt_truncate() {
        let g_short = slang_snn_bptt_truncate(1.0, 0.5, 5);
        let g_long = slang_snn_bptt_truncate(1.0, 0.5, 50);
        assert!(g_long >= g_short);
    }

    // v215
    #[test]
    fn test_nas_search() {
        let f = slang_snn_nas_search(4, 128, 0.5);
        assert!(f > 0);
    }

    #[test]
    fn test_nas_evaluate() {
        let acc = slang_snn_nas_evaluate(5, 256);
        assert!(acc > 700 && acc <= 1000);
    }

    // v216
    #[test]
    fn test_fed_aggregate() {
        let avg = slang_snn_fed_aggregate(3.0, 3);
        assert_eq!(avg, 1000); // 3.0/3 = 1.0
    }

    #[test]
    fn test_fed_round() {
        let r1 = slang_snn_fed_round();
        let r2 = slang_snn_fed_round();
        assert_eq!(r2, r1 + 1);
    }

    // v217
    #[test]
    fn test_transfer_freeze() {
        slang_snn_transfer_freeze(3);
        assert_eq!(slang_snn_transfer_finetune(1.0, 2), 0); // frozen
        assert_eq!(slang_snn_transfer_finetune(1.0, 5), 1000); // not frozen
    }

    #[test]
    fn test_transfer_similarity() {
        assert_eq!(slang_snn_transfer_similarity(10.0, 10.0), 1000);
        let sim = slang_snn_transfer_similarity(10.0, 5.0);
        assert!(sim > 0 && sim < 1000);
    }

    // v218
    #[test]
    fn test_continual_learn() {
        let w = slang_snn_continual_learn(1.0, 0.1, 1.0, 1.0, 0.01, 0.1);
        // new_w = 1.0 - 0.01 * (0.1 + 0.1*1.0*(1.0-1.0)) = 1.0 - 0.001 = 0.999
        assert!(w > 900 && w < 1100);
    }

    #[test]
    fn test_continual_replay() {
        let p_new = slang_snn_continual_replay(0, 100); // newest → high prob
        let p_old = slang_snn_continual_replay(100, 100); // oldest → low prob
        assert!(p_new > p_old);
    }

    #[test]
    fn test_continual_forget() {
        let f = slang_snn_continual_forget_score(0.95, 0.80);
        assert_eq!(f, 150); // 0.15 * 1000
    }
}
