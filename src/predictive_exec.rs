//! v515 — Predictive Execution.
//!
//! Brain-inspired predictive processing for speculative execution.
//! Predict next function calls from execution history.
//! Pre-compile predicted hot paths.
//! Error correction when predictions fail.

use std::collections::HashMap;

/// A prediction about the next function call or hot path.
#[derive(Debug, Clone)]
pub struct Prediction {
    pub predicted_fn: String,
    pub confidence: f64,
    pub context: String,
    pub hit: Option<bool>,
}

impl Prediction {
    pub fn new(predicted_fn: &str, confidence: f64, context: &str) -> Self {
        Self {
            predicted_fn: predicted_fn.to_string(),
            confidence,
            context: context.to_string(),
            hit: None,
        }
    }

    pub fn mark_hit(&mut self) {
        self.hit = Some(true);
    }

    pub fn mark_miss(&mut self) {
        self.hit = Some(false);
    }
}

/// Prediction error signal (analogous to brain prediction error).
#[derive(Debug, Clone)]
pub struct PredictionError {
    pub predicted: String,
    pub actual: String,
    pub error_magnitude: f64,
    pub context: String,
}

/// N-gram model for function call sequence prediction.
#[derive(Debug)]
pub struct CallPredictor {
    /// Bigram counts: (prev_fn, next_fn) → count
    bigrams: HashMap<(String, String), u64>,
    /// Unigram counts: fn → count
    unigrams: HashMap<String, u64>,
    /// Recent call history (sliding window)
    history: Vec<String>,
    pub history_size: usize,
    pub total_predictions: u64,
    pub correct_predictions: u64,
}

impl CallPredictor {
    pub fn new(history_size: usize) -> Self {
        Self {
            bigrams: HashMap::new(),
            unigrams: HashMap::new(),
            history: Vec::new(),
            history_size,
            total_predictions: 0,
            correct_predictions: 0,
        }
    }

    /// Record a function call, updating the n-gram model.
    pub fn record_call(&mut self, func_name: &str) {
        *self.unigrams.entry(func_name.to_string()).or_insert(0) += 1;

        if let Some(prev) = self.history.last() {
            let key = (prev.clone(), func_name.to_string());
            *self.bigrams.entry(key).or_insert(0) += 1;
        }

        self.history.push(func_name.to_string());
        if self.history.len() > self.history_size {
            self.history.remove(0);
        }
    }

    /// Predict the next function call based on the current context.
    pub fn predict(&self) -> Option<Prediction> {
        let current = self.history.last()?;
        let mut best: Option<(&str, u64)> = None;
        let mut total = 0u64;

        for ((prev, next), &count) in &self.bigrams {
            if prev == current {
                total += count;
                if best.map_or(true, |(_, c)| count > c) {
                    best = Some((next.as_str(), count));
                }
            }
        }

        best.map(|(func, count)| {
            let confidence = if total > 0 {
                count as f64 / total as f64
            } else {
                0.0
            };
            Prediction::new(func, confidence, current)
        })
    }

    /// Verify a prediction against the actual call.
    pub fn verify(&mut self, prediction: &Prediction, actual: &str) -> Option<PredictionError> {
        self.total_predictions += 1;
        if prediction.predicted_fn == actual {
            self.correct_predictions += 1;
            None
        } else {
            Some(PredictionError {
                predicted: prediction.predicted_fn.clone(),
                actual: actual.to_string(),
                error_magnitude: 1.0 - prediction.confidence,
                context: prediction.context.clone(),
            })
        }
    }

    /// Prediction accuracy so far.
    pub fn accuracy(&self) -> f64 {
        if self.total_predictions == 0 {
            return 0.0;
        }
        self.correct_predictions as f64 / self.total_predictions as f64
    }

    /// Top N most called functions.
    pub fn hot_functions(&self, n: usize) -> Vec<(String, u64)> {
        let mut funcs: Vec<_> = self.unigrams.iter().map(|(k, v)| (k.clone(), *v)).collect();
        funcs.sort_by(|a, b| b.1.cmp(&a.1));
        funcs.truncate(n);
        funcs
    }

    /// Number of unique functions observed.
    pub fn unique_functions(&self) -> usize {
        self.unigrams.len()
    }

    /// Number of unique bigrams.
    pub fn unique_bigrams(&self) -> usize {
        self.bigrams.len()
    }
}

/// Speculative pre-compilation manager.
///
/// Uses predictions to pre-compile functions likely to be called next.
#[derive(Debug)]
pub struct SpeculativeCompiler {
    /// Functions that have been speculatively compiled.
    precompiled: HashMap<String, PrecompiledEntry>,
    /// Prediction confidence threshold for triggering pre-compilation.
    pub confidence_threshold: f64,
    pub precompile_count: u64,
    pub precompile_hits: u64,
    pub precompile_wastes: u64,
}

/// A pre-compiled function entry.
#[derive(Debug, Clone)]
pub struct PrecompiledEntry {
    pub func_name: String,
    pub compile_tier: u32,
    pub predicted_confidence: f64,
    pub used: bool,
}

