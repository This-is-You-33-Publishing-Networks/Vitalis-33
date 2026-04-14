//! v505 — Synaptic Memory Allocator.
//!
//! Memory allocator inspired by synaptic plasticity. Frequently accessed regions
//! get stronger (faster) allocation paths. Rarely used regions decay and coalesce.
//! LTP/LTD-based cache warming.

use std::collections::HashMap;

/// Allocation handle returned by the synaptic allocator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SynapticHandle(pub u64);

/// Strength of a memory region (analogous to synaptic weight).
#[derive(Debug, Clone)]
pub struct RegionStrength {
    pub weight: f64,
    pub access_count: u64,
    pub last_access_tick: u64,
    pub potentiation_count: u64,
    pub depression_count: u64,
}

impl RegionStrength {
    pub fn new() -> Self {
        Self {
            weight: 1.0,
            access_count: 0,
            last_access_tick: 0,
            potentiation_count: 0,
            depression_count: 0,
        }
    }

    /// Long-Term Potentiation: strengthen this region.
    pub fn potentiate(&mut self, amount: f64) {
        self.weight = (self.weight + amount).min(10.0);
        self.potentiation_count += 1;
    }

    /// Long-Term Depression: weaken this region.
    pub fn depress(&mut self, amount: f64) {
        self.weight = (self.weight - amount).max(0.01);
        self.depression_count += 1;
    }

    /// Decay towards baseline over time.
    pub fn decay(&mut self, rate: f64) {
        let diff = self.weight - 1.0;
        self.weight -= diff * rate;
    }

    /// Record an access at the given tick.
    pub fn access(&mut self, tick: u64) {
        self.access_count += 1;
        self.last_access_tick = tick;
    }

    /// Is this region "hot" (frequently accessed)?
    pub fn is_hot(&self) -> bool {
        self.weight > 3.0
    }

    /// Is this region "cold" (rarely accessed)?
    pub fn is_cold(&self) -> bool {
        self.weight < 0.5
    }
}

/// A memory block managed by the synaptic allocator.
#[derive(Debug, Clone)]
pub struct SynapticBlock {
    pub handle: SynapticHandle,
    pub size: usize,
    pub data: Vec<u8>,
    pub strength: RegionStrength,
    pub freed: bool,
    pub generation: u64,
}

/// Cache tier for hot/warm/cold classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheTier {
    Hot,
    Warm,
    Cold,
}

/// Statistics for the synaptic allocator.
#[derive(Debug, Clone, Default)]
pub struct SynapticAllocStats {
    pub total_allocs: u64,
    pub total_frees: u64,
    pub total_bytes_allocated: u64,
    pub total_bytes_freed: u64,
    pub coalesce_events: u64,
    pub ltp_events: u64,
    pub ltd_events: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
}

/// Synaptic Memory Allocator.
///
/// Allocation paths are strengthened (LTP) when accessed frequently
/// and weakened (LTD) when idle. Cold regions are coalesced.
pub struct SynapticAllocator {
    blocks: HashMap<SynapticHandle, SynapticBlock>,
    free_list: Vec<SynapticHandle>,
    hot_cache: Vec<SynapticHandle>,
    next_id: u64,
    current_tick: u64,
    decay_rate: f64,
    ltp_amount: f64,
    ltd_amount: f64,
    pub stats: SynapticAllocStats,
}

impl SynapticAllocator {
    pub fn new() -> Self {
        Self {
            blocks: HashMap::new(),
            free_list: Vec::new(),
            hot_cache: Vec::new(),
            next_id: 0,
            current_tick: 0,
            decay_rate: 0.05,
            ltp_amount: 0.3,
            ltd_amount: 0.1,
            stats: SynapticAllocStats::default(),
        }
    }

    /// Configure plasticity parameters.
    pub fn with_plasticity(mut self, ltp: f64, ltd: f64, decay: f64) -> Self {
        self.ltp_amount = ltp;
        self.ltd_amount = ltd;
        self.decay_rate = decay;
        self
    }

