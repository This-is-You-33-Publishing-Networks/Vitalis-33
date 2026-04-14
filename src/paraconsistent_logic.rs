//! Paraconsistent Logic — Vitalis v1221
//!
//! Tolerates controlled contradictions without explosion.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<PCLState>> = LazyLock::new(|| Mutex::new(PCLState::default()));

#[derive(Default)]
struct PCLState {
    truths: Vec<String>,
    contradictions: Vec<String>,
    explosion_prevented: u64,
}

pub struct ParaconsistentLogic;

impl ParaconsistentLogic {
    pub fn assert_truth(prop: &str) {
        let mut s = STATE.lock().unwrap();
        if s.truths.iter().any(|t| t == &format!("not_{}", prop)) {
            s.contradictions.push(prop.to_string());
            s.explosion_prevented += 1;
        }
        s.truths.push(prop.to_string());
    }

    pub fn assert_negation(prop: &str) {
        let neg = format!("not_{}", prop);
        let mut s = STATE.lock().unwrap();
        if s.truths.iter().any(|t| t == prop) {
            s.contradictions.push(prop.to_string());
            s.explosion_prevented += 1;
        }
        s.truths.push(neg);
    }

    pub fn truth_count() -> usize { STATE.lock().unwrap().truths.len() }

    pub fn contradiction_count() -> usize { STATE.lock().unwrap().contradictions.len() }

    pub fn explosions_prevented() -> u64 { STATE.lock().unwrap().explosion_prevented }

    pub fn reset() { *STATE.lock().unwrap() = PCLState::default(); }
}

#[unsafe(no_mangle)]
pub extern "C" fn pcl_truths() -> i64 { ParaconsistentLogic::truth_count() as i64 }
#[unsafe(no_mangle)]
pub extern "C" fn pcl_contradictions() -> i64 { ParaconsistentLogic::contradiction_count() as i64 }
#[unsafe(no_mangle)]
pub extern "C" fn pcl_prevented() -> i64 { ParaconsistentLogic::explosions_prevented() as i64 }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_assert() { ParaconsistentLogic::reset(); ParaconsistentLogic::assert_truth("P"); assert_eq!(ParaconsistentLogic::truth_count(), 1); }
    #[test] fn test_contradiction() { ParaconsistentLogic::reset(); ParaconsistentLogic::assert_truth("P"); ParaconsistentLogic::assert_negation("P"); assert_eq!(ParaconsistentLogic::contradiction_count(), 1); }
    #[test] fn test_no_explosion() { ParaconsistentLogic::reset(); ParaconsistentLogic::assert_truth("P"); ParaconsistentLogic::assert_negation("P"); assert_eq!(ParaconsistentLogic::explosions_prevented(), 1); }
    #[test] fn test_no_contradiction() { ParaconsistentLogic::reset(); ParaconsistentLogic::assert_truth("A"); ParaconsistentLogic::assert_truth("B"); assert_eq!(ParaconsistentLogic::contradiction_count(), 0); }
    #[test] fn test_negation_first() { ParaconsistentLogic::reset(); ParaconsistentLogic::assert_negation("Q"); ParaconsistentLogic::assert_truth("Q"); assert_eq!(ParaconsistentLogic::contradiction_count(), 1); }
    #[test] fn test_reset() { ParaconsistentLogic::reset(); ParaconsistentLogic::assert_truth("X"); ParaconsistentLogic::reset(); assert_eq!(ParaconsistentLogic::truth_count(), 0); }
    #[test] fn test_ffi() { ParaconsistentLogic::reset(); assert_eq!(pcl_truths(), 0); assert_eq!(pcl_contradictions(), 0); }
}
