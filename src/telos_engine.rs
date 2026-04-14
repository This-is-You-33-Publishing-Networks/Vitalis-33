//! Telos Engine — Vitalis v1301
//!
//! Purpose-driven compilation understanding teleological goals.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<TelosEngineState>> = LazyLock::new(|| Mutex::new(TelosEngineState::default()));

#[derive(Default)]
struct TelosEngineState {
    goals: Vec<(String, f64)>,
    achieved: Vec<String>,
    purpose_clarity: f64,
}

pub struct TelosEngine;

impl TelosEngine {
    pub fn set_goal(name: &str, priority: f64) {
        let mut s = STATE.lock().unwrap();
        s.goals.push((name.to_string(), priority));
        s.purpose_clarity = s.goals.iter().map(|g| g.1).sum::<f64>() / s.goals.len() as f64;
    }
    pub fn achieve(name: &str) {
        let mut s = STATE.lock().unwrap();
        s.achieved.push(name.to_string());
    }
    pub fn goal_count() -> usize { STATE.lock().unwrap().goals.len() }
    pub fn achieved_count() -> usize { STATE.lock().unwrap().achieved.len() }
    pub fn purpose_clarity() -> f64 { STATE.lock().unwrap().purpose_clarity }
    pub fn reset() { *STATE.lock().unwrap() = TelosEngineState::default(); }
}

#[unsafe(no_mangle)]
pub extern "C" fn tel_goal_count() -> usize { TelosEngine::goal_count() }
#[unsafe(no_mangle)]
pub extern "C" fn tel_achieved_count() -> usize { TelosEngine::achieved_count() }
#[unsafe(no_mangle)]
pub extern "C" fn tel_purpose_clarity() -> f64 { TelosEngine::purpose_clarity() }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_set_goal() { TelosEngine::reset(); TelosEngine::set_goal("performance", 0.9); assert_eq!(TelosEngine::goal_count(), 1); }
    #[test] fn test_achieve() { TelosEngine::reset(); TelosEngine::achieve("safety"); assert_eq!(TelosEngine::achieved_count(), 1); }
    #[test] fn test_purpose_clarity() { TelosEngine::reset(); TelosEngine::set_goal("a", 0.8); assert!((TelosEngine::purpose_clarity() - 0.8).abs() < 1e-9); }
    #[test] fn test_multiple_goals() { TelosEngine::reset(); TelosEngine::set_goal("a", 0.5); TelosEngine::set_goal("b", 1.0); assert!((TelosEngine::purpose_clarity() - 0.75).abs() < 1e-9); }
    #[test] fn test_achieve_multiple() { TelosEngine::reset(); TelosEngine::achieve("x"); TelosEngine::achieve("y"); assert_eq!(TelosEngine::achieved_count(), 2); }
    #[test] fn test_initial_clarity() { TelosEngine::reset(); assert!((TelosEngine::purpose_clarity() - 0.0).abs() < 1e-9); }
    #[test] fn test_reset() { TelosEngine::reset(); TelosEngine::set_goal("g", 1.0); TelosEngine::reset(); assert_eq!(TelosEngine::goal_count(), 0); }
}
