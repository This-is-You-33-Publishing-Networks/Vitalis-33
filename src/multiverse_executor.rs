//! Multiverse Executor — Vitalis v1103
//!
//! Executes programs across multiple "universes" (execution contexts)
//! simultaneously, selecting the best outcome.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<MveState>> = LazyLock::new(|| Mutex::new(MveState::default()));

#[derive(Default)]
struct MveState {
    universes: Vec<(String, f64, bool)>,
    selected: Option<String>,
}

pub struct MultiverseExecutor;

impl MultiverseExecutor {
    pub fn spawn_universe(name: &str) -> usize {
        let mut s = STATE.lock().unwrap();
        s.universes.push((name.to_string(), 0.0, true));
        s.universes.len()
    }

    pub fn execute_in(name: &str, result: f64) -> bool {
        let mut s = STATE.lock().unwrap();
        if let Some(u) = s.universes.iter_mut().find(|(n, _, a)| n == name && *a) { u.1 = result; true } else { false }
    }

    pub fn collapse_to_best() -> Option<String> {
        let mut s = STATE.lock().unwrap();
        let best = s.universes.iter().filter(|(_, _, a)| *a).max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        if let Some((name, _, _)) = best {
            let name = name.clone();
            s.selected = Some(name.clone());
            for u in s.universes.iter_mut() { u.2 = u.0 == name; }
            Some(name)
        } else { None }
    }

    pub fn prune(threshold: f64) -> usize {
        let mut s = STATE.lock().unwrap();
        let before = s.universes.len();
        s.universes.retain(|(_, r, _)| *r >= threshold);
        before - s.universes.len()
    }

    pub fn universe_count() -> usize { STATE.lock().unwrap().universes.len() }
    pub fn active_count() -> usize { STATE.lock().unwrap().universes.iter().filter(|(_, _, a)| *a).count() }
    pub fn selected() -> Option<String> { STATE.lock().unwrap().selected.clone() }
    pub fn reset() { *STATE.lock().unwrap() = MveState::default(); }
}

#[unsafe(no_mangle)]
pub extern "C" fn mve_spawn(id: i64) -> i64 { MultiverseExecutor::spawn_universe(&format!("u{}", id)) as i64 }
#[unsafe(no_mangle)]
pub extern "C" fn mve_execute(id: i64, result: f64) -> i64 { if MultiverseExecutor::execute_in(&format!("u{}", id), result) { 1 } else { 0 } }
#[unsafe(no_mangle)]
pub extern "C" fn mve_collapse() -> i64 { if MultiverseExecutor::collapse_to_best().is_some() { 1 } else { 0 } }
#[unsafe(no_mangle)]
pub extern "C" fn mve_count() -> i64 { MultiverseExecutor::universe_count() as i64 }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_spawn() { MultiverseExecutor::reset(); assert_eq!(MultiverseExecutor::spawn_universe("u1"), 1); }
    #[test] fn test_execute() { MultiverseExecutor::reset(); MultiverseExecutor::spawn_universe("u1"); assert!(MultiverseExecutor::execute_in("u1", 0.9)); }
    #[test] fn test_collapse() { MultiverseExecutor::reset(); MultiverseExecutor::spawn_universe("a"); MultiverseExecutor::spawn_universe("b"); MultiverseExecutor::execute_in("a", 0.3); MultiverseExecutor::execute_in("b", 0.9); assert_eq!(MultiverseExecutor::collapse_to_best(), Some("b".to_string())); }
    #[test] fn test_prune() { MultiverseExecutor::reset(); MultiverseExecutor::spawn_universe("a"); MultiverseExecutor::execute_in("a", 0.1); assert_eq!(MultiverseExecutor::prune(0.5), 1); }
    #[test] fn test_active() { MultiverseExecutor::reset(); MultiverseExecutor::spawn_universe("a"); MultiverseExecutor::spawn_universe("b"); MultiverseExecutor::execute_in("a", 0.9); MultiverseExecutor::execute_in("b", 0.1); MultiverseExecutor::collapse_to_best(); assert_eq!(MultiverseExecutor::active_count(), 1); }
    #[test] fn test_selected() { MultiverseExecutor::reset(); MultiverseExecutor::spawn_universe("x"); MultiverseExecutor::execute_in("x", 1.0); MultiverseExecutor::collapse_to_best(); assert_eq!(MultiverseExecutor::selected(), Some("x".to_string())); }
    #[test] fn test_empty_collapse() { MultiverseExecutor::reset(); assert!(MultiverseExecutor::collapse_to_best().is_none()); }
    #[test] fn test_ffi() { MultiverseExecutor::reset(); assert_eq!(mve_count(), 0); }
}
