//! Metrics Engine — v394
//! Prometheus-style metrics: counters, gauges, histograms, and percentile computation.

use std::sync::{LazyLock, Mutex};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum MetricValue {
    Counter(u64),
    Gauge(f64),
    Histogram(HistogramData),
}

#[derive(Debug, Clone)]
pub struct HistogramData {
    pub values: Vec<f64>,
    pub buckets: Vec<(f64, u64)>,
    pub sum: f64,
    pub count: u64,
}

impl HistogramData {
    pub fn new(bucket_bounds: &[f64]) -> Self {
        let buckets = bucket_bounds.iter().map(|&b| (b, 0u64)).collect();
        Self { values: Vec::new(), buckets, sum: 0.0, count: 0 }
    }

    pub fn observe(&mut self, value: f64) {
        self.values.push(value);
        self.sum += value;
        self.count += 1;
        for bucket in &mut self.buckets {
            if value <= bucket.0 {
                bucket.1 += 1;
            }
        }
    }

    pub fn percentile(&self, p: f64) -> f64 {
        if self.values.is_empty() {
            return 0.0;
        }
        let mut sorted = self.values.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let idx = ((p / 100.0) * (sorted.len() - 1) as f64).round() as usize;
        sorted[idx.min(sorted.len() - 1)]
    }

    pub fn mean(&self) -> f64 {
        if self.count == 0 { 0.0 } else { self.sum / self.count as f64 }
    }

    pub fn min(&self) -> f64 {
        self.values.iter().cloned().fold(f64::INFINITY, f64::min)
    }

    pub fn max(&self) -> f64 {
        self.values.iter().cloned().fold(f64::NEG_INFINITY, f64::max)
    }
}

#[derive(Debug)]
pub struct MetricsRegistry {
    counters: HashMap<String, u64>,
    gauges: HashMap<String, f64>,
    histograms: HashMap<String, HistogramData>,
}

impl MetricsRegistry {
    pub fn new() -> Self {
        Self {
            counters: HashMap::new(),
            gauges: HashMap::new(),
            histograms: HashMap::new(),
        }
    }

    pub fn counter_inc(&mut self, name: &str, delta: u64) {
        *self.counters.entry(name.to_string()).or_insert(0) += delta;
    }

    pub fn counter_get(&self, name: &str) -> u64 {
        self.counters.get(name).copied().unwrap_or(0)
    }

    pub fn gauge_set(&mut self, name: &str, value: f64) {
        self.gauges.insert(name.to_string(), value);
    }

    pub fn gauge_get(&self, name: &str) -> f64 {
        self.gauges.get(name).copied().unwrap_or(0.0)
    }

    pub fn gauge_inc(&mut self, name: &str, delta: f64) {
        let val = self.gauges.entry(name.to_string()).or_insert(0.0);
        *val += delta;
    }

    pub fn histogram_observe(&mut self, name: &str, value: f64) {
        let hist = self.histograms
            .entry(name.to_string())
            .or_insert_with(|| {
                HistogramData::new(&[0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0])
            });
        hist.observe(value);
    }

    pub fn histogram_get(&self, name: &str) -> Option<&HistogramData> {
        self.histograms.get(name)
    }

    pub fn total_metrics(&self) -> usize {
        self.counters.len() + self.gauges.len() + self.histograms.len()
    }

    pub fn reset(&mut self) {
        self.counters.clear();
        self.gauges.clear();
        self.histograms.clear();
    }
}

static METRICS: LazyLock<Mutex<MetricsRegistry>> =
    LazyLock::new(|| Mutex::new(MetricsRegistry::new()));

