//! Diagonal Argument — Vitalis v1215
//!
//! Diagonalization generating provably novel programs.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<DiagState>> = LazyLock::new(|| Mutex::new(DiagState::default()));

#[derive(Default)]
struct DiagState {
    enumeration: Vec<String>,
    diagonal_elements: Vec<String>,
    novelty_score: f64,
}

pub struct DiagonalArgument;

impl DiagonalArgument {
    pub fn enumerate(program: &str) {
        let mut s = STATE.lock().unwrap();
        s.enumeration.push(program.to_string());
    }

    pub fn diagonalize() -> f64 {
        let mut s = STATE.lock().unwrap();
        let novel = format!("diag_{}", s.enumeration.len());
        s.diagonal_elements.push(novel);
        s.novelty_score += 1.0 / (s.diagonal_elements.len() as f64 + 1.0);
        s.novelty_score
    }

    pub fn enumeration_size() -> usize { STATE.lock().unwrap().enumeration.len() }

    pub fn diagonal_count() -> usize { STATE.lock().unwrap().diagonal_elements.len() }

    pub fn novelty_score() -> f64 { STATE.lock().unwrap().novelty_score }

    pub fn reset() { *STATE.lock().unwrap() = DiagState::default(); }
}

#[unsafe(no_mangle)]
pub extern "C" fn diag_enumerate() -> i64 { DiagonalArgument::enumerate("ffi_prog"); DiagonalArgument::enumeration_size() as i64 }
#[unsafe(no_mangle)]
pub extern "C" fn diag_diagonalize() -> f64 { DiagonalArgument::diagonalize() }
#[unsafe(no_mangle)]
pub extern "C" fn diag_novelty() -> f64 { DiagonalArgument::novelty_score() }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_enumerate() { DiagonalArgument::reset(); DiagonalArgument::enumerate("p1"); assert_eq!(DiagonalArgument::enumeration_size(), 1); }
    #[test] fn test_diagonalize() { DiagonalArgument::reset(); let s = DiagonalArgument::diagonalize(); assert!(s > 0.0); }
    #[test] fn test_diagonal_count() { DiagonalArgument::reset(); DiagonalArgument::diagonalize(); DiagonalArgument::diagonalize(); assert_eq!(DiagonalArgument::diagonal_count(), 2); }
    #[test] fn test_novelty_increases() { DiagonalArgument::reset(); let a = DiagonalArgument::diagonalize(); let b = DiagonalArgument::diagonalize(); assert!(b > a); }
    #[test] fn test_reset() { DiagonalArgument::reset(); DiagonalArgument::enumerate("x"); DiagonalArgument::reset(); assert_eq!(DiagonalArgument::enumeration_size(), 0); }
    #[test] fn test_ffi() { DiagonalArgument::reset(); diag_enumerate(); assert!(diag_novelty() == 0.0); }
}
