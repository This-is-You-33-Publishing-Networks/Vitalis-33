//! Spike-Timed Memory Hierarchy — Vitalis v745
//!
//! Simulates a spike-timed memory hierarchy where access patterns determine
//! cache promotion and hit rates based on temporal locality.

use std::sync::{LazyLock, Mutex};
use std::collections::{HashMap, VecDeque};

static STATE: LazyLock<Mutex<SpikeMemoryHierarchy>> = LazyLock::new(|| Mutex::new(SpikeMemoryHierarchy::new()));

pub struct SpikeMemoryHierarchy {
    l1: VecDeque<u64>,
    access_log: Vec<u64>,
    hits: usize,
    l1_capacity: usize,
}

impl SpikeMemoryHierarchy {
    pub fn new() -> Self {
        Self {
            l1: VecDeque::new(),
            access_log: Vec::new(),
            hits: 0,
            l1_capacity: 8,
        }
    }

    pub fn spike_access(&mut self, addr: u64) {
        self.access_log.push(addr);
        if self.l1.contains(&addr) {
            self.hits += 1;
        } else {
            if self.l1.len() >= self.l1_capacity {
                self.l1.pop_front();
            }
            self.l1.push_back(addr);
        }
    }

    pub fn cache_hit_rate(&self) -> f64 {
        if self.access_log.is_empty() { return 0.0; }
        self.hits as f64 / self.access_log.len() as f64
    }

    pub fn l1_size(&self) -> usize {
        self.l1.len()
    }

    pub fn promote(&mut self, addr: u64) {
        if !self.l1.contains(&addr) {
            if self.l1.len() >= self.l1_capacity {
                self.l1.pop_front();
            }
            self.l1.push_back(addr);
        }
    }
}

impl Default for SpikeMemoryHierarchy {
    fn default() -> Self { Self::new() }
}

#[unsafe(no_mangle)]
pub extern "C" fn smh_access(addr: i64) -> i64 {
    STATE.lock().unwrap().spike_access(addr as u64);
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn smh_hit_rate() -> f64 {
    STATE.lock().unwrap().cache_hit_rate()
}

#[unsafe(no_mangle)]
pub extern "C" fn smh_l1_size() -> i64 {
    STATE.lock().unwrap().l1_size() as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn smh_promote(addr: i64) -> i64 {
    STATE.lock().unwrap().promote(addr as u64);
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_hit_rate_zero() {
        let smh = SpikeMemoryHierarchy::new();
        assert_eq!(smh.cache_hit_rate(), 0.0);
    }

    #[test]
    fn test_repeated_access_increases_hits() {
        let mut smh = SpikeMemoryHierarchy::new();
        smh.spike_access(0x100);
        smh.spike_access(0x100);
        assert!(smh.cache_hit_rate() > 0.0);
    }

    #[test]
    fn test_l1_size_bounded() {
        let mut smh = SpikeMemoryHierarchy::new();
        for i in 0..20u64 {
            smh.spike_access(i * 0x100);
        }
        assert!(smh.l1_size() <= 8);
    }

    #[test]
    fn test_promote_adds_to_l1() {
        let mut smh = SpikeMemoryHierarchy::new();
        smh.promote(0xDEAD);
        assert_eq!(smh.l1_size(), 1);
    }

    #[test]
    fn test_no_duplicate_in_l1() {
        let mut smh = SpikeMemoryHierarchy::new();
        smh.promote(0x42);
        smh.promote(0x42);
        assert_eq!(smh.l1_size(), 1);
    }

    #[test]
    fn test_ffi_smh_access_and_size() {
        smh_access(0x1000);
        assert!(smh_l1_size() >= 1);
    }
}