pub extern "C" fn slang_metric_counter(name_hash: i64, delta: i64) -> i64 {
    let name = format!("counter_{}", name_hash);
    let mut reg = METRICS.lock().unwrap();
    reg.counter_inc(&name, delta.max(0) as u64);
    reg.counter_get(&name) as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_metric_inc(name_hash: i64) -> i64 {
    let name = format!("counter_{}", name_hash);
    let mut reg = METRICS.lock().unwrap();
    reg.counter_inc(&name, 1);
    reg.counter_get(&name) as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_metric_gauge_set(name_hash: i64, value: f64) -> i64 {
    let name = format!("gauge_{}", name_hash);
    let mut reg = METRICS.lock().unwrap();
    reg.gauge_set(&name, value);
    1
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_metric_gauge_get(name_hash: i64) -> f64 {
    let name = format!("gauge_{}", name_hash);
    let reg = METRICS.lock().unwrap();
    reg.gauge_get(&name)
}

pub extern "C" fn slang_metric_histogram(name_hash: i64, value: f64) -> i64 {
    let name = format!("hist_{}", name_hash);
    let mut reg = METRICS.lock().unwrap();
    reg.histogram_observe(&name, value);
    1
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_metric_p50(name_hash: i64) -> f64 {
    let name = format!("hist_{}", name_hash);
    let reg = METRICS.lock().unwrap();
    reg.histogram_get(&name).map(|h| h.percentile(50.0)).unwrap_or(0.0)
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_metric_p99(name_hash: i64) -> f64 {
    let name = format!("hist_{}", name_hash);
    let reg = METRICS.lock().unwrap();
    reg.histogram_get(&name).map(|h| h.percentile(99.0)).unwrap_or(0.0)
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_metric_count(name_hash: i64) -> i64 {
    let name = format!("hist_{}", name_hash);
    let reg = METRICS.lock().unwrap();
    reg.histogram_get(&name).map(|h| h.count as i64).unwrap_or(0)
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_metric_sum(name_hash: i64) -> f64 {
    let name = format!("hist_{}", name_hash);
    let reg = METRICS.lock().unwrap();
    reg.histogram_get(&name).map(|h| h.sum).unwrap_or(0.0)
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_metric_reset() -> i64 {
    let mut reg = METRICS.lock().unwrap();
    reg.reset();
    1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_counter_inc() {
        let mut reg = MetricsRegistry::new();
        reg.counter_inc("requests", 1);
        assert_eq!(reg.counter_get("requests"), 1);
    }

    #[test]
    fn test_counter_multi_inc() {
        let mut reg = MetricsRegistry::new();
        reg.counter_inc("requests", 5);
        reg.counter_inc("requests", 3);
        assert_eq!(reg.counter_get("requests"), 8);
    }

    #[test]
    fn test_counter_default() {
        let reg = MetricsRegistry::new();
        assert_eq!(reg.counter_get("missing"), 0);
    }

    #[test]
    fn test_gauge_set_get() {
        let mut reg = MetricsRegistry::new();
        reg.gauge_set("cpu", 75.5);
        assert_eq!(reg.gauge_get("cpu"), 75.5);
    }

    #[test]
    fn test_gauge_overwrite() {
        let mut reg = MetricsRegistry::new();
        reg.gauge_set("cpu", 75.0);
        reg.gauge_set("cpu", 80.0);
        assert_eq!(reg.gauge_get("cpu"), 80.0);
    }

    #[test]
    fn test_gauge_inc() {
        let mut reg = MetricsRegistry::new();
        reg.gauge_set("conns", 10.0);
        reg.gauge_inc("conns", 5.0);
        assert_eq!(reg.gauge_get("conns"), 15.0);
    }

    #[test]
    fn test_gauge_default() {
        let reg = MetricsRegistry::new();
        assert_eq!(reg.gauge_get("missing"), 0.0);
    }

    #[test]
    fn test_histogram_observe() {
        let mut reg = MetricsRegistry::new();
        reg.histogram_observe("latency", 0.1);
        reg.histogram_observe("latency", 0.2);
        let h = reg.histogram_get("latency").unwrap();
        assert_eq!(h.count, 2);
    }

    #[test]
    fn test_histogram_sum() {
        let mut reg = MetricsRegistry::new();
        reg.histogram_observe("latency", 0.1);
        reg.histogram_observe("latency", 0.3);
        let h = reg.histogram_get("latency").unwrap();
        assert!((h.sum - 0.4).abs() < 1e-10);
    }

    #[test]
    fn test_histogram_mean() {
        let mut reg = MetricsRegistry::new();
        reg.histogram_observe("latency", 1.0);
        reg.histogram_observe("latency", 3.0);
        let h = reg.histogram_get("latency").unwrap();
        assert_eq!(h.mean(), 2.0);
    }

    #[test]
    fn test_histogram_percentile_50() {
        let mut reg = MetricsRegistry::new();
        for i in 1..=100 {
            reg.histogram_observe("latency", i as f64);
        }
        let h = reg.histogram_get("latency").unwrap();
        let p50 = h.percentile(50.0);
        assert!((p50 - 50.0).abs() <= 1.0);
    }

    #[test]
    fn test_histogram_percentile_99() {
        let mut reg = MetricsRegistry::new();
        for i in 1..=100 {
            reg.histogram_observe("latency", i as f64);
        }
        let h = reg.histogram_get("latency").unwrap();
        let p99 = h.percentile(99.0);
        assert!(p99 >= 98.0);
    }

    #[test]
    fn test_histogram_min_max() {
        let mut reg = MetricsRegistry::new();
        reg.histogram_observe("lat", 5.0);
        reg.histogram_observe("lat", 1.0);
        reg.histogram_observe("lat", 10.0);
        let h = reg.histogram_get("lat").unwrap();
        assert_eq!(h.min(), 1.0);
        assert_eq!(h.max(), 10.0);
    }

    #[test]
    fn test_histogram_buckets() {
        let mut reg = MetricsRegistry::new();
        reg.histogram_observe("lat", 0.003);
        reg.histogram_observe("lat", 0.05);
        let h = reg.histogram_get("lat").unwrap();
        // 0.003 <= 0.005, so first bucket should have count 1.
        assert!(h.buckets[0].1 >= 1);
    }

    #[test]
    fn test_histogram_empty() {
        let h = HistogramData::new(&[1.0, 5.0, 10.0]);
        assert_eq!(h.count, 0);
        assert_eq!(h.mean(), 0.0);
        assert_eq!(h.percentile(50.0), 0.0);
    }

    #[test]
    fn test_total_metrics() {
        let mut reg = MetricsRegistry::new();
        reg.counter_inc("a", 1);
        reg.gauge_set("b", 1.0);
        reg.histogram_observe("c", 1.0);
        assert_eq!(reg.total_metrics(), 3);
    }

    #[test]
    fn test_reset() {
        let mut reg = MetricsRegistry::new();
        reg.counter_inc("a", 1);
        reg.gauge_set("b", 1.0);
        reg.reset();
        assert_eq!(reg.total_metrics(), 0);
    }

    #[test]
    fn test_multiple_counters() {
        let mut reg = MetricsRegistry::new();
        reg.counter_inc("a", 1);
        reg.counter_inc("b", 2);
        assert_eq!(reg.counter_get("a"), 1);
        assert_eq!(reg.counter_get("b"), 2);
    }

    #[test]
    fn test_histogram_single_value() {
        let mut h = HistogramData::new(&[1.0]);
        h.observe(0.5);
        assert_eq!(h.percentile(50.0), 0.5);
        assert_eq!(h.percentile(99.0), 0.5);
    }

    #[test]
    fn test_gauge_negative() {
        let mut reg = MetricsRegistry::new();
        reg.gauge_set("temp", -10.5);
        assert_eq!(reg.gauge_get("temp"), -10.5);
    }
}
