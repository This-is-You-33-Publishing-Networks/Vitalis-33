//! Root Cause Analysis — Vitalis v651
//!
//! Analyzes error chains to find the most likely root cause of failures.
//! Maintains a cause chain and provides confidence scoring.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<RootCauseAnalyzer>> = LazyLock::new(|| Mutex::new(RootCauseAnalyzer::new()));

pub struct RootCauseAnalyzer {
    chain: Vec<String>,
    confidence: f64,
}

impl RootCauseAnalyzer {
    pub fn new() -> Self {
        Self { chain: Vec::new(), confidence: 0.0 }
    }

    pub fn analyze(&mut self, error: &str) -> String {
        self.chain.clear();
        let parts: Vec<&str> = error.split(':').collect();
        for p in &parts {
            self.chain.push(p.trim().to_string());
        }
        if self.chain.is_empty() {
            self.chain.push(error.to_string());
        }
        self.confidence = (self.chain.len() as f64 * 0.2).min(1.0);
        self.chain.first().cloned().unwrap_or_default()
    }

    pub fn chain_len(&self) -> usize {
        self.chain.len()
    }

    pub fn most_likely_cause(&self) -> String {
        self.chain.last().cloned().unwrap_or_else(|| "unknown".to_string())
    }

    pub fn confidence(&self) -> f64 {
        self.confidence
    }

    pub fn reset(&mut self) {
        self.chain.clear();
        self.confidence = 0.0;
    }
}

impl Default for RootCauseAnalyzer {
    fn default() -> Self { Self::new() }
}

#[unsafe(no_mangle)]
pub extern "C" fn rca_analyze(error_len: i64) -> i64 {
    let error = "null:ptr:deref:".repeat(1) + &"e".repeat(error_len.max(0) as usize);
    let mut s = STATE.lock().unwrap();
    s.analyze(&error);
    s.chain_len() as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn rca_chain_len() -> i64 {
    STATE.lock().unwrap().chain_len() as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn rca_confidence() -> f64 {
    STATE.lock().unwrap().confidence()
}

#[unsafe(no_mangle)]
pub extern "C" fn rca_reset() -> i64 {
    STATE.lock().unwrap().reset();
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analyze_simple_error() {
        let mut rca = RootCauseAnalyzer::new();
        let cause = rca.analyze("NullPointerException");
        assert!(!cause.is_empty());
    }

    #[test]
    fn test_analyze_chained_error() {
        let mut rca = RootCauseAnalyzer::new();
        rca.analyze("IoError: FileNotFound: permission denied");
        assert_eq!(rca.chain_len(), 3);
    }

    #[test]
    fn test_most_likely_cause() {
        let mut rca = RootCauseAnalyzer::new();
        rca.analyze("outer: middle: root");
        assert_eq!(rca.most_likely_cause(), "root");
    }

    #[test]
    fn test_confidence_increases_with_chain() {
        let mut rca = RootCauseAnalyzer::new();
        rca.analyze("a: b: c: d: e");
        assert!(rca.confidence() > 0.5);
    }

    #[test]
    fn test_reset_clears_state() {
        let mut rca = RootCauseAnalyzer::new();
        rca.analyze("err: cause");
        rca.reset();
        assert_eq!(rca.chain_len(), 0);
        assert_eq!(rca.confidence(), 0.0);
    }

    #[test]
    fn test_ffi_rca_analyze() {
        let len = rca_analyze(5);
        assert!(len >= 0);
    }
}
