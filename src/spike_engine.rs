//! Spike Engine — Neuromorphic Core (v201–v206)
//!
//! Priority-queue spike event system, multi-compartment neurons, advanced synapse
//! models, neural population dynamics, spike encoding/decoding, and neuromorphic
//! memory architecture. This module forms the foundation of Vitalis's neuromorphic
//! computing stack.

use std::collections::BTreeMap;
use std::sync::{LazyLock, Mutex, atomic::{AtomicI64, Ordering}};

// ── v201: Spike Event System ─────────────────────────────────────────────────

/// Spike event: (timestamp, neuron_id, value)
static SPIKE_EVENTS: LazyLock<Mutex<Vec<(i64, i64, i64)>>> =
    LazyLock::new(|| Mutex::new(Vec::new()));

/// Emit a spike event at the given timestamp from neuron_id with a value.
/// Events are kept sorted by timestamp (ascending).
#[unsafe(no_mangle)]
pub extern "C" fn slang_spike_emit(timestamp: i64, neuron_id: i64, value: i64) -> i64 {
    let mut q = SPIKE_EVENTS.lock().unwrap();
    q.push((timestamp, neuron_id, value));
    q.sort_by_key(|e| e.0); // maintain timestamp order
    q.len() as i64
}

/// Return the number of pending spike events in the queue.
#[unsafe(no_mangle)]
pub extern "C" fn slang_spike_queue_len() -> i64 {
    SPIKE_EVENTS.lock().unwrap().len() as i64
}

/// Pop and return the neuron_id of the earliest spike event, or -1 if empty.
#[unsafe(no_mangle)]
pub extern "C" fn slang_spike_next() -> i64 {
    let mut q = SPIKE_EVENTS.lock().unwrap();
    if q.is_empty() {
        -1
    } else {
        let ev = q.remove(0);
        ev.1 // neuron_id
    }
}

/// Clear all pending spike events. Returns count removed.
#[unsafe(no_mangle)]
pub extern "C" fn slang_spike_clear() -> i64 {
    let mut q = SPIKE_EVENTS.lock().unwrap();
    let n = q.len() as i64;
    q.clear();
    n
}

// ── v202: Multi-Compartment Neurons ──────────────────────────────────────────

/// Compartment: (voltage, threshold, leak_factor, n_dendrites)
static COMPARTMENTS: LazyLock<Mutex<Vec<(f64, f64, f64, i64)>>> =
    LazyLock::new(|| Mutex::new(Vec::new()));
static COMPARTMENT_COUNTER: AtomicI64 = AtomicI64::new(0);

/// Create a new multi-compartment neuron with given threshold and leak factor.
/// Returns compartment ID.
#[unsafe(no_mangle)]
pub extern "C" fn slang_neuro_compartment_create(threshold: f64, leak: f64) -> i64 {
    let id = COMPARTMENT_COUNTER.fetch_add(1, Ordering::SeqCst);
    let mut comps = COMPARTMENTS.lock().unwrap();
    comps.push((0.0, threshold, leak, 0));
    id
}

/// Step a compartment forward: apply leak, add current. Returns 1 if spike, 0 otherwise.
#[unsafe(no_mangle)]
pub extern "C" fn slang_neuro_compartment_step(id: i64, current: f64) -> i64 {
    let mut comps = COMPARTMENTS.lock().unwrap();
    let idx = id as usize;
    if idx >= comps.len() {
        return -1;
    }
    let (ref mut v, threshold, leak, _) = comps[idx];
    *v = *v * leak + current;
    if *v >= threshold {
        *v = 0.0; // reset after spike
        1
    } else {
        0
    }
}

/// Propagate input through dendritic tree: attenuates by distance factor.
/// Returns attenuated value (distance as integer steps, attenuation = 0.9^distance).
#[unsafe(no_mangle)]
pub extern "C" fn slang_neuro_dendrite_propagate(value: f64, distance: i64) -> i64 {
    let attenuation = 0.9_f64.powi(distance as i32);
    let result = value * attenuation;
    (result * 1000.0).round() as i64 // fixed-point ×1000
}

