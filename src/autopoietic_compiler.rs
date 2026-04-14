//! Autopoietic Compiler — Vitalis v1273
//!
//! Self-producing compiler maintaining own organization (autopoiesis).

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<AutopoieticCompilerState>> = LazyLock::new(|| Mutex::new(AutopoieticCompilerState::default()));

#[derive(Default)]
struct AutopoieticCompilerState {
    components: Vec<String>,
    productions: u64,
    integrity: f64,
}

pub struct AutopoieticCompiler;

impl AutopoieticCompiler {
    pub fn add_component(name: &str) {
        let mut s = STATE.lock().unwrap();
        s.components.push(name.to_string());
        s.integrity = 1.0 - (1.0 / (s.components.len() as f64 + 1.0));
    }
    pub fn produce() -> u64 {
        let mut s = STATE.lock().unwrap();
        s.productions += 1;
        s.productions
    }
    pub fn integrity() -> f64 { STATE.lock().unwrap().integrity }
    pub fn component_count() -> usize { STATE.lock().unwrap().components.len() }
    pub fn reset() { *STATE.lock().unwrap() = AutopoieticCompilerState::default(); }
}

#[unsafe(no_mangle)]
pub extern "C" fn auto_produce() -> u64 { AutopoieticCompiler::produce() }
#[unsafe(no_mangle)]
pub extern "C" fn auto_integrity() -> f64 { AutopoieticCompiler::integrity() }
#[unsafe(no_mangle)]
pub extern "C" fn auto_component_count() -> usize { AutopoieticCompiler::component_count() }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_add_component() { AutopoieticCompiler::reset(); AutopoieticCompiler::add_component("lexer"); assert_eq!(AutopoieticCompiler::component_count(), 1); }
    #[test] fn test_produce() { AutopoieticCompiler::reset(); let p = AutopoieticCompiler::produce(); assert_eq!(p, 1); }
    #[test] fn test_integrity_grows() { AutopoieticCompiler::reset(); AutopoieticCompiler::add_component("a"); AutopoieticCompiler::add_component("b"); assert!(AutopoieticCompiler::integrity() > 0.5); }
    #[test] fn test_initial_integrity() { AutopoieticCompiler::reset(); assert!((AutopoieticCompiler::integrity() - 0.0).abs() < 1e-9); }
    #[test] fn test_productions_accumulate() { AutopoieticCompiler::reset(); AutopoieticCompiler::produce(); let p = AutopoieticCompiler::produce(); assert_eq!(p, 2); }
    #[test] fn test_reset() { AutopoieticCompiler::reset(); AutopoieticCompiler::add_component("x"); AutopoieticCompiler::reset(); assert_eq!(AutopoieticCompiler::component_count(), 0); }
}
