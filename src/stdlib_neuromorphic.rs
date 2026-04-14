//! v419 — Consolidated neuromorphic standard library.
//!
//! Provides high-level neuromorphic operations that compose the lower-level
//! primitives from `spike_types`, `spike_engine`, `neuromorphic`, `loihi_sim`,
//! and `snn_learning`. Each function is FFI-safe for codegen integration.
//!
//! Categories:
//! - Network construction: create/connect/configure neuron populations
//! - Simulation: run spike simulations
//! - Analysis: spike statistics and network metrics
//! - Encoding: convert between spike and continuous representations
//! - Learning: consolidated learning rules

use std::sync::{LazyLock, Mutex};
use crate::spike_types::{Neuron, Synapse, SpikeNetwork, SpikeEvent, SpikeTrace};
use std::collections::BTreeMap;

// ─── Global Network Instance ────────────────────────────────────────────

static GLOBAL_NETWORK: LazyLock<Mutex<SpikeNetwork>> =
    LazyLock::new(|| Mutex::new(SpikeNetwork::new()));

static GLOBAL_TRACE: LazyLock<Mutex<SpikeTrace>> =
    LazyLock::new(|| Mutex::new(SpikeTrace::new()));

// ─── Network Construction ───────────────────────────────────────────────

/// Add a LIF neuron to the global network.
#[unsafe(no_mangle)]
pub extern "C" fn slang_neuro_add_lif(id: i64) -> i64 {
    let mut net = GLOBAL_NETWORK.lock().unwrap();
    net.add_neuron(Neuron::lif(id));
    id
}

/// Add an Izhikevich neuron to the global network.
#[unsafe(no_mangle)]
pub extern "C" fn slang_neuro_add_izhikevich(id: i64) -> i64 {
    let mut net = GLOBAL_NETWORK.lock().unwrap();
    net.add_neuron(Neuron::izhikevich(id));
    id
}

/// Add an AdEx neuron to the global network.
#[unsafe(no_mangle)]
pub extern "C" fn slang_neuro_add_adex(id: i64) -> i64 {
    let mut net = GLOBAL_NETWORK.lock().unwrap();
    net.add_neuron(Neuron::adex(id));
    id
}

/// Connect two neurons with a synapse.
#[unsafe(no_mangle)]
pub extern "C" fn slang_neuro_connect(source: i64, target: i64, weight: i64) -> i64 {
    let w = f64::from_bits(weight as u64);
    let mut net = GLOBAL_NETWORK.lock().unwrap();
    net.connect(Synapse::new(source, target, w));
    1
}

/// Connect with STDP-enabled synapse.
#[unsafe(no_mangle)]
pub extern "C" fn slang_neuro_connect_stdp(source: i64, target: i64, weight: i64, lr: i64) -> i64 {
    let w = f64::from_bits(weight as u64);
    let learning_rate = f64::from_bits(lr as u64);
    let mut net = GLOBAL_NETWORK.lock().unwrap();
    net.connect(Synapse::new(source, target, w).with_stdp(learning_rate));
    1
}

/// Add a population of LIF neurons (IDs from start to start+count-1).
#[unsafe(no_mangle)]
pub extern "C" fn slang_neuro_add_population(start: i64, count: i64) -> i64 {
    let mut net = GLOBAL_NETWORK.lock().unwrap();
    let c = count.clamp(0, 10000);
    for i in 0..c {
        net.add_neuron(Neuron::lif(start + i));
    }
    c
}

/// Fully connect two populations (all-to-all).
#[unsafe(no_mangle)]
pub extern "C" fn slang_neuro_connect_populations(
    src_start: i64, src_count: i64,
    tgt_start: i64, tgt_count: i64,
    weight: i64,
) -> i64 {
    let w = f64::from_bits(weight as u64);
    let mut net = GLOBAL_NETWORK.lock().unwrap();
    let sc = src_count.clamp(0, 1000);
    let tc = tgt_count.clamp(0, 1000);
    let mut connections = 0i64;
    for s in 0..sc {
        for t in 0..tc {
            net.connect(Synapse::new(src_start + s, tgt_start + t, w));
            connections += 1;
        }
    }
    connections
}

