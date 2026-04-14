//! Program Morphogenesis — Vitalis v910
//!
//! Grows programs from seed specifications through iterative development
//! steps, modeling biological morphogenesis in software.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<Morphogenesis>> = LazyLock::new(|| Mutex::new(Morphogenesis::new()));

pub struct Morphogenesis {
    seed: String,
    nodes: usize,
    complexity: f64,
}

impl Morphogenesis {
    pub fn new() -> Self {
        Self { seed: String::new(), nodes: 0, complexity: 0.0 }
    }

    pub fn seed(&mut self, spec: &str) {
        self.seed = spec.to_string();
        self.nodes = spec.len();
        self.complexity = 0.1;
    }

    pub fn grow(&mut self, steps: u32) -> usize {
        for _ in 0..steps {
            self.nodes = (self.nodes as f64 * 1.5 + 1.0) as usize;
            self.complexity = (self.complexity * 1.2 + 0.05).min(100.0);
        }
        self.nodes
    }

    pub fn complexity(&self) -> f64 {
        self.complexity
    }

    pub fn reset(&mut self) {
        self.seed.clear();
        self.nodes = 0;
        self.complexity = 0.0;
    }
}

impl Default for Morphogenesis {
    fn default() -> Self { Self::new() }
}

#[unsafe(no_mangle)]
pub extern "C" fn morph_seed(spec_len: i64) -> i64 {
    let spec = "s".repeat(spec_len.max(0) as usize);
    STATE.lock().unwrap().seed(&spec);
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn morph_grow(steps: i64) -> i64 {
    STATE.lock().unwrap().grow(steps.max(0) as u32) as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn morph_complexity() -> f64 {
    STATE.lock().unwrap().complexity()
}

#[unsafe(no_mangle)]
pub extern "C" fn morph_reset() -> i64 {
    STATE.lock().unwrap().reset();
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_seed_sets_initial_nodes() {
        let mut m = Morphogenesis::new();
        m.seed("abc");
        assert_eq!(m.nodes, 3);
    }

    #[test]
    fn test_grow_increases_nodes() {
        let mut m = Morphogenesis::new();
        m.seed("x");
        let n = m.grow(3);
        assert!(n > 1);
    }

    #[test]
    fn test_complexity_increases_with_growth() {
        let mut m = Morphogenesis::new();
        m.seed("spec");
        m.grow(5);
        assert!(m.complexity() > 0.1);
    }

    #[test]
    fn test_complexity_bounded() {
        let mut m = Morphogenesis::new();
        m.seed("spec");
        m.grow(1000);
        assert!(m.complexity() <= 100.0);
    }

    #[test]
    fn test_reset() {
        let mut m = Morphogenesis::new();
        m.seed("xyz");
        m.grow(3);
        m.reset();
        assert_eq!(m.nodes, 0);
        assert_eq!(m.complexity(), 0.0);
    }

    #[test]
    fn test_ffi_morph_grow() {
        morph_seed(5);
        let n = morph_grow(2);
        assert!(n > 0);
    }
}
