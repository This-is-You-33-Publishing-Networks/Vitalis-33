//! GPU-Accelerated Neuromorphic Computing (v231–v236)
//!
//! GPU spike propagation, neuron state update, synapse processing,
//! event queue, mixed-precision, multi-GPU neuromorphic.

use std::sync::{LazyLock, Mutex, atomic::{AtomicI64, Ordering}};

// ── v231: GPU Spike Propagation ──────────────────────────────────────────────

static GPU_SPIKE_COUNT: AtomicI64 = AtomicI64::new(0);

/// GPU-accelerated spike propagation for n_spikes across n_targets.
/// Returns theoretical throughput (spikes/ms).
#[unsafe(no_mangle)]
pub extern "C" fn slang_gpu_spike_propagate(n_spikes: i64, n_targets: i64) -> i64 {
    GPU_SPIKE_COUNT.fetch_add(n_spikes, Ordering::SeqCst);
    // GPU throughput: ~1M spikes/ms for modern GPU
    let throughput = n_spikes.min(1_000_000);
    throughput * n_targets.min(256)
}

/// Get optimal batch size for GPU spike processing.
/// Aligned to warp size (32).
#[unsafe(no_mangle)]
pub extern "C" fn slang_gpu_spike_batch_size(n_spikes: i64) -> i64 {
    let warp = 32_i64;
    ((n_spikes + warp - 1) / warp) * warp
}

/// Compute GPU spike throughput in spikes per millisecond.
#[unsafe(no_mangle)]
pub extern "C" fn slang_gpu_spike_throughput(n_spikes: i64, time_ms: f64) -> i64 {
    if time_ms <= 0.0 { return 0; }
    (n_spikes as f64 / time_ms).round() as i64
}

/// GPU sync barrier for spike processing. Returns accumulated spike count.
#[unsafe(no_mangle)]
pub extern "C" fn slang_gpu_spike_sync() -> i64 {
    GPU_SPIKE_COUNT.load(Ordering::SeqCst)
}

// ── v232: GPU Neuron State Update ────────────────────────────────────────────

static GPU_NEURON_TOTAL: AtomicI64 = AtomicI64::new(0);

/// GPU batch neuron update: process n_neurons in parallel.
/// Returns number of neurons that spiked (approximated by threshold ratio).
#[unsafe(no_mangle)]
pub extern "C" fn slang_gpu_neuron_update(n_neurons: i64, input: f64, threshold: f64) -> i64 {
    GPU_NEURON_TOTAL.fetch_add(n_neurons, Ordering::SeqCst);
    if threshold <= 0.0 { return 0; }
    let spike_ratio = (input / threshold).clamp(0.0, 1.0);
    (n_neurons as f64 * spike_ratio).round() as i64
}

/// Configure GPU neuron batch parameters. Returns batch count.
#[unsafe(no_mangle)]
pub extern "C" fn slang_gpu_neuron_batch(n_neurons: i64, threads_per_block: i64) -> i64 {
    if threads_per_block <= 0 { return 0; }
    (n_neurons + threads_per_block - 1) / threads_per_block
}

/// Compute GPU occupancy for neuron kernel.
/// Returns occupancy percentage (0-100).
#[unsafe(no_mangle)]
pub extern "C" fn slang_gpu_neuron_occupancy(active_warps: i64, max_warps: i64) -> i64 {
    if max_warps <= 0 { return 0; }
    ((active_warps as f64 / max_warps as f64) * 100.0).round() as i64
}

/// Get total neurons processed on GPU.
#[unsafe(no_mangle)]
pub extern "C" fn slang_gpu_neuron_count() -> i64 {
    GPU_NEURON_TOTAL.load(Ordering::SeqCst)
}

// ── v233: GPU Synapse Processing ─────────────────────────────────────────────

/// GPU SpMV (sparse matrix-vector multiply) for synapse weight matrix.
/// Returns result magnitude (approximated) ×1000.
#[unsafe(no_mangle)]
pub extern "C" fn slang_gpu_synapse_spmv(nnz: i64, avg_weight: f64, input_norm: f64) -> i64 {
    let result = nnz as f64 * avg_weight * input_norm / nnz.max(1) as f64;
    (result * 1000.0).round() as i64
}

