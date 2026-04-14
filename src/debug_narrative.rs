//! Human-Readable Debug Narratives — Vitalis v680
//!
//! Generates plain-English narratives explaining errors and simplifies
//! technical messages for developer comprehension.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<DebugNarrative>> = LazyLock::new(|| Mutex::new(DebugNarrative::new()));

pub struct DebugNarrative {
    narratives: Vec<String>,
}

impl DebugNarrative {
    pub fn new() -> Self {
        Self { narratives: Vec::new() }
    }

    pub fn generate(&mut self, error_type: &str, context: &str) -> String {
        let narrative = format!(
            "A '{}' occurred while processing '{}'. Check your inputs and try again.",
            error_type, context
        );
        self.narratives.push(narrative.clone());
        narrative
    }

    pub fn simplify(&mut self, technical: &str) -> String {
        let simple = technical
            .replace("NullPointerException", "null value accessed")
            .replace("StackOverflowError", "too many nested calls")
            .replace("OutOfMemoryError", "not enough memory");
        let result = format!("Simplified: {simple}");
        self.narratives.push(result.clone());
        result
    }

    pub fn narrative_count(&self) -> usize {
        self.narratives.len()
    }

    pub fn reset(&mut self) {
        self.narratives.clear();
    }
}

impl Default for DebugNarrative {
    fn default() -> Self { Self::new() }
}

#[unsafe(no_mangle)]
pub extern "C" fn dbg_narr_generate(error_len: i64, ctx_len: i64) -> i64 {
    let err = "E".repeat(error_len.max(0) as usize);
    let ctx = "C".repeat(ctx_len.max(0) as usize);
    let mut s = STATE.lock().unwrap();
    s.generate(&err, &ctx).len() as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn dbg_narr_simplify(msg_len: i64) -> i64 {
    let msg = "NullPointerException ".repeat(1) + &"x".repeat(msg_len.max(0) as usize);
    let mut s = STATE.lock().unwrap();
    s.simplify(&msg).len() as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn dbg_narr_count() -> i64 {
    STATE.lock().unwrap().narrative_count() as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn dbg_narr_reset() -> i64 {
    STATE.lock().unwrap().reset();
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_narrative() {
        let mut dn = DebugNarrative::new();
        let n = dn.generate("TypeError", "parser");
        assert!(n.contains("TypeError"));
        assert!(n.contains("parser"));
    }

    #[test]
    fn test_simplify_null_pointer() {
        let mut dn = DebugNarrative::new();
        let s = dn.simplify("NullPointerException in main");
        assert!(s.contains("null value accessed"));
    }

    #[test]
    fn test_narrative_count() {
        let mut dn = DebugNarrative::new();
        dn.generate("E1", "c1");
        dn.generate("E2", "c2");
        assert_eq!(dn.narrative_count(), 2);
    }

    #[test]
    fn test_reset() {
        let mut dn = DebugNarrative::new();
        dn.generate("E", "c");
        dn.reset();
        assert_eq!(dn.narrative_count(), 0);
    }

    #[test]
    fn test_simplify_stack_overflow() {
        let mut dn = DebugNarrative::new();
        let s = dn.simplify("StackOverflowError detected");
        assert!(s.contains("too many nested calls"));
    }

    #[test]
    fn test_ffi_dbg_narr_count() {
        dbg_narr_reset();
        dbg_narr_generate(5, 3);
        assert!(dbg_narr_count() >= 1);
    }
}
