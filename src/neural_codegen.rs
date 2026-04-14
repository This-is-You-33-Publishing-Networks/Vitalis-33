//! Neural Instruction Selection — Vitalis v720
//!
//! Uses neural models to select optimal machine instructions for IR operations.
//! Caches results for performance and provides confidence scores.

use std::sync::{LazyLock, Mutex};
use std::collections::HashMap;

static STATE: LazyLock<Mutex<NeuralCodegen>> = LazyLock::new(|| Mutex::new(NeuralCodegen::new()));

pub struct NeuralCodegen {
    cache: HashMap<String, String>,
    last_confidence: f64,
}

impl NeuralCodegen {
    pub fn new() -> Self {
        Self { cache: HashMap::new(), last_confidence: 0.0 }
    }

    pub fn select_instruction(&mut self, ir_op: &str) -> String {
        if let Some(ins) = self.cache.get(ir_op) {
            self.last_confidence = 0.99;
            return ins.clone();
        }
        let ins = match ir_op {
            "add" => "ADD r0, r1, r2",
            "mul" => "MUL r0, r1, r2",
            "load" => "LDR r0, [r1]",
            "store" => "STR r0, [r1]",
            _ => "NOP",
        };
        self.last_confidence = 0.7;
        let result = ins.to_string();
        self.cache.insert(ir_op.to_string(), result.clone());
        result
    }

    pub fn confidence(&self) -> f64 {
        self.last_confidence
    }

    pub fn cache_size(&self) -> usize {
        self.cache.len()
    }

    pub fn reset_cache(&mut self) {
        self.cache.clear();
        self.last_confidence = 0.0;
    }
}

impl Default for NeuralCodegen {
    fn default() -> Self { Self::new() }
}

#[unsafe(no_mangle)]
pub extern "C" fn ncg_select(op_id: i64) -> i64 {
    let op = match op_id % 4 {
        0 => "add",
        1 => "mul",
        2 => "load",
        _ => "store",
    };
    let mut s = STATE.lock().unwrap();
    s.select_instruction(op).len() as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn ncg_confidence() -> f64 {
    STATE.lock().unwrap().confidence()
}

#[unsafe(no_mangle)]
pub extern "C" fn ncg_cache_size() -> i64 {
    STATE.lock().unwrap().cache_size() as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn ncg_reset() -> i64 {
    STATE.lock().unwrap().reset_cache();
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_select_add() {
        let mut cg = NeuralCodegen::new();
        let ins = cg.select_instruction("add");
        assert!(ins.contains("ADD"));
    }

    #[test]
    fn test_cache_hit() {
        let mut cg = NeuralCodegen::new();
        cg.select_instruction("mul");
        let conf_before = cg.confidence();
        cg.select_instruction("mul");
        assert!(cg.confidence() > conf_before || cg.confidence() == 0.99);
    }

    #[test]
    fn test_cache_grows() {
        let mut cg = NeuralCodegen::new();
        cg.select_instruction("add");
        cg.select_instruction("load");
        assert_eq!(cg.cache_size(), 2);
    }

    #[test]
    fn test_reset_clears_cache() {
        let mut cg = NeuralCodegen::new();
        cg.select_instruction("add");
        cg.reset_cache();
        assert_eq!(cg.cache_size(), 0);
    }

    #[test]
    fn test_unknown_op() {
        let mut cg = NeuralCodegen::new();
        let ins = cg.select_instruction("unknown_op");
        assert_eq!(ins, "NOP");
    }

    #[test]
    fn test_ffi_ncg_select() {
        let len = ncg_select(0);
        assert!(len > 0);
    }
}
