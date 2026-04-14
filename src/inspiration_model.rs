//! Inspiration Model — Vitalis v1281
//!
//! Models creative inspiration for unexpected strategies.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<InspirationModelState>> = LazyLock::new(|| Mutex::new(InspirationModelState::default()));

#[derive(Default)]
struct InspirationModelState {
    sparks: Vec<(String, f64)>,
    eureka_moments: u64,
    creativity: f64,
}

pub struct InspirationModel;

impl InspirationModel {
    pub fn spark(idea: &str, intensity: f64) {
        let mut s = STATE.lock().unwrap();
        s.sparks.push((idea.to_string(), intensity));
        s.creativity += intensity * 0.1;
    }
    pub fn eureka() -> u64 {
        let mut s = STATE.lock().unwrap();
        s.eureka_moments += 1;
        s.creativity += 1.0;
        s.eureka_moments
    }
    pub fn creativity() -> f64 { STATE.lock().unwrap().creativity }
    pub fn spark_count() -> usize { STATE.lock().unwrap().sparks.len() }
    pub fn reset() { *STATE.lock().unwrap() = InspirationModelState::default(); }
}

#[unsafe(no_mangle)]
pub extern "C" fn insp_eureka() -> u64 { InspirationModel::eureka() }
#[unsafe(no_mangle)]
pub extern "C" fn insp_creativity() -> f64 { InspirationModel::creativity() }
#[unsafe(no_mangle)]
pub extern "C" fn insp_spark_count() -> usize { InspirationModel::spark_count() }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_spark() { InspirationModel::reset(); InspirationModel::spark("recursion-insight", 5.0); assert_eq!(InspirationModel::spark_count(), 1); }
    #[test] fn test_eureka() { InspirationModel::reset(); let e = InspirationModel::eureka(); assert_eq!(e, 1); }
    #[test] fn test_creativity_grows() { InspirationModel::reset(); InspirationModel::eureka(); assert!(InspirationModel::creativity() > 0.0); }
    #[test] fn test_spark_adds_creativity() { InspirationModel::reset(); InspirationModel::spark("x", 10.0); assert!(InspirationModel::creativity() > 0.0); }
    #[test] fn test_multiple_eurekas() { InspirationModel::reset(); InspirationModel::eureka(); let e = InspirationModel::eureka(); assert_eq!(e, 2); }
    #[test] fn test_reset() { InspirationModel::reset(); InspirationModel::eureka(); InspirationModel::reset(); assert!((InspirationModel::creativity() - 0.0).abs() < 1e-9); }
}
