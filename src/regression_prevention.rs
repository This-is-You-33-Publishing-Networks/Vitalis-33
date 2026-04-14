//! Regression Prediction — Vitalis v665
//!
//! Predicts regression risk based on code change patterns.
//! Tracks file-level delta lines to estimate risk scores.

use std::sync::{LazyLock, Mutex};
use std::collections::HashMap;

static STATE: LazyLock<Mutex<RegressionPredictor>> = LazyLock::new(|| Mutex::new(RegressionPredictor::new()));

pub struct RegressionPredictor {
    changes: HashMap<String, i64>,
    threshold: i64,
}

impl RegressionPredictor {
    pub fn new() -> Self {
        Self { changes: HashMap::new(), threshold: 50 }
    }

    pub fn record_change(&mut self, file: &str, delta_lines: i64) {
        *self.changes.entry(file.to_string()).or_insert(0) += delta_lines.abs();
    }

    pub fn predict_risk(&self) -> f64 {
        if self.changes.is_empty() { return 0.0; }
        let total: i64 = self.changes.values().sum();
        (total as f64 / (self.threshold as f64 * self.changes.len() as f64)).min(1.0)
    }

    pub fn high_risk_files(&self) -> Vec<String> {
        self.changes.iter()
            .filter(|(_, v)| **v >= self.threshold)
            .map(|(k, _)| k.clone())
            .collect()
    }

    pub fn reset(&mut self) {
        self.changes.clear();
    }
}

impl Default for RegressionPredictor {
    fn default() -> Self { Self::new() }
}

#[unsafe(no_mangle)]
pub extern "C" fn regr_record(file_id: i64, delta: i64) -> i64 {
    let file = format!("file_{file_id}");
    STATE.lock().unwrap().record_change(&file, delta);
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn regr_risk() -> f64 {
    STATE.lock().unwrap().predict_risk()
}

#[unsafe(no_mangle)]
pub extern "C" fn regr_high_risk_count() -> i64 {
    STATE.lock().unwrap().high_risk_files().len() as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn regr_reset() -> i64 {
    STATE.lock().unwrap().reset();
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_changes_zero_risk() {
        let rp = RegressionPredictor::new();
        assert_eq!(rp.predict_risk(), 0.0);
    }

    #[test]
    fn test_small_change_low_risk() {
        let mut rp = RegressionPredictor::new();
        rp.record_change("main.rs", 5);
        assert!(rp.predict_risk() < 1.0);
    }

    #[test]
    fn test_large_change_high_risk() {
        let mut rp = RegressionPredictor::new();
        rp.record_change("core.rs", 200);
        assert!(rp.predict_risk() > 0.5);
    }

    #[test]
    fn test_high_risk_files() {
        let mut rp = RegressionPredictor::new();
        rp.record_change("big.rs", 100);
        rp.record_change("small.rs", 2);
        let hr = rp.high_risk_files();
        assert!(hr.contains(&"big.rs".to_string()));
        assert!(!hr.contains(&"small.rs".to_string()));
    }

    #[test]
    fn test_reset() {
        let mut rp = RegressionPredictor::new();
        rp.record_change("x.rs", 100);
        rp.reset();
        assert_eq!(rp.predict_risk(), 0.0);
    }

    #[test]
    fn test_ffi_regr_record() {
        regr_reset();
        regr_record(1, 10);
        assert!(regr_risk() >= 0.0);
    }
}
