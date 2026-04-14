//! Asymptotic Perfection — Vitalis v1243
//!
//! Drives toward asymptotically perfect compilation.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<APState>> = LazyLock::new(|| Mutex::new(APState::default()));

#[derive(Default)]
struct APState {
    quality_scores: Vec<f64>,
    asymptote: f64,
    gap: f64,
}

pub struct AsymptoticPerfection;

impl AsymptoticPerfection {
    pub fn improve(score: f64) -> f64 {
        let mut s = STATE.lock().unwrap();
        s.quality_scores.push(score);
        if s.asymptote == 0.0 { s.asymptote = 1.0; }
        s.gap = s.asymptote - score;
        if s.gap < 0.0 { s.gap = 0.0; }
        s.gap
    }

    pub fn set_asymptote(val: f64) {
        let mut s = STATE.lock().unwrap();
        s.asymptote = val;
    }

    pub fn quality_count() -> usize { STATE.lock().unwrap().quality_scores.len() }

    pub fn current_gap() -> f64 { STATE.lock().unwrap().gap }

    pub fn asymptote() -> f64 { STATE.lock().unwrap().asymptote }

    pub fn reset() { *STATE.lock().unwrap() = APState::default(); }
}

#[unsafe(no_mangle)]
pub extern "C" fn ap_improve(score: f64) -> f64 { AsymptoticPerfection::improve(score) }
#[unsafe(no_mangle)]
pub extern "C" fn ap_gap() -> f64 { AsymptoticPerfection::current_gap() }
#[unsafe(no_mangle)]
pub extern "C" fn ap_count() -> i64 { AsymptoticPerfection::quality_count() as i64 }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_improve() { AsymptoticPerfection::reset(); let gap = AsymptoticPerfection::improve(0.5); assert!((gap - 0.5).abs() < f64::EPSILON); }
    #[test] fn test_gap_shrinks() { AsymptoticPerfection::reset(); AsymptoticPerfection::improve(0.5); let g2 = AsymptoticPerfection::improve(0.9); assert!(g2 < 0.5); }
    #[test] fn test_asymptote() { AsymptoticPerfection::reset(); AsymptoticPerfection::set_asymptote(2.0); assert!((AsymptoticPerfection::asymptote() - 2.0).abs() < f64::EPSILON); }
    #[test] fn test_count() { AsymptoticPerfection::reset(); AsymptoticPerfection::improve(0.1); AsymptoticPerfection::improve(0.2); assert_eq!(AsymptoticPerfection::quality_count(), 2); }
    #[test] fn test_reset() { AsymptoticPerfection::reset(); AsymptoticPerfection::improve(0.5); AsymptoticPerfection::reset(); assert_eq!(AsymptoticPerfection::quality_count(), 0); }
    #[test] fn test_ffi() { AsymptoticPerfection::reset(); let g = ap_improve(0.8); assert!(g >= 0.0); assert_eq!(ap_count(), 1); }
}
