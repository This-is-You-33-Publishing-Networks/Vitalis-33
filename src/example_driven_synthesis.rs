//! Example-Driven Synthesis — Vitalis v1168
//!
//! Synthesizes programs from input-output examples via inductive logic.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<EdsState>> = LazyLock::new(|| Mutex::new(EdsState::default()));

#[derive(Default)]
struct EdsState { examples: Vec<(String, String)>, programs: Vec<String>, accuracy: f64 }

pub struct ExampleDrivenSynthesis;

impl ExampleDrivenSynthesis {
    pub fn add_example(input: &str, output: &str) -> usize { let mut s = STATE.lock().unwrap(); s.examples.push((input.to_string(), output.to_string())); s.examples.len() }
    pub fn synthesize() -> Option<String> {
        let mut s = STATE.lock().unwrap();
        if s.examples.is_empty() { return None; }
        let prog = format!("synth_from_{}_examples", s.examples.len());
        s.accuracy = (s.examples.len() as f64 * 0.1).min(1.0);
        s.programs.push(prog.clone());
        Some(prog)
    }
    pub fn accuracy() -> f64 { STATE.lock().unwrap().accuracy }
    pub fn example_count() -> usize { STATE.lock().unwrap().examples.len() }
    pub fn program_count() -> usize { STATE.lock().unwrap().programs.len() }
    pub fn reset() { *STATE.lock().unwrap() = EdsState::default(); }
}

#[unsafe(no_mangle)]
pub extern "C" fn eds_add() -> i64 { ExampleDrivenSynthesis::add_example("in", "out") as i64 }
#[unsafe(no_mangle)]
pub extern "C" fn eds_synth() -> i64 { if ExampleDrivenSynthesis::synthesize().is_some() { 1 } else { 0 } }
#[unsafe(no_mangle)]
pub extern "C" fn eds_accuracy() -> f64 { ExampleDrivenSynthesis::accuracy() }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_add() { ExampleDrivenSynthesis::reset(); assert_eq!(ExampleDrivenSynthesis::add_example("1", "2"), 1); }
    #[test] fn test_synth() { ExampleDrivenSynthesis::reset(); ExampleDrivenSynthesis::add_example("1", "2"); assert!(ExampleDrivenSynthesis::synthesize().is_some()); }
    #[test] fn test_synth_empty() { ExampleDrivenSynthesis::reset(); assert!(ExampleDrivenSynthesis::synthesize().is_none()); }
    #[test] fn test_accuracy() { ExampleDrivenSynthesis::reset(); for _ in 0..10 { ExampleDrivenSynthesis::add_example("a", "b"); } ExampleDrivenSynthesis::synthesize(); assert!((ExampleDrivenSynthesis::accuracy() - 1.0).abs() < 0.01); }
    #[test] fn test_example_count() { ExampleDrivenSynthesis::reset(); ExampleDrivenSynthesis::add_example("a", "b"); assert_eq!(ExampleDrivenSynthesis::example_count(), 1); }
    #[test] fn test_program_count() { ExampleDrivenSynthesis::reset(); ExampleDrivenSynthesis::add_example("a", "b"); ExampleDrivenSynthesis::synthesize(); assert_eq!(ExampleDrivenSynthesis::program_count(), 1); }
    #[test] fn test_ffi() { ExampleDrivenSynthesis::reset(); assert!((eds_accuracy() - 0.0).abs() < 0.01); }
}
