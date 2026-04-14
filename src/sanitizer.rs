//! Sanitizer — v379
//! Memory, thread, and undefined behavior sanitizer instrumentation.

use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, PartialEq)]
pub enum Violation {
    OutOfBounds { address: i64, size: i64 },
    NullPointer { address: i64 },
    UseAfterFree { address: i64 },
    DoubleFree { address: i64 },
    MemoryLeak { address: i64, size: i64 },
    IntegerOverflow { op: String, a: i64, b: i64 },
    DataRace { address: i64 },
    Uninitialized { address: i64 },
}

#[derive(Debug)]
pub struct MemorySanitizer {
    allocated: HashMap<i64, (i64, bool)>, // address → (size, is_valid)
    freed: HashSet<i64>,
    violations: Vec<Violation>,
    shadow: HashMap<i64, bool>, // initialized tracking
}

impl MemorySanitizer {
    pub fn new() -> Self {
        Self {
            allocated: HashMap::new(),
            freed: HashSet::new(),
            violations: Vec::new(),
            shadow: HashMap::new(),
        }
    }

    pub fn alloc(&mut self, address: i64, size: i64) {
        self.allocated.insert(address, (size, true));
        for i in 0..size {
            self.shadow.insert(address + i, false);
        }
    }

    pub fn free(&mut self, address: i64) {
        if self.freed.contains(&address) {
            self.violations.push(Violation::DoubleFree { address });
            return;
        }
        if let Some((size, valid)) = self.allocated.get_mut(&address) {
            *valid = false;
            self.freed.insert(address);
            for i in 0..*size {
                self.shadow.remove(&(address + i));
            }
        }
    }

    pub fn check_bounds(&mut self, address: i64, size: i64) -> bool {
        for (&base, &(alloc_size, valid)) in &self.allocated {
            if valid && address >= base && address + size <= base + alloc_size {
                return true;
            }
        }
        self.violations.push(Violation::OutOfBounds { address, size });
        false
    }

    pub fn check_null(&mut self, address: i64) -> bool {
        if address == 0 {
            self.violations.push(Violation::NullPointer { address });
            false
        } else {
            true
        }
    }

    pub fn check_use_after_free(&mut self, address: i64) -> bool {
        if self.freed.contains(&address) {
            self.violations.push(Violation::UseAfterFree { address });
            false
        } else {
            true
        }
    }

    pub fn check_overflow(&mut self, op: &str, a: i64, b: i64) -> bool {
        let overflow = match op {
            "add" => a.checked_add(b).is_none(),
            "sub" => a.checked_sub(b).is_none(),
            "mul" => a.checked_mul(b).is_none(),
            _ => false,
        };
        if overflow {
            self.violations.push(Violation::IntegerOverflow {
                op: op.to_string(), a, b,
            });
        }
        !overflow
    }

    pub fn check_leak(&mut self) -> Vec<(i64, i64)> {
        let mut leaks = Vec::new();
        for (&addr, &(size, valid)) in &self.allocated {
            if valid && !self.freed.contains(&addr) {
                leaks.push((addr, size));
                self.violations.push(Violation::MemoryLeak { address: addr, size });
            }
        }
        leaks
    }

    pub fn write_byte(&mut self, address: i64) {
        self.shadow.insert(address, true);
    }

    pub fn check_initialized(&mut self, address: i64) -> bool {
        match self.shadow.get(&address) {
            Some(true) => true,
            Some(false) => {
                self.violations.push(Violation::Uninitialized { address });
                false
            }
            None => true, // not tracked
        }
    }

    pub fn violation_count(&self) -> usize {
        self.violations.len()
    }

    pub fn clear_reports(&mut self) {
        self.violations.clear();
    }

    pub fn has_violations(&self) -> bool {
        !self.violations.is_empty()
    }
}

use std::sync::Mutex;
use std::sync::LazyLock;
static SAN: LazyLock<Mutex<MemorySanitizer>> = LazyLock::new(|| Mutex::new(MemorySanitizer::new()));

