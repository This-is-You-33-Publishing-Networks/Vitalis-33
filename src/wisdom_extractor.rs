//! Wisdom Extractor — Vitalis v1144
//!
//! Extracts higher-order patterns from accumulated compilation experience.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<WeState>> = LazyLock::new(|| Mutex::new(WeState::default()));

#[derive(Default)]
struct WeState { experiences: Vec<(String, f64)>, wisdom: Vec<(String, f64)>, extractions: u64 }

pub struct WisdomExtractor;

impl WisdomExtractor {
    pub fn record_experience(name: &str, value: f64) -> usize { let mut s = STATE.lock().unwrap(); s.experiences.push((name.to_string(), value)); s.experiences.len() }
    pub fn extract() -> usize {
        let mut s = STATE.lock().unwrap();
        s.extractions += 1;
        let threshold = 0.7;
        let new_wisdom: Vec<_> = s.experiences.iter().filter(|(_, v)| *v > threshold).map(|(n, v)| (format!("wisdom_{}", n), *v)).collect();
        let count = new_wisdom.len();
        s.wisdom.extend(new_wisdom);
        count
    }
    pub fn wisdom_count() -> usize { STATE.lock().unwrap().wisdom.len() }
    pub fn deepest_wisdom() -> Option<String> { STATE.lock().unwrap().wisdom.iter().max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal)).map(|(n, _)| n.clone()) }
    pub fn reset() { *STATE.lock().unwrap() = WeState::default(); }
}

#[unsafe(no_mangle)]
pub extern "C" fn we_record(val: f64) -> i64 { WisdomExtractor::record_experience("ffi", val) as i64 }
#[unsafe(no_mangle)]
pub extern "C" fn we_extract() -> i64 { WisdomExtractor::extract() as i64 }
#[unsafe(no_mangle)]
pub extern "C" fn we_wisdom() -> i64 { WisdomExtractor::wisdom_count() as i64 }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_record() { WisdomExtractor::reset(); assert_eq!(WisdomExtractor::record_experience("opt", 0.9), 1); }
    #[test] fn test_extract() { WisdomExtractor::reset(); WisdomExtractor::record_experience("a", 0.9); let n = WisdomExtractor::extract(); assert_eq!(n, 1); }
    #[test] fn test_no_extract() { WisdomExtractor::reset(); WisdomExtractor::record_experience("a", 0.1); assert_eq!(WisdomExtractor::extract(), 0); }
    #[test] fn test_deepest() { WisdomExtractor::reset(); WisdomExtractor::record_experience("a", 0.8); WisdomExtractor::record_experience("b", 0.99); WisdomExtractor::extract(); assert!(WisdomExtractor::deepest_wisdom().is_some()); }
    #[test] fn test_wisdom_count() { WisdomExtractor::reset(); WisdomExtractor::record_experience("a", 0.9); WisdomExtractor::extract(); assert_eq!(WisdomExtractor::wisdom_count(), 1); }
    #[test] fn test_empty() { WisdomExtractor::reset(); assert!(WisdomExtractor::deepest_wisdom().is_none()); }
    #[test] fn test_ffi() { WisdomExtractor::reset(); assert_eq!(we_wisdom(), 0); }
}
