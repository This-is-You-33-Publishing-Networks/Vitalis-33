//! v415 — Neuromorphic memory model.
//!
//! Memory allocation and management patterns for spiking neural networks:
//! - `SynapticPool`: weight storage with sparse representation (CSR format)
//! - `SpikeBuffer`: ring buffer for spike events with configurable depth
//! - `PlasticityMemory`: STDP trace storage with exponential decay
//! - `NeuroAllocator`: memory budget tracking for neuromorphic resources

use std::collections::BTreeMap;

// ─── Synaptic Pool (CSR) ────────────────────────────────────────────────

/// Sparse synaptic weight storage in Compressed Sparse Row (CSR) format.
/// Efficient for sparse connectivity patterns typical in biological networks.
#[derive(Debug, Clone)]
pub struct SynapticPool {
    /// Row pointers: neuron i's connections are at indices row_ptr[i]..row_ptr[i+1]
    pub row_ptr: Vec<usize>,
    /// Column indices: target neuron IDs
    pub col_idx: Vec<u32>,
    /// Weight values
    pub weights: Vec<f64>,
    /// Number of neurons (rows)
    pub neuron_count: usize,
}

impl SynapticPool {
    /// Create an empty pool for `n` neurons.
    pub fn new(neuron_count: usize) -> Self {
        Self {
            row_ptr: vec![0; neuron_count + 1],
            col_idx: Vec::new(),
            weights: Vec::new(),
            neuron_count,
        }
    }

    /// Build from a list of (source, target, weight) connections.
    pub fn from_connections(neuron_count: usize, connections: &[(u32, u32, f64)]) -> Self {
        // Sort by source neuron
        let mut sorted: Vec<_> = connections.to_vec();
        sorted.sort_by_key(|&(src, tgt, _)| (src, tgt));

        let mut row_ptr = vec![0usize; neuron_count + 1];
        let mut col_idx = Vec::with_capacity(sorted.len());
        let mut weights = Vec::with_capacity(sorted.len());

        for &(src, tgt, w) in &sorted {
            col_idx.push(tgt);
            weights.push(w);
            row_ptr[src as usize + 1] += 1;
        }

        // Cumulative sum
        for i in 1..=neuron_count {
            row_ptr[i] += row_ptr[i - 1];
        }

        Self { row_ptr, col_idx, weights, neuron_count }
    }

    /// Get outgoing connections from a neuron.
    pub fn connections_from(&self, neuron: u32) -> &[f64] {
        let n = neuron as usize;
        if n >= self.neuron_count { return &[]; }
        let start = self.row_ptr[n];
        let end = self.row_ptr[n + 1];
        &self.weights[start..end]
    }

    /// Get target IDs for outgoing connections from a neuron.
    pub fn targets_from(&self, neuron: u32) -> &[u32] {
        let n = neuron as usize;
        if n >= self.neuron_count { return &[]; }
        let start = self.row_ptr[n];
        let end = self.row_ptr[n + 1];
        &self.col_idx[start..end]
    }

    /// Get weight between source and target. Returns None if not connected.
    pub fn get_weight(&self, source: u32, target: u32) -> Option<f64> {
        let s = source as usize;
        if s >= self.neuron_count { return None; }
        let start = self.row_ptr[s];
        let end = self.row_ptr[s + 1];
        for i in start..end {
            if self.col_idx[i] == target {
                return Some(self.weights[i]);
            }
        }
        None
    }

    /// Update weight between source and target. Returns true if found.
    pub fn update_weight(&mut self, source: u32, target: u32, new_weight: f64) -> bool {
        let s = source as usize;
        if s >= self.neuron_count { return false; }
        let start = self.row_ptr[s];
        let end = self.row_ptr[s + 1];
        for i in start..end {
            if self.col_idx[i] == target {
                self.weights[i] = new_weight;
                return true;
            }
        }
        false
    }

    /// Total number of connections (non-zeros).
    pub fn nnz(&self) -> usize {
        self.col_idx.len()
    }