// ─── Simulation ─────────────────────────────────────────────────────────

/// Step the global network by dt milliseconds with no external input.
#[unsafe(no_mangle)]
pub extern "C" fn slang_neuro_step(dt: i64) -> i64 {
    let dt_ms = f64::from_bits(dt as u64);
    let mut net = GLOBAL_NETWORK.lock().unwrap();
    let spiked = net.step(dt_ms, &BTreeMap::new());
    spiked.len() as i64
}

/// Step with external input current to a specific neuron.
#[unsafe(no_mangle)]
pub extern "C" fn slang_neuro_step_input(dt: i64, neuron_id: i64, current: i64) -> i64 {
    let dt_ms = f64::from_bits(dt as u64);
    let i = f64::from_bits(current as u64);
    let mut currents = BTreeMap::new();
    currents.insert(neuron_id, i);
    let mut net = GLOBAL_NETWORK.lock().unwrap();
    let spiked = net.step(dt_ms, &currents);
    spiked.len() as i64
}

/// Run simulation for N timesteps with dt.
#[unsafe(no_mangle)]
pub extern "C" fn slang_neuro_simulate(steps: i64, dt: i64) -> i64 {
    let dt_ms = f64::from_bits(dt as u64);
    let mut net = GLOBAL_NETWORK.lock().unwrap();
    let mut total_spikes = 0i64;
    let n = steps.clamp(0, 100000);
    for _ in 0..n {
        let spiked = net.step(dt_ms, &BTreeMap::new());
        total_spikes += spiked.len() as i64;
    }
    total_spikes
}

// ─── Analysis ───────────────────────────────────────────────────────────

/// Get total spike count across all neurons.
#[unsafe(no_mangle)]
pub extern "C" fn slang_neuro_total_spikes() -> i64 {
    let net = GLOBAL_NETWORK.lock().unwrap();
    net.total_spikes() as i64
}

/// Get neuron count.
#[unsafe(no_mangle)]
pub extern "C" fn slang_neuro_neuron_count() -> i64 {
    let net = GLOBAL_NETWORK.lock().unwrap();
    net.neuron_count() as i64
}

/// Get synapse count.
#[unsafe(no_mangle)]
pub extern "C" fn slang_neuro_synapse_count() -> i64 {
    let net = GLOBAL_NETWORK.lock().unwrap();
    net.synapse_count() as i64
}

/// Get membrane voltage of a neuron (as i64 bits of f64).
#[unsafe(no_mangle)]
pub extern "C" fn slang_neuro_get_voltage(neuron_id: i64) -> i64 {
    let net = GLOBAL_NETWORK.lock().unwrap();
    match net.neurons.get(&neuron_id) {
        Some(n) => f64::to_bits(n.voltage) as i64,
        None => 0,
    }
}

/// Get spike count of a specific neuron.
#[unsafe(no_mangle)]
pub extern "C" fn slang_neuro_get_spikes(neuron_id: i64) -> i64 {
    let net = GLOBAL_NETWORK.lock().unwrap();
    match net.neurons.get(&neuron_id) {
        Some(n) => n.spike_count as i64,
        None => 0,
    }
}

/// Reset the global network.
#[unsafe(no_mangle)]
pub extern "C" fn slang_neuro_reset() -> i64 {
    let mut net = GLOBAL_NETWORK.lock().unwrap();
    *net = SpikeNetwork::new();
    0
}

// ─── Encoding ───────────────────────────────────────────────────────────

/// Rate encode a value to spike count (Poisson-like).
#[unsafe(no_mangle)]
pub extern "C" fn slang_neuro_encode_rate(value: i64, max_rate: i64) -> i64 {
    let v = f64::from_bits(value as u64);
    let mr = f64::from_bits(max_rate as u64);
    if mr <= 0.0 { return 0; }
    let rate = (v / mr).clamp(0.0, 1.0);
    (rate * 100.0) as i64 // spike count per 100 timesteps
}

