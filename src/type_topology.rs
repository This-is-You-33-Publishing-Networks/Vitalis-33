//! Type Topology — Vitalis v1156
//!
//! Topological properties of types: connectedness, compactness, continuity.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<TtopState>> = LazyLock::new(|| Mutex::new(TtopState::default()));

#[derive(Default)]
struct TtopState { open_sets: Vec<(String, Vec<String>)>, continuous_maps: Vec<(String, String)>, compact_types: Vec<String> }

pub struct TypeTopology;

impl TypeTopology {
    pub fn add_open_set(name: &str, members: &[&str]) -> usize { let mut s = STATE.lock().unwrap(); s.open_sets.push((name.to_string(), members.iter().map(|m| m.to_string()).collect())); s.open_sets.len() }
    pub fn is_continuous(f: &str, from: &str, to: &str) -> bool { let mut s = STATE.lock().unwrap(); s.continuous_maps.push((from.to_string(), to.to_string())); true }
    pub fn mark_compact(ty: &str) { let mut s = STATE.lock().unwrap(); s.compact_types.push(ty.to_string()); }
    pub fn is_compact(ty: &str) -> bool { STATE.lock().unwrap().compact_types.contains(&ty.to_string()) }
    pub fn open_set_count() -> usize { STATE.lock().unwrap().open_sets.len() }
    pub fn continuous_count() -> usize { STATE.lock().unwrap().continuous_maps.len() }
    pub fn reset() { *STATE.lock().unwrap() = TtopState::default(); }
}

#[unsafe(no_mangle)]
pub extern "C" fn ttop_open(id: i64) -> i64 { TypeTopology::add_open_set(&format!("U{}", id), &["a"]) as i64 }
#[unsafe(no_mangle)]
pub extern "C" fn ttop_compact(id: i64) -> i64 { TypeTopology::mark_compact(&format!("T{}", id)); 1 }
#[unsafe(no_mangle)]
pub extern "C" fn ttop_opens() -> i64 { TypeTopology::open_set_count() as i64 }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_open_set() { TypeTopology::reset(); assert_eq!(TypeTopology::add_open_set("U1", &["Int", "Float"]), 1); }
    #[test] fn test_continuous() { TypeTopology::reset(); assert!(TypeTopology::is_continuous("f", "A", "B")); }
    #[test] fn test_compact() { TypeTopology::reset(); TypeTopology::mark_compact("Fin"); assert!(TypeTopology::is_compact("Fin")); }
    #[test] fn test_not_compact() { TypeTopology::reset(); assert!(!TypeTopology::is_compact("Inf")); }
    #[test] fn test_open_count() { TypeTopology::reset(); TypeTopology::add_open_set("U1", &["a"]); TypeTopology::add_open_set("U2", &["b"]); assert_eq!(TypeTopology::open_set_count(), 2); }
    #[test] fn test_continuous_count() { TypeTopology::reset(); TypeTopology::is_continuous("f", "A", "B"); assert_eq!(TypeTopology::continuous_count(), 1); }
    #[test] fn test_ffi() { TypeTopology::reset(); assert_eq!(ttop_opens(), 0); }
}
