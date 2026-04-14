//! Omega Convergence — Vitalis v1237
//!
//! Drives all subsystems toward unified omega-point.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<OmgcState>> = LazyLock::new(|| Mutex::new(OmgcState::default()));

#[derive(Default)]
struct OmgcState {
    subsystem_scores: Vec<(String, f64)>,
    convergence_rate: f64,
    epoch: u64,
}

pub struct OmegaConvergence;

impl OmegaConvergence {
    pub fn update_subsystem(name: &str, score: f64) {
        let mut s = STATE.lock().unwrap();
        if let Some(entry) = s.subsystem_scores.iter_mut().find(|(n, _)| n == name) {
            entry.1 = score;
        } else {
            s.subsystem_scores.push((name.to_string(), score));
        }
        s.epoch += 1;
        let n = s.subsystem_scores.len() as f64;
        let mean = s.subsystem_scores.iter().map(|(_, v)| v).sum::<f64>() / n;
        let variance = s.subsystem_scores.iter().map(|(_, v)| (v - mean).powi(2)).sum::<f64>() / n;
        s.convergence_rate = 1.0 / (1.0 + variance);
    }

    pub fn convergence_rate() -> f64 { STATE.lock().unwrap().convergence_rate }

    pub fn epoch() -> u64 { STATE.lock().unwrap().epoch }

    pub fn subsystem_count() -> usize { STATE.lock().unwrap().subsystem_scores.len() }

    pub fn mean_score() -> f64 {
        let s = STATE.lock().unwrap();
        if s.subsystem_scores.is_empty() { return 0.0; }
        s.subsystem_scores.iter().map(|(_, v)| v).sum::<f64>() / s.subsystem_scores.len() as f64
    }

    pub fn reset() { *STATE.lock().unwrap() = OmgcState::default(); }
}

#[unsafe(no_mangle)]
pub extern "C" fn omgc_update(score: f64) -> f64 { OmegaConvergence::update_subsystem("ffi", score); OmegaConvergence::convergence_rate() }
#[unsafe(no_mangle)]
pub extern "C" fn omgc_rate() -> f64 { OmegaConvergence::convergence_rate() }
#[unsafe(no_mangle)]
pub extern "C" fn omgc_epoch() -> i64 { OmegaConvergence::epoch() as i64 }
#[unsafe(no_mangle)]
pub extern "C" fn omgc_mean() -> f64 { OmegaConvergence::mean_score() }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_update() { OmegaConvergence::reset(); OmegaConvergence::update_subsystem("a", 0.8); assert_eq!(OmegaConvergence::subsystem_count(), 1); }
    #[test] fn test_convergence() { OmegaConvergence::reset(); OmegaConvergence::update_subsystem("a", 0.9); OmegaConvergence::update_subsystem("b", 0.9); assert!(OmegaConvergence::convergence_rate() > 0.5); }
    #[test] fn test_epoch() { OmegaConvergence::reset(); OmegaConvergence::update_subsystem("a", 1.0); assert_eq!(OmegaConvergence::epoch(), 1); }
    #[test] fn test_mean() { OmegaConvergence::reset(); OmegaConvergence::update_subsystem("a", 4.0); OmegaConvergence::update_subsystem("b", 6.0); assert!((OmegaConvergence::mean_score() - 5.0).abs() < f64::EPSILON); }
    #[test] fn test_update_existing() { OmegaConvergence::reset(); OmegaConvergence::update_subsystem("a", 1.0); OmegaConvergence::update_subsystem("a", 2.0); assert_eq!(OmegaConvergence::subsystem_count(), 1); }
    #[test] fn test_divergent() { OmegaConvergence::reset(); OmegaConvergence::update_subsystem("a", 0.0); OmegaConvergence::update_subsystem("b", 100.0); assert!(OmegaConvergence::convergence_rate() < 0.5); }
    #[test] fn test_reset() { OmegaConvergence::reset(); OmegaConvergence::update_subsystem("x", 1.0); OmegaConvergence::reset(); assert_eq!(OmegaConvergence::subsystem_count(), 0); }
    #[test] fn test_ffi() { OmegaConvergence::reset(); omgc_update(0.5); assert_eq!(omgc_epoch(), 1); }
}
