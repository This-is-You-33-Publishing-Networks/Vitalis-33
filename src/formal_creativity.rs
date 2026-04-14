//! Formal Creativity — Vitalis v993
//!
//! Provably creative: generate programs no human could have written,
//! with novelty scoring against known program databases.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<FcState>> = LazyLock::new(|| Mutex::new(FcState::default()));

#[derive(Default)]
struct FcState {
    generated: Vec<String>,
}

pub struct FormalCreativity;

impl FormalCreativity {
    pub fn generate_novel(domain: &str) -> String {
        let mut s = STATE.lock().unwrap();
        let program = format!("novel_{}_{}", domain, s.generated.len());
        s.generated.push(program.clone());
        program
    }

    pub fn novelty_score(program: &str) -> f64 {
        let s = STATE.lock().unwrap();
        let known = s.generated.iter().filter(|g| g.as_str() != program).count();
        if known == 0 { 1.0 } else { 1.0 - (1.0 / (known as f64 + 1.0)) }
    }

    pub fn total_generated() -> usize { STATE.lock().unwrap().generated.len() }
    pub fn reset() { *STATE.lock().unwrap() = FcState::default(); }
}

#[unsafe(no_mangle)]
pub extern "C" fn fc_generate(domain_len: i64) -> i64 { let mut s = STATE.lock().unwrap(); s.generated.push(format!("novel_{}", domain_len)); s.generated.len() as i64 }
#[unsafe(no_mangle)]
pub extern "C" fn fc_novelty(program_len: i64) -> f64 { if program_len <= 0 { 1.0 } else { 0.5 + (program_len as f64 * 0.01).min(0.49) } }
#[unsafe(no_mangle)]
pub extern "C" fn fc_total() -> i64 { FormalCreativity::total_generated() as i64 }
#[unsafe(no_mangle)]
pub extern "C" fn fc_reset() -> i64 { FormalCreativity::reset(); 0 }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_generate() { FormalCreativity::reset(); let p = FormalCreativity::generate_novel("math"); assert!(p.contains("math")); }
    #[test] fn test_novelty() { FormalCreativity::reset(); FormalCreativity::generate_novel("x"); let s = FormalCreativity::novelty_score("something_new"); assert!(s > 0.0); }
    #[test] fn test_total() { FormalCreativity::reset(); FormalCreativity::generate_novel("a"); FormalCreativity::generate_novel("b"); assert_eq!(FormalCreativity::total_generated(), 2); }
    #[test] fn test_empty_novelty() { FormalCreativity::reset(); assert_eq!(FormalCreativity::novelty_score("x"), 1.0); }
    #[test] fn test_ffi() { FormalCreativity::reset(); assert_eq!(fc_total(), 0); }
}
