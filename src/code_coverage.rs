//! Code Coverage — v386
//! Line, branch, and path coverage instrumentation and reporting.

use std::sync::{LazyLock, Mutex};
use std::collections::{HashMap, HashSet};

/// Tracks coverage for a single file.
#[derive(Debug, Clone)]
pub struct FileCoverage {
    pub filename: String,
    /// Lines that are executable (instrumented).
    pub executable_lines: HashSet<usize>,
    /// Lines that have been executed.
    pub hit_lines: HashSet<usize>,
    /// Branches: (line, branch_id) → hit count.
    pub branches: HashMap<(usize, u32), u64>,
    /// Branch totals: how many branches at each line.
    pub branch_totals: HashMap<usize, u32>,
    /// Path coverage: executed paths (as hash of branch sequence).
    pub paths_hit: HashSet<u64>,
    pub total_paths: usize,
}

impl FileCoverage {
    pub fn new(filename: &str) -> Self {
        Self {
            filename: filename.to_string(),
            executable_lines: HashSet::new(),
            hit_lines: HashSet::new(),
            branches: HashMap::new(),
            branch_totals: HashMap::new(),
            paths_hit: HashSet::new(),
            total_paths: 0,
        }
    }

    pub fn mark_executable(&mut self, line: usize) {
        self.executable_lines.insert(line);
    }

    pub fn hit_line(&mut self, line: usize) {
        self.hit_lines.insert(line);
    }

    pub fn add_branch(&mut self, line: usize, total_branches: u32) {
        self.branch_totals.insert(line, total_branches);
        for i in 0..total_branches {
            self.branches.entry((line, i)).or_insert(0);
        }
    }

    pub fn hit_branch(&mut self, line: usize, branch_id: u32) {
        *self.branches.entry((line, branch_id)).or_insert(0) += 1;
    }

    pub fn record_path(&mut self, path_hash: u64) {
        self.paths_hit.insert(path_hash);
    }

    pub fn line_coverage_pct(&self) -> f64 {
        if self.executable_lines.is_empty() {
            return 100.0;
        }
        (self.hit_lines.len() as f64 / self.executable_lines.len() as f64) * 100.0
    }

    pub fn branch_coverage_pct(&self) -> f64 {
        let total: usize = self.branch_totals.values().map(|v| *v as usize).sum();
        if total == 0 {
            return 100.0;
        }
        let hit = self.branches.values().filter(|&&count| count > 0).count();
        (hit as f64 / total as f64) * 100.0
    }

    pub fn path_coverage_pct(&self) -> f64 {
        if self.total_paths == 0 {
            return 100.0;
        }
        (self.paths_hit.len() as f64 / self.total_paths as f64) * 100.0
    }

    pub fn uncovered_lines(&self) -> Vec<usize> {
        let mut uncovered: Vec<usize> = self.executable_lines
            .difference(&self.hit_lines)
            .copied()
            .collect();
        uncovered.sort();
        uncovered
    }

    pub fn reset(&mut self) {
        self.hit_lines.clear();
        for val in self.branches.values_mut() {
            *val = 0;
        }
        self.paths_hit.clear();
    }
}

/// Aggregate coverage across multiple files.
#[derive(Debug)]
pub struct CoverageReport {
    files: HashMap<String, FileCoverage>,
}

impl CoverageReport {
    pub fn new() -> Self {
        Self { files: HashMap::new() }
    }

    pub fn add_file(&mut self, coverage: FileCoverage) {
        self.files.insert(coverage.filename.clone(), coverage);
    }

    pub fn get_file(&self, filename: &str) -> Option<&FileCoverage> {
        self.files.get(filename)
    }

    pub fn get_file_mut(&mut self, filename: &str) -> Option<&mut FileCoverage> {
        self.files.get_mut(filename)
    }

    pub fn total_line_coverage(&self) -> f64 {
        let total_exec: usize = self.files.values().map(|f| f.executable_lines.len()).sum();
        let total_hit: usize = self.files.values().map(|f| f.hit_lines.len()).sum();
        if total_exec == 0 { 100.0 } else { (total_hit as f64 / total_exec as f64) * 100.0 }
    }

    pub fn total_branch_coverage(&self) -> f64 {
        let total: usize = self.files.values()
            .flat_map(|f| f.branch_totals.values())
            .map(|v| *v as usize)
            .sum();
        let hit: usize = self.files.values()
            .flat_map(|f| f.branches.values())
            .filter(|&&c| c > 0)
            .count();
        if total == 0 { 100.0 } else { (hit as f64 / total as f64) * 100.0 }
    }

