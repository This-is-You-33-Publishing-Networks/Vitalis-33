//! Bootstrap Infinity — Vitalis v1203
//!
//! Infinite bootstrap chain where each stage is more capable.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<BootstrapInfState>> = LazyLock::new(|| Mutex::new(BootstrapInfState::default()));

#[derive(Default)]
struct BootstrapInfState {
    stages: Vec<String>,
    current_stage: u64,
    capability_multiplier: f64,
}

pub struct BootstrapInfinity;

impl BootstrapInfinity {
    pub fn advance_stage(name: &str) -> f64 {
        let mut s = STATE.lock().unwrap();
        s.stages.push(name.to_string());
        s.current_stage += 1;
        if s.capability_multiplier == 0.0 { s.capability_multiplier = 1.0; }
        s.capability_multiplier *= 1.5;
        s.capability_multiplier
    }

    pub fn current_stage() -> u64 { STATE.lock().unwrap().current_stage }

    pub fn multiplier() -> f64 { STATE.lock().unwrap().capability_multiplier }

    pub fn stage_names() -> Vec<String> { STATE.lock().unwrap().stages.clone() }

    pub fn reset() { *STATE.lock().unwrap() = BootstrapInfState::default(); }
}

#[unsafe(no_mangle)]
pub extern "C" fn binf_advance() -> f64 { BootstrapInfinity::advance_stage("ffi_stage") }
#[unsafe(no_mangle)]
pub extern "C" fn binf_stage() -> i64 { BootstrapInfinity::current_stage() as i64 }
#[unsafe(no_mangle)]
pub extern "C" fn binf_multiplier() -> f64 { BootstrapInfinity::multiplier() }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_advance() { BootstrapInfinity::reset(); let m = BootstrapInfinity::advance_stage("s1"); assert!(m > 1.0); }
    #[test] fn test_stage() { BootstrapInfinity::reset(); BootstrapInfinity::advance_stage("s1"); assert_eq!(BootstrapInfinity::current_stage(), 1); }
    #[test] fn test_multiplier_growth() { BootstrapInfinity::reset(); BootstrapInfinity::advance_stage("s1"); BootstrapInfinity::advance_stage("s2"); assert!(BootstrapInfinity::multiplier() > 2.0); }
    #[test] fn test_names() { BootstrapInfinity::reset(); BootstrapInfinity::advance_stage("alpha"); assert_eq!(BootstrapInfinity::stage_names(), vec!["alpha".to_string()]); }
    #[test] fn test_reset() { BootstrapInfinity::reset(); BootstrapInfinity::advance_stage("s"); BootstrapInfinity::reset(); assert_eq!(BootstrapInfinity::current_stage(), 0); }
    #[test] fn test_ffi_advance() { BootstrapInfinity::reset(); binf_advance(); assert_eq!(binf_stage(), 1); }
    #[test] fn test_ffi_multiplier() { BootstrapInfinity::reset(); binf_advance(); assert!(binf_multiplier() > 0.0); }
}
