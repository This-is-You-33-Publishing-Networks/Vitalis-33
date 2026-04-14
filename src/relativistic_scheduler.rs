//! Relativistic Scheduler — Vitalis v1095
//!
//! Schedules tasks respecting information propagation limits,
//! like speed-of-light constraints for distributed data.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<RsState>> = LazyLock::new(|| Mutex::new(RsState::default()));

#[derive(Default)]
struct RsState {
    tasks: Vec<(String, f64, f64)>,
    scheduled: Vec<(String, f64)>,
    max_propagation: f64,
}

pub struct RelativisticScheduler;

impl RelativisticScheduler {
    pub fn set_max_propagation(limit: f64) { STATE.lock().unwrap().max_propagation = limit; }

    pub fn submit_task(name: &str, location: f64, deadline: f64) -> usize {
        let mut s = STATE.lock().unwrap();
        s.tasks.push((name.to_string(), location, deadline));
        s.tasks.len()
    }

    pub fn schedule() -> usize {
        let mut s = STATE.lock().unwrap();
        let limit = if s.max_propagation > 0.0 { s.max_propagation } else { f64::INFINITY };
        let mut result = vec![];
        let mut current_loc = 0.0_f64;
        for (name, loc, deadline) in s.tasks.iter() {
            let travel = (loc - current_loc).abs();
            if travel <= limit && *deadline > travel {
                result.push((name.clone(), *deadline));
                current_loc = *loc;
            }
        }
        s.scheduled = result;
        s.scheduled.len()
    }

    pub fn scheduled_count() -> usize { STATE.lock().unwrap().scheduled.len() }
    pub fn task_count() -> usize { STATE.lock().unwrap().tasks.len() }
    pub fn reset() { *STATE.lock().unwrap() = RsState::default(); }
}

#[unsafe(no_mangle)]
pub extern "C" fn rs_submit(loc: f64, deadline: f64) -> i64 { RelativisticScheduler::submit_task("ffi", loc, deadline) as i64 }
#[unsafe(no_mangle)]
pub extern "C" fn rs_schedule() -> i64 { RelativisticScheduler::schedule() as i64 }
#[unsafe(no_mangle)]
pub extern "C" fn rs_scheduled() -> i64 { RelativisticScheduler::scheduled_count() as i64 }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_submit() { RelativisticScheduler::reset(); assert_eq!(RelativisticScheduler::submit_task("a", 0.0, 10.0), 1); }
    #[test] fn test_schedule() { RelativisticScheduler::reset(); RelativisticScheduler::submit_task("a", 0.0, 10.0); let n = RelativisticScheduler::schedule(); assert!(n > 0); }
    #[test] fn test_propagation_limit() { RelativisticScheduler::reset(); RelativisticScheduler::set_max_propagation(1.0); RelativisticScheduler::submit_task("a", 0.0, 10.0); RelativisticScheduler::submit_task("b", 100.0, 10.0); let n = RelativisticScheduler::schedule(); assert_eq!(n, 1); }
    #[test] fn test_task_count() { RelativisticScheduler::reset(); RelativisticScheduler::submit_task("a", 0.0, 1.0); assert_eq!(RelativisticScheduler::task_count(), 1); }
    #[test] fn test_scheduled_count() { RelativisticScheduler::reset(); RelativisticScheduler::submit_task("a", 0.0, 10.0); RelativisticScheduler::schedule(); assert_eq!(RelativisticScheduler::scheduled_count(), 1); }
    #[test] fn test_empty_schedule() { RelativisticScheduler::reset(); assert_eq!(RelativisticScheduler::schedule(), 0); }
    #[test] fn test_ffi() { RelativisticScheduler::reset(); assert_eq!(rs_scheduled(), 0); }
}
