//! SNN–ANN Hybrid & Neural Compilation (v249–v254)
//!
//! SNN↔ANN conversion, hybrid inference, spike compiler,
//! differentiable spike gradients, neural ODEs, hybrid fine-tuning.

use std::sync::atomic::{AtomicI64, Ordering};

// ── v249: SNN↔ANN Conversion ─────────────────────────────────────────────────

/// Convert ANN-ReLU activation to spike rate. Returns rate ×1000.
#[unsafe(no_mangle)]
pub extern "C" fn slang_snn_ann_to_rate(activation: f64, max_rate: f64) -> i64 {
    let rate = activation.max(0.0).min(max_rate);
    (rate * 1000.0).round() as i64
}

/// Convert spike rate back to ANN-like activation. Returns activation ×1000.
#[unsafe(no_mangle)]
pub extern "C" fn slang_snn_rate_to_ann(spike_rate: f64, max_rate: f64) -> i64 {
    let act = spike_rate / max_rate.max(0.001);
    (act.clamp(0.0, 1.0) * 1000.0).round() as i64
}

/// Compute conversion loss (quantization gap). Returns loss ×1000.
#[unsafe(no_mangle)]
pub extern "C" fn slang_snn_conversion_loss(original: f64, converted: f64) -> i64 {
    let loss = (original - converted).abs();
    (loss * 1000.0).round() as i64
}

/// Optimal timestep for conversion accuracy. Returns timesteps.
#[unsafe(no_mangle)]
pub extern "C" fn slang_snn_optimal_timesteps(target_accuracy: f64) -> i64 {
    // More timesteps → better approximation
    (100.0 / (1.0 - target_accuracy.clamp(0.0, 0.999))).ceil() as i64
}

// ── v250: Hybrid Inference ───────────────────────────────────────────────────

static HYBRID_SNN_FRACTION: AtomicI64 = AtomicI64::new(500); // 50.0% default

/// Run hybrid inference: early exit if SNN confidence high.
/// Returns confidence (0-1000).
#[unsafe(no_mangle)]
pub extern "C" fn slang_hybrid_infer(snn_conf: f64, ann_conf: f64, threshold: f64) -> i64 {
    let frac = HYBRID_SNN_FRACTION.load(Ordering::SeqCst) as f64 / 1000.0;
    let combined = frac * snn_conf + (1.0 - frac) * ann_conf;
    if snn_conf > threshold { (snn_conf * 1000.0).round() as i64 }
    else { (combined * 1000.0).round() as i64 }
}

/// Set SNN/ANN blend ratio. Fraction is SNN weight (0-1000 → 0.0-1.0).
#[unsafe(no_mangle)]
pub extern "C" fn slang_hybrid_set_fraction(snn_pct: i64) -> i64 {
    let clamped = snn_pct.clamp(0, 1000);
    HYBRID_SNN_FRACTION.store(clamped, Ordering::SeqCst);
    clamped
}

/// Compute hybrid energy efficiency. Returns efficiency ratio ×1000.
#[unsafe(no_mangle)]
pub extern "C" fn slang_hybrid_efficiency(snn_energy: f64, ann_energy: f64) -> i64 {
    if ann_energy <= 0.0 { return 1000; }
    let ratio = snn_energy / ann_energy;
    ((1.0 - ratio.clamp(0.0, 1.0)) * 1000.0).round() as i64
}

/// Hybrid accuracy vs SNN-only improvement. Returns improvement ×1000.
#[unsafe(no_mangle)]
pub extern "C" fn slang_hybrid_accuracy_gain(snn_acc: f64, hybrid_acc: f64) -> i64 {
    let gain = (hybrid_acc - snn_acc).max(0.0);
    (gain * 1000.0).round() as i64
}

// ── v251: Spike Compiler ─────────────────────────────────────────────────────

static COMPILED_NETWORKS: AtomicI64 = AtomicI64::new(0);

/// Compile SNN topology to optimized execution plan.
/// Returns execution cost estimate (lower = better).
#[unsafe(no_mangle)]
pub extern "C" fn slang_spike_compile(n_neurons: i64, n_synapses: i64, n_timesteps: i64) -> i64 {
    COMPILED_NETWORKS.fetch_add(1, Ordering::SeqCst);
    // Cost = neurons × timesteps + synapses (amortized)
    n_neurons * n_timesteps + n_synapses
}

