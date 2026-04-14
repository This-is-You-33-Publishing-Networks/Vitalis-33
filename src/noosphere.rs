//! Noosphere — Vitalis v1039
//!
//! Global knowledge sphere aggregating all compiler intelligence
//! into a unified field of understanding spanning all domains.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<NooState>> = LazyLock::new(|| Mutex::new(NooState::default()));

#[derive(Default)]
struct NooState {
    domains: Vec<(String, f64)>,
    connections: Vec<(String, String, f64)>,
    coherence: f64,
    thought_count: u64,
}

pub struct Noosphere;

impl Noosphere {
    pub fn add_domain(name: &str, knowledge: f64) -> usize {
        let mut s = STATE.lock().unwrap();
        s.domains.push((name.to_string(), knowledge));
        s.thought_count += 1;
        s.domains.len()
    }

    pub fn connect_domains(a: &str, b: &str, strength: f64) -> usize {
        let mut s = STATE.lock().unwrap();
        s.connections.push((a.to_string(), b.to_string(), strength));
        let total_strength: f64 = s.connections.iter().map(|(_, _, st)| st).sum();
        let max_possible = s.connections.len() as f64;
        s.coherence = if max_possible > 0.0 { total_strength / max_possible } else { 0.0 };
        s.connections.len()
    }

    pub fn query(domain: &str) -> f64 {
        let mut s = STATE.lock().unwrap();
        s.thought_count += 1;
        s.domains.iter().find(|(d, _)| d == domain).map(|(_, k)| *k).unwrap_or(0.0)
    }

    pub fn coherence() -> f64 { STATE.lock().unwrap().coherence }
    pub fn domain_count() -> usize { STATE.lock().unwrap().domains.len() }
    pub fn thought_count() -> u64 { STATE.lock().unwrap().thought_count }
    pub fn connection_count() -> usize { STATE.lock().unwrap().connections.len() }
    pub fn reset() { *STATE.lock().unwrap() = NooState::default(); }
}

#[unsafe(no_mangle)]
pub extern "C" fn noo_add(knowledge: f64) -> i64 { Noosphere::add_domain("ffi_domain", knowledge) as i64 }
#[unsafe(no_mangle)]
pub extern "C" fn noo_connect(strength: f64) -> i64 { Noosphere::connect_domains("a", "b", strength) as i64 }
#[unsafe(no_mangle)]
pub extern "C" fn noo_coherence() -> f64 { Noosphere::coherence() }
#[unsafe(no_mangle)]
pub extern "C" fn noo_thoughts() -> i64 { Noosphere::thought_count() as i64 }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_add_domain() { Noosphere::reset(); assert_eq!(Noosphere::add_domain("math", 0.9), 1); }
    #[test] fn test_connect() { Noosphere::reset(); Noosphere::add_domain("a", 0.5); Noosphere::add_domain("b", 0.8); assert_eq!(Noosphere::connect_domains("a", "b", 0.7), 1); }
    #[test] fn test_query() { Noosphere::reset(); Noosphere::add_domain("ml", 0.95); assert!((Noosphere::query("ml") - 0.95).abs() < 0.01); }
    #[test] fn test_query_miss() { Noosphere::reset(); assert!((Noosphere::query("none") - 0.0).abs() < 0.01); }
    #[test] fn test_coherence() { Noosphere::reset(); Noosphere::connect_domains("a", "b", 0.8); assert!(Noosphere::coherence() > 0.0); }
    #[test] fn test_thoughts() { Noosphere::reset(); Noosphere::add_domain("x", 0.5); Noosphere::query("x"); assert_eq!(Noosphere::thought_count(), 2); }
    #[test] fn test_ffi() { Noosphere::reset(); assert_eq!(noo_thoughts(), 0); }
}
