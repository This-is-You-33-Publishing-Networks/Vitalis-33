//! Processing-in-Memory & Von Neumann Bottleneck Elimination (v219–v224)
//!
//! PIM compute, data-centric computation, sparse spike engine, cache-oblivious
//! algorithms, memory-compute fusion, zero-copy spike propagation.

use std::sync::{LazyLock, Mutex};

// ── v219: Processing-in-Memory (PIM) ─────────────────────────────────────────

/// PIM memory bank: simulates compute-capable memory cells.
static PIM_BANK: LazyLock<Mutex<Vec<i64>>> = LazyLock::new(|| Mutex::new(Vec::new()));

/// Allocate n PIM cells. Returns start address.
#[unsafe(no_mangle)]
pub extern "C" fn slang_pim_alloc(n: i64) -> i64 {
    let mut bank = PIM_BANK.lock().unwrap();
    let start = bank.len() as i64;
    for _ in 0..n.max(0) {
        bank.push(0);
    }
    start
}

/// PIM in-place add: bank[addr] += operand. Returns new value or -1 if OOB.
#[unsafe(no_mangle)]
pub extern "C" fn slang_pim_compute_add(addr: i64, operand: i64) -> i64 {
    let mut bank = PIM_BANK.lock().unwrap();
    let idx = addr as usize;
    if idx < bank.len() {
        bank[idx] = bank[idx].wrapping_add(operand);
        bank[idx]
    } else {
        -1
    }
}

/// PIM in-place mul: bank[addr] *= operand. Returns new value or -1 if OOB.
#[unsafe(no_mangle)]
pub extern "C" fn slang_pim_compute_mul(addr: i64, operand: i64) -> i64 {
    let mut bank = PIM_BANK.lock().unwrap();
    let idx = addr as usize;
    if idx < bank.len() {
        bank[idx] = bank[idx].wrapping_mul(operand);
        bank[idx]
    } else {
        -1
    }
}

/// Compute data transfer cost: PIM eliminates bus traffic.
/// Returns energy saved (in fJ) vs conventional: n_words * bus_energy.
#[unsafe(no_mangle)]
pub extern "C" fn slang_pim_transfer_cost(n_words: i64, bus_energy_fj: i64) -> i64 {
    n_words * bus_energy_fj
}

// ── v220: Data-Centric Computation ───────────────────────────────────────────

/// Data-centric map: apply f(x) = x * scale + bias to n elements.
/// Returns sum of results.
#[unsafe(no_mangle)]
pub extern "C" fn slang_datacentric_map(n: i64, scale: f64, bias: f64) -> i64 {
    let mut sum = 0.0_f64;
    for i in 0..n {
        sum += i as f64 * scale + bias;
    }
    (sum * 1000.0).round() as i64
}

/// Data-centric reduce: sum n values starting from base with stride.
/// Returns sum.
#[unsafe(no_mangle)]
pub extern "C" fn slang_datacentric_reduce(n: i64, base: i64, stride: i64) -> i64 {
    let mut sum = 0_i64;
    for i in 0..n {
        sum = sum.wrapping_add(base + i * stride);
    }
    sum
}

/// Scatter: distribute value to n locations. Returns count distributed.
#[unsafe(no_mangle)]
pub extern "C" fn slang_datacentric_scatter(_value: i64, n_targets: i64) -> i64 {
    // In PIM, scatter is free (no bus contention)
    n_targets
}

/// Gather: collect n values. Returns simulated latency in cycles.
#[unsafe(no_mangle)]
pub extern "C" fn slang_datacentric_gather(n_sources: i64, locality_factor: f64) -> i64 {
    // Higher locality → lower latency
    let base_cycles = n_sources;
    let effective = (base_cycles as f64 * (2.0 - locality_factor.clamp(0.0, 1.0))).round() as i64;
    effective
}

// ── v221: Sparse Computation Engine ──────────────────────────────────────────

/// Sparse spike propagation: only propagate n_active out of n_total.
/// Returns compute savings as percentage ×10 (990 = 99.0% savings).
#[unsafe(no_mangle)]
pub extern "C" fn slang_sparse_spike_propagate(n_active: i64, n_total: i64) -> i64 {
    if n_total <= 0 { return 0; }
    let sparsity = 1.0 - (n_active as f64 / n_total as f64);
    (sparsity * 1000.0).round() as i64
}

