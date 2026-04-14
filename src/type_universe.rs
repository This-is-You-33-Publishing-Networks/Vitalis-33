//! Type Universe — Vitalis v1152
//!
//! A single type-theoretic universe containing all types from all paradigms.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<TuState>> = LazyLock::new(|| Mutex::new(TuState::default()));

#[derive(Default)]
struct TuState { types: Vec<(String, u32)>, universe_level: u32, inhabitants: Vec<(String, String)> }

pub struct TypeUniverse;

impl TypeUniverse {
    pub fn add_type(name: &str, level: u32) -> usize { let mut s = STATE.lock().unwrap(); s.types.push((name.to_string(), level)); if level > s.universe_level { s.universe_level = level; } s.types.len() }
    pub fn inhabit(ty: &str, value: &str) -> bool { let mut s = STATE.lock().unwrap(); if s.types.iter().any(|(n, _)| n == ty) { s.inhabitants.push((ty.to_string(), value.to_string())); true } else { false } }
    pub fn is_subtype(a: &str, b: &str) -> bool { let s = STATE.lock().unwrap(); let la = s.types.iter().find(|(n, _)| n == a).map(|(_, l)| *l); let lb = s.types.iter().find(|(n, _)| n == b).map(|(_, l)| *l); matches!((la, lb), (Some(a), Some(b)) if a <= b) }
    pub fn universe_level() -> u32 { STATE.lock().unwrap().universe_level }
    pub fn type_count() -> usize { STATE.lock().unwrap().types.len() }
    pub fn inhabitant_count() -> usize { STATE.lock().unwrap().inhabitants.len() }
    pub fn reset() { *STATE.lock().unwrap() = TuState::default(); }
}

#[unsafe(no_mangle)]
pub extern "C" fn tu_add(level: i64) -> i64 { TypeUniverse::add_type(&format!("T{}", level), level as u32) as i64 }
#[unsafe(no_mangle)]
pub extern "C" fn tu_level() -> i64 { TypeUniverse::universe_level() as i64 }
#[unsafe(no_mangle)]
pub extern "C" fn tu_types() -> i64 { TypeUniverse::type_count() as i64 }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_add() { TypeUniverse::reset(); assert_eq!(TypeUniverse::add_type("Int", 0), 1); }
    #[test] fn test_inhabit() { TypeUniverse::reset(); TypeUniverse::add_type("Int", 0); assert!(TypeUniverse::inhabit("Int", "42")); }
    #[test] fn test_inhabit_fail() { TypeUniverse::reset(); assert!(!TypeUniverse::inhabit("Nope", "x")); }
    #[test] fn test_subtype() { TypeUniverse::reset(); TypeUniverse::add_type("A", 0); TypeUniverse::add_type("B", 1); assert!(TypeUniverse::is_subtype("A", "B")); }
    #[test] fn test_not_subtype() { TypeUniverse::reset(); TypeUniverse::add_type("A", 5); TypeUniverse::add_type("B", 1); assert!(!TypeUniverse::is_subtype("A", "B")); }
    #[test] fn test_universe_level() { TypeUniverse::reset(); TypeUniverse::add_type("A", 3); assert_eq!(TypeUniverse::universe_level(), 3); }
    #[test] fn test_type_count() { TypeUniverse::reset(); TypeUniverse::add_type("A", 0); TypeUniverse::add_type("B", 1); assert_eq!(TypeUniverse::type_count(), 2); }
    #[test] fn test_ffi() { TypeUniverse::reset(); assert_eq!(tu_types(), 0); }
}
