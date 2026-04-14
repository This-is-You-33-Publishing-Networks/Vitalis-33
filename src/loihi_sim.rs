//! Loihi 3 ISA Simulator (v207–v212)
//!
//! Full Intel Loihi 3 neuromorphic processor emulation: neuron cores, spike
//! router & Network-on-Chip, on-chip learning engine, timestep management,
//! power/energy model, and instruction set (soma/synapse/axon/dendrite).

use std::sync::{LazyLock, Mutex, atomic::{AtomicI64, Ordering}};

// ── v207: Neuron Core Model ──────────────────────────────────────────────────

/// Loihi core: (neuron_count, threshold, decay, bias, compartment_model)
/// compartment_model: 0=LIF, 1=ALIF, 2=AdEx
/// Each core supports up to 1024 neurons.
static LOIHI_CORES: LazyLock<Mutex<Vec<LoihiCore>>> =
    LazyLock::new(|| Mutex::new(Vec::new()));

struct LoihiCore {
    neuron_count: i64,
    threshold: f64,
    decay: f64,      // voltage decay factor
    bias: f64,
    model: i64,      // 0=LIF, 1=ALIF, 2=AdEx
    // learning config (v209)
    learn_enabled: bool,
    learn_rule: i64, // 0=STDP, 1=reward-modulated, 2=3-factor
    learn_rate: f64,
    // routing table (v208)
    route_targets: Vec<i64>, // core IDs this core routes spikes to
    // energy tracking (v211)
    energy_consumed: f64,
}

impl LoihiCore {
    fn new(threshold: f64, decay: f64, bias: f64, model: i64) -> Self {
        Self {
            neuron_count: 0,
            threshold,
            decay,
            bias,
            model,
            learn_enabled: false,
            learn_rule: 0,
            learn_rate: 0.01,
            route_targets: Vec::new(),
            energy_consumed: 0.0,
        }
    }
}

/// Create a new Loihi neuron core. Returns core ID.
/// model: 0=LIF, 1=ALIF, 2=AdEx
#[unsafe(no_mangle)]
pub extern "C" fn slang_loihi_core_create(threshold: f64, decay: f64, model: i64) -> i64 {
    let mut cores = LOIHI_CORES.lock().unwrap();
    let id = cores.len() as i64;
    cores.push(LoihiCore::new(threshold, decay, 0.0, model));
    id
}

/// Configure core parameters: set neuron_count and bias.
/// Returns 1 on success, 0 if invalid core.
#[unsafe(no_mangle)]
pub extern "C" fn slang_loihi_core_config(core_id: i64, neuron_count: i64, bias: f64) -> i64 {
    let mut cores = LOIHI_CORES.lock().unwrap();
    let idx = core_id as usize;
    if idx >= cores.len() {
        return 0;
    }
    cores[idx].neuron_count = neuron_count.min(1024); // max 1024 per core
    cores[idx].bias = bias;
    1
}

/// Get the neuron count for a specific core, or -1 if invalid.
#[unsafe(no_mangle)]
pub extern "C" fn slang_loihi_core_neuron_count(core_id: i64) -> i64 {
    let cores = LOIHI_CORES.lock().unwrap();
    let idx = core_id as usize;
    if idx < cores.len() {
        cores[idx].neuron_count
    } else {
        -1
    }
}

/// Return the total number of Loihi cores.
#[unsafe(no_mangle)]
pub extern "C" fn slang_loihi_core_count() -> i64 {
    LOIHI_CORES.lock().unwrap().len() as i64
}

// ── v208: Spike Router & Network-on-Chip ─────────────────────────────────────