/// Count non-zero elements in a simulated sparse vector.
/// Approximates: for n elements with density d, nnz ≈ n * d.
#[unsafe(no_mangle)]
pub extern "C" fn slang_sparse_nonzero_count(n: i64, density_pct: i64) -> i64 {
    let density = (density_pct as f64 / 100.0).clamp(0.0, 1.0);
    (n as f64 * density).round() as i64
}

/// Compress: compute compression ratio for sparse data.
/// CSR overhead: (nnz * 2 + n_rows) vs dense (n_rows * n_cols).
/// Returns ratio ×1000.
#[unsafe(no_mangle)]
pub extern "C" fn slang_sparse_compress(n_rows: i64, n_cols: i64, nnz: i64) -> i64 {
    let dense = n_rows * n_cols;
    if dense <= 0 { return 1000; }
    let sparse = nnz * 2 + n_rows;
    (sparse as f64 / dense as f64 * 1000.0).round() as i64
}

/// Decompress: expand sparse representation. Returns element count.
#[unsafe(no_mangle)]
pub extern "C" fn slang_sparse_decompress(nnz: i64, n_cols: i64) -> i64 {
    nnz * n_cols
}

// ── v222: Cache-Oblivious Algorithms ─────────────────────────────────────────

/// Cache-oblivious matrix transpose via recursive decomposition.
/// Returns number of cache-friendly operations for n×n matrix.
#[unsafe(no_mangle)]
pub extern "C" fn slang_cache_oblivious_transpose(n: i64) -> i64 {
    // Recursive: T(n) = 4*T(n/2) + O(1), base case at cache line
    if n <= 8 { return n * n; }
    4 * slang_cache_oblivious_transpose(n / 2)
}

/// Cache-oblivious FFT: Cooley-Tukey with cache-friendly access.
/// Returns operation count: O(n log n).
#[unsafe(no_mangle)]
pub extern "C" fn slang_cache_oblivious_fft(n: i64) -> i64 {
    if n <= 1 { return 1; }
    let log_n = (n as f64).log2().ceil().round() as i64;
    n * log_n
}

/// Cache-oblivious sort: funnel sort operation count for n elements.
/// Returns O(n log n / B * log(n/M)) operations (simplified).
#[unsafe(no_mangle)]
pub extern "C" fn slang_cache_oblivious_sort(n: i64) -> i64 {
    if n <= 1 { return 1; }
    let log_n = (n as f64).log2().ceil().round() as i64;
    n * log_n // simplified
}

/// Cache-oblivious matrix multiply: recursive n×n.
/// Returns operation count.
#[unsafe(no_mangle)]
pub extern "C" fn slang_cache_oblivious_matmul(n: i64) -> i64 {
    if n <= 4 { return n * n * n; }
    8 * slang_cache_oblivious_matmul(n / 2)
}

// ── v223: Memory-Compute Fusion ──────────────────────────────────────────────

/// Fused multiply-accumulate in memory: acc += a * b.
/// Returns accumulated value ×1000.
#[unsafe(no_mangle)]
pub extern "C" fn slang_memcompute_fused_mac(a: f64, b: f64, acc: f64) -> i64 {
    let result = acc + a * b;
    (result * 1000.0).round() as i64
}

/// Fused compare in memory: returns sign(a - b) as -1/0/1.
#[unsafe(no_mangle)]
pub extern "C" fn slang_memcompute_fused_compare(a: i64, b: i64) -> i64 {
    match a.cmp(&b) {
        std::cmp::Ordering::Less => -1,
        std::cmp::Ordering::Equal => 0,
        std::cmp::Ordering::Greater => 1,
    }
}

/// Fused accumulate: sum n values with carry. Returns sum.
#[unsafe(no_mangle)]
pub extern "C" fn slang_memcompute_fused_accumulate(base: i64, n: i64, stride: i64) -> i64 {
    let mut sum = 0_i64;
    for i in 0..n {
        sum = sum.wrapping_add(base + i * stride);
    }
    sum
}

/// Pipeline depth: number of stages in memory-compute pipeline.
/// Returns optimal depth for given compute intensity.
#[unsafe(no_mangle)]
pub extern "C" fn slang_memcompute_pipeline_depth(compute_intensity: i64) -> i64 {
    // Optimal pipeline depth ≈ sqrt(compute_intensity), capped at 32
    let depth = (compute_intensity as f64).sqrt().round() as i64;
    depth.clamp(1, 32)
}

// ── v224: Zero-Copy Spike Propagation ────────────────────────────────────────

static ZEROCOPY_BUFFER: LazyLock<Mutex<Vec<i64>>> = LazyLock::new(|| Mutex::new(Vec::new()));

