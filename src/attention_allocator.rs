//! Attention Allocator — Vitalis v1023
//!
//! Dynamic resource allocation based on which parts of compilation
//! need the most focus. Prioritizes hot spots and complex regions.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<AaState>> = LazyLock::new(|| Mutex::new(AaState::default()));

#[derive(Default)]
struct AaState {
    focus_areas: Vec<(String, f64)>,
    budget: f64,
    allocated: f64,
}

pub struct AttentionAllocator;

impl AttentionAllocator {
    pub fn set_budget(budget: f64) {
        let mut s = STATE.lock().unwrap();
        s.budget = budget;
    }

    pub fn request_focus(area: &str, priority: f64) -> f64 {
        let mut s = STATE.lock().unwrap();
        let remaining = (s.budget - s.allocated).max(0.0);
        let grant = (priority * 0.5).min(remaining);
        s.allocated += grant;
        s.focus_areas.push((area.to_string(), grant));
        grant
    }

    pub fn top_focus() -> Option<String> {
        let s = STATE.lock().unwrap();
        s.focus_areas.iter().max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal)).map(|(n, _)| n.clone())
    }

    pub fn utilization() -> f64 {
        let s = STATE.lock().unwrap();
        if s.budget == 0.0 { 0.0 } else { s.allocated / s.budget }
    }

    pub fn focus_count() -> usize { STATE.lock().unwrap().focus_areas.len() }
    pub fn remaining() -> f64 { let s = STATE.lock().unwrap(); (s.budget - s.allocated).max(0.0) }
    pub fn reset() { *STATE.lock().unwrap() = AaState::default(); }
}

#[unsafe(no_mangle)]
pub extern "C" fn aa_budget(b: f64) -> f64 { AttentionAllocator::set_budget(b); b }
#[unsafe(no_mangle)]
pub extern "C" fn aa_focus(priority: f64) -> f64 { AttentionAllocator::request_focus("ffi_area", priority) }
#[unsafe(no_mangle)]
pub extern "C" fn aa_util() -> f64 { AttentionAllocator::utilization() }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_budget() { AttentionAllocator::reset(); AttentionAllocator::set_budget(10.0); assert!((AttentionAllocator::remaining() - 10.0).abs() < 0.01); }
    #[test] fn test_focus() { AttentionAllocator::reset(); AttentionAllocator::set_budget(10.0); let g = AttentionAllocator::request_focus("a", 4.0); assert!(g > 0.0); }
    #[test] fn test_exhaust() { AttentionAllocator::reset(); AttentionAllocator::set_budget(1.0); AttentionAllocator::request_focus("a", 100.0); assert!(AttentionAllocator::remaining() < 1.0); }
    #[test] fn test_top_focus() { AttentionAllocator::reset(); AttentionAllocator::set_budget(10.0); AttentionAllocator::request_focus("a", 1.0); AttentionAllocator::request_focus("b", 5.0); assert_eq!(AttentionAllocator::top_focus(), Some("b".to_string())); }
    #[test] fn test_utilization() { AttentionAllocator::reset(); AttentionAllocator::set_budget(10.0); AttentionAllocator::request_focus("a", 10.0); assert!(AttentionAllocator::utilization() > 0.0); }
    #[test] fn test_focus_count() { AttentionAllocator::reset(); AttentionAllocator::request_focus("a", 1.0); assert_eq!(AttentionAllocator::focus_count(), 1); }
    #[test] fn test_ffi() { AttentionAllocator::reset(); assert!((aa_util() - 0.0).abs() < 0.01); }
}