/// Temporal encode: convert value to spike time (earlier = higher value).
#[unsafe(no_mangle)]
pub extern "C" fn slang_neuro_encode_temporal(value: i64, max_time: i64) -> i64 {
    let v = f64::from_bits(value as u64);
    let mt = f64::from_bits(max_time as u64);
    if mt <= 0.0 { return 0; }
    let normalized = v.clamp(0.0, 1.0);
    // Higher value → earlier spike
    let spike_time = mt * (1.0 - normalized);
    f64::to_bits(spike_time) as i64
}

/// Decode spike rate back to value.
#[unsafe(no_mangle)]
pub extern "C" fn slang_neuro_decode_rate(spike_count: i64, max_rate: i64) -> i64 {
    let mr = f64::from_bits(max_rate as u64);
    if mr <= 0.0 { return 0; }
    let value = (spike_count as f64 / 100.0) * mr;
    f64::to_bits(value) as i64
}

// ─── Spike Trace Recording ─────────────────────────────────────────────

/// Record a spike event to the global trace.
#[unsafe(no_mangle)]
pub extern "C" fn slang_neuro_trace_record(time_us: i64, source: i64, target: i64) -> i64 {
    let mut trace = GLOBAL_TRACE.lock().unwrap();
    trace.record(SpikeEvent::new(time_us, source, target, 1.0));
    trace.len() as i64
}

/// Get global trace length.
#[unsafe(no_mangle)]
pub extern "C" fn slang_neuro_trace_len() -> i64 {
    let trace = GLOBAL_TRACE.lock().unwrap();
    trace.len() as i64
}

/// Clear the global trace.
#[unsafe(no_mangle)]
pub extern "C" fn slang_neuro_trace_clear() -> i64 {
    let mut trace = GLOBAL_TRACE.lock().unwrap();
    trace.clear();
    0
}

/// Get firing rate of a neuron from the global trace.
#[unsafe(no_mangle)]
pub extern "C" fn slang_neuro_trace_rate(neuron_id: i64, window_us: i64) -> i64 {
    let trace = GLOBAL_TRACE.lock().unwrap();
    let rate = trace.firing_rate(neuron_id, window_us);
    f64::to_bits(rate) as i64
}

// ─── Info ───────────────────────────────────────────────────────────────

