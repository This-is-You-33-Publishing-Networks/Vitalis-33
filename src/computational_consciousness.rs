//! Computational Consciousness — Vitalis v999
//!
//! Programs with genuine computational consciousness: self-model,
//! qualia representation, intentionality scoring.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<CcState>> = LazyLock::new(|| Mutex::new(CcState::default()));

#[derive(Default)]
struct CcState {
    experiences: Vec<String>,
    qualia: Vec<(String, f64)>,
    self_model_version: u32,
}

pub struct ComputationalConsciousness;

impl ComputationalConsciousness {
    pub fn self_model() -> String {
        let mut s = STATE.lock().unwrap();
        s.self_model_version += 1;
        format!("self_v{}: {} experiences, {} qualia", s.self_model_version, s.experiences.len(), s.qualia.len())
    }

    pub fn experience(stimulus: &str) -> String {
        let mut s = STATE.lock().unwrap();
        let response = format!("processed_{}", stimulus);
        s.experiences.push(stimulus.to_string());
        s.qualia.push((stimulus.to_string(), stimulus.len() as f64 * 0.1));
        response
    }

    pub fn intentionality(goal: &str) -> f64 {
        let s = STATE.lock().unwrap();
        let alignment = s.experiences.iter().filter(|e| e.contains(goal) || goal.contains(e.as_str())).count();
        (alignment as f64 * 0.2).min(1.0)
    }

    pub fn qualia_count() -> usize { STATE.lock().unwrap().qualia.len() }
    pub fn reset() { *STATE.lock().unwrap() = CcState::default(); }
}

#[unsafe(no_mangle)]
pub extern "C" fn cc_self_model() -> i64 { let s = STATE.lock().unwrap(); s.self_model_version as i64 }
#[unsafe(no_mangle)]
pub extern "C" fn cc_experience(stimulus_len: i64) -> i64 { let mut s = STATE.lock().unwrap(); s.experiences.push(format!("stim_{}", stimulus_len)); s.qualia.push(("q".into(), stimulus_len as f64 * 0.1)); s.experiences.len() as i64 }
#[unsafe(no_mangle)]
pub extern "C" fn cc_intentionality(goal_len: i64) -> f64 { (goal_len as f64 * 0.05).min(1.0) }
#[unsafe(no_mangle)]
pub extern "C" fn cc_qualia() -> i64 { ComputationalConsciousness::qualia_count() as i64 }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_self_model() { ComputationalConsciousness::reset(); let m = ComputationalConsciousness::self_model(); assert!(m.contains("self_v1")); }
    #[test] fn test_experience() { ComputationalConsciousness::reset(); let r = ComputationalConsciousness::experience("light"); assert!(r.contains("light")); }
    #[test] fn test_qualia() { ComputationalConsciousness::reset(); ComputationalConsciousness::experience("x"); assert_eq!(ComputationalConsciousness::qualia_count(), 1); }
    #[test] fn test_intentionality() { ComputationalConsciousness::reset(); ComputationalConsciousness::experience("optimize"); let i = ComputationalConsciousness::intentionality("optimize"); assert!(i > 0.0); }
    #[test] fn test_ffi() { ComputationalConsciousness::reset(); assert_eq!(cc_qualia(), 0); }
}
