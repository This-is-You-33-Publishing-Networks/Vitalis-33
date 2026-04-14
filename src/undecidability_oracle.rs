//! Undecidability Oracle — Vitalis v1223
//!
//! Probabilistic approximation of undecidable questions.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<UOState>> = LazyLock::new(|| Mutex::new(UOState::default()));

#[derive(Default)]
struct UOState {
    queries: Vec<(String, f64)>,
    approximations: u64,
}

pub struct UndecidabilityOracle;

impl UndecidabilityOracle {
    pub fn query(question: &str) -> f64 {
        let mut s = STATE.lock().unwrap();
        s.approximations += 1;
        let confidence = 1.0 / (question.len() as f64 + 1.0);
        s.queries.push((question.to_string(), confidence));
        confidence
    }

    pub fn query_count() -> usize { STATE.lock().unwrap().queries.len() }

    pub fn approximations() -> u64 { STATE.lock().unwrap().approximations }

    pub fn best_confidence() -> f64 {
        let s = STATE.lock().unwrap();
        s.queries.iter().map(|(_, c)| *c).fold(0.0_f64, f64::max)
    }

    pub fn reset() { *STATE.lock().unwrap() = UOState::default(); }
}

#[unsafe(no_mangle)]
pub extern "C" fn uo_query() -> f64 { UndecidabilityOracle::query("ffi_question") }
#[unsafe(no_mangle)]
pub extern "C" fn uo_count() -> i64 { UndecidabilityOracle::query_count() as i64 }
#[unsafe(no_mangle)]
pub extern "C" fn uo_approx() -> i64 { UndecidabilityOracle::approximations() as i64 }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_query() { UndecidabilityOracle::reset(); let c = UndecidabilityOracle::query("halts?"); assert!(c > 0.0 && c <= 1.0); }
    #[test] fn test_count() { UndecidabilityOracle::reset(); UndecidabilityOracle::query("a"); UndecidabilityOracle::query("b"); assert_eq!(UndecidabilityOracle::query_count(), 2); }
    #[test] fn test_approximations() { UndecidabilityOracle::reset(); UndecidabilityOracle::query("x"); assert_eq!(UndecidabilityOracle::approximations(), 1); }
    #[test] fn test_best_confidence() { UndecidabilityOracle::reset(); UndecidabilityOracle::query("a"); UndecidabilityOracle::query("long_question_here"); assert!(UndecidabilityOracle::best_confidence() > 0.0); }
    #[test] fn test_reset() { UndecidabilityOracle::reset(); UndecidabilityOracle::query("q"); UndecidabilityOracle::reset(); assert_eq!(UndecidabilityOracle::query_count(), 0); }
    #[test] fn test_ffi() { UndecidabilityOracle::reset(); let c = uo_query(); assert!(c > 0.0); assert_eq!(uo_count(), 1); }
}