/// Create a zero-copy spike buffer of size n. Returns buffer size.
#[unsafe(no_mangle)]
pub extern "C" fn slang_zerocopy_spike_buffer(n: i64) -> i64 {
    let mut buf = ZEROCOPY_BUFFER.lock().unwrap();
    buf.clear();
    for i in 0..n.max(0) {
        buf.push(i);
    }
    buf.len() as i64
}

/// Fan-out: duplicate spike to n targets without copying data.
/// Returns total fan-out count.
#[unsafe(no_mangle)]
pub extern "C" fn slang_zerocopy_fanout(_spike_id: i64, n_targets: i64) -> i64 {
    // Zero-copy: just reference counting, no actual data movement
    n_targets
}

/// Gather spikes from n sources into local buffer.
/// Returns gathered count.
#[unsafe(no_mangle)]
pub extern "C" fn slang_zerocopy_gather(n_sources: i64) -> i64 {
    let buf = ZEROCOPY_BUFFER.lock().unwrap();
    n_sources.min(buf.len() as i64)
}

/// Count active (non-zero) entries in zero-copy buffer.
#[unsafe(no_mangle)]
pub extern "C" fn slang_zerocopy_active_count() -> i64 {
    let buf = ZEROCOPY_BUFFER.lock().unwrap();
    buf.iter().filter(|&&v| v != 0).count() as i64
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // v219
    #[test]
    fn test_pim_alloc_and_compute() {
        let addr = slang_pim_alloc(10);
        assert!(addr >= 0);
        slang_pim_compute_add(addr, 5);
        assert_eq!(slang_pim_compute_add(addr, 3), 8);
        assert_eq!(slang_pim_compute_mul(addr, 2), 16);
    }

    #[test]
    fn test_pim_oob() {
        assert_eq!(slang_pim_compute_add(999999, 1), -1);
    }

    #[test]
    fn test_pim_transfer_cost() {
        assert_eq!(slang_pim_transfer_cost(1000, 50), 50000);
    }

    // v220
    #[test]
    fn test_datacentric_map() {
        let s = slang_datacentric_map(5, 2.0, 1.0);
        // sum of (0*2+1, 1*2+1, 2*2+1, 3*2+1, 4*2+1) = 1+3+5+7+9 = 25
        assert_eq!(s, 25000);
    }

    #[test]
    fn test_datacentric_reduce() {
        assert_eq!(slang_datacentric_reduce(5, 0, 1), 10); // 0+1+2+3+4
    }

    // v221
    #[test]
    fn test_sparse_propagate() {
        let savings = slang_sparse_spike_propagate(10, 1000);
        assert_eq!(savings, 990); // 99.0%
    }

    #[test]
    fn test_sparse_compress() {
        let ratio = slang_sparse_compress(100, 100, 50);
        // sparse = 50*2+100 = 200, dense = 10000, ratio = 0.02 → 20
        assert_eq!(ratio, 20);
    }

    // v222
    #[test]
    fn test_cache_oblivious_fft() {
        let ops = slang_cache_oblivious_fft(1024);
        assert!(ops > 0);
    }

    #[test]
    fn test_cache_oblivious_matmul() {
        let ops = slang_cache_oblivious_matmul(8);
        assert!(ops > 0);
    }

    // v223
    #[test]
    fn test_fused_mac() {
        let r = slang_memcompute_fused_mac(2.0, 3.0, 1.0);
        assert_eq!(r, 7000); // (1 + 2*3) * 1000
    }

    #[test]
    fn test_fused_compare() {
        assert_eq!(slang_memcompute_fused_compare(5, 3), 1);
        assert_eq!(slang_memcompute_fused_compare(3, 5), -1);
        assert_eq!(slang_memcompute_fused_compare(5, 5), 0);
    }

    #[test]
    fn test_pipeline_depth() {
        assert_eq!(slang_memcompute_pipeline_depth(100), 10);
        assert_eq!(slang_memcompute_pipeline_depth(1), 1);
        assert_eq!(slang_memcompute_pipeline_depth(10000), 32); // capped
    }

    // v224
    #[test]
    fn test_zerocopy_buffer() {
        assert_eq!(slang_zerocopy_spike_buffer(5), 5);
        assert_eq!(slang_zerocopy_active_count(), 4); // 1,2,3,4 non-zero (0 is zero)
    }

    #[test]
    fn test_zerocopy_fanout() {
        assert_eq!(slang_zerocopy_fanout(1, 10), 10);
    }
}