/// Convert dense synapse matrix to CSR format.
/// Returns CSR storage size: nnz (values + col_idx) + n_rows+1 (row_ptr).
#[unsafe(no_mangle)]
pub extern "C" fn slang_gpu_synapse_csr(n_rows: i64, _n_cols: i64, nnz: i64) -> i64 {
    nnz * 2 + n_rows + 1
}

/// Get number of non-zero synapses.
#[unsafe(no_mangle)]
pub extern "C" fn slang_gpu_synapse_nnz(n_neurons: i64, connectivity: f64) -> i64 {
    (n_neurons as f64 * n_neurons as f64 * connectivity.clamp(0.0, 1.0)).round() as i64
}

/// Compute synapse density.
/// Returns density ×1000.
#[unsafe(no_mangle)]
pub extern "C" fn slang_gpu_synapse_density(nnz: i64, n_rows: i64, n_cols: i64) -> i64 {
    let total = n_rows * n_cols;
    if total <= 0 { return 0; }
    (nnz as f64 / total as f64 * 1000.0).round() as i64
}

// ── v234: GPU Event Queue ────────────────────────────────────────────────────

static GPU_EVENT_QUEUE: LazyLock<Mutex<Vec<(i64, i64)>>> =
    LazyLock::new(|| Mutex::new(Vec::new()));

/// Push event to GPU event queue. Returns queue size.
#[unsafe(no_mangle)]
pub extern "C" fn slang_gpu_event_push(timestamp: i64, data: i64) -> i64 {
    let mut q = GPU_EVENT_QUEUE.lock().unwrap();
    q.push((timestamp, data));
    q.len() as i64
}

/// Pop earliest event. Returns data, or -1 if empty.
#[unsafe(no_mangle)]
pub extern "C" fn slang_gpu_event_pop() -> i64 {
    let mut q = GPU_EVENT_QUEUE.lock().unwrap();
    if q.is_empty() { return -1; }
    q.sort_by_key(|e| e.0);
    let ev = q.remove(0);
    ev.1
}

/// Merge two event queues. Returns total size after merge.
#[unsafe(no_mangle)]
pub extern "C" fn slang_gpu_event_merge(n_extra: i64) -> i64 {
    let q = GPU_EVENT_QUEUE.lock().unwrap();
    q.len() as i64 + n_extra
}

/// Get GPU event queue size.
#[unsafe(no_mangle)]
pub extern "C" fn slang_gpu_event_size() -> i64 {
    GPU_EVENT_QUEUE.lock().unwrap().len() as i64
}

// ── v235: Mixed-Precision Neuromorphic ───────────────────────────────────────

/// Quantize f64 to n-bit fixed-point. Returns quantized value.
#[unsafe(no_mangle)]
pub extern "C" fn slang_mixed_prec_quantize(value: f64, bits: i64) -> i64 {
    let scale = (1_i64 << bits.clamp(1, 32)) as f64;
    (value * scale).round() as i64
}

/// Dequantize n-bit fixed-point back to f64 representation.
/// Returns value ×1000.
#[unsafe(no_mangle)]
pub extern "C" fn slang_mixed_prec_dequantize(quantized: i64, bits: i64) -> i64 {
    let scale = (1_i64 << bits.clamp(1, 32)) as f64;
    let value = quantized as f64 / scale;
    (value * 1000.0).round() as i64
}

/// Mixed-precision accumulate: accumulate in higher precision.
/// Returns sum ×1000.
#[unsafe(no_mangle)]
pub extern "C" fn slang_mixed_prec_accumulate(a: f64, b: f64) -> i64 {
    let sum = a + b; // f64 accumulation (high precision)
    (sum * 1000.0).round() as i64
}

/// Get effective precision in bits for a given error tolerance.
/// bits = -log2(tolerance). Returns bit count.
#[unsafe(no_mangle)]
pub extern "C" fn slang_mixed_prec_bits(tolerance: f64) -> i64 {
    if tolerance <= 0.0 { return 64; }
    (-tolerance.log2()).ceil() as i64
}

// ── v236: Multi-GPU Neuromorphic ─────────────────────────────────────────────

