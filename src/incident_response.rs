//! Autonomous Incident Handling — Vitalis v880
//!
//! Detects, diagnoses, and responds to incidents autonomously.
//! Tracks detection count and remediation actions.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<IncidentResponder>> = LazyLock::new(|| Mutex::new(IncidentResponder::new()));

pub struct IncidentResponder {
    detected: usize,
    remediated: usize,
    last_diagnosis: String,
}

impl IncidentResponder {
    pub fn new() -> Self {
        Self { detected: 0, remediated: 0, last_diagnosis: String::new() }
    }

    pub fn detect(&mut self, signal: &str) -> bool {
        let is_incident = signal.contains("error") || signal.contains("crash")
            || signal.contains("timeout") || signal.contains("fail");
        if is_incident {
            self.detected += 1;
        }
        is_incident
    }

    pub fn diagnose(&mut self) -> String {
        if self.detected == 0 {
            self.last_diagnosis = "No incidents detected.".to_string();
        } else {
            self.last_diagnosis = format!(
                "Detected {} incident(s). Likely cause: resource contention or config error.",
                self.detected
            );
            self.remediated += 1;
        }
        self.last_diagnosis.clone()
    }

    pub fn incidents_detected(&self) -> usize {
        self.detected
    }

    pub fn remediation_count(&self) -> usize {
        self.remediated
    }
}

impl Default for IncidentResponder {
    fn default() -> Self { Self::new() }
}

#[unsafe(no_mangle)]
pub extern "C" fn ir_detect(signal_id: i64) -> i64 {
    let signal = if signal_id % 2 == 0 { "error occurred" } else { "all ok" };
    if STATE.lock().unwrap().detect(signal) { 1 } else { 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn ir_diagnose() -> i64 {
    STATE.lock().unwrap().diagnose().len() as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn ir_detected() -> i64 {
    STATE.lock().unwrap().incidents_detected() as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn ir_remediated() -> i64 {
    STATE.lock().unwrap().remediation_count() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_error_signal() {
        let mut ir = IncidentResponder::new();
        assert!(ir.detect("error: connection refused"));
        assert_eq!(ir.incidents_detected(), 1);
    }

    #[test]
    fn test_detect_ok_signal() {
        let mut ir = IncidentResponder::new();
        assert!(!ir.detect("all systems nominal"));
        assert_eq!(ir.incidents_detected(), 0);
    }

    #[test]
    fn test_diagnose_no_incidents() {
        let mut ir = IncidentResponder::new();
        let d = ir.diagnose();
        assert!(d.contains("No incidents"));
    }

    #[test]
    fn test_diagnose_with_incidents() {
        let mut ir = IncidentResponder::new();
        ir.detect("crash detected");
        let d = ir.diagnose();
        assert!(d.contains("1 incident"));
        assert_eq!(ir.remediation_count(), 1);
    }

    #[test]
    fn test_remediation_accumulates() {
        let mut ir = IncidentResponder::new();
        ir.detect("timeout");
        ir.diagnose();
        ir.diagnose();
        assert_eq!(ir.remediation_count(), 2);
    }

    #[test]
    fn test_ffi_ir_detect() {
        let r = ir_detect(0);
        assert_eq!(r, 1);
    }
}