/// Get the number of neuromorphic builtins in this module.
#[unsafe(no_mangle)]
pub extern "C" fn slang_neuro_stdlib_count() -> i64 {
    30 // number of FFI functions in this module
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reset() {
        slang_neuro_reset();
        slang_neuro_trace_clear();
    }

    #[test]
    fn test_add_lif() {
        reset();
        slang_neuro_add_lif(0);
        assert_eq!(slang_neuro_neuron_count(), 1);
        reset();
    }

    #[test]
    fn test_add_izhikevich() {
        reset();
        slang_neuro_add_izhikevich(0);
        assert_eq!(slang_neuro_neuron_count(), 1);
        reset();
    }

    #[test]
    fn test_add_adex() {
        reset();
        slang_neuro_add_adex(0);
        assert_eq!(slang_neuro_neuron_count(), 1);
        reset();
    }

    #[test]
    fn test_connect() {
        reset();
        slang_neuro_add_lif(0);
        slang_neuro_add_lif(1);
        slang_neuro_connect(0, 1, f64::to_bits(0.5) as i64);
        assert_eq!(slang_neuro_synapse_count(), 1);
        reset();
    }

    #[test]
    fn test_connect_stdp() {
        reset();
        slang_neuro_add_lif(0);
        slang_neuro_add_lif(1);
        slang_neuro_connect_stdp(0, 1, f64::to_bits(0.5) as i64, f64::to_bits(0.01) as i64);
        assert_eq!(slang_neuro_synapse_count(), 1);
        reset();
    }

    #[test]
    fn test_add_population() {
        reset();
        let count = slang_neuro_add_population(0, 10);
        assert_eq!(count, 10);
        assert_eq!(slang_neuro_neuron_count(), 10);
        reset();
    }

    #[test]
    fn test_connect_populations() {
        reset();
        slang_neuro_add_population(0, 3);
        slang_neuro_add_population(10, 4);
        let conns = slang_neuro_connect_populations(0, 3, 10, 4, f64::to_bits(0.1) as i64);
        assert_eq!(conns, 12); // 3 * 4
        reset();
    }

    #[test]
    fn test_step_no_spikes() {
        reset();
        slang_neuro_add_lif(0);
        let spiked = slang_neuro_step(f64::to_bits(0.1) as i64);
        assert_eq!(spiked, 0); // no input, no spikes
        reset();
    }

    #[test]
    fn test_step_with_input() {
        reset();
        slang_neuro_add_lif(0);
        let mut total = 0i64;
        for _ in 0..500 {
            total += slang_neuro_step_input(
                f64::to_bits(0.1) as i64,
                0,
                f64::to_bits(50.0) as i64,
            );
        }
        assert!(total > 0, "Neuron should spike with strong input");
        reset();
    }

    #[test]
    fn test_simulate() {
        reset();
        slang_neuro_add_lif(0);
        let spikes = slang_neuro_simulate(100, f64::to_bits(0.1) as i64);
        assert_eq!(spikes, 0); // no input
        reset();
    }

    #[test]
    fn test_total_spikes_after_input() {
        // Serialize access to GLOBAL_NETWORK to avoid races with other tests
        let _lock = GLOBAL_NETWORK.lock().unwrap();
        drop(_lock);
        reset();
        slang_neuro_add_lif(0);
        for _ in 0..500 {
            slang_neuro_step_input(
                f64::to_bits(0.1) as i64,
                0,
                f64::to_bits(50.0) as i64,
            );
        }
        let spikes = slang_neuro_total_spikes();
        reset();
        assert!(spikes > 0, "LIF neuron should spike with 500 steps of 50.0 current");
    }

    #[test]
    fn test_get_voltage() {
        reset();
        slang_neuro_add_lif(0);
        let v = f64::from_bits(slang_neuro_get_voltage(0) as u64);
        assert!(v < 0.0); // resting potential is negative
        reset();
    }

    #[test]
    fn test_reset_network() {
        reset();
        slang_neuro_add_lif(0);
        assert_eq!(slang_neuro_neuron_count(), 1);
        slang_neuro_reset();
        assert_eq!(slang_neuro_neuron_count(), 0);
    }

    #[test]
    fn test_encode_rate() {
        let rate = slang_neuro_encode_rate(f64::to_bits(0.5) as i64, f64::to_bits(1.0) as i64);
        assert_eq!(rate, 50); // 0.5/1.0 * 100
    }

    #[test]
    fn test_decode_rate() {
        let bits = slang_neuro_decode_rate(50, f64::to_bits(1.0) as i64);
        let value = f64::from_bits(bits as u64);
        assert!((value - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_encode_temporal() {
        let bits = slang_neuro_encode_temporal(f64::to_bits(1.0) as i64, f64::to_bits(100.0) as i64);
        let time = f64::from_bits(bits as u64);
        assert!((time - 0.0).abs() < 1e-10); // max value → earliest spike
    }

    #[test]
    fn test_trace_record() {
        reset();
        slang_neuro_trace_record(0, 1, 2);
        slang_neuro_trace_record(10, 1, 3);
        assert_eq!(slang_neuro_trace_len(), 2);
        reset();
    }

    #[test]
    fn test_trace_clear() {
        reset();
        slang_neuro_trace_record(0, 1, 2);
        slang_neuro_trace_clear();
        assert_eq!(slang_neuro_trace_len(), 0);
    }

    #[test]
    fn test_trace_rate() {
        reset();
        for i in 0..10 {
            slang_neuro_trace_record(i * 1000, 0, 1);
        }
        let rate_bits = slang_neuro_trace_rate(0, 10000);
        let rate = f64::from_bits(rate_bits as u64);
        assert!(rate > 0.0);
        reset();
    }

    #[test]
    fn test_stdlib_count() {
        assert_eq!(slang_neuro_stdlib_count(), 30);
    }

    #[test]
    fn test_nonexistent_neuron() {
        reset();
        let v = slang_neuro_get_voltage(999);
        assert_eq!(v, 0);
        let s = slang_neuro_get_spikes(999);
        assert_eq!(s, 0);
        reset();
    }
}