/// Return the number of compartments created.
#[unsafe(no_mangle)]
pub extern "C" fn slang_neuro_compartment_count() -> i64 {
    COMPARTMENTS.lock().unwrap().len() as i64
}

// ── v203: Advanced Synapse Models ────────────────────────────────────────────

/// Synapse state: (weight, facilitation_u, depression_x, conductance)
static SYNAPSES: LazyLock<Mutex<BTreeMap<i64, (f64, f64, f64, f64)>>> =
    LazyLock::new(|| Mutex::new(BTreeMap::new()));
static SYNAPSE_COUNTER: AtomicI64 = AtomicI64::new(0);

/// Compute conductance-based synaptic current: g * (E_rev - V_post).
/// Returns current as fixed-point ×1000.
#[unsafe(no_mangle)]
pub extern "C" fn slang_synapse_conductance(g: f64, e_rev: f64, v_post: f64) -> i64 {
    let current = g * (e_rev - v_post);
    (current * 1000.0).round() as i64
}

/// Create synapse and apply short-term facilitation: U → U + f_rate * (1 - U).
/// Returns new synapse ID.
#[unsafe(no_mangle)]
pub extern "C" fn slang_synapse_stp_facilitate(weight: f64, f_rate: f64) -> i64 {
    let id = SYNAPSE_COUNTER.fetch_add(1, Ordering::SeqCst);
    let u = 0.2; // baseline release probability
    let new_u = u + f_rate * (1.0 - u);
    let mut syns = SYNAPSES.lock().unwrap();
    syns.insert(id, (weight, new_u, 1.0, weight * new_u));
    id
}

/// Apply short-term depression: X → X * (1 - d_rate).
/// Returns depressed weight as fixed-point ×1000, or -1 if invalid ID.
#[unsafe(no_mangle)]
pub extern "C" fn slang_synapse_stp_depress(syn_id: i64, d_rate: f64) -> i64 {
    let mut syns = SYNAPSES.lock().unwrap();
    if let Some(s) = syns.get_mut(&syn_id) {
        s.2 *= 1.0 - d_rate; // depression factor
        s.3 = s.0 * s.1 * s.2; // update conductance
        (s.3 * 1000.0).round() as i64
    } else {
        -1
    }
}

/// Return the total number of synapses created.
#[unsafe(no_mangle)]
pub extern "C" fn slang_synapse_count() -> i64 {
    SYNAPSES.lock().unwrap().len() as i64
}

// ── v204: Neural Population Dynamics ─────────────────────────────────────────

/// Wilson-Cowan model: excitatory/inhibitory population dynamics.
/// dE/dt = -E + S(w_ee*E - w_ei*I + input_e)
/// dI/dt = -I + S(w_ie*E - w_ii*I + input_i)
/// S(x) = 1 / (1 + exp(-x))
/// Returns excitatory activity as fixed-point ×1000 after n_steps.
#[unsafe(no_mangle)]
pub extern "C" fn slang_neuro_wilson_cowan(
    w_ee: f64, w_ei: f64, w_ie: f64, w_ii: f64,
    input_e: f64, input_i: f64, n_steps: i64,
) -> i64 {
    let dt = 0.01;
    let mut e = 0.1_f64;
    let mut i = 0.1_f64;
    let sigmoid = |x: f64| 1.0 / (1.0 + (-x).exp());
    for _ in 0..n_steps {
        let de = -e + sigmoid(w_ee * e - w_ei * i + input_e);
        let di = -i + sigmoid(w_ie * e - w_ii * i + input_i);
        e += de * dt;
        i += di * dt;
    }
    (e * 1000.0).round() as i64
}