    /// Memory usage in bytes (approximate).
    pub fn memory_bytes(&self) -> usize {
        self.row_ptr.len() * 8 + self.col_idx.len() * 4 + self.weights.len() * 8
    }

    /// Connection density (nnz / possible connections).
    pub fn density(&self) -> f64 {
        if self.neuron_count == 0 { return 0.0; }
        let possible = self.neuron_count * self.neuron_count;
        self.nnz() as f64 / possible as f64
    }
}

// ─── Spike Buffer ───────────────────────────────────────────────────────

/// Ring buffer for spike events with configurable depth.
#[derive(Debug, Clone)]
pub struct SpikeBuffer {
    /// Buffer storage: (time_us, neuron_id, value)
    buffer: Vec<(i64, i64, f64)>,
    /// Current write position.
    head: usize,
    /// Capacity.
    capacity: usize,
    /// Total spikes written (wraps around).
    total_written: u64,
}

impl SpikeBuffer {
    pub fn new(capacity: usize) -> Self {
        Self {
            buffer: Vec::with_capacity(capacity),
            head: 0,
            capacity,
            total_written: 0,
        }
    }

    /// Push a spike event into the buffer.
    pub fn push(&mut self, time_us: i64, neuron_id: i64, value: f64) {
        if self.buffer.len() < self.capacity {
            self.buffer.push((time_us, neuron_id, value));
        } else {
            self.buffer[self.head] = (time_us, neuron_id, value);
        }
        self.head = (self.head + 1) % self.capacity;
        self.total_written += 1;
    }

    /// Get all buffered spikes (oldest to newest).
    pub fn drain_ordered(&self) -> Vec<(i64, i64, f64)> {
        if self.buffer.len() < self.capacity {
            return self.buffer.clone();
        }
        let mut result = Vec::with_capacity(self.capacity);
        // From head to end
        result.extend_from_slice(&self.buffer[self.head..]);
        // From start to head
        result.extend_from_slice(&self.buffer[..self.head]);
        result
    }

    /// Get the most recent spike.
    pub fn latest(&self) -> Option<(i64, i64, f64)> {
        if self.buffer.is_empty() { return None; }
        let idx = if self.head == 0 { self.buffer.len() - 1 } else { self.head - 1 };
        Some(self.buffer[idx])
    }

    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    pub fn is_full(&self) -> bool {
        self.buffer.len() == self.capacity
    }

    pub fn total_written(&self) -> u64 {
        self.total_written
    }

    pub fn clear(&mut self) {
        self.buffer.clear();
        self.head = 0;
    }
}

// ─── Plasticity Memory ─────────────────────────────────────────────────

/// STDP trace storage with exponential decay.
#[derive(Debug, Clone)]
pub struct PlasticityMemory {
    /// Pre-synaptic traces indexed by neuron ID.
    pre_traces: BTreeMap<i64, f64>,
    /// Post-synaptic traces indexed by neuron ID.
    post_traces: BTreeMap<i64, f64>,
    /// Time constant for trace decay (ms).
    pub tau: f64,
    /// Last update time (ms).
    pub last_time: f64,
}

impl PlasticityMemory {
    pub fn new(tau: f64) -> Self {
        Self {
            pre_traces: BTreeMap::new(),
            post_traces: BTreeMap::new(),
            tau,
            last_time: 0.0,
        }
    }

    /// Record a pre-synaptic spike.
    pub fn record_pre(&mut self, neuron_id: i64, time: f64) {
        self.decay_to(time);
        *self.pre_traces.entry(neuron_id).or_insert(0.0) += 1.0;
        self.last_time = time;
    }

    /// Record a post-synaptic spike.
    pub fn record_post(&mut self, neuron_id: i64, time: f64) {
        self.decay_to(time);
        *self.post_traces.entry(neuron_id).or_insert(0.0) += 1.0;
        self.last_time = time;
    }

    /// Get pre-synaptic trace for a neuron.
    pub fn pre_trace(&self, neuron_id: i64) -> f64 {
        self.pre_traces.get(&neuron_id).copied().unwrap_or(0.0)
    }