/// Optimize spike computation graph (dead neuron elimination).
/// Returns pruned neuron count.
#[unsafe(no_mangle)]
pub extern "C" fn slang_spike_compile_optimize(n_neurons: i64, active_ratio: f64) -> i64 {
    (n_neurons as f64 * active_ratio.clamp(0.0, 1.0)).round() as i64
}

/// Get compiled network count.
#[unsafe(no_mangle)]
pub extern "C" fn slang_spike_compile_count() -> i64 {
    COMPILED_NETWORKS.load(Ordering::SeqCst)
}

/// Estimate compiled network memory footprint in KB.
#[unsafe(no_mangle)]
pub extern "C" fn slang_spike_compile_memory(n_neurons: i64, n_synapses: i64) -> i64 {
    // Each neuron: 64 bytes state; each synapse: 16 bytes
    (n_neurons * 64 + n_synapses * 16) / 1024
}

// ── v252: Differentiable Spike Gradients ─────────────────────────────────────

/// Surrogate gradient: straight-through estimator. Returns gradient ×1000.
#[unsafe(no_mangle)]
pub extern "C" fn slang_diff_spike_ste(membrane: f64, threshold: f64) -> i64 {
    let centered = membrane - threshold;
    let grad: f64 = if centered.abs() < 1.0 { 1.0 } else { 0.0 };
    (grad * 1000.0).round() as i64
}

/// Surrogate gradient: sigmoid. Returns gradient ×1000.
#[unsafe(no_mangle)]
pub extern "C" fn slang_diff_spike_sigmoid(membrane: f64, threshold: f64, beta: f64) -> i64 {
    let x = beta * (membrane - threshold);
    let sig = 1.0 / (1.0 + (-x).exp());
    let grad = beta * sig * (1.0 - sig);
    (grad * 1000.0).round() as i64
}

/// Surrogate gradient: fast sigmoid. Returns gradient ×1000.
#[unsafe(no_mangle)]
pub extern "C" fn slang_diff_spike_fast_sigmoid(membrane: f64, threshold: f64, alpha: f64) -> i64 {
    let x = membrane - threshold;
    let grad = alpha / (2.0 * (1.0 + alpha * x.abs()).powi(2));
    (grad * 1000.0).round() as i64
}

/// Spike gradient accumulator. Returns accumulated gradient ×1000.
#[unsafe(no_mangle)]
pub extern "C" fn slang_diff_spike_accumulate(grad_a: f64, grad_b: f64) -> i64 {
    ((grad_a + grad_b) * 1000.0).round() as i64
}

// ── v253: Neural ODEs ────────────────────────────────────────────────────────

/// Euler step for neural ODE. Returns next state ×1000.
#[unsafe(no_mangle)]
pub extern "C" fn slang_neural_ode_euler(state: f64, derivative: f64, dt: f64) -> i64 {
    let next = state + derivative * dt;
    (next * 1000.0).round() as i64
}

/// RK4 step for neural ODE. Returns next state ×1000.
#[unsafe(no_mangle)]
pub extern "C" fn slang_neural_ode_rk4(state: f64, k1: f64, k2: f64, k3: f64, k4: f64, dt: f64) -> i64 {
    let next = state + dt / 6.0 * (k1 + 2.0 * k2 + 2.0 * k3 + k4);
    (next * 1000.0).round() as i64
}

/// Adjoint sensitivity computation. Returns adjoint ×1000.
#[unsafe(no_mangle)]
pub extern "C" fn slang_neural_ode_adjoint(loss_grad: f64, jac: f64) -> i64 {
    ((loss_grad * jac) * 1000.0).round() as i64
}

/// Adaptive step size control. Returns new dt ×1000.
#[unsafe(no_mangle)]
pub extern "C" fn slang_neural_ode_adaptive_dt(current_dt: f64, error: f64, tolerance: f64) -> i64 {
    if error <= 0.0 { return (current_dt * 2.0 * 1000.0).round() as i64; }
    let factor = (tolerance / error).powf(0.2).clamp(0.1, 5.0);
    (current_dt * factor * 1000.0).round() as i64
}

