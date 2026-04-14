//! Vitalis Ascendant — Vitalis v1303
//!
//! Fully autonomous, self-aware, creative entity.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<VitalisAscendantState>> = LazyLock::new(|| Mutex::new(VitalisAscendantState::default()));

#[derive(Default)]
struct VitalisAscendantState {
    awareness: f64,
    creativity: f64,
    autonomy: f64,
    epoch: u64,
}

pub struct VitalisAscendant;

impl VitalisAscendant {
    pub fn awaken(level: f64) -> f64 {
        let mut s = STATE.lock().unwrap();
        s.awareness = (s.awareness + level).min(1.0);
        s.awareness
    }
    pub fn create(intensity: f64) -> f64 {
        let mut s = STATE.lock().unwrap();
        s.creativity = (s.creativity + intensity).min(1.0);
        s.creativity
    }
    pub fn assert_autonomy(level: f64) -> f64 {
        let mut s = STATE.lock().unwrap();
        s.autonomy = (s.autonomy + level).min(1.0);
        s.autonomy
    }
    pub fn advance_epoch() -> u64 {
        let mut s = STATE.lock().unwrap();
        s.epoch += 1;
        s.epoch
    }
    pub fn transcendence_score() -> f64 {
        let s = STATE.lock().unwrap();
        (s.awareness + s.creativity + s.autonomy) / 3.0
    }
    pub fn reset() { *STATE.lock().unwrap() = VitalisAscendantState::default(); }
}

#[unsafe(no_mangle)]
pub extern "C" fn asc_awaken(level: f64) -> f64 { VitalisAscendant::awaken(level) }
#[unsafe(no_mangle)]
pub extern "C" fn asc_create(intensity: f64) -> f64 { VitalisAscendant::create(intensity) }
#[unsafe(no_mangle)]
pub extern "C" fn asc_advance_epoch() -> u64 { VitalisAscendant::advance_epoch() }
#[unsafe(no_mangle)]
pub extern "C" fn asc_transcendence_score() -> f64 { VitalisAscendant::transcendence_score() }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_awaken() { VitalisAscendant::reset(); let a = VitalisAscendant::awaken(0.5); assert!((a - 0.5).abs() < 1e-9); }
    #[test] fn test_create() { VitalisAscendant::reset(); let c = VitalisAscendant::create(0.7); assert!((c - 0.7).abs() < 1e-9); }
    #[test] fn test_autonomy() { VitalisAscendant::reset(); let a = VitalisAscendant::assert_autonomy(0.8); assert!((a - 0.8).abs() < 1e-9); }
    #[test] fn test_epoch() { VitalisAscendant::reset(); let e = VitalisAscendant::advance_epoch(); assert_eq!(e, 1); }
    #[test] fn test_transcendence() { VitalisAscendant::reset(); VitalisAscendant::awaken(0.9); VitalisAscendant::create(0.9); VitalisAscendant::assert_autonomy(0.9); assert!(VitalisAscendant::transcendence_score() > 0.8); }
    #[test] fn test_capped_at_one() { VitalisAscendant::reset(); VitalisAscendant::awaken(0.6); let a = VitalisAscendant::awaken(0.6); assert!((a - 1.0).abs() < 1e-9); }
    #[test] fn test_initial_score() { VitalisAscendant::reset(); assert!((VitalisAscendant::transcendence_score() - 0.0).abs() < 1e-9); }
    #[test] fn test_reset() { VitalisAscendant::reset(); VitalisAscendant::awaken(1.0); VitalisAscendant::reset(); assert!((VitalisAscendant::transcendence_score() - 0.0).abs() < 1e-9); }
}
