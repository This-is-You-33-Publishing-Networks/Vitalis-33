//! Symbiotic Toolchain — Vitalis v1287
//!
//! Toolchain components co-evolving symbiotically.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<SymbioticToolchainState>> = LazyLock::new(|| Mutex::new(SymbioticToolchainState::default()));

#[derive(Default)]
struct SymbioticToolchainState {
    tools: Vec<(String, f64)>,
    co_evolutions: u64,
    synergy: f64,
}

pub struct SymbioticToolchain;

impl SymbioticToolchain {
    pub fn add_tool(name: &str, fitness: f64) {
        let mut s = STATE.lock().unwrap();
        s.tools.push((name.to_string(), fitness));
    }
    pub fn co_evolve() -> u64 {
        let mut s = STATE.lock().unwrap();
        s.co_evolutions += 1;
        for t in s.tools.iter_mut() { t.1 *= 1.05; }
        s.synergy = s.tools.iter().map(|t| t.1).sum::<f64>() / s.tools.len().max(1) as f64;
        s.co_evolutions
    }
    pub fn synergy() -> f64 { STATE.lock().unwrap().synergy }
    pub fn tool_count() -> usize { STATE.lock().unwrap().tools.len() }
    pub fn reset() { *STATE.lock().unwrap() = SymbioticToolchainState::default(); }
}

#[unsafe(no_mangle)]
pub extern "C" fn stool_co_evolve() -> u64 { SymbioticToolchain::co_evolve() }
#[unsafe(no_mangle)]
pub extern "C" fn stool_synergy() -> f64 { SymbioticToolchain::synergy() }
#[unsafe(no_mangle)]
pub extern "C" fn stool_tool_count() -> usize { SymbioticToolchain::tool_count() }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_add_tool() { SymbioticToolchain::reset(); SymbioticToolchain::add_tool("linter", 1.0); assert_eq!(SymbioticToolchain::tool_count(), 1); }
    #[test] fn test_co_evolve() { SymbioticToolchain::reset(); SymbioticToolchain::add_tool("a", 1.0); let c = SymbioticToolchain::co_evolve(); assert_eq!(c, 1); }
    #[test] fn test_synergy_after_evolve() { SymbioticToolchain::reset(); SymbioticToolchain::add_tool("b", 2.0); SymbioticToolchain::co_evolve(); assert!(SymbioticToolchain::synergy() > 2.0); }
    #[test] fn test_initial_synergy() { SymbioticToolchain::reset(); assert!((SymbioticToolchain::synergy() - 0.0).abs() < 1e-9); }
    #[test] fn test_multiple_evolves() { SymbioticToolchain::reset(); SymbioticToolchain::add_tool("c", 1.0); SymbioticToolchain::co_evolve(); let c = SymbioticToolchain::co_evolve(); assert_eq!(c, 2); }
    #[test] fn test_fitness_grows() { SymbioticToolchain::reset(); SymbioticToolchain::add_tool("d", 1.0); SymbioticToolchain::co_evolve(); assert!(STATE.lock().unwrap().tools[0].1 > 1.0); }
    #[test] fn test_reset() { SymbioticToolchain::reset(); SymbioticToolchain::add_tool("x", 1.0); SymbioticToolchain::reset(); assert_eq!(SymbioticToolchain::tool_count(), 0); }
}
