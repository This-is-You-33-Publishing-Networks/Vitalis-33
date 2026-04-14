//! Supertask Executor — Vitalis v1229
//!
//! Infinitely many steps in bounded time via acceleration.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<STEState>> = LazyLock::new(|| Mutex::new(STEState::default()));

#[derive(Default)]
struct STEState {
    steps_completed: u64,
    acceleration_factor: f64,
    bounded_time: f64,
}

pub struct SupertaskExecutor;

impl SupertaskExecutor {
    pub fn execute_step() -> f64 {
        let mut s = STATE.lock().unwrap();
        s.steps_completed += 1;
        if s.acceleration_factor == 0.0 { s.acceleration_factor = 1.0; }
        let step_time = 1.0 / (s.acceleration_factor * s.steps_completed as f64);
        s.bounded_time += step_time;
        s.acceleration_factor *= 2.0;
        s.bounded_time
    }

    pub fn steps() -> u64 { STATE.lock().unwrap().steps_completed }

    pub fn acceleration() -> f64 { STATE.lock().unwrap().acceleration_factor }

    pub fn elapsed_time() -> f64 { STATE.lock().unwrap().bounded_time }

    pub fn converges() -> bool { STATE.lock().unwrap().bounded_time < 10.0 }

    pub fn reset() { *STATE.lock().unwrap() = STEState::default(); }
}

#[unsafe(no_mangle)]
pub extern "C" fn ste_step() -> f64 { SupertaskExecutor::execute_step() }
#[unsafe(no_mangle)]
pub extern "C" fn ste_steps() -> i64 { SupertaskExecutor::steps() as i64 }
#[unsafe(no_mangle)]
pub extern "C" fn ste_time() -> f64 { SupertaskExecutor::elapsed_time() }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_step() { SupertaskExecutor::reset(); let t = SupertaskExecutor::execute_step(); assert!(t > 0.0); }
    #[test] fn test_steps() { SupertaskExecutor::reset(); SupertaskExecutor::execute_step(); assert_eq!(SupertaskExecutor::steps(), 1); }
    #[test] fn test_acceleration() { SupertaskExecutor::reset(); SupertaskExecutor::execute_step(); SupertaskExecutor::execute_step(); assert!(SupertaskExecutor::acceleration() > 1.0); }
    #[test] fn test_bounded_time() { SupertaskExecutor::reset(); for _ in 0..20 { SupertaskExecutor::execute_step(); } assert!(SupertaskExecutor::converges()); }
    #[test] fn test_elapsed() { SupertaskExecutor::reset(); SupertaskExecutor::execute_step(); assert!(SupertaskExecutor::elapsed_time() > 0.0); }
    #[test] fn test_reset() { SupertaskExecutor::reset(); SupertaskExecutor::execute_step(); SupertaskExecutor::reset(); assert_eq!(SupertaskExecutor::steps(), 0); }
    #[test] fn test_ffi() { SupertaskExecutor::reset(); ste_step(); assert_eq!(ste_steps(), 1); }
}
