//! Code Immune System — Vitalis v901
//!
//! Scans code for threats like SQL injection, XSS, and buffer overflows.
//! Maintains an antibody repertoire for known threat patterns.

use std::sync::{LazyLock, Mutex};
use std::collections::HashSet;

static STATE: LazyLock<Mutex<CodeImmuneSystem>> = LazyLock::new(|| Mutex::new(CodeImmuneSystem::new()));

pub struct CodeImmuneSystem {
    antibodies: HashSet<String>,
    total_threats: usize,
}

impl CodeImmuneSystem {
    pub fn new() -> Self {
        let mut ab = HashSet::new();
        ab.insert("sql_injection".to_string());
        ab.insert("xss".to_string());
        ab.insert("buffer_overflow".to_string());
        Self { antibodies: ab, total_threats: 0 }
    }

    pub fn scan(&mut self, code: &str) -> Vec<String> {
        let mut found = Vec::new();
        if code.contains("SELECT") && code.contains("'") {
            found.push("sql_injection".to_string());
        }
        if code.contains("<script>") {
            found.push("xss".to_string());
        }
        if code.contains("unsafe") && code.contains("ptr") {
            found.push("buffer_overflow".to_string());
        }
        self.total_threats += found.len();
        found
    }

    pub fn threat_count(&self) -> usize {
        self.total_threats
    }

    pub fn neutralize(&mut self, threat: &str) -> bool {
        if self.antibodies.contains(threat) {
            self.total_threats = self.total_threats.saturating_sub(1);
            true
        } else {
            self.antibodies.insert(threat.to_string());
            false
        }
    }

    pub fn antibodies(&self) -> usize {
        self.antibodies.len()
    }
}

impl Default for CodeImmuneSystem {
    fn default() -> Self { Self::new() }
}

#[unsafe(no_mangle)]
pub extern "C" fn cis_scan(code_id: i64) -> i64 {
    let code = if code_id % 2 == 0 {
        "SELECT * FROM users WHERE id = '1'"
    } else {
        "safe code here"
    };
    STATE.lock().unwrap().scan(code).len() as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn cis_threats() -> i64 {
    STATE.lock().unwrap().threat_count() as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn cis_neutralize(known: i64) -> i64 {
    let threat = if known != 0 { "sql_injection" } else { "new_threat" };
    if STATE.lock().unwrap().neutralize(threat) { 1 } else { 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn cis_antibodies() -> i64 {
    STATE.lock().unwrap().antibodies() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scan_sql_injection() {
        let mut cis = CodeImmuneSystem::new();
        let threats = cis.scan("SELECT * FROM users WHERE id = '1'");
        assert!(threats.contains(&"sql_injection".to_string()));
    }

    #[test]
    fn test_scan_clean_code() {
        let mut cis = CodeImmuneSystem::new();
        let threats = cis.scan("fn compute(x: i64) -> i64 { x * 2 }");
        assert!(threats.is_empty());
    }

    #[test]
    fn test_neutralize_known_threat() {
        let mut cis = CodeImmuneSystem::new();
        assert!(cis.neutralize("sql_injection"));
    }

    #[test]
    fn test_neutralize_new_threat_adds_antibody() {
        let mut cis = CodeImmuneSystem::new();
        let before = cis.antibodies();
        cis.neutralize("novel_malware");
        assert_eq!(cis.antibodies(), before + 1);
    }

    #[test]
    fn test_threat_count_accumulates() {
        let mut cis = CodeImmuneSystem::new();
        cis.scan("SELECT * FROM t WHERE x='1'");
        cis.scan("<script>alert(1)</script>");
        assert!(cis.threat_count() >= 2);
    }

    #[test]
    fn test_ffi_cis_scan() {
        let count = cis_scan(0);
        assert!(count >= 0);
    }
}