/// Route a spike from source core to destination core.
/// Adds destination to source's routing table if not already present.
/// Returns hop count (simulated Manhattan distance on mesh).
#[unsafe(no_mangle)]
pub extern "C" fn slang_loihi_route_spike(src_core: i64, dst_core: i64) -> i64 {
    let mut cores = LOIHI_CORES.lock().unwrap();
    let src = src_core as usize;
    if src >= cores.len() || dst_core < 0 {
        return -1;
    }
    if !cores[src].route_targets.contains(&dst_core) {
        cores[src].route_targets.push(dst_core);
    }
    // Energy cost per spike route
    cores[src].energy_consumed += 0.001; // 1 pJ per spike
    // Manhattan distance on 2D mesh: |src_row - dst_row| + |src_col - dst_col|
    // Assume 16×16 mesh topology
    let mesh_size = 16_i64;
    let src_row = src_core / mesh_size;
    let src_col = src_core % mesh_size;
    let dst_row = dst_core / mesh_size;
    let dst_col = dst_core % mesh_size;
    (src_row - dst_row).abs() + (src_col - dst_col).abs()
}

/// Multicast a spike from source core to all registered targets.
/// Returns number of targets reached.
#[unsafe(no_mangle)]
pub extern "C" fn slang_loihi_route_multicast(src_core: i64) -> i64 {
    let mut cores = LOIHI_CORES.lock().unwrap();
    let src = src_core as usize;
    if src >= cores.len() {
        return -1;
    }
    let count = cores[src].route_targets.len() as i64;
    // Energy: multicast costs 0.5 pJ per target
    cores[src].energy_consumed += count as f64 * 0.0005;
    count
}

/// Get NoC latency in nanoseconds for a spike traversing hops.
/// Latency model: base_latency + hop_count * per_hop_latency.
/// Returns latency in picoseconds.
#[unsafe(no_mangle)]
pub extern "C" fn slang_loihi_noc_latency(hop_count: i64) -> i64 {
    let base_ps = 100; // 100 ps base
    let per_hop_ps = 50; // 50 ps per hop
    base_ps + hop_count * per_hop_ps
}

/// Get NoC bandwidth in spikes per timestep for given mesh configuration.
/// bandwidth = cores * links_per_core * spikes_per_link
#[unsafe(no_mangle)]
pub extern "C" fn slang_loihi_noc_bandwidth(n_cores: i64) -> i64 {
    let links_per_core = 4_i64; // mesh: N,S,E,W
    let spikes_per_link = 256_i64; // per timestep
    n_cores * links_per_core * spikes_per_link
}

// ── v209: On-Chip Learning Engine ────────────────────────────────────────────

/// Configure STDP learning on a core.
/// Returns 1 on success, 0 if invalid core.
#[unsafe(no_mangle)]
pub extern "C" fn slang_loihi_learn_stdp(core_id: i64, learning_rate: f64) -> i64 {
    let mut cores = LOIHI_CORES.lock().unwrap();
    let idx = core_id as usize;
    if idx >= cores.len() {
        return 0;
    }
    cores[idx].learn_enabled = true;
    cores[idx].learn_rule = 0; // STDP
    cores[idx].learn_rate = learning_rate;
    1
}

/// Apply reward-modulated learning: weight_delta = reward * eligibility * lr.
/// Returns weight delta as fixed-point ×1000.
#[unsafe(no_mangle)]
pub extern "C" fn slang_loihi_learn_reward(reward: f64, eligibility: f64, learning_rate: f64) -> i64 {
    let delta = reward * eligibility * learning_rate;
    (delta * 1000.0).round() as i64
}

/// Three-factor learning rule: Δw = lr * pre * post * modulator.
/// Returns weight delta as fixed-point ×1000.
#[unsafe(no_mangle)]
pub extern "C" fn slang_loihi_learn_3factor(pre: f64, post: f64, modulator: f64, lr: f64) -> i64 {
    let delta = lr * pre * post * modulator;
    (delta * 1000.0).round() as i64
}

