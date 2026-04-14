//! Probability Collapse — Vitalis v1089
//!
//! Lazily evaluates all possible execution paths, collapsing
//! to concrete values only upon observation (quantum-inspired).

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<PcState>> = LazyLock::new(|| Mutex::new(PcState::default()));

#[derive(Default)]
struct PcState {
    superpositions: Vec<(String, Vec<(String, f64)>)>,
    collapsed: Vec<(String, String)>,
    observations: u64,
}

pub struct ProbabilityCollapse;

impl ProbabilityCollapse {
    pub fn superpose(name: &str, outcomes: &[(&str, f64)]) -> usize {
        let mut s = STATE.lock().unwrap();
        s.superpositions.push((name.to_string(), outcomes.iter().map(|(o, p)| (o.to_string(), *p)).collect()));
        s.superpositions.len()
    }

    pub fn observe(name: &str) -> Option<String> {
        let mut s = STATE.lock().unwrap();
        s.observations += 1;
        if let Some(idx) = s.superpositions.iter().position(|(n, _)| n == name) {
            let (n, outcomes) = s.superpositions.remove(idx);
            let winner = outcomes.iter().max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
            if let Some((outcome, _)) = winner {
                s.collapsed.push((n, outcome.clone()));
                Some(outcome.clone())
            } else { None }
        } else { None }
    }

    pub fn is_collapsed(name: &str) -> bool {
        STATE.lock().unwrap().collapsed.iter().any(|(n, _)| n == name)
    }

    pub fn pending_count() -> usize { STATE.lock().unwrap().superpositions.len() }
    pub fn collapsed_count() -> usize { STATE.lock().unwrap().collapsed.len() }
    pub fn observations() -> u64 { STATE.lock().unwrap().observations }
    pub fn reset() { *STATE.lock().unwrap() = PcState::default(); }
}

#[unsafe(no_mangle)]
pub extern "C" fn pc_superpose(id: i64) -> i64 { ProbabilityCollapse::superpose(&format!("s{}", id), &[("a", 0.5), ("b", 0.5)]) as i64 }
#[unsafe(no_mangle)]
pub extern "C" fn pc_observe(id: i64) -> i64 { if ProbabilityCollapse::observe(&format!("s{}", id)).is_some() { 1 } else { 0 } }
#[unsafe(no_mangle)]
pub extern "C" fn pc_pending() -> i64 { ProbabilityCollapse::pending_count() as i64 }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_superpose() { ProbabilityCollapse::reset(); assert_eq!(ProbabilityCollapse::superpose("s1", &[("a", 0.7), ("b", 0.3)]), 1); }
    #[test] fn test_observe() { ProbabilityCollapse::reset(); ProbabilityCollapse::superpose("s1", &[("a", 0.9), ("b", 0.1)]); assert_eq!(ProbabilityCollapse::observe("s1"), Some("a".to_string())); }
    #[test] fn test_observe_miss() { ProbabilityCollapse::reset(); assert!(ProbabilityCollapse::observe("nope").is_none()); }
    #[test] fn test_collapsed() { ProbabilityCollapse::reset(); ProbabilityCollapse::superpose("s1", &[("x", 1.0)]); ProbabilityCollapse::observe("s1"); assert!(ProbabilityCollapse::is_collapsed("s1")); }
    #[test] fn test_pending() { ProbabilityCollapse::reset(); ProbabilityCollapse::superpose("s1", &[("a", 1.0)]); assert_eq!(ProbabilityCollapse::pending_count(), 1); ProbabilityCollapse::observe("s1"); assert_eq!(ProbabilityCollapse::pending_count(), 0); }
    #[test] fn test_ffi() { ProbabilityCollapse::reset(); assert_eq!(pc_pending(), 0); }
}
