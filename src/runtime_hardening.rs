//! Runtime hardening primitives: crash reports, panic boundaries, and fault injection.

use std::collections::BTreeMap;
use std::panic::{self, UnwindSafe};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeBoundary {
    Repl,
    Ide,
    Ffi,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrashReport {
    pub boundary: RuntimeBoundary,
    pub component: String,
    pub panic_message: String,
    pub timestamp_ms: u128,
    pub stack_symbols: Vec<String>,
    pub context: BTreeMap<String, String>,
}

fn now_ms() -> u128 {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(d) => d.as_millis(),
        Err(_) => 0,
    }
}

fn panic_payload_to_string(payload: &(dyn std::any::Any + Send)) -> String {
    if let Some(msg) = payload.downcast_ref::<&str>() {
        (*msg).to_string()
    } else if let Some(msg) = payload.downcast_ref::<String>() {
        msg.clone()
    } else {
        "non-string panic payload".to_string()
    }
}

pub fn symbolize_frames(frame_ids: &[u64]) -> Vec<String> {
    frame_ids
        .iter()
        .enumerate()
        .map(|(i, id)| format!("frame#{:03}:0x{:016x}", i, id))
        .collect()
}

pub fn execute_boundary<T, F>(
    boundary: RuntimeBoundary,
    component: &str,
    context: BTreeMap<String, String>,
    f: F,
) -> Result<T, CrashReport>
where
    F: FnOnce() -> T + UnwindSafe,
{
    match panic::catch_unwind(f) {
        Ok(value) => Ok(value),
        Err(payload) => {
            let panic_message = panic_payload_to_string(payload.as_ref());
            let synthetic_frames = [0x1000u64, 0x2000, 0x3000, 0x4000];
            Err(CrashReport {
                boundary,
                component: component.to_string(),
                panic_message,
                timestamp_ms: now_ms(),
                stack_symbols: symbolize_frames(&synthetic_frames),
                context,
            })
        }
    }
}

#[derive(Debug, Clone)]
pub struct FaultInjector {
    seed: u64,
    probabilities: BTreeMap<String, f64>,
}

impl FaultInjector {
    pub fn new(seed: u64) -> Self {
        Self {
            seed,
            probabilities: BTreeMap::new(),
        }
    }

    pub fn set_probability(&mut self, subsystem: &str, probability: f64) {
        self.probabilities
            .insert(subsystem.to_string(), probability.clamp(0.0, 1.0));
    }

    fn next_f64(&mut self) -> f64 {
        self.seed ^= self.seed << 13;
        self.seed ^= self.seed >> 7;
        self.seed ^= self.seed << 17;
        (self.seed as f64) / (u64::MAX as f64)
    }

    pub fn should_inject(&mut self, subsystem: &str) -> bool {
        let p = self.probabilities.get(subsystem).copied().unwrap_or(0.0);
        self.next_f64() < p
    }
}

pub fn run_fault_matrix(
    injector: &mut FaultInjector,
    subsystems: &[&str],
    iterations: usize,
) -> BTreeMap<String, usize> {
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for &name in subsystems {
        counts.insert(name.to_string(), 0);
    }

    for _ in 0..iterations {
        for &name in subsystems {
            if injector.should_inject(name) {
                *counts.entry(name.to_string()).or_insert(0) += 1;
            }
        }
    }

    counts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execute_boundary_success() {
        let result = execute_boundary(RuntimeBoundary::Repl, "repl_eval", BTreeMap::new(), || 42);
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_execute_boundary_captures_panic() {
        let mut ctx = BTreeMap::new();
        ctx.insert("file".to_string(), "repl.sl".to_string());
        let result = execute_boundary(RuntimeBoundary::Ide, "ide_check", ctx.clone(), || {
            panic!("boom")
        });

        assert!(result.is_err());
        let report = result.err().unwrap();
        assert_eq!(report.boundary, RuntimeBoundary::Ide);
        assert_eq!(report.component, "ide_check");
        assert!(report.panic_message.contains("boom"));
        assert_eq!(report.context, ctx);
        assert!(!report.stack_symbols.is_empty());
    }

    #[test]
    fn test_fault_injector_deterministic_extremes() {
        let mut injector = FaultInjector::new(123);
        injector.set_probability("io", 0.0);
        injector.set_probability("ffi", 1.0);

        for _ in 0..100 {
            assert!(!injector.should_inject("io"));
            assert!(injector.should_inject("ffi"));
        }
    }

    #[test]
    fn test_fault_matrix_counts_ordered_and_bounded() {
        let mut injector = FaultInjector::new(99);
        injector.set_probability("repl", 0.2);
        injector.set_probability("ffi", 0.4);

        let counts = run_fault_matrix(&mut injector, &["repl", "ffi"], 200);
        assert!(counts["repl"] <= 200);
        assert!(counts["ffi"] <= 200);
        assert!(counts["ffi"] >= counts["repl"]);
    }
}
