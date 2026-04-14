//! Introspection Engine — Vitalis v1027
//!
//! Deep self-analysis of the compiler's decision-making quality.
//! Scores decisions, tracks regret, and improves future choices.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<IeState>> = LazyLock::new(|| Mutex::new(IeState::default()));

#[derive(Default)]
struct IeState {
    decisions: Vec<(String, f64, f64)>,
    regret_sum: f64,
    best_score: f64,
}

pub struct IntrospectionEngine;

impl IntrospectionEngine {
    pub fn record_decision(name: &str, expected: f64, actual: f64) -> f64 {
        let mut s = STATE.lock().unwrap();
        let regret = (expected - actual).max(0.0);
        s.regret_sum += regret;
        if actual > s.best_score { s.best_score = actual; }
        s.decisions.push((name.to_string(), expected, actual));
        regret
    }

    pub fn average_regret() -> f64 {
        let s = STATE.lock().unwrap();
        if s.decisions.is_empty() { 0.0 } else { s.regret_sum / s.decisions.len() as f64 }
    }

    pub fn decision_quality() -> f64 {
        let s = STATE.lock().unwrap();
        if s.decisions.is_empty() { return 1.0; }
        let good = s.decisions.iter().filter(|(_, e, a)| a >= e).count();
        good as f64 / s.decisions.len() as f64
    }

    pub fn worst_decision() -> Option<String> {
        let s = STATE.lock().unwrap();
        s.decisions.iter().max_by(|a, b| (a.1 - a.2).partial_cmp(&(b.1 - b.2)).unwrap_or(std::cmp::Ordering::Equal)).map(|(n, _, _)| n.clone())
    }

    pub fn decision_count() -> usize { STATE.lock().unwrap().decisions.len() }
    pub fn reset() { *STATE.lock().unwrap() = IeState::default(); }
}

#[unsafe(no_mangle)]
pub extern "C" fn ie_record(expected: f64, actual: f64) -> f64 { IntrospectionEngine::record_decision("ffi", expected, actual) }
#[unsafe(no_mangle)]
pub extern "C" fn ie_regret() -> f64 { IntrospectionEngine::average_regret() }
#[unsafe(no_mangle)]
pub extern "C" fn ie_quality() -> f64 { IntrospectionEngine::decision_quality() }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_no_regret() { IntrospectionEngine::reset(); let r = IntrospectionEngine::record_decision("d1", 0.5, 0.8); assert!((r - 0.0).abs() < 0.01); }
    #[test] fn test_regret() { IntrospectionEngine::reset(); let r = IntrospectionEngine::record_decision("d1", 0.9, 0.3); assert!(r > 0.0); }
    #[test] fn test_avg_regret() { IntrospectionEngine::reset(); IntrospectionEngine::record_decision("a", 1.0, 0.5); IntrospectionEngine::record_decision("b", 1.0, 1.0); assert!(IntrospectionEngine::average_regret() > 0.0); }
    #[test] fn test_quality() { IntrospectionEngine::reset(); IntrospectionEngine::record_decision("a", 0.5, 0.8); assert!((IntrospectionEngine::decision_quality() - 1.0).abs() < 0.01); }
    #[test] fn test_worst() { IntrospectionEngine::reset(); IntrospectionEngine::record_decision("good", 0.5, 0.8); IntrospectionEngine::record_decision("bad", 0.9, 0.1); assert_eq!(IntrospectionEngine::worst_decision(), Some("bad".to_string())); }
    #[test] fn test_ffi() { IntrospectionEngine::reset(); assert!((ie_quality() - 1.0).abs() < 0.01); }
}
