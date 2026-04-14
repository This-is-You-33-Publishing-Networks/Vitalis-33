//! Distributed Neuromorphic Computing (v261–v266)
//!
//! Cluster management, spike consensus, federated neuro,
//! edge deployment, streaming spikes, cross-platform.

use std::sync::atomic::{AtomicI64, Ordering};

// ── v261: Neuromorphic Cluster ───────────────────────────────────────────────

static CLUSTER_NODES: AtomicI64 = AtomicI64::new(1);
static CLUSTER_LOAD_SUM: AtomicI64 = AtomicI64::new(0);

/// Initialize neuromorphic cluster. Returns node count.
#[unsafe(no_mangle)]
pub extern "C" fn slang_neuro_cluster_init(n_nodes: i64) -> i64 {
    let nodes = n_nodes.max(1);
    CLUSTER_NODES.store(nodes, Ordering::SeqCst);
    CLUSTER_LOAD_SUM.store(0, Ordering::SeqCst);
    nodes
}

/// Distribute SNN layers across cluster nodes. Returns layers per node.
#[unsafe(no_mangle)]
pub extern "C" fn slang_neuro_cluster_distribute(n_layers: i64) -> i64 {
    let nodes = CLUSTER_NODES.load(Ordering::SeqCst);
    (n_layers + nodes - 1) / nodes
}

/// Report node load. Returns average cluster load (0-1000).
#[unsafe(no_mangle)]
pub extern "C" fn slang_neuro_cluster_load(node_load: i64) -> i64 {
    CLUSTER_LOAD_SUM.fetch_add(node_load.clamp(0, 1000), Ordering::SeqCst);
    let nodes = CLUSTER_NODES.load(Ordering::SeqCst);
    let total = CLUSTER_LOAD_SUM.load(Ordering::SeqCst);
    total / nodes
}

/// Get cluster node count.
#[unsafe(no_mangle)]
pub extern "C" fn slang_neuro_cluster_nodes() -> i64 {
    CLUSTER_NODES.load(Ordering::SeqCst)
}

// ── v262: Spike Consensus ────────────────────────────────────────────────────

/// Spike-based consensus: majority vote. Returns consensus value.
#[unsafe(no_mangle)]
pub extern "C" fn slang_spike_consensus_vote(votes_for: i64, votes_against: i64) -> i64 {
    if votes_for > votes_against { 1 } else { 0 }
}

/// Byzantine fault tolerance threshold. Returns max faulty nodes.
#[unsafe(no_mangle)]
pub extern "C" fn slang_spike_consensus_bft(n_nodes: i64) -> i64 {
    (n_nodes - 1) / 3
}

/// Spike synchronization clock. Returns global tick.
#[unsafe(no_mangle)]
pub extern "C" fn slang_spike_consensus_tick(local_ticks: i64, drift_us: i64) -> i64 {
    local_ticks + drift_us / 1000
}

/// Consensus latency estimate. Returns latency in microseconds.
#[unsafe(no_mangle)]
pub extern "C" fn slang_spike_consensus_latency(n_nodes: i64, msg_size_bytes: i64) -> i64 {
    let rounds = (n_nodes as f64).log2().ceil().round() as i64;
    rounds * (100 + msg_size_bytes / 1000)
}

// ── v263: Federated Neuromorphic ─────────────────────────────────────────────

/// Federated SNN averaging. Returns averaged weight ×1000.
#[unsafe(no_mangle)]
pub extern "C" fn slang_fed_neuro_average(local_weight: f64, global_weight: f64, alpha: f64) -> i64 {
    let avg = alpha * local_weight + (1.0 - alpha) * global_weight;
    (avg * 1000.0).round() as i64
}

/// Federated spike privacy: differential privacy noise. Returns noisy spike count.
#[unsafe(no_mangle)]
pub extern "C" fn slang_fed_neuro_dp_noise(spike_count: i64, epsilon: f64) -> i64 {
    let noise = (1.0 / epsilon.max(0.01)) as i64;
    spike_count + noise
}

/// Federated gradient compression. Returns compressed gradient count.
#[unsafe(no_mangle)]
pub extern "C" fn slang_fed_neuro_compress(n_grads: i64, top_k_pct: f64) -> i64 {
    (n_grads as f64 * top_k_pct.clamp(0.01, 1.0)).round() as i64
}

/// Federated round counter.
#[unsafe(no_mangle)]
pub extern "C" fn slang_fed_neuro_round(current_round: i64) -> i64 {
    current_round + 1
}

// ── v264: Edge Neuromorphic Deployment ───────────────────────────────────────

/// Estimate edge power budget. Returns max neurons for given power (mW).
#[unsafe(no_mangle)]
pub extern "C" fn slang_edge_neuro_budget(power_mw: i64) -> i64 {
    power_mw * 50_000
}

/// Edge quantization savings percentage.
#[unsafe(no_mangle)]
pub extern "C" fn slang_edge_neuro_quantize(original_bits: i64, target_bits: i64) -> i64 {
    if original_bits <= 0 { return 0; }
    ((1.0 - target_bits.max(1) as f64 / original_bits as f64) * 100.0).round() as i64
}