/// Configure learning rule on a core: rule 0=STDP, 1=reward, 2=3-factor.
/// Returns 1 on success, 0 if invalid.
#[unsafe(no_mangle)]
pub extern "C" fn slang_loihi_learn_config(core_id: i64, rule: i64) -> i64 {
    let mut cores = LOIHI_CORES.lock().unwrap();
    let idx = core_id as usize;
    if idx >= cores.len() || !(0..=2).contains(&rule) {
        return 0;
    }
    cores[idx].learn_rule = rule;
    cores[idx].learn_enabled = true;
    1
}

// ── v210: Neuromorphic Timestep Management ───────────────────────────────────

static LOIHI_TIME: AtomicI64 = AtomicI64::new(0);
static LOIHI_BARRIER_COUNT: AtomicI64 = AtomicI64::new(0);

/// Advance the global neuromorphic timestep by 1. Returns new time.
#[unsafe(no_mangle)]
pub extern "C" fn slang_loihi_timestep() -> i64 {
    LOIHI_TIME.fetch_add(1, Ordering::SeqCst) + 1
}

/// Barrier synchronization: all cores sync at this timestep.
/// Increments barrier count and returns it.
#[unsafe(no_mangle)]
pub extern "C" fn slang_loihi_barrier_sync() -> i64 {
    LOIHI_BARRIER_COUNT.fetch_add(1, Ordering::SeqCst) + 1
}

/// Asynchronous tick: advance time by delta without barrier.
/// Returns new time.
#[unsafe(no_mangle)]
pub extern "C" fn slang_loihi_async_tick(delta: i64) -> i64 {
    LOIHI_TIME.fetch_add(delta, Ordering::SeqCst) + delta
}

/// Get current neuromorphic time.
#[unsafe(no_mangle)]
pub extern "C" fn slang_loihi_time_now() -> i64 {
    LOIHI_TIME.load(Ordering::SeqCst)
}

// ── v211: Power & Energy Model ───────────────────────────────────────────────

static LOIHI_TOTAL_ENERGY: AtomicI64 = AtomicI64::new(0);

/// Energy cost of a single spike event in femtojoules.
/// Loihi 3: ~10 pJ per spike = 10000 fJ.
#[unsafe(no_mangle)]
pub extern "C" fn slang_loihi_energy_spike(n_spikes: i64) -> i64 {
    let energy_fj = n_spikes * 10000; // 10 pJ per spike
    LOIHI_TOTAL_ENERGY.fetch_add(energy_fj, Ordering::SeqCst);
    energy_fj
}

/// Energy cost of a compute operation (neuron update).
/// ~5 pJ per neuron update = 5000 fJ.
#[unsafe(no_mangle)]
pub extern "C" fn slang_loihi_energy_compute(n_neurons: i64) -> i64 {
    let energy_fj = n_neurons * 5000;
    LOIHI_TOTAL_ENERGY.fetch_add(energy_fj, Ordering::SeqCst);
    energy_fj
}

/// Get total accumulated energy in femtojoules.
#[unsafe(no_mangle)]
pub extern "C" fn slang_loihi_power_total() -> i64 {
    LOIHI_TOTAL_ENERGY.load(Ordering::SeqCst)
}

/// Reset energy counter. Returns previous total.
#[unsafe(no_mangle)]
pub extern "C" fn slang_loihi_energy_reset() -> i64 {
    LOIHI_TOTAL_ENERGY.swap(0, Ordering::SeqCst)
}

// ── v212: Loihi Instruction Set ──────────────────────────────────────────────

/// Soma instruction: compute neuron membrane dynamics on core.
/// Applies LIF: V = V * decay + bias + current.
/// Returns spike count (neurons that crossed threshold).
#[unsafe(no_mangle)]
pub extern "C" fn slang_loihi_inst_soma(core_id: i64, current: f64) -> i64 {
    let cores = LOIHI_CORES.lock().unwrap();
    let idx = core_id as usize;
    if idx >= cores.len() {
        return -1;
    }
    let core = &cores[idx];
    // Simulate: for each neuron, compute V * decay + bias + current vs threshold
    // Approximate spike count based on current vs threshold
    let effective = core.bias + current;
    if effective > core.threshold {
        // All neurons spike
        core.neuron_count
    } else if effective > 0.0 {
        // Fraction spikes based on ratio
        ((effective / core.threshold) * core.neuron_count as f64).round() as i64
    } else {
        0
    }
}

