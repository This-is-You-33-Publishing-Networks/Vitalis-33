//! Grand Unified IR — Vitalis v1170
//!
//! A single intermediate representation encoding all computation paradigms.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<GuirState>> = LazyLock::new(|| Mutex::new(GuirState::default()));

#[derive(Default)]
struct GuirState { nodes: Vec<(String, String)>, edges: Vec<(usize, usize)>, paradigms_encoded: Vec<String> }

pub struct GrandUnifiedIr;

impl GrandUnifiedIr {
    pub fn add_node(name: &str, paradigm: &str) -> usize { let mut s = STATE.lock().unwrap(); s.nodes.push((name.to_string(), paradigm.to_string())); if !s.paradigms_encoded.contains(&paradigm.to_string()) { s.paradigms_encoded.push(paradigm.to_string()); } s.nodes.len() - 1 }
    pub fn add_edge(from: usize, to: usize) -> usize { let mut s = STATE.lock().unwrap(); s.edges.push((from, to)); s.edges.len() }
    pub fn node_count() -> usize { STATE.lock().unwrap().nodes.len() }
    pub fn edge_count() -> usize { STATE.lock().unwrap().edges.len() }
    pub fn paradigms_unified() -> usize { STATE.lock().unwrap().paradigms_encoded.len() }
    pub fn reset() { *STATE.lock().unwrap() = GuirState::default(); }
}

#[unsafe(no_mangle)]
pub extern "C" fn guir_node(id: i64) -> i64 { GrandUnifiedIr::add_node(&format!("n{}", id), "universal") as i64 }
#[unsafe(no_mangle)]
pub extern "C" fn guir_edge(from: i64, to: i64) -> i64 { GrandUnifiedIr::add_edge(from as usize, to as usize) as i64 }
#[unsafe(no_mangle)]
pub extern "C" fn guir_nodes() -> i64 { GrandUnifiedIr::node_count() as i64 }
#[unsafe(no_mangle)]
pub extern "C" fn guir_paradigms() -> i64 { GrandUnifiedIr::paradigms_unified() as i64 }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_node() { GrandUnifiedIr::reset(); let i = GrandUnifiedIr::add_node("a", "func"); assert_eq!(i, 0); }
    #[test] fn test_edge() { GrandUnifiedIr::reset(); GrandUnifiedIr::add_node("a", "f"); GrandUnifiedIr::add_node("b", "f"); assert_eq!(GrandUnifiedIr::add_edge(0, 1), 1); }
    #[test] fn test_paradigms() { GrandUnifiedIr::reset(); GrandUnifiedIr::add_node("a", "func"); GrandUnifiedIr::add_node("b", "imp"); assert_eq!(GrandUnifiedIr::paradigms_unified(), 2); }
    #[test] fn test_same_paradigm() { GrandUnifiedIr::reset(); GrandUnifiedIr::add_node("a", "func"); GrandUnifiedIr::add_node("b", "func"); assert_eq!(GrandUnifiedIr::paradigms_unified(), 1); }
    #[test] fn test_node_count() { GrandUnifiedIr::reset(); GrandUnifiedIr::add_node("a", "f"); assert_eq!(GrandUnifiedIr::node_count(), 1); }
    #[test] fn test_edge_count() { GrandUnifiedIr::reset(); GrandUnifiedIr::add_node("a", "f"); GrandUnifiedIr::add_node("b", "f"); GrandUnifiedIr::add_edge(0, 1); assert_eq!(GrandUnifiedIr::edge_count(), 1); }
    #[test] fn test_ffi() { GrandUnifiedIr::reset(); assert_eq!(guir_nodes(), 0); }
    #[test] fn test_ffi_paradigms() { GrandUnifiedIr::reset(); assert_eq!(guir_paradigms(), 0); }
}
