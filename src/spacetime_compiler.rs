//! Spacetime Compiler — Vitalis v1093
//!
//! Treats computation as events in a spacetime manifold with
//! light-cone causality constraining information flow.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<StcState>> = LazyLock::new(|| Mutex::new(StcState::default()));

#[derive(Default)]
struct StcState {
    events: Vec<(f64, f64, String)>,
    causal_links: Vec<(usize, usize)>,
    light_speed: f64,
}

pub struct SpacetimeCompiler;

impl SpacetimeCompiler {
    pub fn set_light_speed(c: f64) { STATE.lock().unwrap().light_speed = c; }

    pub fn emit_event(time: f64, space: f64, name: &str) -> usize {
        let mut s = STATE.lock().unwrap();
        s.events.push((time, space, name.to_string()));
        s.events.len() - 1
    }

    pub fn can_cause(a: usize, b: usize) -> bool {
        let s = STATE.lock().unwrap();
        if a >= s.events.len() || b >= s.events.len() { return false; }
        let c = if s.light_speed > 0.0 { s.light_speed } else { 1.0 };
        let dt = s.events[b].0 - s.events[a].0;
        let dx = (s.events[b].1 - s.events[a].1).abs();
        dt > 0.0 && dx / dt <= c
    }

    pub fn link_causal(a: usize, b: usize) -> bool {
        let mut s = STATE.lock().unwrap();
        if a >= s.events.len() || b >= s.events.len() { return false; }
        s.causal_links.push((a, b));
        true
    }

    pub fn event_count() -> usize { STATE.lock().unwrap().events.len() }
    pub fn link_count() -> usize { STATE.lock().unwrap().causal_links.len() }
    pub fn reset() { *STATE.lock().unwrap() = StcState::default(); }
}

#[unsafe(no_mangle)]
pub extern "C" fn stc_emit(time: f64, space: f64) -> i64 { SpacetimeCompiler::emit_event(time, space, "ffi") as i64 }
#[unsafe(no_mangle)]
pub extern "C" fn stc_can_cause(a: i64, b: i64) -> i64 { if SpacetimeCompiler::can_cause(a as usize, b as usize) { 1 } else { 0 } }
#[unsafe(no_mangle)]
pub extern "C" fn stc_events() -> i64 { SpacetimeCompiler::event_count() as i64 }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_emit() { SpacetimeCompiler::reset(); let i = SpacetimeCompiler::emit_event(0.0, 0.0, "start"); assert_eq!(i, 0); }
    #[test] fn test_causal() { SpacetimeCompiler::reset(); SpacetimeCompiler::set_light_speed(1.0); SpacetimeCompiler::emit_event(0.0, 0.0, "a"); SpacetimeCompiler::emit_event(2.0, 1.0, "b"); assert!(SpacetimeCompiler::can_cause(0, 1)); }
    #[test] fn test_spacelike() { SpacetimeCompiler::reset(); SpacetimeCompiler::set_light_speed(1.0); SpacetimeCompiler::emit_event(0.0, 0.0, "a"); SpacetimeCompiler::emit_event(0.1, 100.0, "b"); assert!(!SpacetimeCompiler::can_cause(0, 1)); }
    #[test] fn test_link() { SpacetimeCompiler::reset(); SpacetimeCompiler::emit_event(0.0, 0.0, "a"); SpacetimeCompiler::emit_event(1.0, 0.0, "b"); assert!(SpacetimeCompiler::link_causal(0, 1)); }
    #[test] fn test_event_count() { SpacetimeCompiler::reset(); SpacetimeCompiler::emit_event(0.0, 0.0, "a"); assert_eq!(SpacetimeCompiler::event_count(), 1); }
    #[test] fn test_backward() { SpacetimeCompiler::reset(); SpacetimeCompiler::set_light_speed(1.0); SpacetimeCompiler::emit_event(5.0, 0.0, "a"); SpacetimeCompiler::emit_event(1.0, 0.0, "b"); assert!(!SpacetimeCompiler::can_cause(0, 1)); }
    #[test] fn test_link_count() { SpacetimeCompiler::reset(); SpacetimeCompiler::emit_event(0.0, 0.0, "a"); SpacetimeCompiler::emit_event(1.0, 0.0, "b"); SpacetimeCompiler::link_causal(0, 1); assert_eq!(SpacetimeCompiler::link_count(), 1); }
    #[test] fn test_ffi() { SpacetimeCompiler::reset(); assert_eq!(stc_events(), 0); }
}