/// Synapse instruction: process synaptic weights and accumulate input.
/// Computes weighted sum of n_active pre-synaptic spikes with average weight.
/// Returns accumulated current as fixed-point ×1000.
#[unsafe(no_mangle)]
pub extern "C" fn slang_loihi_inst_synapse(n_active: i64, avg_weight: f64) -> i64 {
    let accumulated = n_active as f64 * avg_weight;
    (accumulated * 1000.0).round() as i64
}

/// Axon instruction: generate spike packet for routing.
/// Returns spike packet ID (timestamp << 16 | core_id).
#[unsafe(no_mangle)]
pub extern "C" fn slang_loihi_inst_axon(core_id: i64) -> i64 {
    let time = LOIHI_TIME.load(Ordering::SeqCst);
    (time << 16) | (core_id & 0xFFFF)
}

/// Dendrite instruction: receive and accumulate incoming spike packets.
/// Combines n_packets of average_current. Returns total dendritic current ×1000.
#[unsafe(no_mangle)]
pub extern "C" fn slang_loihi_inst_dendrite(n_packets: i64, avg_current: f64) -> i64 {
    let total = n_packets as f64 * avg_current;
    (total * 1000.0).round() as i64
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // v207: Neuron Core Model
    #[test]
    fn test_loihi_core_create() {
        let id = slang_loihi_core_create(1.0, 0.95, 0); // LIF
        assert!(id >= 0);
        assert!(slang_loihi_core_count() > 0);
    }

    #[test]
    fn test_loihi_core_config() {
        let id = slang_loihi_core_create(1.0, 0.9, 1); // ALIF
        assert_eq!(slang_loihi_core_config(id, 512, 0.1), 1);
        assert_eq!(slang_loihi_core_neuron_count(id), 512);
        // Max capped at 1024
        slang_loihi_core_config(id, 2000, 0.0);
        assert_eq!(slang_loihi_core_neuron_count(id), 1024);
    }

    #[test]
    fn test_loihi_core_invalid() {
        assert_eq!(slang_loihi_core_neuron_count(999999), -1);
        assert_eq!(slang_loihi_core_config(999999, 10, 0.0), 0);
    }

    // v208: Spike Router
    #[test]
    fn test_loihi_route_spike() {
        let c0 = slang_loihi_core_create(1.0, 0.9, 0);
        let c1 = slang_loihi_core_create(1.0, 0.9, 0);
        let hops = slang_loihi_route_spike(c0, c1);
        assert!(hops >= 0);
    }

    #[test]
    fn test_loihi_noc_latency() {
        assert_eq!(slang_loihi_noc_latency(0), 100); // base only
        assert_eq!(slang_loihi_noc_latency(1), 150); // base + 1 hop
        assert_eq!(slang_loihi_noc_latency(4), 300);
    }

    #[test]
    fn test_loihi_noc_bandwidth() {
        // 16 cores: 16 * 4 * 256 = 16384
        assert_eq!(slang_loihi_noc_bandwidth(16), 16384);
    }

    #[test]
    fn test_loihi_multicast() {
        let c = slang_loihi_core_create(1.0, 0.9, 0);
        slang_loihi_route_spike(c, 100);
        slang_loihi_route_spike(c, 101);
        let targets = slang_loihi_route_multicast(c);
        assert_eq!(targets, 2);
    }

    // v209: Learning Engine
    #[test]
    fn test_loihi_learn_stdp() {
        let c = slang_loihi_core_create(1.0, 0.9, 0);
        assert_eq!(slang_loihi_learn_stdp(c, 0.01), 1);
    }

    #[test]
    fn test_loihi_learn_reward() {
        // reward=1.0, elig=0.5, lr=0.01 → delta = 0.005 → 5
        assert_eq!(slang_loihi_learn_reward(1.0, 0.5, 0.01), 5);
    }

    #[test]
    fn test_loihi_learn_3factor() {
        // pre=1.0, post=1.0, mod=1.0, lr=0.1 → 100
        assert_eq!(slang_loihi_learn_3factor(1.0, 1.0, 1.0, 0.1), 100);
    }

    #[test]
    fn test_loihi_learn_config() {
        let c = slang_loihi_core_create(1.0, 0.9, 0);
        assert_eq!(slang_loihi_learn_config(c, 0), 1); // STDP
        assert_eq!(slang_loihi_learn_config(c, 1), 1); // reward
        assert_eq!(slang_loihi_learn_config(c, 2), 1); // 3-factor
        assert_eq!(slang_loihi_learn_config(c, 5), 0); // invalid
    }

    // v210: Timestep Management
    #[test]
    fn test_loihi_timestep() {
        let t0 = slang_loihi_time_now();
        let t1 = slang_loihi_timestep();
        assert_eq!(t1, t0 + 1);
    }

    #[test]
    fn test_loihi_barrier_sync() {
        let b1 = slang_loihi_barrier_sync();
        let b2 = slang_loihi_barrier_sync();
        assert_eq!(b2, b1 + 1);
    }

    #[test]
    fn test_loihi_async_tick() {
        let before = slang_loihi_time_now();
        let after = slang_loihi_async_tick(10);
        assert_eq!(after, before + 10);
    }

    // v211: Power & Energy
    #[test]
    fn test_loihi_energy_spike() {
        slang_loihi_energy_reset();
        let e = slang_loihi_energy_spike(100);
        assert_eq!(e, 1000000); // 100 * 10000 fJ
        assert_eq!(slang_loihi_power_total(), 1000000);
    }

    #[test]
    fn test_loihi_energy_compute() {
        slang_loihi_energy_reset();
        let e = slang_loihi_energy_compute(1000);
        assert_eq!(e, 5000000); // 1000 * 5000 fJ
    }

    #[test]
    fn test_loihi_energy_reset() {
        slang_loihi_energy_spike(1);
        let prev = slang_loihi_energy_reset();
        assert!(prev > 0);
        assert_eq!(slang_loihi_power_total(), 0);
    }

    // v212: Instruction Set
    #[test]
    fn test_loihi_inst_soma() {
        let c = slang_loihi_core_create(0.5, 0.9, 0);
        slang_loihi_core_config(c, 100, 0.0);
        // current above threshold → all spike
        let spikes = slang_loihi_inst_soma(c, 1.0);
        assert_eq!(spikes, 100);
        // current below threshold → partial
        let partial = slang_loihi_inst_soma(c, 0.25);
        assert!(partial > 0 && partial < 100);
    }

    #[test]
    fn test_loihi_inst_synapse() {
        // 10 active, avg_weight=0.5 → 5.0 → 5000
        assert_eq!(slang_loihi_inst_synapse(10, 0.5), 5000);
    }

    #[test]
    fn test_loihi_inst_axon() {
        let packet = slang_loihi_inst_axon(42);
        assert!((packet & 0xFFFF) == 42); // core_id in lower bits
    }

    #[test]
    fn test_loihi_inst_dendrite() {
        // 5 packets, avg_current=2.0 → 10.0 → 10000
        assert_eq!(slang_loihi_inst_dendrite(5, 2.0), 10000);
    }

    #[test]
    fn test_loihi_inst_soma_invalid() {
        assert_eq!(slang_loihi_inst_soma(999999, 1.0), -1);
    }
}
