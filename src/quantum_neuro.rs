//! Quantum-Neuromorphic Bridge (v273–v278)
//!
//! Quantum spike encoding, quantum plasticity, quantum reservoir,
//! variational QSNN, quantum error correction, classical-quantum bridge.

use std::sync::atomic::{AtomicI64, Ordering};

// ── v273: Quantum Spike Encoding ─────────────────────────────────────────────

/// Encode classical spike as quantum amplitude. Returns amplitude ×1000.
#[unsafe(no_mangle)]
pub extern "C" fn slang_quantum_spike_encode(spike_rate: f64, n_qubits: i64) -> i64 {
    let amplitude = (spike_rate.clamp(0.0, 1.0) * std::f64::consts::FRAC_PI_2).sin();
    let quantized = (amplitude * (1_i64 << n_qubits.clamp(1, 16)) as f64).round() as i64;
    quantized
}

/// Decode quantum measurement to spike probability. Returns probability ×1000.
#[unsafe(no_mangle)]
pub extern "C" fn slang_quantum_spike_decode(measurement: i64, n_qubits: i64) -> i64 {
    let max_val = (1_i64 << n_qubits.clamp(1, 16)) as f64;
    let prob = (measurement as f64 / max_val).clamp(0.0, 1.0);
    (prob * 1000.0).round() as i64
}

/// Quantum superposition of spike states. Returns state count.
#[unsafe(no_mangle)]
pub extern "C" fn slang_quantum_spike_superpose(n_patterns: i64, n_qubits: i64) -> i64 {
    let max_states = 1_i64 << n_qubits.clamp(1, 20);
    n_patterns.min(max_states)
}

/// Quantum spike entanglement fidelity. Returns fidelity ×1000.
#[unsafe(no_mangle)]
pub extern "C" fn slang_quantum_spike_fidelity(noise_level: f64) -> i64 {
    let fidelity = 1.0 - noise_level.clamp(0.0, 1.0);
    (fidelity * 1000.0).round() as i64
}

// ── v274: Quantum Plasticity ─────────────────────────────────────────────────

/// Quantum STDP: weight update via quantum interference. Returns weight ×1000.
#[unsafe(no_mangle)]
pub extern "C" fn slang_quantum_plasticity_stdp(pre_phase: f64, post_phase: f64) -> i64 {
    let interference = (pre_phase - post_phase).cos();
    (interference * 1000.0).round() as i64
}

/// Quantum annealing for weight optimization. Returns optimized weight ×1000.
#[unsafe(no_mangle)]
pub extern "C" fn slang_quantum_plasticity_anneal(current_weight: f64, temperature: f64) -> i64 {
    let optimized = current_weight * (1.0 - temperature.clamp(0.0, 1.0) * 0.1);
    (optimized * 1000.0).round() as i64
}

/// Quantum tunneling probability for weight barrier crossing. Returns prob ×1000.
#[unsafe(no_mangle)]
pub extern "C" fn slang_quantum_plasticity_tunnel(barrier_height: f64, particle_energy: f64) -> i64 {
    if barrier_height <= 0.0 { return 1000; }
    let ratio = particle_energy / barrier_height;
    let prob = (-2.0 * (1.0 - ratio.clamp(0.0, 0.99)).sqrt()).exp();
    (prob * 1000.0).round() as i64
}

/// Quantum decoherence time estimate. Returns T2 in microseconds.
#[unsafe(no_mangle)]
pub extern "C" fn slang_quantum_plasticity_t2(n_qubits: i64, coupling: f64) -> i64 {
    let base_t2 = 100.0; // 100μs base
    (base_t2 / (1.0 + coupling * n_qubits as f64)).max(1.0).round() as i64
}

// ── v275: Quantum Reservoir Computing ────────────────────────────────────────

static RESERVOIR_DIM: AtomicI64 = AtomicI64::new(0);

/// Initialize quantum reservoir. Returns reservoir dimension.
#[unsafe(no_mangle)]
pub extern "C" fn slang_quantum_reservoir_init(n_qubits: i64) -> i64 {
    let dim = 1_i64 << n_qubits.clamp(1, 16);
    RESERVOIR_DIM.store(dim, Ordering::SeqCst);
    dim
}

/// Quantum reservoir state projection. Returns projected value ×1000.
#[unsafe(no_mangle)]
pub extern "C" fn slang_quantum_reservoir_project(input: f64, reservoir_dim: i64) -> i64 {
    let projected = input * (reservoir_dim as f64).sqrt();
    (projected * 1000.0).round() as i64
}

/// Quantum reservoir kernel. Returns kernel value ×1000.
#[unsafe(no_mangle)]
pub extern "C" fn slang_quantum_reservoir_kernel(x1: f64, x2: f64, gamma: f64) -> i64 {
    let diff = x1 - x2;
    let kernel = (-gamma * diff * diff).exp();
    (kernel * 1000.0).round() as i64
}

/// Get reservoir dimension.
#[unsafe(no_mangle)]
pub extern "C" fn slang_quantum_reservoir_dim() -> i64 {
    RESERVOIR_DIM.load(Ordering::SeqCst)
}

// ── v276: Variational QSNN ──────────────────────────────────────────────────

/// QSNN variational parameter update. Returns updated param ×1000.
#[unsafe(no_mangle)]
pub extern "C" fn slang_qsnn_var_update(param: f64, gradient: f64, lr: f64) -> i64 {
    let updated = param - lr * gradient;
    (updated * 1000.0).round() as i64
}

