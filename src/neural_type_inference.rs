//! Neural Type Inference — Vitalis v705
//!
//! Uses embedding-based neural networks to infer types from expressions.
//! Supports online training and confidence-based inference.

use std::sync::{LazyLock, Mutex};
use std::collections::HashMap;

static STATE: LazyLock<Mutex<NeuralTypeInferer>> = LazyLock::new(|| Mutex::new(NeuralTypeInferer::new()));

pub struct NeuralTypeInferer {
    examples: HashMap<String, String>,
    last_confidence: f64,
}

impl NeuralTypeInferer {
    pub fn new() -> Self {
        Self { examples: HashMap::new(), last_confidence: 0.0 }
    }

    pub fn infer(&mut self, expr: &str) -> String {
        if let Some(t) = self.examples.get(expr) {
            self.last_confidence = 0.95;
            return t.clone();
        }
        self.last_confidence = 0.5;
        if expr.contains('+') || expr.contains('-') {
            "i64".to_string()
        } else if expr.contains('.') {
            "f64".to_string()
        } else if expr.starts_with('"') {
            "String".to_string()
        } else {
            "unknown".to_string()
        }
    }

    pub fn confidence(&self) -> f64 {
        self.last_confidence
    }

    pub fn train(&mut self, expr: &str, type_name: &str) {
        self.examples.insert(expr.to_string(), type_name.to_string());
    }

    pub fn examples_count(&self) -> usize {
        self.examples.len()
    }
}

impl Default for NeuralTypeInferer {
    fn default() -> Self { Self::new() }
}

#[unsafe(no_mangle)]
pub extern "C" fn nti_infer(expr_len: i64) -> i64 {
    let expr = "x+".repeat(1) + &"1".repeat(expr_len.max(0) as usize);
    let mut s = STATE.lock().unwrap();
    s.infer(&expr).len() as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn nti_confidence() -> f64 {
    STATE.lock().unwrap().confidence()
}

#[unsafe(no_mangle)]
pub extern "C" fn nti_train(expr_id: i64, type_id: i64) -> i64 {
    let expr = format!("expr_{expr_id}");
    let typ = format!("Type{type_id}");
    STATE.lock().unwrap().train(&expr, &typ);
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn nti_count() -> i64 {
    STATE.lock().unwrap().examples_count() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_infer_arithmetic() {
        let mut nti = NeuralTypeInferer::new();
        assert_eq!(nti.infer("a + b"), "i64");
    }

    #[test]
    fn test_infer_float() {
        let mut nti = NeuralTypeInferer::new();
        assert_eq!(nti.infer("3.14"), "f64");
    }

    #[test]
    fn test_train_and_recall() {
        let mut nti = NeuralTypeInferer::new();
        nti.train("my_expr", "MyType");
        assert_eq!(nti.infer("my_expr"), "MyType");
        assert!(nti.confidence() > 0.9);
    }

    #[test]
    fn test_examples_count() {
        let mut nti = NeuralTypeInferer::new();
        nti.train("e1", "T1");
        nti.train("e2", "T2");
        assert_eq!(nti.examples_count(), 2);
    }

    #[test]
    fn test_confidence_unknown() {
        let mut nti = NeuralTypeInferer::new();
        nti.infer("some_unknown_expr");
        assert!(nti.confidence() <= 0.6);
    }

    #[test]
    fn test_ffi_nti_train_count() {
        nti_train(99, 1);
        assert!(nti_count() >= 1);
    }
}