/// Neural mass model: mean-field population firing rate.
/// Computes steady-state rate = max_rate * sigmoid(gain * (input - threshold)).
/// Returns rate as fixed-point ×1000.
#[unsafe(no_mangle)]
pub extern "C" fn slang_neuro_neural_mass(input: f64, threshold: f64, gain: f64, max_rate: f64) -> i64 {
    let sigmoid = 1.0 / (1.0 + (-(gain * (input - threshold))).exp());
    let rate = max_rate * sigmoid;
    (rate * 1000.0).round() as i64
}

/// Compute population activity: weighted sum of individual neuron rates.
/// Takes n firing rates and n weights, returns weighted sum ×1000.
#[unsafe(no_mangle)]
pub extern "C" fn slang_neuro_population_activity(n: i64, rates_sum: f64, weights_sum: f64) -> i64 {
    if n <= 0 {
        return 0;
    }
    let avg_rate = rates_sum / n as f64;
    let activity = avg_rate * weights_sum;
    (activity * 1000.0).round() as i64
}

/// Measure population synchrony via coefficient of variation of ISIs.
/// CV < 0.5 → synchronized, CV > 1.0 → asynchronous.
/// Returns synchrony index ×1000 (1000 = perfectly synchronized).
#[unsafe(no_mangle)]
pub extern "C" fn slang_neuro_population_sync(mean_isi: f64, std_isi: f64) -> i64 {
    if mean_isi <= 0.0 {
        return 0;
    }
    let cv = std_isi / mean_isi;
    let sync = (1.0 - cv.min(1.0)).max(0.0);
    (sync * 1000.0).round() as i64
}

// ── v205: Spike Encoding/Decoding ────────────────────────────────────────────

/// Rate coding: convert analog value to spike count.
/// spike_count = (value / max_value) * max_spikes, clamped to [0, max_spikes].
#[unsafe(no_mangle)]
pub extern "C" fn slang_spike_encode_rate(value: f64, max_value: f64, max_spikes: i64) -> i64 {
    if max_value <= 0.0 {
        return 0;
    }
    let ratio = (value / max_value).clamp(0.0, 1.0);
    (ratio * max_spikes as f64).round() as i64
}

/// Temporal coding: convert value to spike timing (lower value = earlier spike).
/// Returns spike time in microseconds: time = (1 - value/max) * window_us.
#[unsafe(no_mangle)]
pub extern "C" fn slang_spike_encode_temporal(value: f64, max_value: f64, window_us: i64) -> i64 {
    if max_value <= 0.0 {
        return window_us;
    }
    let ratio = (value / max_value).clamp(0.0, 1.0);
    ((1.0 - ratio) * window_us as f64).round() as i64
}

/// Decode: convert spike count back to analog value.
/// value = (spike_count / max_spikes) * max_value.
#[unsafe(no_mangle)]
pub extern "C" fn slang_spike_decode_rate(spike_count: i64, max_spikes: i64, max_value: f64) -> i64 {
    if max_spikes <= 0 {
        return 0;
    }
    let ratio = (spike_count as f64 / max_spikes as f64).clamp(0.0, 1.0);
    (ratio * max_value * 1000.0).round() as i64 // fixed-point ×1000
}

/// Phase coding: encode value as phase offset within oscillation cycle.
/// Returns phase in millidegrees (0-360000).
#[unsafe(no_mangle)]
pub extern "C" fn slang_spike_encode_phase(value: f64, max_value: f64) -> i64 {
    if max_value <= 0.0 {
        return 0;
    }
    let ratio = (value / max_value).clamp(0.0, 1.0);
    (ratio * 360.0 * 1000.0).round() as i64
}

// ── v206: Neuromorphic Memory Architecture ───────────────────────────────────

/// Neuromorphic memory bank: simulates co-located compute+storage.
/// Each cell: (value, last_compute_result)
static NEURO_MEM: LazyLock<Mutex<Vec<(i64, i64)>>> =
    LazyLock::new(|| Mutex::new(Vec::new()));

/// Allocate n cells of neuromorphic memory. Returns start index.
#[unsafe(no_mangle)]
pub extern "C" fn slang_neuro_mem_alloc(n: i64) -> i64 {
    let mut mem = NEURO_MEM.lock().unwrap();
    let start = mem.len() as i64;
    for _ in 0..n.max(0) {
        mem.push((0, 0));
    }
    start
}

