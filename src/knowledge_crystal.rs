//! Knowledge Crystal — Vitalis v1142
//!
//! Distills compilation knowledge into reusable, composable,
//! transferable crystalline artifacts.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<KcState>> = LazyLock::new(|| Mutex::new(KcState::default()));

#[derive(Default)]
struct KcState {
    crystals: Vec<(String, Vec<String>, f64)>,
    transfers: u64,
}

pub struct KnowledgeCrystal;

impl KnowledgeCrystal {
    pub fn crystallize(name: &str, insights: &[&str], purity: f64) -> usize {
        let mut s = STATE.lock().unwrap();
        s.crystals.push((name.to_string(), insights.iter().map(|i| i.to_string()).collect(), purity));
        s.crystals.len()
    }

    pub fn transfer(name: &str) -> Option<f64> {
        let mut s = STATE.lock().unwrap();
        s.transfers += 1;
        s.crystals.iter().find(|(n, _, _)| n == name).map(|(_, _, p)| *p)
    }

    pub fn compose(a: &str, b: &str) -> Option<f64> {
        let s = STATE.lock().unwrap();
        let pa = s.crystals.iter().find(|(n, _, _)| n == a).map(|(_, _, p)| *p);
        let pb = s.crystals.iter().find(|(n, _, _)| n == b).map(|(_, _, p)| *p);
        if let (Some(a), Some(b)) = (pa, pb) { Some((a + b) / 2.0) } else { None }
    }

    pub fn crystal_count() -> usize { STATE.lock().unwrap().crystals.len() }
    pub fn purest() -> Option<String> { STATE.lock().unwrap().crystals.iter().max_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal)).map(|(n, _, _)| n.clone()) }
    pub fn transfers() -> u64 { STATE.lock().unwrap().transfers }
    pub fn reset() { *STATE.lock().unwrap() = KcState::default(); }
}

#[unsafe(no_mangle)]
pub extern "C" fn kc_crystallize(purity: f64) -> i64 { KnowledgeCrystal::crystallize("ffi", &["insight"], purity) as i64 }
#[unsafe(no_mangle)]
pub extern "C" fn kc_transfer() -> f64 { KnowledgeCrystal::transfer("ffi").unwrap_or(0.0) }
#[unsafe(no_mangle)]
pub extern "C" fn kc_count() -> i64 { KnowledgeCrystal::crystal_count() as i64 }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_crystallize() { KnowledgeCrystal::reset(); assert_eq!(KnowledgeCrystal::crystallize("c1", &["a", "b"], 0.9), 1); }
    #[test] fn test_transfer() { KnowledgeCrystal::reset(); KnowledgeCrystal::crystallize("c1", &["a"], 0.8); assert_eq!(KnowledgeCrystal::transfer("c1"), Some(0.8)); }
    #[test] fn test_transfer_miss() { KnowledgeCrystal::reset(); assert!(KnowledgeCrystal::transfer("nope").is_none()); }
    #[test] fn test_compose() { KnowledgeCrystal::reset(); KnowledgeCrystal::crystallize("a", &[], 0.8); KnowledgeCrystal::crystallize("b", &[], 0.6); let c = KnowledgeCrystal::compose("a", "b").unwrap(); assert!((c - 0.7).abs() < 0.01); }
    #[test] fn test_purest() { KnowledgeCrystal::reset(); KnowledgeCrystal::crystallize("a", &[], 0.3); KnowledgeCrystal::crystallize("b", &[], 0.99); assert_eq!(KnowledgeCrystal::purest(), Some("b".to_string())); }
    #[test] fn test_count() { KnowledgeCrystal::reset(); KnowledgeCrystal::crystallize("a", &[], 1.0); assert_eq!(KnowledgeCrystal::crystal_count(), 1); }
    #[test] fn test_transfers() { KnowledgeCrystal::reset(); KnowledgeCrystal::transfer("x"); assert_eq!(KnowledgeCrystal::transfers(), 1); }
    #[test] fn test_ffi() { KnowledgeCrystal::reset(); assert_eq!(kc_count(), 0); }
}
