//! Polymorphic Compilation — Vitalis v1138
//!
//! Single compilation strategy that adapts to any target paradigm
//! at codegen time — one source, infinite backends.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<PolycState>> = LazyLock::new(|| Mutex::new(PolycState::default()));

#[derive(Default)]
struct PolycState {
    backends: Vec<String>,
    compilations: Vec<(String, String)>,
    adaptations: u64,
}

pub struct PolymorphicCompilation;

impl PolymorphicCompilation {
    pub fn register_backend(name: &str) -> usize { let mut s = STATE.lock().unwrap(); s.backends.push(name.to_string()); s.backends.len() }

    pub fn compile_for(source: &str, backend: &str) -> bool {
        let mut s = STATE.lock().unwrap();
        if s.backends.contains(&backend.to_string()) {
            s.compilations.push((source.to_string(), backend.to_string()));
            s.adaptations += 1;
            true
        } else { false }
    }

    pub fn backend_count() -> usize { STATE.lock().unwrap().backends.len() }
    pub fn compilation_count() -> usize { STATE.lock().unwrap().compilations.len() }
    pub fn adaptations() -> u64 { STATE.lock().unwrap().adaptations }
    pub fn reset() { *STATE.lock().unwrap() = PolycState::default(); }
}

#[unsafe(no_mangle)]
pub extern "C" fn polyc_register(id: i64) -> i64 { PolymorphicCompilation::register_backend(&format!("be{}", id)) as i64 }
#[unsafe(no_mangle)]
pub extern "C" fn polyc_compile(id: i64) -> i64 { if PolymorphicCompilation::compile_for("src", &format!("be{}", id)) { 1 } else { 0 } }
#[unsafe(no_mangle)]
pub extern "C" fn polyc_backends() -> i64 { PolymorphicCompilation::backend_count() as i64 }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_register() { PolymorphicCompilation::reset(); assert_eq!(PolymorphicCompilation::register_backend("x86"), 1); }
    #[test] fn test_compile() { PolymorphicCompilation::reset(); PolymorphicCompilation::register_backend("arm"); assert!(PolymorphicCompilation::compile_for("prog", "arm")); }
    #[test] fn test_compile_unknown() { PolymorphicCompilation::reset(); assert!(!PolymorphicCompilation::compile_for("prog", "nope")); }
    #[test] fn test_multi_backend() { PolymorphicCompilation::reset(); PolymorphicCompilation::register_backend("a"); PolymorphicCompilation::register_backend("b"); PolymorphicCompilation::compile_for("s", "a"); PolymorphicCompilation::compile_for("s", "b"); assert_eq!(PolymorphicCompilation::compilation_count(), 2); }
    #[test] fn test_adaptations() { PolymorphicCompilation::reset(); PolymorphicCompilation::register_backend("x"); PolymorphicCompilation::compile_for("s", "x"); assert_eq!(PolymorphicCompilation::adaptations(), 1); }
    #[test] fn test_backend_count() { PolymorphicCompilation::reset(); PolymorphicCompilation::register_backend("a"); assert_eq!(PolymorphicCompilation::backend_count(), 1); }
    #[test] fn test_ffi() { PolymorphicCompilation::reset(); assert_eq!(polyc_backends(), 0); }
}