    /// Allocate a block of the given size.
    pub fn alloc(&mut self, size: usize) -> SynapticHandle {
        self.stats.total_allocs += 1;
        self.stats.total_bytes_allocated += size as u64;

        // Try to reuse a freed block of suitable size from the hot cache first
        if let Some(pos) = self.hot_cache.iter().position(|h| {
            self.blocks
                .get(h)
                .is_some_and(|b| b.freed && b.size >= size)
        }) {
            let handle = self.hot_cache.remove(pos);
            if let Some(block) = self.blocks.get_mut(&handle) {
                block.freed = false;
                block.data.resize(size, 0);
                block.strength.potentiate(self.ltp_amount);
                block.strength.access(self.current_tick);
                self.stats.cache_hits += 1;
                self.stats.ltp_events += 1;
            }
            return handle;
        }

        // Try free list
        if let Some(pos) = self.free_list.iter().position(|h| {
            self.blocks
                .get(h)
                .is_some_and(|b| b.freed && b.size >= size)
        }) {
            let handle = self.free_list.remove(pos);
            if let Some(block) = self.blocks.get_mut(&handle) {
                block.freed = false;
                block.data.resize(size, 0);
                block.strength.access(self.current_tick);
                self.stats.cache_misses += 1;
            }
            return handle;
        }

        // Fresh allocation
        let handle = SynapticHandle(self.next_id);
        self.next_id += 1;
        let block = SynapticBlock {
            handle,
            size,
            data: vec![0u8; size],
            strength: RegionStrength::new(),
            freed: false,
            generation: 0,
        };
        self.blocks.insert(handle, block);
        self.stats.cache_misses += 1;
        handle
    }

    /// Free a block, returning it to the pool.
    pub fn free(&mut self, handle: SynapticHandle) -> bool {
        if let Some(block) = self.blocks.get_mut(&handle) {
            if !block.freed {
                block.freed = true;
                block.generation += 1;
                self.stats.total_frees += 1;
                self.stats.total_bytes_freed += block.size as u64;

                if block.strength.is_hot() {
                    self.hot_cache.push(handle);
                } else {
                    self.free_list.push(handle);
                }
                return true;
            }
        }
        false
    }

    /// Read data from a block (strengthens the path via LTP).
    pub fn read(&mut self, handle: SynapticHandle) -> Option<&[u8]> {
        let tick = self.current_tick;
        let ltp = self.ltp_amount;
        if let Some(block) = self.blocks.get_mut(&handle) {
            if !block.freed {
                block.strength.access(tick);
                block.strength.potentiate(ltp);
                self.stats.ltp_events += 1;
                // Return data as slice - need to re-borrow immutably
            }
        }
        self.blocks
            .get(&handle)
            .filter(|b| !b.freed)
            .map(|b| b.data.as_slice())
    }

    /// Write data to a block.
    pub fn write(&mut self, handle: SynapticHandle, data: &[u8]) -> bool {
        if let Some(block) = self.blocks.get_mut(&handle) {
            if !block.freed && data.len() <= block.size {
                block.data[..data.len()].copy_from_slice(data);
                block.strength.access(self.current_tick);
                block.strength.potentiate(self.ltp_amount);
                self.stats.ltp_events += 1;
                return true;
            }
        }
        false
    }

    /// Advance time by one tick, applying decay to all blocks.
    pub fn tick(&mut self) {
        self.current_tick += 1;
        let rate = self.decay_rate;
        let ltd = self.ltd_amount;
        for block in self.blocks.values_mut() {
            block.strength.decay(rate);
            // Apply LTD to blocks not accessed recently
            if self.current_tick > block.strength.last_access_tick + 10 {
                block.strength.depress(ltd);
            }
        }
    }

    /// Coalesce adjacent cold freed blocks to reduce fragmentation.
    pub fn coalesce_cold(&mut self) -> usize {
        let cold_freed: Vec<SynapticHandle> = self
            .blocks
            .iter()
            .filter(|(_, b)| b.freed && b.strength.is_cold())
            .map(|(h, _)| *h)
            .collect();

        let count = cold_freed.len();
        for handle in &cold_freed {
            self.blocks.remove(handle);
            self.free_list.retain(|h| h != handle);
            self.hot_cache.retain(|h| h != handle);
        }
        self.stats.coalesce_events += count as u64;
        count
    }

    /// Classify a block into a cache tier.
    pub fn cache_tier(&self, handle: SynapticHandle) -> Option<CacheTier> {
        self.blocks.get(&handle).map(|b| {
            if b.strength.is_hot() {
                CacheTier::Hot
            } else if b.strength.is_cold() {
                CacheTier::Cold
            } else {
                CacheTier::Warm
            }
        })
    }

