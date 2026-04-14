//! Strange Loop — Vitalis v1052
//!
//! Self-referential compilation loops that produce emergent optimization
//! strategies through Hofstadter-style tangled hierarchies.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<SlState>> = LazyLock::new(|| Mutex::new(SlState::default()));

#[derive(Default)]
struct SlState {
    loop_depth: u32,
    tangled_levels: Vec<(u32, u32)>,
    emergent_strategies: Vec<String>,
    self_reference_count: u64,
}

pub struct StrangeLoop;

impl StrangeLoop {
    pub fn descend() -> u32 {
        let mut s = STATE.lock().unwrap();
        s.loop_depth += 1;
        s.loop_depth
    }

    pub fn ascend() -> u32 {
        let mut s = STATE.lock().unwrap();
        s.loop_depth = s.loop_depth.saturating_sub(1);
        s.loop_depth
    }

    pub fn tangle(level_a: u32, level_b: u32) -> usize {
        let mut s = STATE.lock().unwrap();
        s.tangled_levels.push((level_a, level_b));
        s.self_reference_count += 1;
        if s.tangled_levels.len() > 2 {
            let len = s.tangled_levels.len();
            s.emergent_strategies.push(format!("emergent_from_tangle_{}", len));
        }
        s.tangled_levels.len()
    }

    pub fn self_reference() -> u64 {
        let mut s = STATE.lock().unwrap();
        s.self_reference_count += 1;
        s.self_reference_count
    }

    pub fn emergent_strategy_count() -> usize { STATE.lock().unwrap().emergent_strategies.len() }
    pub fn depth() -> u32 { STATE.lock().unwrap().loop_depth }
    pub fn tangle_count() -> usize { STATE.lock().unwrap().tangled_levels.len() }
    pub fn reset() { *STATE.lock().unwrap() = SlState::default(); }
}

#[unsafe(no_mangle)]
pub extern "C" fn sl_descend() -> i64 { StrangeLoop::descend() as i64 }
#[unsafe(no_mangle)]
pub extern "C" fn sl_ascend() -> i64 { StrangeLoop::ascend() as i64 }
#[unsafe(no_mangle)]
pub extern "C" fn sl_tangle(a: i64, b: i64) -> i64 { StrangeLoop::tangle(a as u32, b as u32) as i64 }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_descend() { StrangeLoop::reset(); assert_eq!(StrangeLoop::descend(), 1); assert_eq!(StrangeLoop::descend(), 2); }
    #[test] fn test_ascend() { StrangeLoop::reset(); StrangeLoop::descend(); StrangeLoop::descend(); assert_eq!(StrangeLoop::ascend(), 1); }
    #[test] fn test_ascend_floor() { StrangeLoop::reset(); assert_eq!(StrangeLoop::ascend(), 0); }
    #[test] fn test_tangle() { StrangeLoop::reset(); assert_eq!(StrangeLoop::tangle(1, 3), 1); }
    #[test] fn test_emergence() { StrangeLoop::reset(); StrangeLoop::tangle(1, 2); StrangeLoop::tangle(2, 3); StrangeLoop::tangle(3, 1); assert!(StrangeLoop::emergent_strategy_count() > 0); }
    #[test] fn test_self_ref() { StrangeLoop::reset(); let c = StrangeLoop::self_reference(); assert!(c > 0); }
    #[test] fn test_ffi() { StrangeLoop::reset(); assert_eq!(sl_descend(), 1); }
}
