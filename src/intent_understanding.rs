//! Deep Intent Inference — Vitalis v605
//!
//! Infers developer intent from code patterns, providing completions
//! and confidence scores for intent-driven development.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<IntentModel>> = LazyLock::new(|| Mutex::new(IntentModel::new()));

pub struct IntentModel {
    last_intent: String,
    confidence: f64,
    suggestions: Vec<String>,
}

impl IntentModel {
    pub fn new() -> Self {
        Self {
            last_intent: String::new(),
            confidence: 0.0,
            suggestions: Vec::new(),
        }
    }

    pub fn infer_intent(&mut self, code: &str) -> String {
        let intent = if code.contains("fn ") {
            "define_function"
        } else if code.contains("struct ") {
            "define_type"
        } else if code.contains("let ") {
            "bind_variable"
        } else {
            "unknown"
        };
        self.last_intent = intent.to_string();
        self.confidence = if code.len() > 5 { 0.8 } else { 0.3 };
        intent.to_string()
    }

    pub fn confidence(&self) -> f64 {
        self.confidence
    }

    pub fn suggest_completion(&mut self, partial: &str) -> Vec<String> {
        let base = partial.trim_end();
        self.suggestions = vec![
            format!("{base}_impl"),
            format!("{base}_helper"),
            format!("{base}_test"),
        ];
        self.suggestions.clone()
    }

    pub fn reset(&mut self) {
        self.last_intent.clear();
        self.confidence = 0.0;
        self.suggestions.clear();
    }
}

impl Default for IntentModel {
    fn default() -> Self { Self::new() }
}

#[unsafe(no_mangle)]
pub extern "C" fn intent_infer(code_len: i64) -> i64 {
    let code = "fn ".repeat(1) + &"x".repeat(code_len.max(0) as usize);
    let mut s = STATE.lock().unwrap();
    s.infer_intent(&code).len() as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn intent_confidence() -> f64 {
    STATE.lock().unwrap().confidence()
}

#[unsafe(no_mangle)]
pub extern "C" fn intent_suggest_count(partial_len: i64) -> i64 {
    let partial = "f".repeat(partial_len.max(0) as usize);
    let mut s = STATE.lock().unwrap();
    s.suggest_completion(&partial).len() as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn intent_reset() -> i64 {
    STATE.lock().unwrap().reset();
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_infer_function_intent() {
        let mut m = IntentModel::new();
        assert_eq!(m.infer_intent("fn foo() {}"), "define_function");
    }

    #[test]
    fn test_infer_struct_intent() {
        let mut m = IntentModel::new();
        assert_eq!(m.infer_intent("struct Foo {}"), "define_type");
    }

    #[test]
    fn test_confidence_after_infer() {
        let mut m = IntentModel::new();
        m.infer_intent("fn foo() {}");
        assert!(m.confidence() > 0.5);
    }

    #[test]
    fn test_suggest_completion() {
        let mut m = IntentModel::new();
        let sug = m.suggest_completion("parse");
        assert_eq!(sug.len(), 3);
        assert!(sug[0].contains("parse"));
    }

    #[test]
    fn test_reset() {
        let mut m = IntentModel::new();
        m.infer_intent("fn foo(){}");
        m.reset();
        assert_eq!(m.confidence(), 0.0);
        assert!(m.last_intent.is_empty());
    }

    #[test]
    fn test_ffi_intent_infer() {
        let len = intent_infer(10);
        assert!(len > 0);
    }
}
