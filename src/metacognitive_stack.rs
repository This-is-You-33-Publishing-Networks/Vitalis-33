//! Metacognitive Stack — Vitalis v1021
//!
//! Layered self-awareness: monitors reasoning about reasoning about code.
//! Each layer observes and can modify the layer below it.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<McsState>> = LazyLock::new(|| Mutex::new(McsState::default()));

#[derive(Default)]
struct McsState {
    layers: Vec<(String, u32)>,
    observations: Vec<(u32, String)>,
    depth: u32,
    clarity: f64,
}

pub struct MetacognitiveStack;

impl MetacognitiveStack {
    pub fn push_layer(name: &str) -> u32 {
        let mut s = STATE.lock().unwrap();
        s.depth += 1;
        let depth = s.depth;
        s.layers.push((name.to_string(), depth));
        s.clarity = 1.0 / s.depth as f64;
        s.depth
    }

    pub fn observe(layer: u32, note: &str) -> usize {
        let mut s = STATE.lock().unwrap();
        s.observations.push((layer, note.to_string()));
        s.observations.len()
    }

    pub fn pop_layer() -> Option<String> {
        let mut s = STATE.lock().unwrap();
        if let Some((name, _)) = s.layers.pop() {
            s.depth = s.depth.saturating_sub(1);
            s.clarity = if s.depth == 0 { 1.0 } else { 1.0 / s.depth as f64 };
            Some(name)
        } else { None }
    }

    pub fn clarity() -> f64 { STATE.lock().unwrap().clarity }
    pub fn depth() -> u32 { STATE.lock().unwrap().depth }
    pub fn observation_count() -> usize { STATE.lock().unwrap().observations.len() }
    pub fn reset() { *STATE.lock().unwrap() = McsState::default(); }
}

#[unsafe(no_mangle)]
pub extern "C" fn mcs_push(id: i64) -> i64 { MetacognitiveStack::push_layer(&format!("layer_{}", id)) as i64 }
#[unsafe(no_mangle)]
pub extern "C" fn mcs_pop() -> i64 { if MetacognitiveStack::pop_layer().is_some() { 1 } else { 0 } }
#[unsafe(no_mangle)]
pub extern "C" fn mcs_clarity() -> f64 { MetacognitiveStack::clarity() }
#[unsafe(no_mangle)]
pub extern "C" fn mcs_depth() -> i64 { MetacognitiveStack::depth() as i64 }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_push() { MetacognitiveStack::reset(); assert_eq!(MetacognitiveStack::push_layer("base"), 1); }
    #[test] fn test_pop() { MetacognitiveStack::reset(); MetacognitiveStack::push_layer("a"); assert_eq!(MetacognitiveStack::pop_layer(), Some("a".to_string())); }
    #[test] fn test_pop_empty() { MetacognitiveStack::reset(); assert!(MetacognitiveStack::pop_layer().is_none()); }
    #[test] fn test_observe() { MetacognitiveStack::reset(); MetacognitiveStack::push_layer("a"); assert_eq!(MetacognitiveStack::observe(1, "noted"), 1); }
    #[test] fn test_clarity_decreases() { MetacognitiveStack::reset(); MetacognitiveStack::push_layer("a"); let c1 = MetacognitiveStack::clarity(); MetacognitiveStack::push_layer("b"); assert!(MetacognitiveStack::clarity() < c1); }
    #[test] fn test_depth() { MetacognitiveStack::reset(); MetacognitiveStack::push_layer("a"); MetacognitiveStack::push_layer("b"); assert_eq!(MetacognitiveStack::depth(), 2); }
    #[test] fn test_observation_count() { MetacognitiveStack::reset(); MetacognitiveStack::observe(0, "x"); assert_eq!(MetacognitiveStack::observation_count(), 1); }
    #[test] fn test_ffi() { MetacognitiveStack::reset(); assert_eq!(mcs_depth(), 0); }
}
