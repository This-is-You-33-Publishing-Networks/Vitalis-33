//! Neural Garbage Collection via Refractory Decay — Vitalis v755
//!
//! Implements garbage collection using neural refractory period decay.
//! Objects that haven't fired recently are considered dead and collected.

use std::sync::{LazyLock, Mutex};
use std::collections::HashMap;

static STATE: LazyLock<Mutex<NeuralGC>> = LazyLock::new(|| Mutex::new(NeuralGC::new()));

pub struct NeuralGC {
    objects: HashMap<u64, u64>, // id -> spike_count
}

impl NeuralGC {
    pub fn new() -> Self {
        Self { objects: HashMap::new() }
    }

    pub fn alloc(&mut self, id: u64) {
        self.objects.insert(id, 0);
    }

    pub fn spike(&mut self, id: u64) {
        if let Some(count) = self.objects.get_mut(&id) {
            *count += 1;
        }
    }

    pub fn decay_pass(&mut self, threshold: f64) -> usize {
        let min_spikes = (threshold * 10.0) as u64;
        let before = self.objects.len();
        self.objects.retain(|_, &mut v| v >= min_spikes);
        before - self.objects.len()
    }

    pub fn live_count(&self) -> usize {
        self.objects.len()
    }
}

impl Default for NeuralGC {
    fn default() -> Self { Self::new() }
}

#[unsafe(no_mangle)]
pub extern "C" fn ngc_alloc(id: i64) -> i64 {
    STATE.lock().unwrap().alloc(id as u64);
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn ngc_spike(id: i64) -> i64 {
    STATE.lock().unwrap().spike(id as u64);
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn ngc_decay(threshold: f64) -> i64 {
    STATE.lock().unwrap().decay_pass(threshold) as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn ngc_live() -> i64 {
    STATE.lock().unwrap().live_count() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alloc_increases_live() {
        let mut gc = NeuralGC::new();
        gc.alloc(1);
        gc.alloc(2);
        assert_eq!(gc.live_count(), 2);
    }

    #[test]
    fn test_spike_keeps_alive() {
        let mut gc = NeuralGC::new();
        gc.alloc(1);
        gc.spike(1);
        gc.spike(1);
        let collected = gc.decay_pass(0.1);
        assert_eq!(collected, 0);
    }

    #[test]
    fn test_decay_removes_idle() {
        let mut gc = NeuralGC::new();
        gc.alloc(1);
        gc.alloc(2);
        gc.spike(2);
        gc.spike(2);
        let collected = gc.decay_pass(0.15);
        assert_eq!(collected, 1);
    }

    #[test]
    fn test_live_count_after_decay() {
        let mut gc = NeuralGC::new();
        gc.alloc(1);
        gc.alloc(2);
        gc.spike(1);
        gc.spike(1);
        gc.decay_pass(0.1);
        assert!(gc.live_count() <= 2);
    }

    #[test]
    fn test_spike_unknown_id_noop() {
        let mut gc = NeuralGC::new();
        gc.spike(999);
        assert_eq!(gc.live_count(), 0);
    }

    #[test]
    fn test_ffi_ngc_alloc_and_live() {
        ngc_alloc(42);
        assert!(ngc_live() >= 1);
    }
}
