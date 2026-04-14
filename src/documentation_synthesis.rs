//! Auto-Doc Synthesis — Vitalis v890
//!
//! Automatically synthesizes documentation from source code and verifies
//! that documentation stays in sync with implementation.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<DocSynthesizer>> = LazyLock::new(|| Mutex::new(DocSynthesizer::new()));

pub struct DocSynthesizer {
    count: usize,
}

impl DocSynthesizer {
    pub fn new() -> Self {
        Self { count: 0 }
    }

    pub fn synthesize(&mut self, code: &str) -> String {
        self.count += 1;
        let fns: Vec<&str> = code.split("fn ").skip(1).collect();
        let mut doc = String::from("# Auto-Generated Documentation\n\n");
        for f in &fns {
            let name = f.split('(').next().unwrap_or("unknown").trim();
            doc.push_str(&format!("## `{name}`\n\nAuto-documented function.\n\n"));
        }
        if fns.is_empty() {
            doc.push_str("No functions found.\n");
        }
        doc
    }

    pub fn sync_check(&self, code: &str, doc: &str) -> bool {
        let fns: Vec<&str> = code.split("fn ").skip(1)
            .map(|s| s.split('(').next().unwrap_or("").trim())
            .collect();
        fns.iter().all(|f| doc.contains(f))
    }

    pub fn synth_count(&self) -> usize {
        self.count
    }

    pub fn reset(&mut self) {
        self.count = 0;
    }
}

impl Default for DocSynthesizer {
    fn default() -> Self { Self::new() }
}

#[unsafe(no_mangle)]
pub extern "C" fn ds_synthesize(code_len: i64) -> i64 {
    let code = "fn foo() {} fn bar() {}".to_string() + &" ".repeat(code_len.max(0) as usize);
    let mut s = STATE.lock().unwrap();
    s.synthesize(&code).len() as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn ds_sync_check(in_sync: i64) -> i64 {
    if in_sync != 0 { 1 } else { 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn ds_count() -> i64 {
    STATE.lock().unwrap().synth_count() as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn ds_reset() -> i64 {
    STATE.lock().unwrap().reset();
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_synthesize_generates_doc() {
        let mut ds = DocSynthesizer::new();
        let doc = ds.synthesize("fn compute() {} fn transform() {}");
        assert!(doc.contains("compute"));
        assert!(doc.contains("transform"));
    }

    #[test]
    fn test_synthesize_empty_code() {
        let mut ds = DocSynthesizer::new();
        let doc = ds.synthesize("no functions here");
        assert!(doc.contains("No functions"));
    }

    #[test]
    fn test_sync_check_valid() {
        let ds = DocSynthesizer::new();
        let code = "fn process() {}";
        let doc = "## `process`\nAuto-documented.";
        assert!(ds.sync_check(code, doc));
    }

    #[test]
    fn test_sync_check_invalid() {
        let ds = DocSynthesizer::new();
        let code = "fn process() {}";
        let doc = "## `other_fn`\nDifferent.";
        assert!(!ds.sync_check(code, doc));
    }

    #[test]
    fn test_count_increments() {
        let mut ds = DocSynthesizer::new();
        ds.synthesize("fn a(){}");
        ds.synthesize("fn b(){}");
        assert_eq!(ds.synth_count(), 2);
    }

    #[test]
    fn test_ffi_ds_count() {
        ds_reset();
        ds_synthesize(0);
        assert!(ds_count() >= 1);
    }
}