    /// Get block info.
    pub fn get_block(&self, handle: SynapticHandle) -> Option<&SynapticBlock> {
        self.blocks.get(&handle)
    }

    /// Number of live (non-freed) blocks.
    pub fn live_count(&self) -> usize {
        self.blocks.values().filter(|b| !b.freed).count()
    }

    /// Total blocks (including freed).
    pub fn total_count(&self) -> usize {
        self.blocks.len()
    }

    /// Get the current tick.
    pub fn current_tick(&self) -> u64 {
        self.current_tick
    }

    /// Warm up the hot cache by pre-promoting high-strength freed blocks.
    pub fn warm_cache(&mut self) {
        let hot_freed: Vec<SynapticHandle> = self
            .free_list
            .iter()
            .filter(|h| {
                self.blocks
                    .get(h)
                    .is_some_and(|b| b.strength.is_hot())
            })
            .copied()
            .collect();

        for h in hot_freed {
            self.free_list.retain(|x| *x != h);
            if !self.hot_cache.contains(&h) {
                self.hot_cache.push(h);
            }
        }
    }

    /// Free list size.
    pub fn free_list_size(&self) -> usize {
        self.free_list.len()
    }

    /// Hot cache size.
    pub fn hot_cache_size(&self) -> usize {
        self.hot_cache.len()
    }
}

// ─── FFI ───────────────────────────────────────────────

use std::sync::{LazyLock, Mutex};

static SYNAPTIC_ALLOC: LazyLock<Mutex<SynapticAllocator>> =
    LazyLock::new(|| Mutex::new(SynapticAllocator::new()));