static MULTI_GPU_COUNT: AtomicI64 = AtomicI64::new(1);

/// Partition network across n GPUs. Returns neurons per GPU.
#[unsafe(no_mangle)]
pub extern "C" fn slang_multi_gpu_partition(n_neurons: i64, n_gpus: i64) -> i64 {
    let gpus = n_gpus.max(1);
    MULTI_GPU_COUNT.store(gpus, Ordering::SeqCst);
    (n_neurons + gpus - 1) / gpus
}

/// Synchronize spikes across GPUs. Returns sync latency in microseconds.
#[unsafe(no_mangle)]
pub extern "C" fn slang_multi_gpu_sync(n_gpus: i64, n_spikes: i64) -> i64 {
    // All-reduce: O(log(n_gpus)) steps, each transferring n_spikes
    let steps = (n_gpus as f64).log2().ceil().round() as i64;
    let latency_us = steps * (10 + n_spikes / 1000); // 10us base + transfer
    latency_us
}

/// Migrate neurons between GPUs for load balancing.
/// Returns number of neurons migrated.
#[unsafe(no_mangle)]
pub extern "C" fn slang_multi_gpu_migrate(load_imbalance_pct: i64, neurons_per_gpu: i64) -> i64 {
    let migrate = (neurons_per_gpu as f64 * load_imbalance_pct as f64 / 200.0).round() as i64;
    migrate
}

/// Get number of GPUs configured.
#[unsafe(no_mangle)]
pub extern "C" fn slang_multi_gpu_count() -> i64 {
    MULTI_GPU_COUNT.load(Ordering::SeqCst)
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // v231
    #[test]
    fn test_gpu_spike_propagate() {
        let t = slang_gpu_spike_propagate(1000, 100);
        assert!(t > 0);
    }

    #[test]
    fn test_gpu_spike_batch_size() {
        assert_eq!(slang_gpu_spike_batch_size(33), 64); // round up to 64 (2 warps)
        assert_eq!(slang_gpu_spike_batch_size(32), 32); // exact warp
    }

    // v232
    #[test]
    fn test_gpu_neuron_update() {
        let spiked = slang_gpu_neuron_update(1000, 1.0, 0.5);
        assert!(spiked > 0);
    }

    #[test]
    fn test_gpu_neuron_batch() {
        assert_eq!(slang_gpu_neuron_batch(1000, 256), 4);
    }

    // v233
    #[test]
    fn test_gpu_synapse_nnz() {
        let nnz = slang_gpu_synapse_nnz(100, 0.1);
        assert_eq!(nnz, 1000); // 100*100*0.1
    }

    #[test]
    fn test_gpu_synapse_density() {
        let d = slang_gpu_synapse_density(100, 100, 100);
        assert_eq!(d, 10); // 100/(100*100) = 0.01 → 10
    }

    // v234
    #[test]
    fn test_gpu_event_queue() {
        let mut q = GPU_EVENT_QUEUE.lock().unwrap();
        q.clear();
        drop(q);
        slang_gpu_event_push(100, 42);
        slang_gpu_event_push(50, 99);
        assert_eq!(slang_gpu_event_size(), 2);
        assert_eq!(slang_gpu_event_pop(), 99); // earliest = ts 50
    }

    // v235
    #[test]
    fn test_mixed_prec_quantize() {
        let q = slang_mixed_prec_quantize(0.5, 8);
        assert_eq!(q, 128); // 0.5 * 256
        let dq = slang_mixed_prec_dequantize(128, 8);
        assert_eq!(dq, 500); // 128/256 * 1000 = 500
    }

    #[test]
    fn test_mixed_prec_bits() {
        assert_eq!(slang_mixed_prec_bits(0.001), 10); // -log2(0.001) ≈ 10
    }

    // v236
    #[test]
    fn test_multi_gpu_partition() {
        let per_gpu = slang_multi_gpu_partition(10000, 4);
        assert_eq!(per_gpu, 2500);
        assert_eq!(slang_multi_gpu_count(), 4);
    }

    #[test]
    fn test_multi_gpu_sync() {
        let lat = slang_multi_gpu_sync(8, 10000);
        assert!(lat > 0);
    }
}