/// Edge latency requirement check. Returns 1 if met, 0 if not.
#[unsafe(no_mangle)]
pub extern "C" fn slang_edge_neuro_latency_ok(latency_us: i64, deadline_us: i64) -> i64 {
    if latency_us <= deadline_us { 1 } else { 0 }
}

/// Edge model size in KB.
#[unsafe(no_mangle)]
pub extern "C" fn slang_edge_neuro_model_size(n_params: i64, bits_per_param: i64) -> i64 {
    n_params * bits_per_param / 8 / 1024
}

// ── v265: Streaming Spikes ───────────────────────────────────────────────────

static STREAM_PROCESSED: AtomicI64 = AtomicI64::new(0);

/// Process streaming spike window. Returns spikes processed.
#[unsafe(no_mangle)]
pub extern "C" fn slang_stream_spike_process(window_size: i64, spike_count: i64) -> i64 {
    let processed = spike_count.min(window_size);
    STREAM_PROCESSED.fetch_add(processed, Ordering::SeqCst);
    processed
}

/// Streaming spike rate estimation ×1000.
#[unsafe(no_mangle)]
pub extern "C" fn slang_stream_spike_rate(spikes_in_window: i64, window_ms: f64) -> i64 {
    if window_ms <= 0.0 { return 0; }
    (spikes_in_window as f64 / window_ms * 1000.0).round() as i64
}

/// Streaming backpressure signal. Returns 1 if backpressure needed.
#[unsafe(no_mangle)]
pub extern "C" fn slang_stream_spike_backpressure(queue_size: i64, max_queue: i64) -> i64 {
    if queue_size > max_queue * 8 / 10 { 1 } else { 0 }
}

/// Get total streaming spikes processed.
#[unsafe(no_mangle)]
pub extern "C" fn slang_stream_spike_total() -> i64 {
    STREAM_PROCESSED.load(Ordering::SeqCst)
}

// ── v266: Cross-Platform Neuromorphic ────────────────────────────────────────

/// Platform capability index: 0=CPU,1=GPU,2=Loihi,3=SpiNNaker,4=BrainScaleS.
#[unsafe(no_mangle)]
pub extern "C" fn slang_neuro_platform_caps(platform: i64) -> i64 {
    match platform {
        0 => 100, 1 => 800, 2 => 950, 3 => 900, 4 => 850, _ => 50,
    }
}

/// Map neurons to hardware cores. Returns core utilization (0-1000).
#[unsafe(no_mangle)]
pub extern "C" fn slang_neuro_platform_map(n_neurons: i64, cores_available: i64) -> i64 {
    if cores_available <= 0 { return 0; }
    let per_core = (n_neurons + cores_available - 1) / cores_available;
    let util = per_core as f64 / 1024.0;
    (util.clamp(0.0, 1.0) * 1000.0).round() as i64
}

/// Cross-platform spike format conversion overhead (0-1000).
#[unsafe(no_mangle)]
pub extern "C" fn slang_neuro_platform_overhead(source: i64, target: i64) -> i64 {
    if source == target { return 0; }
    let diff = (source - target).unsigned_abs() as i64;
    (diff * 100).min(500)
}

/// Platform-specific power estimate in mW.
#[unsafe(no_mangle)]
pub extern "C" fn slang_neuro_platform_power(platform: i64, n_neurons: i64) -> i64 {
    let per_neuron_uw = match platform {
        0 => 1000, 1 => 100, 2 => 1, 3 => 5, 4 => 2, _ => 500,
    };
    n_neurons * per_neuron_uw / 1000
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cluster_init() {
        let n = slang_neuro_cluster_init(8);
        assert_eq!(n, 8);
        assert_eq!(slang_neuro_cluster_nodes(), 8);
    }

    #[test]
    fn test_cluster_distribute() {
        CLUSTER_NODES.store(4, Ordering::SeqCst);
        assert_eq!(slang_neuro_cluster_distribute(10), 3);
    }

    #[test]
    fn test_spike_consensus_bft() {
        assert_eq!(slang_spike_consensus_bft(10), 3);
    }

    #[test]
    fn test_fed_neuro_average() {
        assert_eq!(slang_fed_neuro_average(1.0, 0.0, 0.5), 500);
    }

    #[test]
    fn test_edge_budget() {
        assert_eq!(slang_edge_neuro_budget(100), 5_000_000);
    }

    #[test]
    fn test_edge_quantize() {
        assert_eq!(slang_edge_neuro_quantize(32, 8), 75);
    }

    #[test]
    fn test_stream_spike_process() {
        STREAM_PROCESSED.store(0, Ordering::SeqCst);
        assert_eq!(slang_stream_spike_process(100, 50), 50);
    }

    #[test]
    fn test_platform_caps() {
        assert_eq!(slang_neuro_platform_caps(2), 950);
    }

    #[test]
    fn test_platform_power() {
        assert_eq!(slang_neuro_platform_power(2, 1_000_000), 1000);
    }

    #[test]
    fn test_consensus_vote() {
        assert_eq!(slang_spike_consensus_vote(5, 3), 1);
        assert_eq!(slang_spike_consensus_vote(2, 5), 0);
    }
}
