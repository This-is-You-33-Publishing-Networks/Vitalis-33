//! Entropy Oracle — Vitalis v1017
//!
//! Predicts information-theoretic complexity of code paths
//! to guide optimization toward low-entropy, high-efficiency regions.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<EoState>> = LazyLock::new(|| Mutex::new(EoState::default()));

#[derive(Default)]
struct EoState {
    measurements: Vec<(String, f64)>,
    predictions: Vec<(String, f64)>,
    total_entropy: f64,
}

pub struct EntropyOracle;

impl EntropyOracle {
    pub fn measure(path: &str, complexity: f64) -> f64 {
        let mut s = STATE.lock().unwrap();
        let entropy = (complexity + 1.0).ln();
        s.measurements.push((path.to_string(), entropy));
        s.total_entropy += entropy;
        entropy
    }

    pub fn predict_entropy(path: &str) -> f64 {
        let mut s = STATE.lock().unwrap();
        let avg = if s.measurements.is_empty() { 1.0 } else { s.total_entropy / s.measurements.len() as f64 };
        let prediction = avg * (1.0 + path.len() as f64 * 0.01);
        s.predictions.push((path.to_string(), prediction));
        prediction
    }

    pub fn lowest_entropy_path() -> Option<String> {
        let s = STATE.lock().unwrap();
        s.measurements.iter().min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal)).map(|(p, _)| p.clone())
    }

    pub fn total_entropy() -> f64 { STATE.lock().unwrap().total_entropy }
    pub fn measurement_count() -> usize { STATE.lock().unwrap().measurements.len() }
    pub fn reset() { *STATE.lock().unwrap() = EoState::default(); }
}

#[unsafe(no_mangle)]
pub extern "C" fn eo_measure(complexity: f64) -> f64 { EntropyOracle::measure("ffi_path", complexity) }
#[unsafe(no_mangle)]
pub extern "C" fn eo_predict() -> f64 { EntropyOracle::predict_entropy("ffi_path") }
#[unsafe(no_mangle)]
pub extern "C" fn eo_total() -> f64 { EntropyOracle::total_entropy() }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_measure() { EntropyOracle::reset(); let e = EntropyOracle::measure("p1", 10.0); assert!(e > 0.0); }
    #[test] fn test_predict() { EntropyOracle::reset(); EntropyOracle::measure("p1", 5.0); let p = EntropyOracle::predict_entropy("p2"); assert!(p > 0.0); }
    #[test] fn test_lowest() { EntropyOracle::reset(); EntropyOracle::measure("a", 1.0); EntropyOracle::measure("b", 100.0); assert_eq!(EntropyOracle::lowest_entropy_path(), Some("a".to_string())); }
    #[test] fn test_total() { EntropyOracle::reset(); EntropyOracle::measure("x", 5.0); assert!(EntropyOracle::total_entropy() > 0.0); }
    #[test] fn test_empty_lowest() { EntropyOracle::reset(); assert!(EntropyOracle::lowest_entropy_path().is_none()); }
    #[test] fn test_ffi() { EntropyOracle::reset(); assert!((eo_total() - 0.0).abs() < 0.01); }
}
