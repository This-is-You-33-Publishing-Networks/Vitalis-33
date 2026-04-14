//! Legacy Continuum — Vitalis v1295
//!
//! Continuity with all prior versions while transcending them.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<LegacyContinuumState>> = LazyLock::new(|| Mutex::new(LegacyContinuumState::default()));

#[derive(Default)]
struct LegacyContinuumState {
    versions: Vec<(u32, String)>,
    continuity_score: f64,
    breaks: u32,
}

pub struct LegacyContinuum;

impl LegacyContinuum {
    pub fn add_version(num: u32, desc: &str) {
        let mut s = STATE.lock().unwrap();
        s.versions.push((num, desc.to_string()));
        s.continuity_score = 1.0 - (s.breaks as f64 / s.versions.len().max(1) as f64);
    }
    pub fn record_break() -> u32 {
        let mut s = STATE.lock().unwrap();
        s.breaks += 1;
        s.continuity_score = 1.0 - (s.breaks as f64 / s.versions.len().max(1) as f64);
        s.breaks
    }
    pub fn version_count() -> usize { STATE.lock().unwrap().versions.len() }
    pub fn continuity_score() -> f64 { STATE.lock().unwrap().continuity_score }
    pub fn breaks() -> u32 { STATE.lock().unwrap().breaks }
    pub fn reset() { *STATE.lock().unwrap() = LegacyContinuumState::default(); }
}

#[unsafe(no_mangle)]
pub extern "C" fn lc_version_count() -> usize { LegacyContinuum::version_count() }
#[unsafe(no_mangle)]
pub extern "C" fn lc_continuity_score() -> f64 { LegacyContinuum::continuity_score() }
#[unsafe(no_mangle)]
pub extern "C" fn lc_breaks() -> u32 { LegacyContinuum::breaks() }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_add_version() { LegacyContinuum::reset(); LegacyContinuum::add_version(1, "initial"); assert_eq!(LegacyContinuum::version_count(), 1); }
    #[test] fn test_continuity_perfect() { LegacyContinuum::reset(); LegacyContinuum::add_version(1, "a"); assert!((LegacyContinuum::continuity_score() - 1.0).abs() < 1e-9); }
    #[test] fn test_record_break() { LegacyContinuum::reset(); LegacyContinuum::add_version(1, "a"); let b = LegacyContinuum::record_break(); assert_eq!(b, 1); }
    #[test] fn test_continuity_decreases() { LegacyContinuum::reset(); LegacyContinuum::add_version(1, "a"); LegacyContinuum::record_break(); assert!(LegacyContinuum::continuity_score() < 1.0); }
    #[test] fn test_multiple_versions() { LegacyContinuum::reset(); for i in 0..5 { LegacyContinuum::add_version(i, "v"); } assert_eq!(LegacyContinuum::version_count(), 5); }
    #[test] fn test_breaks_count() { LegacyContinuum::reset(); LegacyContinuum::add_version(1, "x"); LegacyContinuum::record_break(); LegacyContinuum::record_break(); assert_eq!(LegacyContinuum::breaks(), 2); }
    #[test] fn test_reset() { LegacyContinuum::reset(); LegacyContinuum::add_version(1, "a"); LegacyContinuum::reset(); assert_eq!(LegacyContinuum::version_count(), 0); }
}
