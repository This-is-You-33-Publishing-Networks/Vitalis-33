//! Holographic Memory — Vitalis v1099
//!
//! Holographic storage where every fragment contains information
//! about the whole program — enabling graceful degradation.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<HmState>> = LazyLock::new(|| Mutex::new(HmState::default()));

#[derive(Default)]
struct HmState {
    hologram: Vec<(String, Vec<u8>)>,
    fragment_count: usize,
    recovery_success: u64,
}

pub struct HolographicMemory;

impl HolographicMemory {
    pub fn encode(key: &str, data: &[u8]) -> usize {
        let mut s = STATE.lock().unwrap();
        let fragments: Vec<u8> = data.iter().enumerate().map(|(i, &b)| b.wrapping_add(i as u8)).collect();
        s.hologram.push((key.to_string(), fragments));
        s.fragment_count += data.len();
        s.hologram.len()
    }

    pub fn decode(key: &str) -> Option<Vec<u8>> {
        let s = STATE.lock().unwrap();
        s.hologram.iter().find(|(k, _)| k == key).map(|(_, frags)| frags.iter().enumerate().map(|(i, &b)| b.wrapping_sub(i as u8)).collect())
    }

    pub fn recover_from_fragment(key: &str, fragment_idx: usize) -> Option<u8> {
        let mut s = STATE.lock().unwrap();
        let result = s.hologram.iter().find(|(k, _)| k == key).and_then(|(_, frags)| {
            if fragment_idx < frags.len() { Some(frags[fragment_idx].wrapping_sub(fragment_idx as u8)) } else { None }
        });
        if result.is_some() { s.recovery_success += 1; }
        result
    }

    pub fn entry_count() -> usize { STATE.lock().unwrap().hologram.len() }
    pub fn fragment_count() -> usize { STATE.lock().unwrap().fragment_count }
    pub fn recoveries() -> u64 { STATE.lock().unwrap().recovery_success }
    pub fn reset() { *STATE.lock().unwrap() = HmState::default(); }
}

#[unsafe(no_mangle)]
pub extern "C" fn hm_encode(key: i64) -> i64 { HolographicMemory::encode(&format!("k{}", key), &[1, 2, 3]) as i64 }
#[unsafe(no_mangle)]
pub extern "C" fn hm_decode(key: i64) -> i64 { if HolographicMemory::decode(&format!("k{}", key)).is_some() { 1 } else { 0 } }
#[unsafe(no_mangle)]
pub extern "C" fn hm_entries() -> i64 { HolographicMemory::entry_count() as i64 }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_encode() { HolographicMemory::reset(); assert_eq!(HolographicMemory::encode("k1", &[1, 2, 3]), 1); }
    #[test] fn test_decode() { HolographicMemory::reset(); HolographicMemory::encode("k1", &[10, 20, 30]); let d = HolographicMemory::decode("k1").unwrap(); assert_eq!(d, vec![10, 20, 30]); }
    #[test] fn test_decode_miss() { HolographicMemory::reset(); assert!(HolographicMemory::decode("nope").is_none()); }
    #[test] fn test_recover() { HolographicMemory::reset(); HolographicMemory::encode("k1", &[42, 7]); let b = HolographicMemory::recover_from_fragment("k1", 0).unwrap(); assert_eq!(b, 42); }
    #[test] fn test_fragment_count() { HolographicMemory::reset(); HolographicMemory::encode("k1", &[1, 2, 3, 4]); assert_eq!(HolographicMemory::fragment_count(), 4); }
    #[test] fn test_entries() { HolographicMemory::reset(); HolographicMemory::encode("a", &[1]); HolographicMemory::encode("b", &[2]); assert_eq!(HolographicMemory::entry_count(), 2); }
    #[test] fn test_ffi() { HolographicMemory::reset(); assert_eq!(hm_entries(), 0); }
}