/// QSNN cost function evaluation. Returns cost ×1000.
#[unsafe(no_mangle)]
pub extern "C" fn slang_qsnn_var_cost(predicted: f64, target: f64) -> i64 {
    let cost = (predicted - target).powi(2);
    (cost * 1000.0).round() as i64
}

/// QSNN circuit depth estimate. Returns gate count.
#[unsafe(no_mangle)]
pub extern "C" fn slang_qsnn_var_depth(n_qubits: i64, n_layers: i64) -> i64 {
    n_qubits * n_layers * 3 // 3 gates per qubit per layer (Ry, CNOT, Rz)
}

/// QSNN expressibility metric. Returns expressibility ×1000.
#[unsafe(no_mangle)]
pub extern "C" fn slang_qsnn_var_expressibility(n_params: i64, hilbert_dim: i64) -> i64 {
    if hilbert_dim <= 0 { return 0; }
    let ratio = n_params as f64 / hilbert_dim as f64;
    (ratio.min(1.0) * 1000.0).round() as i64
}

// ── v277: Quantum Error Correction ───────────────────────────────────────────

/// Shor code: encode logical qubit. Returns physical qubit count.
#[unsafe(no_mangle)]
pub extern "C" fn slang_quantum_qec_shor(n_logical: i64) -> i64 {
    n_logical * 9 // 9 physical per logical
}

/// Surface code distance. Returns error threshold ×1000.
#[unsafe(no_mangle)]
pub extern "C" fn slang_quantum_qec_surface(distance: i64) -> i64 {
    let threshold = 1.0 - (0.99_f64).powi(distance as i32);
    ((1.0 - threshold) * 1000.0).round() as i64
}

/// Steane code syndrome extraction. Returns syndrome bits.
#[unsafe(no_mangle)]
pub extern "C" fn slang_quantum_qec_syndrome(n_data: i64) -> i64 {
    // Steane [[7,1,3]] code: 6 syndrome bits per logical qubit
    (n_data / 7).max(1) * 6
}

/// QEC overhead ratio.
#[unsafe(no_mangle)]
pub extern "C" fn slang_quantum_qec_overhead(n_logical: i64, code_distance: i64) -> i64 {
    let physical = n_logical * code_distance * code_distance * 2;
    physical
}

// ── v278: Classical-Quantum Bridge ───────────────────────────────────────────

/// Convert classical neural weights to quantum rotation angles. Returns angle ×1000.
#[unsafe(no_mangle)]
pub extern "C" fn slang_quantum_bridge_encode(weight: f64) -> i64 {
    let angle = weight.atan() * 2.0 / std::f64::consts::PI;
    (angle * 1000.0).round() as i64
}

/// Convert quantum measurement to classical activation. Returns activation ×1000.
#[unsafe(no_mangle)]
pub extern "C" fn slang_quantum_bridge_decode(prob_one: f64) -> i64 {
    let activation = 2.0 * prob_one - 1.0; // map [0,1] → [-1,1]
    (activation * 1000.0).round() as i64
}

/// Hybrid quantum-classical layer cost. Returns FLOPs estimate.
#[unsafe(no_mangle)]
pub extern "C" fn slang_quantum_bridge_cost(classical_ops: i64, quantum_shots: i64) -> i64 {
    classical_ops + quantum_shots * 100 // quantum shots ~100x classical cost
}

/// Quantum advantage estimate. Returns speedup ×1000 (>1000 means quantum wins).
#[unsafe(no_mangle)]
pub extern "C" fn slang_quantum_bridge_advantage(problem_size: i64, quantum_depth: i64) -> i64 {
    if quantum_depth <= 0 { return 0; }
    let classical_cost = problem_size * problem_size; // O(n²)
    let quantum_cost = problem_size * quantum_depth;  // O(n·d)
    if quantum_cost <= 0 { return 0; }
    (classical_cost as f64 / quantum_cost as f64 * 1000.0).round() as i64
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quantum_spike_encode() {
        let amp = slang_quantum_spike_encode(1.0, 8);
        assert!(amp > 0 && amp <= 256);
    }

    #[test]
    fn test_quantum_spike_fidelity() {
        assert_eq!(slang_quantum_spike_fidelity(0.0), 1000);
        assert_eq!(slang_quantum_spike_fidelity(0.1), 900);
    }

    #[test]
    fn test_quantum_plasticity_stdp() {
        let w = slang_quantum_plasticity_stdp(0.0, 0.0);
        assert_eq!(w, 1000); // cos(0) = 1
    }

    #[test]
    fn test_quantum_reservoir_init() {
        let dim = slang_quantum_reservoir_init(4);
        assert_eq!(dim, 16); // 2^4
    }

    #[test]
    fn test_qsnn_var_cost() {
        let c = slang_qsnn_var_cost(1.0, 0.5);
        assert_eq!(c, 250); // (0.5)^2 * 1000
    }

    #[test]
    fn test_quantum_qec_shor() {
        assert_eq!(slang_quantum_qec_shor(10), 90);
    }

    #[test]
    fn test_quantum_bridge_encode() {
        let a = slang_quantum_bridge_encode(1.0);
        assert!(a > 0 && a <= 1000);
    }

    #[test]
    fn test_quantum_bridge_advantage() {
        let adv = slang_quantum_bridge_advantage(100, 10);
        assert_eq!(adv, 10000); // 10000/1000 = 10x speedup
    }

    #[test]
    fn test_qsnn_var_depth() {
        assert_eq!(slang_qsnn_var_depth(4, 3), 36); // 4*3*3
    }

    #[test]
    fn test_quantum_reservoir_kernel() {
        let k = slang_quantum_reservoir_kernel(1.0, 1.0, 1.0);
        assert_eq!(k, 1000); // exp(0) = 1
    }
}