#[unsafe(no_mangle)]
pub extern "C" fn slang_synaptic_alloc(size: i64) -> i64 {
    let mut alloc = SYNAPTIC_ALLOC.lock().unwrap();
    alloc.alloc(size as usize).0 as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_synaptic_free(handle: i64) -> i64 {
    let mut alloc = SYNAPTIC_ALLOC.lock().unwrap();
    if alloc.free(SynapticHandle(handle as u64)) { 1 } else { 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_synaptic_live_count() -> i64 {
    let alloc = SYNAPTIC_ALLOC.lock().unwrap();
    alloc.live_count() as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_synaptic_tick() -> i64 {
    let mut alloc = SYNAPTIC_ALLOC.lock().unwrap();
    alloc.tick();
    alloc.current_tick() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alloc_and_free() {
        let mut alloc = SynapticAllocator::new();
        let h = alloc.alloc(64);
        assert_eq!(alloc.live_count(), 1);
        assert!(alloc.free(h));
        assert_eq!(alloc.live_count(), 0);
    }

    #[test]
    fn test_write_read() {
        let mut alloc = SynapticAllocator::new();
        let h = alloc.alloc(8);
        assert!(alloc.write(h, &[1, 2, 3, 4]));
        let data = alloc.read(h).unwrap();
        assert_eq!(&data[..4], &[1, 2, 3, 4]);
    }

    #[test]
    fn test_read_freed_fails() {
        let mut alloc = SynapticAllocator::new();
        let h = alloc.alloc(8);
        alloc.free(h);
        assert!(alloc.read(h).is_none());
    }

    #[test]
    fn test_ltp_on_read() {
        let mut alloc = SynapticAllocator::new();
        let h = alloc.alloc(8);
        let initial = alloc.get_block(h).unwrap().strength.weight;
        alloc.read(h);
        let after = alloc.get_block(h).unwrap().strength.weight;
        assert!(after > initial);
    }

    #[test]
    fn test_decay_over_time() {
        let mut alloc = SynapticAllocator::new();
        let h = alloc.alloc(8);
        // Potentiate strongly
        for _ in 0..10 {
            alloc.read(h);
        }
        let high = alloc.get_block(h).unwrap().strength.weight;
        for _ in 0..20 {
            alloc.tick();
        }
        let decayed = alloc.get_block(h).unwrap().strength.weight;
        assert!(decayed < high);
    }

    #[test]
    fn test_hot_block_reuse() {
        let mut alloc = SynapticAllocator::new();
        let h = alloc.alloc(64);
        // Make it hot
        for _ in 0..15 {
            alloc.read(h);
        }
        assert!(alloc.get_block(h).unwrap().strength.is_hot());
        alloc.free(h);
        assert_eq!(alloc.hot_cache_size(), 1);

        // Next alloc should reuse from hot cache
        let h2 = alloc.alloc(32);
        assert_eq!(h2, h); // reused handle
        assert_eq!(alloc.stats.cache_hits, 1);
    }

    #[test]
    fn test_cold_coalesce() {
        let mut alloc = SynapticAllocator::new();
        let mut handles = Vec::new();
        for _ in 0..5 {
            handles.push(alloc.alloc(32));
        }
        for h in &handles {
            alloc.free(*h);
        }
        // Make them cold
        for _ in 0..50 {
            alloc.tick();
        }
        let coalesced = alloc.coalesce_cold();
        assert!(coalesced > 0);
    }

    #[test]
    fn test_cache_tier_classification() {
        let mut alloc = SynapticAllocator::new();
        let h = alloc.alloc(64);
        assert_eq!(alloc.cache_tier(h), Some(CacheTier::Warm));

        for _ in 0..15 {
            alloc.read(h);
        }
        assert_eq!(alloc.cache_tier(h), Some(CacheTier::Hot));
    }

    #[test]
    fn test_region_strength_potentiate() {
        let mut rs = RegionStrength::new();
        rs.potentiate(0.5);
        assert!(rs.weight > 1.0);
        assert_eq!(rs.potentiation_count, 1);
    }

    #[test]
    fn test_region_strength_depress() {
        let mut rs = RegionStrength::new();
        rs.depress(0.5);
        assert!(rs.weight < 1.0);
        assert_eq!(rs.depression_count, 1);
    }

    #[test]
    fn test_region_strength_clamping() {
        let mut rs = RegionStrength::new();
        for _ in 0..100 {
            rs.potentiate(1.0);
        }
        assert!(rs.weight <= 10.0);

        for _ in 0..100 {
            rs.depress(1.0);
        }
        assert!(rs.weight >= 0.01);
    }

    #[test]
    fn test_write_too_large_fails() {
        let mut alloc = SynapticAllocator::new();
        let h = alloc.alloc(4);
        let result = alloc.write(h, &[1, 2, 3, 4, 5]);
        assert!(!result);
    }

    #[test]
    fn test_free_already_freed() {
        let mut alloc = SynapticAllocator::new();
        let h = alloc.alloc(8);
        assert!(alloc.free(h));
        assert!(!alloc.free(h)); // second free fails
    }

    #[test]
    fn test_plasticity_config() {
        let alloc = SynapticAllocator::new().with_plasticity(0.5, 0.2, 0.1);
        assert_eq!(alloc.ltp_amount, 0.5);
        assert_eq!(alloc.ltd_amount, 0.2);
        assert_eq!(alloc.decay_rate, 0.1);
    }

    #[test]
    fn test_stats_tracking() {
        let mut alloc = SynapticAllocator::new();
        alloc.alloc(64);
        alloc.alloc(128);
        assert_eq!(alloc.stats.total_allocs, 2);
        assert_eq!(alloc.stats.total_bytes_allocated, 192);
    }

    #[test]
    fn test_warm_cache() {
        let mut alloc = SynapticAllocator::new();
        let h = alloc.alloc(64);
        for _ in 0..15 {
            alloc.read(h);
        }
        alloc.free(h);
        // Move from free_list to hot_cache
        alloc.warm_cache();
        assert!(alloc.hot_cache_size() >= 1);
    }

    #[test]
    fn test_generation_tracking() {
        let mut alloc = SynapticAllocator::new();
        let h = alloc.alloc(64);
        assert_eq!(alloc.get_block(h).unwrap().generation, 0);
        alloc.free(h);
        assert_eq!(alloc.get_block(h).unwrap().generation, 1);
    }

    #[test]
    fn test_ffi_alloc_free() {
        let h = slang_synaptic_alloc(32);
        assert!(h >= 0);
        let freed = slang_synaptic_free(h);
        assert_eq!(freed, 1);
    }

    #[test]
    fn test_ffi_tick() {
        let tick = slang_synaptic_tick();
        assert!(tick > 0);
    }

    #[test]
    fn test_total_count() {
        let mut alloc = SynapticAllocator::new();
        alloc.alloc(32);
        alloc.alloc(64);
        assert_eq!(alloc.total_count(), 2);
    }
}
