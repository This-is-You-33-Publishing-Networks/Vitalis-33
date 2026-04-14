//! Language Genome Encoding — Vitalis v800
//!
//! Encodes programming language specifications as genomic sequences.
//! Supports mutation and crossover for evolutionary language design.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<LanguageGenome>> = LazyLock::new(|| Mutex::new(LanguageGenome::new()));

pub struct LanguageGenome {
    genome: Vec<u8>,
}

impl LanguageGenome {
    pub fn new() -> Self {
        Self { genome: Vec::new() }
    }

    pub fn encode(&mut self, spec: &str) -> Vec<u8> {
        self.genome = spec.bytes().enumerate()
            .map(|(i, b)| b.wrapping_add(i as u8))
            .collect();
        self.genome.clone()
    }

    pub fn mutate(&mut self, rate: f64) -> Vec<u8> {
        let n = (self.genome.len() as f64 * rate).ceil() as usize;
        let mut result = self.genome.clone();
        for i in (0..result.len()).step_by((result.len() / n.max(1)).max(1)).take(n) {
            result[i] = result[i].wrapping_add(1);
        }
        result
    }

    pub fn crossover(&mut self, other: &[u8]) -> Vec<u8> {
        let half = self.genome.len() / 2;
        let mut child = self.genome[..half].to_vec();
        let other_start = other.len() / 2;
        child.extend_from_slice(&other[other_start..]);
        child
    }

    pub fn genome_len(&self) -> usize {
        self.genome.len()
    }
}

impl Default for LanguageGenome {
    fn default() -> Self { Self::new() }
}

#[unsafe(no_mangle)]
pub extern "C" fn lg_encode(spec_len: i64) -> i64 {
    let spec = "s".repeat(spec_len.max(0) as usize);
    let mut s = STATE.lock().unwrap();
    s.encode(&spec).len() as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn lg_mutate(rate: f64) -> i64 {
    STATE.lock().unwrap().mutate(rate).len() as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn lg_crossover(other_len: i64) -> i64 {
    let other = vec![0u8; other_len.max(0) as usize];
    STATE.lock().unwrap().crossover(&other).len() as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn lg_len() -> i64 {
    STATE.lock().unwrap().genome_len() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_length() {
        let mut lg = LanguageGenome::new();
        let enc = lg.encode("hello");
        assert_eq!(enc.len(), 5);
    }

    #[test]
    fn test_genome_len_after_encode() {
        let mut lg = LanguageGenome::new();
        lg.encode("vitalis");
        assert_eq!(lg.genome_len(), 7);
    }

    #[test]
    fn test_mutate_preserves_length() {
        let mut lg = LanguageGenome::new();
        lg.encode("test_spec");
        let mutated = lg.mutate(0.2);
        assert_eq!(mutated.len(), 9);
    }

    #[test]
    fn test_crossover_produces_output() {
        let mut lg = LanguageGenome::new();
        lg.encode("abcdef");
        let other = b"GHIJKL";
        let child = lg.crossover(other);
        assert!(!child.is_empty());
    }

    #[test]
    fn test_empty_genome_mutate() {
        let mut lg = LanguageGenome::new();
        let m = lg.mutate(0.5);
        assert!(m.is_empty());
    }

    #[test]
    fn test_ffi_lg_encode_and_len() {
        lg_encode(10);
        assert_eq!(lg_len(), 10);
    }
}
