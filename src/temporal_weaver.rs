//! Temporal Weaver — Vitalis v1015
//!
//! Multi-timeline execution model where the compiler explores
//! branching futures simultaneously, weaving optimal paths together.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<TwState>> = LazyLock::new(|| Mutex::new(TwState::default()));

#[derive(Default)]
struct TwState {
    timelines: Vec<(String, Vec<f64>)>,
    active_branches: u32,
    merges: u64,
    best_outcome: f64,
}

pub struct TemporalWeaver;

impl TemporalWeaver {
    pub fn branch(label: &str) -> u32 {
        let mut s = STATE.lock().unwrap();
        s.timelines.push((label.to_string(), vec![]));
        s.active_branches += 1;
        s.active_branches
    }

    pub fn evolve_timeline(label: &str, fitness: f64) -> bool {
        let mut s = STATE.lock().unwrap();
        if let Some(t) = s.timelines.iter_mut().find(|(l, _)| l == label) {
            t.1.push(fitness);
            if fitness > s.best_outcome { s.best_outcome = fitness; }
            true
        } else { false }
    }

    pub fn merge_best() -> f64 {
        let mut s = STATE.lock().unwrap();
        s.merges += 1;
        s.active_branches = s.active_branches.saturating_sub(1).max(1);
        s.best_outcome
    }

    pub fn best_outcome() -> f64 { STATE.lock().unwrap().best_outcome }
    pub fn active_branches() -> u32 { STATE.lock().unwrap().active_branches }
    pub fn merge_count() -> u64 { STATE.lock().unwrap().merges }
    pub fn reset() { *STATE.lock().unwrap() = TwState::default(); }
}

#[unsafe(no_mangle)]
pub extern "C" fn tw_branch(id: i64) -> i64 { TemporalWeaver::branch(&format!("t{}", id)) as i64 }
#[unsafe(no_mangle)]
pub extern "C" fn tw_evolve(fitness: f64) -> i64 { if TemporalWeaver::evolve_timeline("t0", fitness) { 1 } else { 0 } }
#[unsafe(no_mangle)]
pub extern "C" fn tw_merge() -> f64 { TemporalWeaver::merge_best() }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_branch() { TemporalWeaver::reset(); assert_eq!(TemporalWeaver::branch("a"), 1); assert_eq!(TemporalWeaver::branch("b"), 2); }
    #[test] fn test_evolve() { TemporalWeaver::reset(); TemporalWeaver::branch("a"); assert!(TemporalWeaver::evolve_timeline("a", 0.5)); }
    #[test] fn test_evolve_nonexistent() { TemporalWeaver::reset(); assert!(!TemporalWeaver::evolve_timeline("nope", 0.5)); }
    #[test] fn test_merge() { TemporalWeaver::reset(); TemporalWeaver::branch("a"); TemporalWeaver::evolve_timeline("a", 0.9); let best = TemporalWeaver::merge_best(); assert!((best - 0.9).abs() < 0.01); }
    #[test] fn test_best_outcome() { TemporalWeaver::reset(); TemporalWeaver::branch("a"); TemporalWeaver::evolve_timeline("a", 0.3); TemporalWeaver::evolve_timeline("a", 0.7); assert!((TemporalWeaver::best_outcome() - 0.7).abs() < 0.01); }
    #[test] fn test_active_branches() { TemporalWeaver::reset(); TemporalWeaver::branch("a"); TemporalWeaver::branch("b"); assert_eq!(TemporalWeaver::active_branches(), 2); }
    #[test] fn test_ffi() { TemporalWeaver::reset(); assert_eq!(tw_branch(1), 1); }
}
