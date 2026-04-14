//! Natural Language Spec Compiler — Vitalis v610
//!
//! Compiles natural language specifications into Vitalis code stubs.
//! Provides confidence scoring and validation of generated stubs.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<NLSpec>> = LazyLock::new(|| Mutex::new(NLSpec::new()));

pub struct NLSpec {
    last_stubs: Vec<String>,
    last_confidence: f64,
}

impl NLSpec {
    pub fn new() -> Self {
        Self { last_stubs: Vec::new(), last_confidence: 0.0 }
    }

    pub fn parse_nl(&mut self, text: &str) -> Vec<String> {
        let words: Vec<&str> = text.split_whitespace().collect();
        let mut stubs = Vec::new();
        for (i, chunk) in words.chunks(4).enumerate() {
            let name = chunk.join("_").to_lowercase()
                .replace(|c: char| !c.is_alphanumeric() && c != '_', "");
            stubs.push(format!("fn stub_{i}_{name}() {{ /* TODO */ }}"));
        }
        if stubs.is_empty() {
            stubs.push("fn stub_empty() { /* TODO */ }".to_string());
        }
        self.last_stubs = stubs.clone();
        self.last_confidence = if text.len() > 10 { 0.75 } else { 0.4 };
        stubs
    }

    pub fn validate_spec(&self, stubs: &[String]) -> bool {
        stubs.iter().all(|s| s.contains("fn ") && s.contains('{'))
    }

    pub fn spec_confidence(&mut self, text: &str) -> f64 {
        let score = (text.len() as f64 / 100.0).min(1.0) * 0.9;
        self.last_confidence = score;
        score
    }
}

impl Default for NLSpec {
    fn default() -> Self { Self::new() }
}

#[unsafe(no_mangle)]
pub extern "C" fn nl_spec_parse(text_len: i64) -> i64 {
    let text = "a".repeat(text_len.max(0) as usize);
    let mut s = STATE.lock().unwrap();
    s.parse_nl(&text).len() as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn nl_spec_validate(stub_count: i64) -> i64 {
    if stub_count > 0 { 1 } else { 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn nl_spec_confidence(text_len: i64) -> f64 {
    let text = "a".repeat(text_len.max(0) as usize);
    let mut s = STATE.lock().unwrap();
    s.spec_confidence(&text)
}

#[unsafe(no_mangle)]
pub extern "C" fn nl_spec_count() -> i64 {
    let s = STATE.lock().unwrap();
    s.last_stubs.len() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_nl_basic() {
        let mut spec = NLSpec::new();
        let stubs = spec.parse_nl("create a user authentication function");
        assert!(!stubs.is_empty());
    }

    #[test]
    fn test_parse_nl_generates_fn() {
        let mut spec = NLSpec::new();
        let stubs = spec.parse_nl("compute the hash value");
        assert!(stubs.iter().all(|s| s.contains("fn ")));
    }

    #[test]
    fn test_validate_spec_valid() {
        let spec = NLSpec::new();
        let stubs = vec!["fn foo() { /* TODO */ }".to_string()];
        assert!(spec.validate_spec(&stubs));
    }

    #[test]
    fn test_validate_spec_invalid() {
        let spec = NLSpec::new();
        let stubs = vec!["not a function".to_string()];
        assert!(!spec.validate_spec(&stubs));
    }

    #[test]
    fn test_spec_confidence_long_text() {
        let mut spec = NLSpec::new();
        let conf = spec.spec_confidence(&"a".repeat(100));
        assert!(conf > 0.0 && conf <= 1.0);
    }

    #[test]
    fn test_spec_confidence_empty() {
        let mut spec = NLSpec::new();
        let conf = spec.spec_confidence("");
        assert_eq!(conf, 0.0);
    }

    #[test]
    fn test_ffi_nl_spec_validate() {
        assert_eq!(nl_spec_validate(3), 1);
        assert_eq!(nl_spec_validate(0), 0);
    }
}
