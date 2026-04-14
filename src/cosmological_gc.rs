//! Cosmological GC — Vitalis v1105
//!
//! Garbage collection modeled on cosmological expansion: entropy-driven
//! reclamation where cold regions are collected first.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<CgcState>> = LazyLock::new(|| Mutex::new(CgcState::default()));

#[derive(Default)]
struct CgcState {
    objects: Vec<(String, f64, u64)>,
    collected: u64,
    expansion_rate: f64,
    epoch: u64,
}

pub struct CosmologicalGc;

impl CosmologicalGc {
    pub fn allocate(name: &str, temperature: f64) -> usize {
        let mut s = STATE.lock().unwrap();
        let epoch = s.epoch;
        s.objects.push((name.to_string(), temperature, epoch));
        s.objects.len()
    }

    pub fn expand() -> u64 {
        let mut s = STATE.lock().unwrap();
        s.epoch += 1;
        s.expansion_rate += 0.1;
        for o in s.objects.iter_mut() { o.1 *= 0.9; }
        s.epoch
    }

    pub fn collect(threshold: f64) -> u64 {
        let mut s = STATE.lock().unwrap();
        let before = s.objects.len();
        s.objects.retain(|(_, temp, _)| *temp > threshold);
        let freed = (before - s.objects.len()) as u64;
        s.collected += freed;
        freed
    }

    pub fn live_count() -> usize { STATE.lock().unwrap().objects.len() }
    pub fn total_collected() -> u64 { STATE.lock().unwrap().collected }
    pub fn epoch() -> u64 { STATE.lock().unwrap().epoch }
    pub fn reset() { *STATE.lock().unwrap() = CgcState::default(); }
}

#[unsafe(no_mangle)]
pub extern "C" fn cgc_alloc(temp: f64) -> i64 { CosmologicalGc::allocate("ffi", temp) as i64 }
#[unsafe(no_mangle)]
pub extern "C" fn cgc_expand() -> i64 { CosmologicalGc::expand() as i64 }
#[unsafe(no_mangle)]
pub extern "C" fn cgc_collect(threshold: f64) -> i64 { CosmologicalGc::collect(threshold) as i64 }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_alloc() { CosmologicalGc::reset(); assert_eq!(CosmologicalGc::allocate("a", 100.0), 1); }
    #[test] fn test_expand() { CosmologicalGc::reset(); assert_eq!(CosmologicalGc::expand(), 1); }
    #[test] fn test_cooling() { CosmologicalGc::reset(); CosmologicalGc::allocate("a", 10.0); CosmologicalGc::expand(); let s = STATE.lock().unwrap(); assert!(s.objects[0].1 < 10.0); }
    #[test] fn test_collect() { CosmologicalGc::reset(); CosmologicalGc::allocate("a", 0.01); assert_eq!(CosmologicalGc::collect(0.1), 1); }
    #[test] fn test_no_collect() { CosmologicalGc::reset(); CosmologicalGc::allocate("a", 100.0); assert_eq!(CosmologicalGc::collect(0.1), 0); }
    #[test] fn test_ffi() { CosmologicalGc::reset(); assert_eq!(cgc_expand(), 1); }
}
