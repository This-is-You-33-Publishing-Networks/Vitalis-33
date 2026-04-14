//! Aesthetic Compiler — Vitalis v1277
//!
//! Evaluates and optimizes code for elegance and clarity.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<AestheticCompilerState>> = LazyLock::new(|| Mutex::new(AestheticCompilerState::default()));

#[derive(Default)]
struct AestheticCompilerState {
    evaluations: Vec<(String, f64)>,
    beauty_threshold: f64,
    improved: u64,
}

pub struct AestheticCompiler;

impl AestheticCompiler {
    pub fn evaluate(code: &str, beauty: f64) -> f64 {
        let mut s = STATE.lock().unwrap();
        s.evaluations.push((code.to_string(), beauty));
        beauty
    }
    pub fn set_threshold(t: f64) { STATE.lock().unwrap().beauty_threshold = t; }
    pub fn improve() -> u64 {
        let mut s = STATE.lock().unwrap();
        s.improved += 1;
        s.improved
    }
    pub fn average_beauty() -> f64 {
        let s = STATE.lock().unwrap();
        if s.evaluations.is_empty() { return 0.0; }
        s.evaluations.iter().map(|x| x.1).sum::<f64>() / s.evaluations.len() as f64
    }
    pub fn evaluation_count() -> usize { STATE.lock().unwrap().evaluations.len() }
    pub fn reset() { *STATE.lock().unwrap() = AestheticCompilerState::default(); }
}

#[unsafe(no_mangle)]
pub extern "C" fn aesc_improve() -> u64 { AestheticCompiler::improve() }
#[unsafe(no_mangle)]
pub extern "C" fn aesc_average_beauty() -> f64 { AestheticCompiler::average_beauty() }
#[unsafe(no_mangle)]
pub extern "C" fn aesc_evaluation_count() -> usize { AestheticCompiler::evaluation_count() }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_evaluate() { AestheticCompiler::reset(); let b = AestheticCompiler::evaluate("fn x() {}", 0.9); assert!((b - 0.9).abs() < 1e-9); }
    #[test] fn test_threshold() { AestheticCompiler::reset(); AestheticCompiler::set_threshold(0.7); assert!((STATE.lock().unwrap().beauty_threshold - 0.7).abs() < 1e-9); }
    #[test] fn test_improve() { AestheticCompiler::reset(); let i = AestheticCompiler::improve(); assert_eq!(i, 1); }
    #[test] fn test_average_beauty() { AestheticCompiler::reset(); AestheticCompiler::evaluate("a", 0.6); AestheticCompiler::evaluate("b", 0.8); assert!((AestheticCompiler::average_beauty() - 0.7).abs() < 1e-9); }
    #[test] fn test_empty_average() { AestheticCompiler::reset(); assert!((AestheticCompiler::average_beauty() - 0.0).abs() < 1e-9); }
    #[test] fn test_count() { AestheticCompiler::reset(); AestheticCompiler::evaluate("x", 0.5); assert_eq!(AestheticCompiler::evaluation_count(), 1); }
    #[test] fn test_reset() { AestheticCompiler::reset(); AestheticCompiler::evaluate("y", 1.0); AestheticCompiler::reset(); assert_eq!(AestheticCompiler::evaluation_count(), 0); }
}
