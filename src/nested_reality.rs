//! Nested Reality — Vitalis v1077
//!
//! Manages recursive simulation layers where compiled programs
//! spawn their own compilers in an infinite nesting of realities.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<NrState>> = LazyLock::new(|| Mutex::new(NrState::default()));

#[derive(Default)]
struct NrState {
    nesting_depth: u32,
    layers: Vec<(u32, String)>,
    max_depth: u32,
}

pub struct NestedReality;

impl NestedReality {
    pub fn enter_layer(name: &str) -> u32 {
        let mut s = STATE.lock().unwrap();
        s.nesting_depth += 1;
        let depth = s.nesting_depth;
        s.layers.push((depth, name.to_string()));
        if s.nesting_depth > s.max_depth { s.max_depth = s.nesting_depth; }
        s.nesting_depth
    }

    pub fn exit_layer() -> u32 {
        let mut s = STATE.lock().unwrap();
        s.layers.pop();
        s.nesting_depth = s.nesting_depth.saturating_sub(1);
        s.nesting_depth
    }

    pub fn current_layer() -> Option<String> {
        STATE.lock().unwrap().layers.last().map(|(_, n)| n.clone())
    }

    pub fn depth() -> u32 { STATE.lock().unwrap().nesting_depth }
    pub fn max_depth() -> u32 { STATE.lock().unwrap().max_depth }
    pub fn layer_count() -> usize { STATE.lock().unwrap().layers.len() }
    pub fn reset() { *STATE.lock().unwrap() = NrState::default(); }
}

#[unsafe(no_mangle)]
pub extern "C" fn nr_enter(id: i64) -> i64 { NestedReality::enter_layer(&format!("layer_{}", id)) as i64 }
#[unsafe(no_mangle)]
pub extern "C" fn nr_exit() -> i64 { NestedReality::exit_layer() as i64 }
#[unsafe(no_mangle)]
pub extern "C" fn nr_depth() -> i64 { NestedReality::depth() as i64 }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_enter() { NestedReality::reset(); assert_eq!(NestedReality::enter_layer("base"), 1); }
    #[test] fn test_exit() { NestedReality::reset(); NestedReality::enter_layer("a"); assert_eq!(NestedReality::exit_layer(), 0); }
    #[test] fn test_nested() { NestedReality::reset(); NestedReality::enter_layer("a"); NestedReality::enter_layer("b"); assert_eq!(NestedReality::depth(), 2); }
    #[test] fn test_current() { NestedReality::reset(); NestedReality::enter_layer("a"); assert_eq!(NestedReality::current_layer(), Some("a".to_string())); }
    #[test] fn test_max_depth() { NestedReality::reset(); NestedReality::enter_layer("a"); NestedReality::enter_layer("b"); NestedReality::exit_layer(); assert_eq!(NestedReality::max_depth(), 2); }
    #[test] fn test_empty() { NestedReality::reset(); assert!(NestedReality::current_layer().is_none()); }
    #[test] fn test_ffi() { NestedReality::reset(); assert_eq!(nr_depth(), 0); }
}
