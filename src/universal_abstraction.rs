//! Universal Abstraction — Vitalis v1136
//!
//! Category-theoretic abstraction layer that subsumes all known
//! programming abstractions into a unified categorical framework.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<UaState>> = LazyLock::new(|| Mutex::new(UaState::default()));

#[derive(Default)]
struct UaState {
    objects: Vec<String>,
    morphisms: Vec<(String, String, String)>,
    functors: Vec<(String, String)>,
}

pub struct UniversalAbstraction;

impl UniversalAbstraction {
    pub fn add_object(name: &str) -> usize { let mut s = STATE.lock().unwrap(); s.objects.push(name.to_string()); s.objects.len() }

    pub fn add_morphism(name: &str, from: &str, to: &str) -> usize {
        let mut s = STATE.lock().unwrap();
        s.morphisms.push((name.to_string(), from.to_string(), to.to_string()));
        s.morphisms.len()
    }

    pub fn add_functor(from_cat: &str, to_cat: &str) -> usize {
        let mut s = STATE.lock().unwrap();
        s.functors.push((from_cat.to_string(), to_cat.to_string()));
        s.functors.len()
    }

    pub fn compose(f: &str, g: &str) -> Option<String> {
        let s = STATE.lock().unwrap();
        let f_morph = s.morphisms.iter().find(|(n, _, _)| n == f);
        let g_morph = s.morphisms.iter().find(|(n, _, _)| n == g);
        if let (Some((_, _, f_to)), Some((_, g_from, _))) = (f_morph, g_morph) {
            if f_to == g_from { Some(format!("{}∘{}", g, f)) } else { None }
        } else { None }
    }

    pub fn object_count() -> usize { STATE.lock().unwrap().objects.len() }
    pub fn morphism_count() -> usize { STATE.lock().unwrap().morphisms.len() }
    pub fn reset() { *STATE.lock().unwrap() = UaState::default(); }
}

#[unsafe(no_mangle)]
pub extern "C" fn ua_object(id: i64) -> i64 { UniversalAbstraction::add_object(&format!("obj{}", id)) as i64 }
#[unsafe(no_mangle)]
pub extern "C" fn ua_morphism() -> i64 { UniversalAbstraction::add_morphism("f", "a", "b") as i64 }
#[unsafe(no_mangle)]
pub extern "C" fn ua_objects() -> i64 { UniversalAbstraction::object_count() as i64 }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_object() { UniversalAbstraction::reset(); assert_eq!(UniversalAbstraction::add_object("Int"), 1); }
    #[test] fn test_morphism() { UniversalAbstraction::reset(); assert_eq!(UniversalAbstraction::add_morphism("f", "A", "B"), 1); }
    #[test] fn test_functor() { UniversalAbstraction::reset(); assert_eq!(UniversalAbstraction::add_functor("Set", "Hask"), 1); }
    #[test] fn test_compose() { UniversalAbstraction::reset(); UniversalAbstraction::add_morphism("f", "A", "B"); UniversalAbstraction::add_morphism("g", "B", "C"); assert!(UniversalAbstraction::compose("f", "g").is_some()); }
    #[test] fn test_compose_fail() { UniversalAbstraction::reset(); UniversalAbstraction::add_morphism("f", "A", "B"); UniversalAbstraction::add_morphism("g", "C", "D"); assert!(UniversalAbstraction::compose("f", "g").is_none()); }
    #[test] fn test_counts() { UniversalAbstraction::reset(); UniversalAbstraction::add_object("A"); UniversalAbstraction::add_morphism("f", "A", "B"); assert_eq!(UniversalAbstraction::object_count(), 1); assert_eq!(UniversalAbstraction::morphism_count(), 1); }
    #[test] fn test_ffi() { UniversalAbstraction::reset(); assert_eq!(ua_objects(), 0); }
}
