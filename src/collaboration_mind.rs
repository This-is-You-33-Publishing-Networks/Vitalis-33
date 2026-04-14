//! Collaborative Developer AI — Vitalis v801
//!
//! Provides a collaborative AI mind that shares context between developers
//! and answers queries about the codebase.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<CollabMind>> = LazyLock::new(|| Mutex::new(CollabMind::new()));

pub struct CollabMind {
    context: Vec<String>,
}

impl CollabMind {
    pub fn new() -> Self {
        Self { context: Vec::new() }
    }

    pub fn share_context(&mut self, ctx: &str) {
        self.context.push(ctx.to_string());
    }

    pub fn query(&self, question: &str) -> String {
        let relevant: Vec<&str> = self.context.iter()
            .filter(|c| c.contains(&question[..question.len().min(5)]))
            .map(|s| s.as_str())
            .collect();
        if relevant.is_empty() {
            format!("No context found for: {question}")
        } else {
            format!("Found {} relevant contexts.", relevant.len())
        }
    }

    pub fn context_size(&self) -> usize {
        self.context.len()
    }

    pub fn reset(&mut self) {
        self.context.clear();
    }
}

impl Default for CollabMind {
    fn default() -> Self { Self::new() }
}

#[unsafe(no_mangle)]
pub extern "C" fn cm_share(ctx_len: i64) -> i64 {
    let ctx = "ctx_".to_string() + &"x".repeat(ctx_len.max(0) as usize);
    STATE.lock().unwrap().share_context(&ctx);
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn cm_query(q_len: i64) -> i64 {
    let q = "query".to_string() + &"?".repeat(q_len.max(0) as usize);
    STATE.lock().unwrap().query(&q).len() as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn cm_size() -> i64 {
    STATE.lock().unwrap().context_size() as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn cm_reset() -> i64 {
    STATE.lock().unwrap().reset();
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_share_context() {
        let mut cm = CollabMind::new();
        cm.share_context("fn main() {}");
        assert_eq!(cm.context_size(), 1);
    }

    #[test]
    fn test_query_no_context() {
        let cm = CollabMind::new();
        let r = cm.query("anything");
        assert!(r.contains("No context"));
    }

    #[test]
    fn test_query_finds_context() {
        let mut cm = CollabMind::new();
        cm.share_context("fn process_data() {}");
        let r = cm.query("fn pr");
        assert!(r.contains("relevant"));
    }

    #[test]
    fn test_context_size() {
        let mut cm = CollabMind::new();
        cm.share_context("a");
        cm.share_context("b");
        assert_eq!(cm.context_size(), 2);
    }

    #[test]
    fn test_reset() {
        let mut cm = CollabMind::new();
        cm.share_context("x");
        cm.reset();
        assert_eq!(cm.context_size(), 0);
    }

    #[test]
    fn test_ffi_cm_share_and_size() {
        cm_reset();
        cm_share(5);
        assert!(cm_size() >= 1);
    }
}
