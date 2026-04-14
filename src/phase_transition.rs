//! Phase Transition — Vitalis v1073
//!
//! Detects phase transitions in compilation: complexity cliffs,
//! optimization barriers, and critical thresholds in program behavior.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<PtState>> = LazyLock::new(|| Mutex::new(PtState::default()));

#[derive(Default)]
struct PtState {
    measurements: Vec<(f64, f64)>,
    transitions: Vec<(f64, String)>,
    critical_threshold: f64,
}

pub struct PhaseTransition;

impl PhaseTransition {
    pub fn measure(parameter: f64, observable: f64) -> usize {
        let mut s = STATE.lock().unwrap();
        s.measurements.push((parameter, observable));
        if s.measurements.len() >= 2 {
            let n = s.measurements.len();
            let delta = (s.measurements[n - 1].1 - s.measurements[n - 2].1).abs();
            let param_delta = (s.measurements[n - 1].0 - s.measurements[n - 2].0).abs().max(0.001);
            if delta / param_delta > 10.0 {
                s.transitions.push((parameter, "discontinuity".to_string()));
                s.critical_threshold = parameter;
            }
        }
        s.measurements.len()
    }

    pub fn detect_criticality() -> Option<f64> {
        let s = STATE.lock().unwrap();
        if s.transitions.is_empty() { None } else { Some(s.critical_threshold) }
    }

    pub fn transition_count() -> usize { STATE.lock().unwrap().transitions.len() }
    pub fn measurement_count() -> usize { STATE.lock().unwrap().measurements.len() }
    pub fn critical_threshold() -> f64 { STATE.lock().unwrap().critical_threshold }
    pub fn reset() { *STATE.lock().unwrap() = PtState::default(); }
}

#[unsafe(no_mangle)]
pub extern "C" fn pt_measure(param: f64, obs: f64) -> i64 { PhaseTransition::measure(param, obs) as i64 }
#[unsafe(no_mangle)]
pub extern "C" fn pt_transitions() -> i64 { PhaseTransition::transition_count() as i64 }
#[unsafe(no_mangle)]
pub extern "C" fn pt_critical() -> f64 { PhaseTransition::critical_threshold() }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_measure() { PhaseTransition::reset(); assert_eq!(PhaseTransition::measure(0.1, 1.0), 1); }
    #[test] fn test_no_transition() { PhaseTransition::reset(); PhaseTransition::measure(0.1, 1.0); PhaseTransition::measure(0.2, 1.1); assert!(PhaseTransition::detect_criticality().is_none()); }
    #[test] fn test_detect_transition() { PhaseTransition::reset(); PhaseTransition::measure(0.1, 1.0); PhaseTransition::measure(0.11, 100.0); assert!(PhaseTransition::detect_criticality().is_some()); }
    #[test] fn test_transition_count() { PhaseTransition::reset(); PhaseTransition::measure(0.1, 1.0); PhaseTransition::measure(0.11, 100.0); assert!(PhaseTransition::transition_count() > 0); }
    #[test] fn test_measurement_count() { PhaseTransition::reset(); PhaseTransition::measure(0.1, 1.0); assert_eq!(PhaseTransition::measurement_count(), 1); }
    #[test] fn test_ffi() { PhaseTransition::reset(); assert_eq!(pt_transitions(), 0); }
}
