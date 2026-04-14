//! Insight Propagator — Vitalis v1148
//!
//! Propagates insights from one optimization domain to all analogous domains.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<IpState>> = LazyLock::new(|| Mutex::new(IpState::default()));

#[derive(Default)]
struct IpState { insights: Vec<(String, String, f64)>, propagations: u64, domains: Vec<String> }

pub struct InsightPropagator;

impl InsightPropagator {
    pub fn register_domain(name: &str) -> usize { let mut s = STATE.lock().unwrap(); s.domains.push(name.to_string()); s.domains.len() }
    pub fn discover_insight(domain: &str, insight: &str, strength: f64) -> usize { let mut s = STATE.lock().unwrap(); s.insights.push((domain.to_string(), insight.to_string(), strength)); s.insights.len() }
    pub fn propagate(insight: &str) -> usize {
        let mut s = STATE.lock().unwrap();
        let domains = s.domains.clone();
        let propagated = domains.len();
        s.propagations += propagated as u64;
        propagated
    }
    pub fn insight_count() -> usize { STATE.lock().unwrap().insights.len() }
    pub fn propagations() -> u64 { STATE.lock().unwrap().propagations }
    pub fn domain_count() -> usize { STATE.lock().unwrap().domains.len() }
    pub fn reset() { *STATE.lock().unwrap() = IpState::default(); }
}

#[unsafe(no_mangle)]
pub extern "C" fn ip_domain(id: i64) -> i64 { InsightPropagator::register_domain(&format!("d{}", id)) as i64 }
#[unsafe(no_mangle)]
pub extern "C" fn ip_propagate() -> i64 { InsightPropagator::propagate("ffi") as i64 }
#[unsafe(no_mangle)]
pub extern "C" fn ip_insights() -> i64 { InsightPropagator::insight_count() as i64 }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_domain() { InsightPropagator::reset(); assert_eq!(InsightPropagator::register_domain("ml"), 1); }
    #[test] fn test_insight() { InsightPropagator::reset(); assert_eq!(InsightPropagator::discover_insight("ml", "batch", 0.9), 1); }
    #[test] fn test_propagate() { InsightPropagator::reset(); InsightPropagator::register_domain("a"); InsightPropagator::register_domain("b"); let n = InsightPropagator::propagate("x"); assert_eq!(n, 2); }
    #[test] fn test_propagation_count() { InsightPropagator::reset(); InsightPropagator::register_domain("a"); InsightPropagator::propagate("x"); assert!(InsightPropagator::propagations() > 0); }
    #[test] fn test_domain_count() { InsightPropagator::reset(); InsightPropagator::register_domain("a"); assert_eq!(InsightPropagator::domain_count(), 1); }
    #[test] fn test_ffi() { InsightPropagator::reset(); assert_eq!(ip_insights(), 0); }
}
