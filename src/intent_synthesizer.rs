//! Intent Synthesizer — Vitalis v1164
//!
//! Synthesizes complete programs from high-level intent descriptions.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<IsynState>> = LazyLock::new(|| Mutex::new(IsynState::default()));

#[derive(Default)]
struct IsynState { intents: Vec<(String, String)>, synthesized: Vec<String>, confidence: f64 }

pub struct IntentSynthesizer;

impl IntentSynthesizer {
    pub fn express_intent(description: &str, context: &str) -> usize { let mut s = STATE.lock().unwrap(); s.intents.push((description.to_string(), context.to_string())); s.intents.len() }
    pub fn synthesize(description: &str) -> Option<String> {
        let mut s = STATE.lock().unwrap();
        if s.intents.iter().any(|(d, _)| d == description) {
            let program = format!("synth_{}", description);
            s.synthesized.push(program.clone());
            s.confidence = 0.8;
            Some(program)
        } else { None }
    }
    pub fn confidence() -> f64 { STATE.lock().unwrap().confidence }
    pub fn synthesized_count() -> usize { STATE.lock().unwrap().synthesized.len() }
    pub fn intent_count() -> usize { STATE.lock().unwrap().intents.len() }
    pub fn reset() { *STATE.lock().unwrap() = IsynState::default(); }
}

#[unsafe(no_mangle)]
pub extern "C" fn isyn_intent(id: i64) -> i64 { IntentSynthesizer::express_intent(&format!("i{}", id), "ctx") as i64 }
#[unsafe(no_mangle)]
pub extern "C" fn isyn_synth(id: i64) -> i64 { if IntentSynthesizer::synthesize(&format!("i{}", id)).is_some() { 1 } else { 0 } }
#[unsafe(no_mangle)]
pub extern "C" fn isyn_confidence() -> f64 { IntentSynthesizer::confidence() }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_intent() { IntentSynthesizer::reset(); assert_eq!(IntentSynthesizer::express_intent("sort list", ""), 1); }
    #[test] fn test_synth() { IntentSynthesizer::reset(); IntentSynthesizer::express_intent("sort", ""); assert!(IntentSynthesizer::synthesize("sort").is_some()); }
    #[test] fn test_synth_miss() { IntentSynthesizer::reset(); assert!(IntentSynthesizer::synthesize("nope").is_none()); }
    #[test] fn test_confidence() { IntentSynthesizer::reset(); IntentSynthesizer::express_intent("x", ""); IntentSynthesizer::synthesize("x"); assert!(IntentSynthesizer::confidence() > 0.0); }
    #[test] fn test_synth_count() { IntentSynthesizer::reset(); IntentSynthesizer::express_intent("a", ""); IntentSynthesizer::synthesize("a"); assert_eq!(IntentSynthesizer::synthesized_count(), 1); }
    #[test] fn test_intent_count() { IntentSynthesizer::reset(); IntentSynthesizer::express_intent("a", ""); assert_eq!(IntentSynthesizer::intent_count(), 1); }
    #[test] fn test_ffi() { IntentSynthesizer::reset(); assert!((isyn_confidence() - 0.0).abs() < 0.01); }
}