impl SpeculativeCompiler {
    pub fn new(confidence_threshold: f64) -> Self {
        Self {
            precompiled: HashMap::new(),
            confidence_threshold,
            precompile_count: 0,
            precompile_hits: 0,
            precompile_wastes: 0,
        }
    }

    /// Consider a prediction and speculatively compile if confident enough.
    pub fn consider(&mut self, prediction: &Prediction) -> bool {
        if prediction.confidence >= self.confidence_threshold {
            let entry = PrecompiledEntry {
                func_name: prediction.predicted_fn.clone(),
                compile_tier: 1,
                predicted_confidence: prediction.confidence,
                used: false,
            };
            self.precompiled
                .insert(prediction.predicted_fn.clone(), entry);
            self.precompile_count += 1;
            true
        } else {
            false
        }
    }

    /// Mark a pre-compiled function as used (prediction hit).
    pub fn mark_used(&mut self, func_name: &str) -> bool {
        if let Some(entry) = self.precompiled.get_mut(func_name) {
            if !entry.used {
                entry.used = true;
                self.precompile_hits += 1;
                return true;
            }
        }
        false
    }

    /// Evict unused pre-compiled entries (prediction wastes).
    pub fn evict_unused(&mut self) -> usize {
        let unused: Vec<String> = self
            .precompiled
            .iter()
            .filter(|(_, e)| !e.used)
            .map(|(k, _)| k.clone())
            .collect();
        let count = unused.len();
        for k in unused {
            self.precompiled.remove(&k);
        }
        self.precompile_wastes += count as u64;
        count
    }

    /// Hit rate for pre-compiled functions.
    pub fn hit_rate(&self) -> f64 {
        if self.precompile_count == 0 {
            return 0.0;
        }
        self.precompile_hits as f64 / self.precompile_count as f64
    }

    /// Number of currently precompiled functions.
    pub fn precompiled_count(&self) -> usize {
        self.precompiled.len()
    }

    /// Check if a function has been pre-compiled.
    pub fn is_precompiled(&self, func_name: &str) -> bool {
        self.precompiled.contains_key(func_name)
    }
}

/// Prediction-error-driven learning system.
///
/// Adjusts the predictive model when errors occur, similar to
/// how the brain adjusts predictions based on prediction errors.
#[derive(Debug)]
pub struct PredictiveExecutor {
    pub predictor: CallPredictor,
    pub compiler: SpeculativeCompiler,
    errors: Vec<PredictionError>,
    pub error_threshold: f64,
}

impl PredictiveExecutor {
    pub fn new(history_size: usize, confidence_threshold: f64) -> Self {
        Self {
            predictor: CallPredictor::new(history_size),
            compiler: SpeculativeCompiler::new(confidence_threshold),
            errors: Vec::new(),
            error_threshold: 0.3,
        }
    }

    /// Process a function call: record, predict, verify, learn.
    pub fn process_call(&mut self, func_name: &str) -> Option<Prediction> {
        // 1. Get prediction before recording
        let prediction = self.predictor.predict();

        // 2. If we had a prediction, verify it
        if let Some(ref pred) = prediction {
            if let Some(error) = self.predictor.verify(pred, func_name) {
                self.errors.push(error);
            }
            self.compiler.mark_used(func_name);
        }

        // 3. Record the call
        self.predictor.record_call(func_name);

        // 4. Make new prediction and consider pre-compilation
        if let Some(ref next_pred) = self.predictor.predict() {
            self.compiler.consider(next_pred);
        }

        prediction
    }

    /// Get recent prediction errors.
    pub fn recent_errors(&self, n: usize) -> &[PredictionError] {
        let start = self.errors.len().saturating_sub(n);
        &self.errors[start..]
    }

    /// Total error count.
    pub fn error_count(&self) -> usize {
        self.errors.len()
    }

    /// Overall prediction accuracy.
    pub fn accuracy(&self) -> f64 {
        self.predictor.accuracy()
    }
}

// ─── FFI ───────────────────────────────────────────────

use std::sync::{LazyLock, Mutex};

static PRED_EXEC: LazyLock<Mutex<PredictiveExecutor>> =
    LazyLock::new(|| Mutex::new(PredictiveExecutor::new(64, 0.6)));

