//! Code Explanation Engine — Vitalis v810
//!
//! Generates human-readable explanations for errors and type signatures,
//! helping developers understand complex compiler output.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<ExplainEngine>> = LazyLock::new(|| Mutex::new(ExplainEngine::new()));

pub struct ExplainEngine {
    count: usize,
}

impl ExplainEngine {
    pub fn new() -> Self {
        Self { count: 0 }
    }

    pub fn explain_error(&mut self, code: &str, err: &str) -> String {
        self.count += 1;
        format!(
            "In '{}': the error '{}' means a type or value constraint was violated.",
            &code[..code.len().min(30)],
            &err[..err.len().min(50)]
        )
    }

    pub fn explain_type(&mut self, type_name: &str) -> String {
        self.count += 1;
        match type_name {
            "i64" => "A 64-bit signed integer.".to_string(),
            "f64" => "A 64-bit floating-point number.".to_string(),
            "String" => "A heap-allocated UTF-8 string.".to_string(),
            "bool" => "A boolean value: true or false.".to_string(),
            _ => format!("Type '{type_name}': a user-defined or opaque type."),
        }
    }

    pub fn explain_count(&self) -> usize {
        self.count
    }

    pub fn reset(&mut self) {
        self.count = 0;
    }
}

impl Default for ExplainEngine {
    fn default() -> Self { Self::new() }
}

#[unsafe(no_mangle)]
pub extern "C" fn ee_explain_error(code_len: i64, err_len: i64) -> i64 {
    let code = "x".repeat(code_len.max(0) as usize);
    let err = "e".repeat(err_len.max(0) as usize);
    let mut s = STATE.lock().unwrap();
    s.explain_error(&code, &err).len() as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn ee_explain_type(type_id: i64) -> i64 {
    let type_name = match type_id % 4 {
        0 => "i64",
        1 => "f64",
        2 => "String",
        _ => "bool",
    };
    let mut s = STATE.lock().unwrap();
    s.explain_type(type_name).len() as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn ee_count() -> i64 {
    STATE.lock().unwrap().explain_count() as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn ee_reset() -> i64 {
    STATE.lock().unwrap().reset();
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_explain_error_contains_err() {
        let mut ee = ExplainEngine::new();
        let r = ee.explain_error("fn foo(){}", "type mismatch");
        assert!(r.contains("type mismatch"));
    }

    #[test]
    fn test_explain_type_i64() {
        let mut ee = ExplainEngine::new();
        let r = ee.explain_type("i64");
        assert!(r.contains("64-bit signed integer"));
    }

    #[test]
    fn test_explain_type_unknown() {
        let mut ee = ExplainEngine::new();
        let r = ee.explain_type("MyCustomType");
        assert!(r.contains("MyCustomType"));
    }

    #[test]
    fn test_count_increments() {
        let mut ee = ExplainEngine::new();
        ee.explain_error("code", "err");
        ee.explain_type("bool");
        assert_eq!(ee.explain_count(), 2);
    }

    #[test]
    fn test_reset() {
        let mut ee = ExplainEngine::new();
        ee.explain_error("c", "e");
        ee.reset();
        assert_eq!(ee.explain_count(), 0);
    }

    #[test]
    fn test_ffi_ee_count() {
        ee_reset();
        ee_explain_error(5, 5);
        assert!(ee_count() >= 1);
    }
}