/// Read a value from neuromorphic memory at given address.
/// Returns value or -1 if out of bounds.
#[unsafe(no_mangle)]
pub extern "C" fn slang_neuro_mem_read(addr: i64) -> i64 {
    let mem = NEURO_MEM.lock().unwrap();
    let idx = addr as usize;
    if idx < mem.len() {
        mem[idx].0
    } else {
        -1
    }
}

/// Write a value to neuromorphic memory at given address.
/// Returns 1 on success, 0 if out of bounds.
#[unsafe(no_mangle)]
pub extern "C" fn slang_neuro_mem_write(addr: i64, value: i64) -> i64 {
    let mut mem = NEURO_MEM.lock().unwrap();
    let idx = addr as usize;
    if idx < mem.len() {
        mem[idx].0 = value;
        1
    } else {
        0
    }
}

/// Near-memory compute: perform operation on value at address without moving data.
/// op: 0=add, 1=mul, 2=and, 3=or. Stores result in cell's compute field.
/// Returns computed result, or -1 if out of bounds.
#[unsafe(no_mangle)]
pub extern "C" fn slang_neuro_mem_near_compute(addr: i64, op: i64, operand: i64) -> i64 {
    let mut mem = NEURO_MEM.lock().unwrap();
    let idx = addr as usize;
    if idx >= mem.len() {
        return -1;
    }
    let val = mem[idx].0;
    let result = match op {
        0 => val.wrapping_add(operand),
        1 => val.wrapping_mul(operand),
        2 => val & operand,
        3 => val | operand,
        _ => val,
    };
    mem[idx].1 = result;
    result
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // v201: Spike Event System
    #[test]
    fn test_spike_emit_and_queue() {
        slang_spike_clear();
        slang_spike_emit(100, 1, 10);
        slang_spike_emit(50, 2, 20);
        slang_spike_emit(150, 3, 30);
        assert_eq!(slang_spike_queue_len(), 3);
        // Events sorted by timestamp: 50, 100, 150
        assert_eq!(slang_spike_next(), 2); // neuron_id of earliest (ts=50)
        assert_eq!(slang_spike_queue_len(), 2);
    }

    #[test]
    fn test_spike_clear() {
        slang_spike_clear();
        slang_spike_emit(1, 10, 100);
        slang_spike_emit(2, 20, 200);
        assert_eq!(slang_spike_clear(), 2);
        assert_eq!(slang_spike_queue_len(), 0);
        assert_eq!(slang_spike_next(), -1); // empty
    }

    #[test]
    fn test_spike_next_empty() {
        slang_spike_clear();
        assert_eq!(slang_spike_next(), -1);
    }

    // v202: Multi-Compartment Neurons
    #[test]
    fn test_compartment_create_and_step() {
        let id = slang_neuro_compartment_create(1.0, 0.9);
        assert!(id >= 0);
        // Below threshold
        let spike = slang_neuro_compartment_step(id, 0.5);
        assert_eq!(spike, 0);
        // Push above threshold
        let spike2 = slang_neuro_compartment_step(id, 0.7);
        assert_eq!(spike2, 1);
        assert!(slang_neuro_compartment_count() > 0);
    }

    #[test]
    fn test_dendrite_propagate() {
        // 0 distance → full value
        let v0 = slang_neuro_dendrite_propagate(1.0, 0);
        assert_eq!(v0, 1000); // 1.0 * 1000
        // 1 distance → 0.9
        let v1 = slang_neuro_dendrite_propagate(1.0, 1);
        assert_eq!(v1, 900); // 0.9 * 1000
        // 10 distance → 0.9^10 ≈ 0.3486
        let v10 = slang_neuro_dendrite_propagate(1.0, 10);
        assert!(v10 > 300 && v10 < 400);
    }

    // v203: Advanced Synapse Models
    #[test]
    fn test_synapse_conductance() {
        // g=0.5, E_rev=0.0, V_post=-70.0 → current = 0.5 * (0 - (-70)) = 35
        let c = slang_synapse_conductance(0.5, 0.0, -70.0);
        assert_eq!(c, 35000);
    }

    #[test]
    fn test_synapse_stp() {
        let id = slang_synapse_stp_facilitate(1.0, 0.5);
        assert!(id >= 0);
        let depressed = slang_synapse_stp_depress(id, 0.3);
        assert!(depressed > 0);
        assert!(slang_synapse_count() > 0);
    }

    #[test]
    fn test_synapse_depress_invalid() {
        assert_eq!(slang_synapse_stp_depress(999999, 0.1), -1);
    }

    // v204: Neural Population Dynamics
    #[test]
    fn test_wilson_cowan() {
        // Excitatory-dominant network should produce positive activity
        let e = slang_neuro_wilson_cowan(4.0, 1.0, 1.0, 0.5, 1.0, 0.0, 1000);
        assert!(e > 0);
    }

    #[test]
    fn test_neural_mass() {
        // Input above threshold → positive rate
        let rate = slang_neuro_neural_mass(5.0, 2.0, 1.0, 100.0);
        assert!(rate > 50000); // > 50% of max_rate

        // Input below threshold → lower rate
        let rate_low = slang_neuro_neural_mass(-5.0, 2.0, 1.0, 100.0);
        assert!(rate_low < rate);
    }

    #[test]
    fn test_population_sync() {
        // Low CV → high synchrony
        let sync = slang_neuro_population_sync(10.0, 1.0);
        assert_eq!(sync, 900); // (1 - 0.1) * 1000

        // High CV → low synchrony
        let async_val = slang_neuro_population_sync(10.0, 10.0);
        assert_eq!(async_val, 0);
    }

    // v205: Spike Encoding/Decoding
    #[test]
    fn test_rate_coding_roundtrip() {
        let spikes = slang_spike_encode_rate(50.0, 100.0, 200);
        assert_eq!(spikes, 100); // 50% of 200

        let decoded = slang_spike_decode_rate(100, 200, 100.0);
        assert_eq!(decoded, 50000); // 50.0 * 1000
    }

    #[test]
    fn test_temporal_coding() {
        // High value → early spike (low time)
        let early = slang_spike_encode_temporal(90.0, 100.0, 1000);
        let late = slang_spike_encode_temporal(10.0, 100.0, 1000);
        assert!(early < late);
    }

    #[test]
    fn test_phase_coding() {
        let phase = slang_spike_encode_phase(50.0, 100.0);
        assert_eq!(phase, 180000); // 180 degrees * 1000

        let phase_max = slang_spike_encode_phase(100.0, 100.0);
        assert_eq!(phase_max, 360000);
    }

    // v206: Neuromorphic Memory Architecture
    #[test]
    fn test_neuro_mem_alloc_read_write() {
        let start = slang_neuro_mem_alloc(4);
        assert!(start >= 0);
        assert_eq!(slang_neuro_mem_write(start, 42), 1);
        assert_eq!(slang_neuro_mem_read(start), 42);
        assert_eq!(slang_neuro_mem_read(start + 1), 0); // default
    }

    #[test]
    fn test_neuro_mem_near_compute() {
        let addr = slang_neuro_mem_alloc(1);
        slang_neuro_mem_write(addr, 10);
        // Add
        let r = slang_neuro_mem_near_compute(addr, 0, 5);
        assert_eq!(r, 15);
        // Mul (on original value, not compute result)
        let r2 = slang_neuro_mem_near_compute(addr, 1, 3);
        assert_eq!(r2, 30);
    }

    #[test]
    fn test_neuro_mem_out_of_bounds() {
        assert_eq!(slang_neuro_mem_read(999999), -1);
        assert_eq!(slang_neuro_mem_write(999999, 0), 0);
        assert_eq!(slang_neuro_mem_near_compute(999999, 0, 0), -1);
    }
}