    pub fn file_count(&self) -> usize {
        self.files.len()
    }

    pub fn reset_all(&mut self) {
        for f in self.files.values_mut() {
            f.reset();
        }
    }
}

static COV_REPORT: LazyLock<Mutex<CoverageReport>> =
    LazyLock::new(|| Mutex::new(CoverageReport::new()));

#[unsafe(no_mangle)]
pub extern "C" fn slang_cov_init(file_id: i64) -> i64 {
    let filename = format!("file_{}.sl", file_id);
    let cov = FileCoverage::new(&filename);
    let mut report = COV_REPORT.lock().unwrap();
    report.add_file(cov);
    1
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_cov_mark_line(file_id: i64, line: i64) -> i64 {
    let filename = format!("file_{}.sl", file_id);
    let mut report = COV_REPORT.lock().unwrap();
    if let Some(f) = report.get_file_mut(&filename) {
        f.mark_executable(line as usize);
        1
    } else {
        -1
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_cov_hit_line(file_id: i64, line: i64) -> i64 {
    let filename = format!("file_{}.sl", file_id);
    let mut report = COV_REPORT.lock().unwrap();
    if let Some(f) = report.get_file_mut(&filename) {
        f.hit_line(line as usize);
        1
    } else {
        -1
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_cov_line_pct(file_id: i64) -> f64 {
    let filename = format!("file_{}.sl", file_id);
    let report = COV_REPORT.lock().unwrap();
    if let Some(f) = report.get_file(&filename) {
        f.line_coverage_pct()
    } else {
        -1.0
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_cov_branch_pct(file_id: i64) -> f64 {
    let filename = format!("file_{}.sl", file_id);
    let report = COV_REPORT.lock().unwrap();
    if let Some(f) = report.get_file(&filename) {
        f.branch_coverage_pct()
    } else {
        -1.0
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_cov_total_lines(file_id: i64) -> i64 {
    let filename = format!("file_{}.sl", file_id);
    let report = COV_REPORT.lock().unwrap();
    if let Some(f) = report.get_file(&filename) {
        f.executable_lines.len() as i64
    } else {
        -1
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_cov_hit_lines(file_id: i64) -> i64 {
    let filename = format!("file_{}.sl", file_id);
    let report = COV_REPORT.lock().unwrap();
    if let Some(f) = report.get_file(&filename) {
        f.hit_lines.len() as i64
    } else {
        -1
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_cov_reset(file_id: i64) -> i64 {
    let filename = format!("file_{}.sl", file_id);
    let mut report = COV_REPORT.lock().unwrap();
    if let Some(f) = report.get_file_mut(&filename) {
        f.reset();
        1
    } else {
        -1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_coverage_new() {
        let fc = FileCoverage::new("test.sl");
        assert_eq!(fc.filename, "test.sl");
        assert!(fc.executable_lines.is_empty());
    }

    #[test]
    fn test_mark_executable() {
        let mut fc = FileCoverage::new("test.sl");
        fc.mark_executable(1);
        fc.mark_executable(2);
        fc.mark_executable(3);
        assert_eq!(fc.executable_lines.len(), 3);
    }

    #[test]
    fn test_hit_line() {
        let mut fc = FileCoverage::new("test.sl");
        fc.mark_executable(1);
        fc.mark_executable(2);
        fc.hit_line(1);
        assert_eq!(fc.hit_lines.len(), 1);
    }

    #[test]
    fn test_line_coverage_100pct() {
        let mut fc = FileCoverage::new("test.sl");
        fc.mark_executable(1);
        fc.mark_executable(2);
        fc.hit_line(1);
        fc.hit_line(2);
        assert_eq!(fc.line_coverage_pct(), 100.0);
    }

    #[test]
    fn test_line_coverage_50pct() {
        let mut fc = FileCoverage::new("test.sl");
        fc.mark_executable(1);
        fc.mark_executable(2);
        fc.hit_line(1);
        assert_eq!(fc.line_coverage_pct(), 50.0);
    }

    #[test]
    fn test_line_coverage_empty() {
        let fc = FileCoverage::new("test.sl");
        assert_eq!(fc.line_coverage_pct(), 100.0);
    }

    #[test]
    fn test_branch_coverage() {
        let mut fc = FileCoverage::new("test.sl");
        fc.add_branch(10, 2); // if/else at line 10
        fc.hit_branch(10, 0);
        assert_eq!(fc.branch_coverage_pct(), 50.0);
    }

    #[test]
    fn test_branch_coverage_full() {
        let mut fc = FileCoverage::new("test.sl");
        fc.add_branch(10, 2);
        fc.hit_branch(10, 0);
        fc.hit_branch(10, 1);
        assert_eq!(fc.branch_coverage_pct(), 100.0);
    }

    #[test]
    fn test_branch_coverage_none() {
        let mut fc = FileCoverage::new("test.sl");
        fc.add_branch(10, 2);
        assert_eq!(fc.branch_coverage_pct(), 0.0);
    }

    #[test]
    fn test_branch_coverage_empty() {
        let fc = FileCoverage::new("test.sl");
        assert_eq!(fc.branch_coverage_pct(), 100.0);
    }

    #[test]
    fn test_path_coverage() {
        let mut fc = FileCoverage::new("test.sl");
        fc.total_paths = 4;
        fc.record_path(100);
        fc.record_path(200);
        assert_eq!(fc.path_coverage_pct(), 50.0);
    }

    #[test]
    fn test_uncovered_lines() {
        let mut fc = FileCoverage::new("test.sl");
        fc.mark_executable(1);
        fc.mark_executable(2);
        fc.mark_executable(3);
        fc.hit_line(2);
        assert_eq!(fc.uncovered_lines(), vec![1, 3]);
    }

    #[test]
    fn test_reset() {
        let mut fc = FileCoverage::new("test.sl");
        fc.mark_executable(1);
        fc.hit_line(1);
        fc.add_branch(5, 2);
        fc.hit_branch(5, 0);
        fc.reset();
        assert!(fc.hit_lines.is_empty());
        assert_eq!(*fc.branches.get(&(5, 0)).unwrap(), 0);
    }

    #[test]
    fn test_report_new() {
        let report = CoverageReport::new();
        assert_eq!(report.file_count(), 0);
    }

    #[test]
    fn test_report_add_file() {
        let mut report = CoverageReport::new();
        report.add_file(FileCoverage::new("a.sl"));
        report.add_file(FileCoverage::new("b.sl"));
        assert_eq!(report.file_count(), 2);
    }

    #[test]
    fn test_report_total_line_coverage() {
        let mut report = CoverageReport::new();
        let mut f1 = FileCoverage::new("a.sl");
        f1.mark_executable(1);
        f1.mark_executable(2);
        f1.hit_line(1);
        f1.hit_line(2);
        let mut f2 = FileCoverage::new("b.sl");
        f2.mark_executable(1);
        f2.mark_executable(2);
        f2.hit_line(1);
        report.add_file(f1);
        report.add_file(f2);
        assert_eq!(report.total_line_coverage(), 75.0);
    }

    #[test]
    fn test_report_reset_all() {
        let mut report = CoverageReport::new();
        let mut f = FileCoverage::new("a.sl");
        f.mark_executable(1);
        f.hit_line(1);
        report.add_file(f);
        report.reset_all();
        assert_eq!(report.get_file("a.sl").unwrap().hit_lines.len(), 0);
    }

    #[test]
    fn test_get_file() {
        let mut report = CoverageReport::new();
        report.add_file(FileCoverage::new("test.sl"));
        assert!(report.get_file("test.sl").is_some());
        assert!(report.get_file("missing.sl").is_none());
    }

    #[test]
    fn test_duplicate_hit_line() {
        let mut fc = FileCoverage::new("test.sl");
        fc.mark_executable(1);
        fc.hit_line(1);
        fc.hit_line(1); // duplicate
        assert_eq!(fc.hit_lines.len(), 1);
    }

    #[test]
    fn test_branch_hit_count() {
        let mut fc = FileCoverage::new("test.sl");
        fc.add_branch(10, 2);
        fc.hit_branch(10, 0);
        fc.hit_branch(10, 0);
        fc.hit_branch(10, 0);
        assert_eq!(*fc.branches.get(&(10, 0)).unwrap(), 3);
    }

    #[test]
    fn test_path_dedup() {
        let mut fc = FileCoverage::new("test.sl");
        fc.total_paths = 2;
        fc.record_path(100);
        fc.record_path(100); // duplicate
        assert_eq!(fc.paths_hit.len(), 1);
    }
}