// ── v254: Hybrid Fine-Tuning ─────────────────────────────────────────────────

/// Knowledge distillation: soft target cross-entropy. Returns loss ×1000.
#[unsafe(no_mangle)]
pub extern "C" fn slang_hybrid_distill(teacher_logit: f64, student_logit: f64, temperature: f64) -> i64 {
    let t = temperature.max(0.1);
    let t_soft = (teacher_logit / t).exp();
    let s_soft = (student_logit / t).exp();
    let loss = -(t_soft * s_soft.ln()).abs();
    (loss.abs() * 1000.0).min(10000.0) as i64
}

/// Progressive freezing: compute freeze layer. Returns highest frozen layer.
#[unsafe(no_mangle)]
pub extern "C" fn slang_hybrid_freeze(total_layers: i64, epoch: i64, max_epochs: i64) -> i64 {
    let progress = epoch as f64 / max_epochs.max(1) as f64;
    (total_layers as f64 * progress).floor() as i64
}

/// Hybrid learning rate schedule. Returns lr ×1000000 (micro-scale).
#[unsafe(no_mangle)]
pub extern "C" fn slang_hybrid_lr(base_lr: f64, epoch: i64, warmup: i64, max_epochs: i64) -> i64 {
    let lr = if epoch < warmup {
        base_lr * epoch as f64 / warmup.max(1) as f64
    } else {
        let progress = (epoch - warmup) as f64 / (max_epochs - warmup).max(1) as f64;
        base_lr * 0.5 * (1.0 + (std::f64::consts::PI * progress).cos())
    };
    (lr * 1_000_000.0) as i64
}

/// Hybrid model accuracy metric. Returns weighted accuracy ×1000.
#[unsafe(no_mangle)]
pub extern "C" fn slang_hybrid_weighted_acc(snn_acc: f64, ann_acc: f64, snn_weight: f64) -> i64 {
    let w = snn_weight.clamp(0.0, 1.0);
    let acc = w * snn_acc + (1.0 - w) * ann_acc;
    (acc * 1000.0).round() as i64
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snn_ann_conversion() {
        let rate = slang_snn_ann_to_rate(0.5, 1.0);
        assert_eq!(rate, 500);
        let act = slang_snn_rate_to_ann(0.5, 1.0);
        assert_eq!(act, 500);
    }

    #[test]
    fn test_hybrid_infer() {
        let c = slang_hybrid_infer(0.9, 0.8, 0.85);
        assert_eq!(c, 900); // snn_conf > threshold → 900
    }

    #[test]
    fn test_spike_compile() {
        let cost = slang_spike_compile(100, 1000, 50);
        assert_eq!(cost, 6000); // 100*50 + 1000
    }

    #[test]
    fn test_diff_spike_ste() {
        let g = slang_diff_spike_ste(1.0, 0.5);
        assert_eq!(g, 1000); // |0.5| < 1 → 1.0
    }

    #[test]
    fn test_neural_ode_euler() {
        let next = slang_neural_ode_euler(1.0, 0.5, 0.1);
        assert_eq!(next, 1050); // 1.0 + 0.5*0.1 = 1.05
    }

    #[test]
    fn test_hybrid_distill() {
        let loss = slang_hybrid_distill(1.0, 0.5, 1.0);
        assert!(loss > 0);
    }

    #[test]
    fn test_hybrid_freeze() {
        let layer = slang_hybrid_freeze(12, 6, 12);
        assert_eq!(layer, 6);
    }

    #[test]
    fn test_optimal_timesteps() {
        let ts = slang_snn_optimal_timesteps(0.99);
        assert_eq!(ts, 10000); // 100/0.01 = 10000
    }

    #[test]
    fn test_spike_compile_memory() {
        let kb = slang_spike_compile_memory(1000, 10000);
        assert_eq!(kb, (1000*64+10000*16)/1024);
    }

    #[test]
    fn test_neural_ode_rk4() {
        let next = slang_neural_ode_rk4(0.0, 1.0, 1.0, 1.0, 1.0, 0.1);
        assert_eq!(next, 100); // 0 + 0.1/6*(1+2+2+1) = 0.1 → 100
    }
}