#[unsafe(no_mangle)]
pub extern "C" fn slang_san_check_bounds(address: i64, size: i64) -> i64 {
    if SAN.lock().unwrap().check_bounds(address, size) { 1 } else { 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_san_check_null(address: i64) -> i64 {
    if SAN.lock().unwrap().check_null(address) { 1 } else { 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_san_check_overflow(a: i64, b: i64) -> i64 {
    if SAN.lock().unwrap().check_overflow("add", a, b) { 1 } else { 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_san_check_use_after_free(address: i64) -> i64 {
    if SAN.lock().unwrap().check_use_after_free(address) { 1 } else { 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_san_check_double_free(address: i64) -> i64 {
    let mut san = SAN.lock().unwrap();
    let count_before = san.violation_count();
    san.free(address);
    if san.violation_count() > count_before { 0 } else { 1 }
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_san_check_leak() -> i64 {
    SAN.lock().unwrap().check_leak().len() as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_san_report_count() -> i64 {
    SAN.lock().unwrap().violation_count() as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_san_clear_reports() -> i64 {
    SAN.lock().unwrap().clear_reports();
    1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_sanitizer() {
        let san = MemorySanitizer::new();
        assert_eq!(san.violation_count(), 0);
    }

    #[test]
    fn test_alloc_free() {
        let mut san = MemorySanitizer::new();
        san.alloc(1000, 64);
        san.free(1000);
        assert!(!san.has_violations());
    }

    #[test]
    fn test_bounds_check_valid() {
        let mut san = MemorySanitizer::new();
        san.alloc(1000, 64);
        assert!(san.check_bounds(1000, 32));
    }

    #[test]
    fn test_bounds_check_invalid() {
        let mut san = MemorySanitizer::new();
        san.alloc(1000, 64);
        assert!(!san.check_bounds(2000, 8));
    }

    #[test]
    fn test_null_check() {
        let mut san = MemorySanitizer::new();
        assert!(!san.check_null(0));
        assert!(san.check_null(100));
    }

    #[test]
    fn test_use_after_free() {
        let mut san = MemorySanitizer::new();
        san.alloc(1000, 64);
        san.free(1000);
        assert!(!san.check_use_after_free(1000));
    }

    #[test]
    fn test_no_use_after_free() {
        let mut san = MemorySanitizer::new();
        san.alloc(1000, 64);
        assert!(san.check_use_after_free(1000));
    }

    #[test]
    fn test_double_free() {
        let mut san = MemorySanitizer::new();
        san.alloc(1000, 64);
        san.free(1000);
        san.free(1000);
        assert!(san.has_violations());
        assert!(san.violations.iter().any(|v| matches!(v, Violation::DoubleFree { .. })));
    }

    #[test]
    fn test_overflow_add() {
        let mut san = MemorySanitizer::new();
        assert!(!san.check_overflow("add", i64::MAX, 1));
        assert!(san.has_violations());
    }

    #[test]
    fn test_overflow_mul() {
        let mut san = MemorySanitizer::new();
        assert!(!san.check_overflow("mul", i64::MAX, 2));
    }

    #[test]
    fn test_no_overflow() {
        let mut san = MemorySanitizer::new();
        assert!(san.check_overflow("add", 10, 20));
        assert!(!san.has_violations());
    }

    #[test]
    fn test_leak_detection() {
        let mut san = MemorySanitizer::new();
        san.alloc(1000, 64);
        san.alloc(2000, 32);
        san.free(1000);
        let leaks = san.check_leak();
        assert_eq!(leaks.len(), 1);
        assert_eq!(leaks[0], (2000, 32));
    }

    #[test]
    fn test_no_leaks() {
        let mut san = MemorySanitizer::new();
        san.alloc(1000, 64);
        san.free(1000);
        let leaks = san.check_leak();
        assert!(leaks.is_empty());
    }

    #[test]
    fn test_clear_reports() {
        let mut san = MemorySanitizer::new();
        san.check_null(0);
        assert!(san.has_violations());
        san.clear_reports();
        assert!(!san.has_violations());
    }

    #[test]
    fn test_uninitialized() {
        let mut san = MemorySanitizer::new();
        san.alloc(1000, 4);
        assert!(!san.check_initialized(1000));
        san.write_byte(1000);
        assert!(san.check_initialized(1000));
    }

    #[test]
    fn test_violation_count() {
        let mut san = MemorySanitizer::new();
        san.check_null(0);
        san.check_null(0);
        san.check_overflow("add", i64::MAX, 1);
        assert_eq!(san.violation_count(), 3);
    }

    #[test]
    fn test_bounds_at_edge() {
        let mut san = MemorySanitizer::new();
        san.alloc(1000, 64);
        assert!(san.check_bounds(1000, 64)); // exactly fits
        assert!(!san.check_bounds(1000, 65)); // one byte over
    }

    #[test]
    fn test_overflow_sub() {
        let mut san = MemorySanitizer::new();
        assert!(!san.check_overflow("sub", i64::MIN, 1));
    }

    #[test]
    fn test_multiple_allocs() {
        let mut san = MemorySanitizer::new();
        san.alloc(1000, 32);
        san.alloc(2000, 32);
        assert!(san.check_bounds(1000, 16));
        assert!(san.check_bounds(2000, 16));
    }

    #[test]
    fn test_bounds_freed_memory() {
        let mut san = MemorySanitizer::new();
        san.alloc(1000, 64);
        san.free(1000);
        assert!(!san.check_bounds(1000, 8));
    }
}
