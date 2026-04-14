//! Trans-Optimization — Vitalis v1046
//!
//! Optimization beyond classical metrics: seeks aesthetic, elegant,
//! and minimal solutions that transcend pure performance.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<ToptState>> = LazyLock::new(|| Mutex::new(ToptState::default()));

#[derive(Default)]
struct ToptState {
    solutions: Vec<(String, f64, f64, f64)>,
    aesthetic_threshold: f64,
    transcendent_count: u64,
}

pub struct TransOptimization;

impl TransOptimization {
    pub fn evaluate(name: &str, performance: f64, elegance: f64, minimality: f64) -> f64 {
        let mut s = STATE.lock().unwrap();
        let score = performance * 0.3 + elegance * 0.4 + minimality * 0.3;
        s.solutions.push((name.to_string(), performance, elegance, minimality));
        if score > s.aesthetic_threshold { s.transcendent_count += 1; }
        score
    }

    pub fn set_threshold(t: f64) { STATE.lock().unwrap().aesthetic_threshold = t; }

    pub fn most_elegant() -> Option<String> {
        let s = STATE.lock().unwrap();
        s.solutions.iter().max_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal)).map(|(n, _, _, _)| n.clone())
    }

    pub fn most_minimal() -> Option<String> {
        let s = STATE.lock().unwrap();
        s.solutions.iter().max_by(|a, b| a.3.partial_cmp(&b.3).unwrap_or(std::cmp::Ordering::Equal)).map(|(n, _, _, _)| n.clone())
    }

    pub fn transcendent_count() -> u64 { STATE.lock().unwrap().transcendent_count }
    pub fn solution_count() -> usize { STATE.lock().unwrap().solutions.len() }
    pub fn reset() { *STATE.lock().unwrap() = ToptState::default(); }
}

#[unsafe(no_mangle)]
pub extern "C" fn topt_evaluate(perf: f64, elegance: f64, minimal: f64) -> f64 { TransOptimization::evaluate("ffi", perf, elegance, minimal) }
#[unsafe(no_mangle)]
pub extern "C" fn topt_transcendent() -> i64 { TransOptimization::transcendent_count() as i64 }
#[unsafe(no_mangle)]
pub extern "C" fn topt_count() -> i64 { TransOptimization::solution_count() as i64 }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_evaluate() { TransOptimization::reset(); let s = TransOptimization::evaluate("a", 0.8, 0.9, 0.7); assert!(s > 0.0); }
    #[test] fn test_elegant() { TransOptimization::reset(); TransOptimization::evaluate("a", 0.5, 0.9, 0.5); TransOptimization::evaluate("b", 0.9, 0.3, 0.5); assert_eq!(TransOptimization::most_elegant(), Some("a".to_string())); }
    #[test] fn test_minimal() { TransOptimization::reset(); TransOptimization::evaluate("a", 0.5, 0.5, 0.9); TransOptimization::evaluate("b", 0.9, 0.9, 0.1); assert_eq!(TransOptimization::most_minimal(), Some("a".to_string())); }
    #[test] fn test_threshold() { TransOptimization::reset(); TransOptimization::set_threshold(0.5); TransOptimization::evaluate("a", 0.9, 0.9, 0.9); assert!(TransOptimization::transcendent_count() > 0); }
    #[test] fn test_below_threshold() { TransOptimization::reset(); TransOptimization::set_threshold(0.99); TransOptimization::evaluate("a", 0.1, 0.1, 0.1); assert_eq!(TransOptimization::transcendent_count(), 0); }
    #[test] fn test_solution_count() { TransOptimization::reset(); TransOptimization::evaluate("a", 0.5, 0.5, 0.5); assert_eq!(TransOptimization::solution_count(), 1); }
    #[test] fn test_ffi() { TransOptimization::reset(); assert_eq!(topt_count(), 0); }
}