    /// Get post-synaptic trace for a neuron.
    pub fn post_trace(&self, neuron_id: i64) -> f64 {
        self.post_traces.get(&neuron_id).copied().unwrap_or(0.0)
    }

    /// Decay all traces to the given time.
    fn decay_to(&mut self, time: f64) {
        let dt = time - self.last_time;
        if dt <= 0.0 { return; }
        let factor = (-dt / self.tau).exp();
        for v in self.pre_traces.values_mut() {
            *v *= factor;
        }
        for v in self.post_traces.values_mut() {
            *v *= factor;
        }
    }

    /// Clear all traces.
    pub fn clear(&mut self) {
        self.pre_traces.clear();
        self.post_traces.clear();
    }

    /// Number of active traces (pre + post).
    pub fn active_count(&self) -> usize {
        self.pre_traces.len() + self.post_traces.len()
    }
}

// ─── Neuro Allocator ────────────────────────────────────────────────────

/// Memory budget tracker for neuromorphic resources.
#[derive(Debug, Clone)]
pub struct NeuroAllocator {
    /// Maximum weight memory (bytes).
    pub weight_budget: usize,
    /// Maximum spike buffer memory (bytes).
    pub spike_budget: usize,
    /// Maximum trace memory (bytes).
    pub trace_budget: usize,
    /// Currently allocated weight memory.
    pub weight_used: usize,
    /// Currently allocated spike memory.
    pub spike_used: usize,
    /// Currently allocated trace memory.
    pub trace_used: usize,
}

impl NeuroAllocator {
    pub fn new(total_budget: usize) -> Self {
        // Default split: 60% weights, 25% spikes, 15% traces
        Self {
            weight_budget: total_budget * 60 / 100,
            spike_budget: total_budget * 25 / 100,
            trace_budget: total_budget * 15 / 100,
            weight_used: 0,
            spike_used: 0,
            trace_used: 0,
        }
    }

    /// Try to allocate weight memory. Returns true if successful.
    pub fn alloc_weights(&mut self, bytes: usize) -> bool {
        if self.weight_used + bytes > self.weight_budget {
            return false;
        }
        self.weight_used += bytes;
        true
    }

    /// Try to allocate spike buffer memory.
    pub fn alloc_spikes(&mut self, bytes: usize) -> bool {
        if self.spike_used + bytes > self.spike_budget {
            return false;
        }
        self.spike_used += bytes;
        true
    }

    /// Try to allocate trace memory.
    pub fn alloc_traces(&mut self, bytes: usize) -> bool {
        if self.trace_used + bytes > self.trace_budget {
            return false;
        }
        self.trace_used += bytes;
        true
    }

    /// Free weight memory.
    pub fn free_weights(&mut self, bytes: usize) {
        self.weight_used = self.weight_used.saturating_sub(bytes);
    }

    /// Total memory used.
    pub fn total_used(&self) -> usize {
        self.weight_used + self.spike_used + self.trace_used
    }

    /// Total budget.
    pub fn total_budget(&self) -> usize {
        self.weight_budget + self.spike_budget + self.trace_budget
    }

    /// Utilization ratio [0, 1].
    pub fn utilization(&self) -> f64 {
        let budget = self.total_budget();
        if budget == 0 { return 0.0; }
        self.total_used() as f64 / budget as f64
    }

    /// Check if any budget is exceeded.
    pub fn is_over_budget(&self) -> bool {
        self.weight_used > self.weight_budget
            || self.spike_used > self.spike_budget
            || self.trace_used > self.trace_budget
    }
}

// ─── FFI ────────────────────────────────────────────────────────────────

/// Create a synaptic pool and return nnz.
#[unsafe(no_mangle)]
pub extern "C" fn slang_neuro_mem_pool_create(neuron_count: i64) -> i64 {
    let pool = SynapticPool::new(neuron_count.clamp(0, 10000) as usize);
    pool.nnz() as i64
}