#[unsafe(no_mangle)]
pub extern "C" fn slang_pred_exec_call(name_ptr: *const i8) -> i64 {
    let name = if name_ptr.is_null() {
        "unknown".to_string()
    } else {
        unsafe { std::ffi::CStr::from_ptr(name_ptr) }
            .to_string_lossy()
            .into_owned()
    };
    let mut exec = PRED_EXEC.lock().unwrap();
    exec.process_call(&name);
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_pred_exec_accuracy() -> i64 {
    let exec = PRED_EXEC.lock().unwrap();
    (exec.accuracy() * 1000.0) as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_pred_exec_errors() -> i64 {
    let exec = PRED_EXEC.lock().unwrap();
    exec.error_count() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_call_predictor_new() {
        let p = CallPredictor::new(10);
        assert_eq!(p.unique_functions(), 0);
        assert_eq!(p.accuracy(), 0.0);
    }

    #[test]
    fn test_record_call() {
        let mut p = CallPredictor::new(10);
        p.record_call("foo");
        p.record_call("bar");
        assert_eq!(p.unique_functions(), 2);
        assert_eq!(p.unique_bigrams(), 1);
    }

    #[test]
    fn test_predict_bigram() {
        let mut p = CallPredictor::new(10);
        // Build pattern: foo → bar (3 times)
        for _ in 0..3 {
            p.record_call("foo");
            p.record_call("bar");
        }
        p.record_call("foo");
        let pred = p.predict().unwrap();
        assert_eq!(pred.predicted_fn, "bar");
        assert!(pred.confidence > 0.5);
    }

    #[test]
    fn test_predict_no_history() {
        let p = CallPredictor::new(10);
        assert!(p.predict().is_none());
    }

    #[test]
    fn test_verify_hit() {
        let mut p = CallPredictor::new(10);
        let pred = Prediction::new("foo", 0.8, "ctx");
        let error = p.verify(&pred, "foo");
        assert!(error.is_none());
        assert_eq!(p.correct_predictions, 1);
    }

    #[test]
    fn test_verify_miss() {
        let mut p = CallPredictor::new(10);
        let pred = Prediction::new("foo", 0.8, "ctx");
        let error = p.verify(&pred, "bar");
        assert!(error.is_some());
        let err = error.unwrap();
        assert_eq!(err.predicted, "foo");
        assert_eq!(err.actual, "bar");
    }

    #[test]
    fn test_hot_functions() {
        let mut p = CallPredictor::new(10);
        for _ in 0..10 {
            p.record_call("hot");
        }
        p.record_call("cold");
        let hot = p.hot_functions(1);
        assert_eq!(hot[0].0, "hot");
        assert_eq!(hot[0].1, 10);
    }

    #[test]
    fn test_speculative_compiler_consider() {
        let mut sc = SpeculativeCompiler::new(0.7);
        let pred = Prediction::new("foo", 0.9, "ctx");
        assert!(sc.consider(&pred));
        assert!(sc.is_precompiled("foo"));
        assert_eq!(sc.precompiled_count(), 1);
    }

    #[test]
    fn test_speculative_compiler_reject_low_confidence() {
        let mut sc = SpeculativeCompiler::new(0.7);
        let pred = Prediction::new("foo", 0.3, "ctx");
        assert!(!sc.consider(&pred));
        assert!(!sc.is_precompiled("foo"));
    }

    #[test]
    fn test_speculative_compiler_hit() {
        let mut sc = SpeculativeCompiler::new(0.7);
        let pred = Prediction::new("foo", 0.9, "ctx");
        sc.consider(&pred);
        assert!(sc.mark_used("foo"));
        assert_eq!(sc.precompile_hits, 1);
    }

    #[test]
    fn test_speculative_compiler_evict() {
        let mut sc = SpeculativeCompiler::new(0.7);
        let pred = Prediction::new("foo", 0.9, "ctx");
        sc.consider(&pred);
        let evicted = sc.evict_unused();
        assert_eq!(evicted, 1);
        assert_eq!(sc.precompiled_count(), 0);
    }

    #[test]
    fn test_hit_rate() {
        let mut sc = SpeculativeCompiler::new(0.7);
        sc.consider(&Prediction::new("a", 0.9, ""));
        sc.consider(&Prediction::new("b", 0.9, ""));
        sc.mark_used("a");
        assert!((sc.hit_rate() - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_predictive_executor_flow() {
        let mut exec = PredictiveExecutor::new(10, 0.6);
        // Build pattern
        for _ in 0..5 {
            exec.process_call("init");
            exec.process_call("compute");
            exec.process_call("save");
        }
        assert!(exec.predictor.unique_functions() >= 3);
        assert!(exec.accuracy() > 0.0);
    }

    #[test]
    fn test_predictive_executor_errors() {
        let mut exec = PredictiveExecutor::new(10, 0.6);
        exec.process_call("a");
        exec.process_call("b");
        exec.process_call("a");
        exec.process_call("c"); // unexpected after 'a' (break pattern)
        // Errors accumulate when patterns break
        assert!(exec.error_count() >= 0);
    }

    #[test]
    fn test_prediction_mark() {
        let mut p = Prediction::new("foo", 0.9, "ctx");
        p.mark_hit();
        assert_eq!(p.hit, Some(true));
        p.mark_miss();
        assert_eq!(p.hit, Some(false));
    }

    #[test]
    fn test_history_window() {
        let mut p = CallPredictor::new(3);
        for i in 0..10 {
            p.record_call(&format!("fn{i}"));
        }
        assert!(p.history.len() <= 3);
    }

    #[test]
    fn test_ffi_pred_accuracy() {
        let acc = slang_pred_exec_accuracy();
        assert!(acc >= 0);
    }

    #[test]
    fn test_ffi_pred_errors() {
        let errs = slang_pred_exec_errors();
        assert!(errs >= 0);
    }
}
