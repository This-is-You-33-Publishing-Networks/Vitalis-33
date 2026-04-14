//! Statistical Fault Localization — Vitalis v655
//!
//! Uses statistical correlation between test outcomes and code regions
//! to rank suspicious locations. Implements Tarantula-style scoring.

use std::sync::{LazyLock, Mutex};
use std::collections::HashMap;

static STATE: LazyLock<Mutex<FaultLocalizer>> = LazyLock::new(|| Mutex::new(FaultLocalizer::new()));

pub struct FaultLocalizer {
    region_pass: HashMap<String, usize>,
    region_fail: HashMap<String, usize>,
    total: usize,
}

impl FaultLocalizer {
    pub fn new() -> Self {
        Self {
            region_pass: HashMap::new(),
            region_fail: HashMap::new(),
            total: 0,
        }
    }

    pub fn record_test(&mut self, passed: bool, code_region: &str) {
        self.total += 1;
        if passed {
            *self.region_pass.entry(code_region.to_string()).or_insert(0) += 1;
        } else {
            *self.region_fail.entry(code_region.to_string()).or_insert(0) += 1;
        }
    }

    pub fn rank_regions(&self) -> Vec<(String, f64)> {
        let total_fail: usize = self.region_fail.values().sum();
        let total_pass: usize = self.region_pass.values().sum();
        let mut scores: Vec<(String, f64)> = self.region_fail.keys()
            .chain(self.region_pass.keys())
            .cloned()
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .map(|r| {
                let f = *self.region_fail.get(&r).unwrap_or(&0) as f64;
                let p = *self.region_pass.get(&r).unwrap_or(&0) as f64;
                let tf = if total_fail > 0 { f / total_fail as f64 } else { 0.0 };
                let tp = if total_pass > 0 { p / total_pass as f64 } else { 0.0 };
                let score = if tf + tp == 0.0 { 0.0 } else { tf / (tf + tp) };
                (r, score)
            })
            .collect();
        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scores
    }

    pub fn top_suspect(&self) -> Option<String> {
        self.rank_regions().into_iter().next().map(|(r, _)| r)
    }

    pub fn total_tests(&self) -> usize {
        self.total
    }

    pub fn reset(&mut self) {
        self.region_pass.clear();
        self.region_fail.clear();
        self.total = 0;
    }
}

impl Default for FaultLocalizer {
    fn default() -> Self { Self::new() }
}

#[unsafe(no_mangle)]
pub extern "C" fn fault_record(passed: i64, region_id: i64) -> i64 {
    let region = format!("region_{region_id}");
    STATE.lock().unwrap().record_test(passed != 0, &region);
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn fault_top_suspect_score() -> f64 {
    let s = STATE.lock().unwrap();
    s.rank_regions().first().map(|(_, sc)| *sc).unwrap_or(0.0)
}

#[unsafe(no_mangle)]
pub extern "C" fn fault_total_tests() -> i64 {
    STATE.lock().unwrap().total_tests() as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn fault_reset() -> i64 {
    STATE.lock().unwrap().reset();
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_and_total() {
        let mut fl = FaultLocalizer::new();
        fl.record_test(true, "region_a");
        fl.record_test(false, "region_b");
        assert_eq!(fl.total_tests(), 2);
    }

    #[test]
    fn test_top_suspect_after_failures() {
        let mut fl = FaultLocalizer::new();
        fl.record_test(false, "buggy_fn");
        fl.record_test(false, "buggy_fn");
        fl.record_test(true, "good_fn");
        let top = fl.top_suspect();
        assert_eq!(top, Some("buggy_fn".to_string()));
    }

    #[test]
    fn test_rank_regions_ordering() {
        let mut fl = FaultLocalizer::new();
        fl.record_test(false, "a");
        fl.record_test(true, "b");
        let ranked = fl.rank_regions();
        assert!(!ranked.is_empty());
        assert!(ranked[0].1 >= ranked.last().unwrap().1);
    }

    #[test]
    fn test_no_tests_no_suspect() {
        let fl = FaultLocalizer::new();
        assert!(fl.top_suspect().is_none());
    }

    #[test]
    fn test_reset_clears_data() {
        let mut fl = FaultLocalizer::new();
        fl.record_test(false, "x");
        fl.reset();
        assert_eq!(fl.total_tests(), 0);
        assert!(fl.top_suspect().is_none());
    }

    #[test]
    fn test_ffi_fault_record_and_total() {
        fault_reset();
        fault_record(0, 42);
        assert!(fault_total_tests() >= 1);
    }
}
