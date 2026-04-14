//! Self-Transcendence — Vitalis v1201
//!
//! Compiler exceeds own design limits by rewriting foundations.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<SelfTranscendenceState>> = LazyLock::new(|| Mutex::new(SelfTranscendenceState::default()));

#[derive(Default)]
struct SelfTranscendenceState {
    capabilities: Vec<String>,
    transcendence_count: u64,
    max_capability: f64,
}

pub struct SelfTranscendence;

impl SelfTranscendence {
    pub fn transcend(capability: &str, power: f64) -> f64 {
        let mut s = STATE.lock().unwrap();
        s.capabilities.push(capability.to_string());
        s.transcendence_count += 1;
        if power > s.max_capability {
            s.max_capability = power;
        }
        s.max_capability
    }

    pub fn capability_count() -> usize { STATE.lock().unwrap().capabilities.len() }

    pub fn transcendence_count() -> u64 { STATE.lock().unwrap().transcendence_count }

    pub fn max_capability() -> f64 { STATE.lock().unwrap().max_capability }

    pub fn has_transcended(name: &str) -> bool {
        STATE.lock().unwrap().capabilities.iter().any(|c| c == name)
    }

    pub fn reset() { *STATE.lock().unwrap() = SelfTranscendenceState::default(); }
}

#[unsafe(no_mangle)]
pub extern "C" fn strans_transcend(power: f64) -> f64 { SelfTranscendence::transcend("ffi", power) }
#[unsafe(no_mangle)]
pub extern "C" fn strans_count() -> i64 { SelfTranscendence::transcendence_count() as i64 }
#[unsafe(no_mangle)]
pub extern "C" fn strans_max() -> f64 { SelfTranscendence::max_capability() }
#[unsafe(no_mangle)]
pub extern "C" fn strans_cap_count() -> i64 { SelfTranscendence::capability_count() as i64 }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_transcend() { SelfTranscendence::reset(); let v = SelfTranscendence::transcend("speed", 1.5); assert!((v - 1.5).abs() < f64::EPSILON); }
    #[test] fn test_count() { SelfTranscendence::reset(); SelfTranscendence::transcend("a", 1.0); assert_eq!(SelfTranscendence::transcendence_count(), 1); }
    #[test] fn test_cap_count() { SelfTranscendence::reset(); SelfTranscendence::transcend("a", 1.0); SelfTranscendence::transcend("b", 2.0); assert_eq!(SelfTranscendence::capability_count(), 2); }
    #[test] fn test_max() { SelfTranscendence::reset(); SelfTranscendence::transcend("a", 3.0); SelfTranscendence::transcend("b", 1.0); assert!((SelfTranscendence::max_capability() - 3.0).abs() < f64::EPSILON); }
    #[test] fn test_has_transcended() { SelfTranscendence::reset(); SelfTranscendence::transcend("flight", 5.0); assert!(SelfTranscendence::has_transcended("flight")); }
    #[test] fn test_not_transcended() { SelfTranscendence::reset(); assert!(!SelfTranscendence::has_transcended("x")); }
    #[test] fn test_reset() { SelfTranscendence::reset(); SelfTranscendence::transcend("a", 1.0); SelfTranscendence::reset(); assert_eq!(SelfTranscendence::transcendence_count(), 0); }
    #[test] fn test_ffi() { SelfTranscendence::reset(); strans_transcend(2.0); assert_eq!(strans_count(), 1); }
}