/// Create a spike buffer and return capacity.
#[unsafe(no_mangle)]
pub extern "C" fn slang_neuro_mem_buffer_create(capacity: i64) -> i64 {
    let buf = SpikeBuffer::new(capacity.clamp(1, 100000) as usize);
    buf.capacity as i64
}

/// Create a neuro allocator and return total budget.
#[unsafe(no_mangle)]
pub extern "C" fn slang_neuro_mem_alloc_budget(total: i64) -> i64 {
    let alloc = NeuroAllocator::new(total.clamp(0, i64::MAX) as usize);
    alloc.total_budget() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── SynapticPool ────────────────────────────────────────────────

    #[test]
    fn test_pool_empty() {
        let pool = SynapticPool::new(4);
        assert_eq!(pool.nnz(), 0);
        assert_eq!(pool.neuron_count, 4);
    }

    #[test]
    fn test_pool_from_connections() {
        let conns = vec![(0, 1, 0.5), (0, 2, 0.3), (1, 2, 0.8)];
        let pool = SynapticPool::from_connections(3, &conns);
        assert_eq!(pool.nnz(), 3);
    }

    #[test]
    fn test_pool_get_weight() {
        let conns = vec![(0, 1, 0.5), (0, 2, 0.3), (1, 2, 0.8)];
        let pool = SynapticPool::from_connections(3, &conns);
        assert!((pool.get_weight(0, 1).unwrap() - 0.5).abs() < 1e-10);
        assert!((pool.get_weight(1, 2).unwrap() - 0.8).abs() < 1e-10);
        assert!(pool.get_weight(2, 0).is_none());
    }

    #[test]
    fn test_pool_update_weight() {
        let conns = vec![(0, 1, 0.5)];
        let mut pool = SynapticPool::from_connections(2, &conns);
        assert!(pool.update_weight(0, 1, 0.9));
        assert!((pool.get_weight(0, 1).unwrap() - 0.9).abs() < 1e-10);
    }

    #[test]
    fn test_pool_targets() {
        let conns = vec![(0, 1, 0.5), (0, 3, 0.3)];
        let pool = SynapticPool::from_connections(4, &conns);
        let targets = pool.targets_from(0);
        assert_eq!(targets.len(), 2);
        assert!(targets.contains(&1));
        assert!(targets.contains(&3));
    }

    #[test]
    fn test_pool_connections_from() {
        let conns = vec![(0, 1, 0.5), (0, 2, 0.3)];
        let pool = SynapticPool::from_connections(3, &conns);
        let weights = pool.connections_from(0);
        assert_eq!(weights.len(), 2);
    }

    #[test]
    fn test_pool_density() {
        let conns = vec![(0, 1, 0.5)];
        let pool = SynapticPool::from_connections(2, &conns);
        assert!((pool.density() - 0.25).abs() < 1e-10); // 1 / (2*2)
    }

    #[test]
    fn test_pool_memory() {
        let conns = vec![(0, 1, 0.5)];
        let pool = SynapticPool::from_connections(2, &conns);
        assert!(pool.memory_bytes() > 0);
    }

    // ── SpikeBuffer ─────────────────────────────────────────────────

    #[test]
    fn test_buffer_new() {
        let buf = SpikeBuffer::new(10);
        assert!(buf.is_empty());
        assert!(!buf.is_full());
    }

    #[test]
    fn test_buffer_push() {
        let mut buf = SpikeBuffer::new(3);
        buf.push(0, 1, 1.0);
        buf.push(10, 2, 1.0);
        assert_eq!(buf.len(), 2);
    }

    #[test]
    fn test_buffer_wrap() {
        let mut buf = SpikeBuffer::new(3);
        buf.push(0, 1, 1.0);
        buf.push(10, 2, 1.0);
        buf.push(20, 3, 1.0);
        assert!(buf.is_full());
        buf.push(30, 4, 1.0); // overwrites oldest
        assert_eq!(buf.len(), 3);
        assert_eq!(buf.total_written(), 4);
    }

    #[test]
    fn test_buffer_latest() {
        let mut buf = SpikeBuffer::new(10);
        buf.push(0, 1, 1.0);
        buf.push(10, 2, 2.0);
        let (t, n, v) = buf.latest().unwrap();
        assert_eq!(t, 10);
        assert_eq!(n, 2);
        assert!((v - 2.0).abs() < 1e-10);
    }

    #[test]
    fn test_buffer_drain_ordered() {
        let mut buf = SpikeBuffer::new(3);
        buf.push(0, 1, 1.0);
        buf.push(10, 2, 1.0);
        buf.push(20, 3, 1.0);
        buf.push(30, 4, 1.0); // wraps
        let ordered = buf.drain_ordered();
        assert_eq!(ordered.len(), 3);
        assert_eq!(ordered[0].0, 10); // oldest surviving
        assert_eq!(ordered[2].0, 30); // newest
    }

    #[test]
    fn test_buffer_clear() {
        let mut buf = SpikeBuffer::new(10);
        buf.push(0, 1, 1.0);
        buf.clear();
        assert!(buf.is_empty());
    }

    // ── PlasticityMemory ────────────────────────────────────────────

    #[test]
    fn test_plasticity_new() {
        let mem = PlasticityMemory::new(20.0);
        assert!((mem.tau - 20.0).abs() < 1e-10);
        assert_eq!(mem.active_count(), 0);
    }

    #[test]
    fn test_plasticity_record_pre() {
        let mut mem = PlasticityMemory::new(20.0);
        mem.record_pre(0, 0.0);
        assert!((mem.pre_trace(0) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_plasticity_record_post() {
        let mut mem = PlasticityMemory::new(20.0);
        mem.record_post(1, 0.0);
        assert!((mem.post_trace(1) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_plasticity_decay() {
        let mut mem = PlasticityMemory::new(20.0);
        mem.record_pre(0, 0.0);
        mem.record_pre(0, 10.0); // decays then adds
        let trace = mem.pre_trace(0);
        // After 10ms with tau=20ms: 1.0 * exp(-10/20) + 1.0 ≈ 1.607
        assert!(trace > 1.5 && trace < 1.7);
    }

    #[test]
    fn test_plasticity_clear() {
        let mut mem = PlasticityMemory::new(20.0);
        mem.record_pre(0, 0.0);
        mem.clear();
        assert_eq!(mem.active_count(), 0);
    }

    // ── NeuroAllocator ──────────────────────────────────────────────

    #[test]
    fn test_allocator_new() {
        let alloc = NeuroAllocator::new(1000);
        assert_eq!(alloc.total_budget(), 1000);
        assert_eq!(alloc.total_used(), 0);
    }

    #[test]
    fn test_allocator_alloc_weights() {
        let mut alloc = NeuroAllocator::new(1000);
        assert!(alloc.alloc_weights(500));
        assert_eq!(alloc.weight_used, 500);
    }

    #[test]
    fn test_allocator_over_budget() {
        let mut alloc = NeuroAllocator::new(1000);
        assert!(!alloc.alloc_weights(700)); // 60% of 1000 = 600
    }

    #[test]
    fn test_allocator_free() {
        let mut alloc = NeuroAllocator::new(1000);
        alloc.alloc_weights(300);
        alloc.free_weights(100);
        assert_eq!(alloc.weight_used, 200);
    }

    #[test]
    fn test_allocator_utilization() {
        let mut alloc = NeuroAllocator::new(1000);
        alloc.alloc_weights(300);
        alloc.alloc_spikes(100);
        let util = alloc.utilization();
        assert!(util > 0.3 && util < 0.5);
    }

    // ── FFI ─────────────────────────────────────────────────────────

    #[test]
    fn test_ffi_pool_create() {
        assert_eq!(slang_neuro_mem_pool_create(10), 0);
    }

    #[test]
    fn test_ffi_buffer_create() {
        assert_eq!(slang_neuro_mem_buffer_create(100), 100);
    }

    #[test]
    fn test_ffi_alloc_budget() {
        assert_eq!(slang_neuro_mem_alloc_budget(1000), 1000);
    }
}
