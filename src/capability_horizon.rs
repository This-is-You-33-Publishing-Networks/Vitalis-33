//! Capability Horizon — Vitalis v1205
//!
//! Discovers and extends capability boundaries dynamically.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<CapHorizonState>> = LazyLock::new(|| Mutex::new(CapHorizonState::default()));

#[derive(Default)]
struct CapHorizonState {
    boundaries: Vec<String>,
    extensions: u64,
    horizon_distance: f64,
}

pub struct CapabilityHorizon;

impl CapabilityHorizon {
    pub fn extend_boundary(name: &str, distance: f64) -> f64 {
        let mut s = STATE.lock().unwrap();
        s.boundaries.push(name.to_string());
        s.extensions += 1;
        s.horizon_distance += distance;
        s.horizon_distance
    }

    pub fn boundary_count() -> usize { STATE.lock().unwrap().boundaries.len() }

    pub fn extensions() -> u64 { STATE.lock().unwrap().extensions }

    pub fn horizon_distance() -> f64 { STATE.lock().unwrap().horizon_distance }

    pub fn reset() { *STATE.lock().unwrap() = CapHorizonState::default(); }
}

#[unsafe(no_mangle)]
pub extern "C" fn ch_extend(dist: f64) -> f64 { CapabilityHorizon::extend_boundary("ffi", dist) }
#[unsafe(no_mangle)]
pub extern "C" fn ch_boundaries() -> i64 { CapabilityHorizon::boundary_count() as i64 }
#[unsafe(no_mangle)]
pub extern "C" fn ch_distance() -> f64 { CapabilityHorizon::horizon_distance() }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_extend() { CapabilityHorizon::reset(); let d = CapabilityHorizon::extend_boundary("mem", 10.0); assert!((d - 10.0).abs() < f64::EPSILON); }
    #[test] fn test_count() { CapabilityHorizon::reset(); CapabilityHorizon::extend_boundary("a", 1.0); assert_eq!(CapabilityHorizon::boundary_count(), 1); }
    #[test] fn test_extensions() { CapabilityHorizon::reset(); CapabilityHorizon::extend_boundary("a", 1.0); CapabilityHorizon::extend_boundary("b", 2.0); assert_eq!(CapabilityHorizon::extensions(), 2); }
    #[test] fn test_distance_accumulates() { CapabilityHorizon::reset(); CapabilityHorizon::extend_boundary("a", 5.0); CapabilityHorizon::extend_boundary("b", 3.0); assert!((CapabilityHorizon::horizon_distance() - 8.0).abs() < f64::EPSILON); }
    #[test] fn test_reset() { CapabilityHorizon::reset(); CapabilityHorizon::extend_boundary("x", 1.0); CapabilityHorizon::reset(); assert_eq!(CapabilityHorizon::extensions(), 0); }
    #[test] fn test_ffi_extend() { CapabilityHorizon::reset(); ch_extend(5.0); assert_eq!(ch_boundaries(), 1); }
    #[test] fn test_ffi_distance() { CapabilityHorizon::reset(); ch_extend(7.0); assert!((ch_distance() - 7.0).abs() < f64::EPSILON); }
}
