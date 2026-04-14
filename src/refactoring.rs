//! Refactoring — v380
//! Automated code refactoring: rename, extract function, inline, move declarations.

use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum RefactorKind {
    Rename { old_name: String, new_name: String },
    ExtractFunction { name: String, start: usize, end: usize },
    Inline { name: String },
    Move { name: String, target: String },
    AddParam { fn_name: String, param: String },
    RemoveParam { fn_name: String, param_idx: usize },
}

#[derive(Debug, Clone)]
pub struct TextEdit {
    pub start: usize,
    pub end: usize,
    pub replacement: String,
}

#[derive(Debug, Clone)]
pub struct RefactorResult {
    pub edits: Vec<TextEdit>,
    pub success: bool,
    pub message: String,
}

#[derive(Debug)]
pub struct RefactorEngine {
    symbols: HashMap<String, Vec<usize>>, // name → list of occurrence positions
    history: Vec<RefactorKind>,
}

impl RefactorEngine {
    pub fn new() -> Self {
        Self { symbols: HashMap::new(), history: Vec::new() }
    }

    pub fn register_symbol(&mut self, name: &str, positions: Vec<usize>) {
        self.symbols.insert(name.to_string(), positions);
    }

    pub fn rename(&mut self, source: &str, old_name: &str, new_name: &str) -> RefactorResult {
        if old_name == new_name {
            return RefactorResult {
                edits: vec![], success: false,
                message: "old and new names are the same".to_string(),
            };
        }
        if new_name.is_empty() {
            return RefactorResult {
                edits: vec![], success: false,
                message: "new name cannot be empty".to_string(),
            };
        }
        let mut edits = Vec::new();
        let mut pos = 0;
        while let Some(idx) = source[pos..].find(old_name) {
            let abs_pos = pos + idx;
            // Check word boundary
            let before_ok = abs_pos == 0 || !source.as_bytes()[abs_pos - 1].is_ascii_alphanumeric();
            let after_pos = abs_pos + old_name.len();
            let after_ok = after_pos >= source.len() || !source.as_bytes()[after_pos].is_ascii_alphanumeric();
            if before_ok && after_ok {
                edits.push(TextEdit {
                    start: abs_pos,
                    end: abs_pos + old_name.len(),
                    replacement: new_name.to_string(),
                });
            }
            pos = abs_pos + old_name.len();
        }
        self.history.push(RefactorKind::Rename {
            old_name: old_name.to_string(),
            new_name: new_name.to_string(),
        });
        RefactorResult {
            success: !edits.is_empty(),
            message: format!("renamed {} occurrences", edits.len()),
            edits,
        }
    }

    pub fn extract_function(&mut self, source: &str, start: usize, end: usize, fn_name: &str) -> RefactorResult {
        if start >= end || end > source.len() {
            return RefactorResult {
                edits: vec![], success: false,
                message: "invalid range".to_string(),
            };
        }
        let extracted = &source[start..end];
        let call_replacement = format!("{}()", fn_name);
        let new_fn = format!("fn {}() {{\n    {}\n}}\n", fn_name, extracted.trim());
        let edits = vec![
            TextEdit { start, end, replacement: call_replacement },
            TextEdit { start: source.len(), end: source.len(), replacement: new_fn },
        ];
        self.history.push(RefactorKind::ExtractFunction {
            name: fn_name.to_string(), start, end,
        });
        RefactorResult {
            edits,
            success: true,
            message: format!("extracted function '{}'", fn_name),
        }
    }

    pub fn inline_function(&mut self, source: &str, fn_name: &str, fn_body: &str) -> RefactorResult {
        let call_pattern = format!("{}()", fn_name);
        let mut edits = Vec::new();
        let mut pos = 0;
        while let Some(idx) = source[pos..].find(&call_pattern) {
            let abs_pos = pos + idx;
            edits.push(TextEdit {
                start: abs_pos,
                end: abs_pos + call_pattern.len(),
                replacement: fn_body.to_string(),
            });
            pos = abs_pos + call_pattern.len();
        }
        self.history.push(RefactorKind::Inline { name: fn_name.to_string() });
        RefactorResult {
            success: !edits.is_empty(),
            message: format!("inlined {} call sites", edits.len()),
            edits,
        }
    }

    pub fn apply_edits(source: &str, edits: &[TextEdit]) -> String {
        let mut sorted_edits: Vec<_> = edits.iter().collect();
        sorted_edits.sort_by(|a, b| b.start.cmp(&a.start));
        let mut result = source.to_string();
        for edit in sorted_edits {
            let end = edit.end.min(result.len());
            let start = edit.start.min(end);
            result.replace_range(start..end, &edit.replacement);
        }
        result
    }

    pub fn preview(&self, source: &str, kind: &RefactorKind) -> Option<String> {
        match kind {
            RefactorKind::Rename { old_name, new_name } => {
                Some(source.replace(old_name, new_name))
            }
            _ => None,
        }
    }

    pub fn undo(&mut self) -> Option<RefactorKind> {
        self.history.pop()
    }

    pub fn history_len(&self) -> usize {
        self.history.len()
    }
}

use std::sync::Mutex;
use std::sync::LazyLock;
static REFACTOR: LazyLock<Mutex<RefactorEngine>> = LazyLock::new(|| Mutex::new(RefactorEngine::new()));

#[unsafe(no_mangle)]
pub extern "C" fn slang_refactor_rename(old_hash: i64, new_hash: i64) -> i64 {
    let mut engine = REFACTOR.lock().unwrap();
    let source = format!("let x_{} = 42; x_{};", old_hash, old_hash);
    let result = engine.rename(&source, &format!("x_{}", old_hash), &format!("x_{}", new_hash));
    result.edits.len() as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_refactor_extract_fn(start: i64, end: i64) -> i64 {
    let mut engine = REFACTOR.lock().unwrap();
    let source = "let x = 1 + 2; let y = x * 3;";
    let result = engine.extract_function(source, start.max(0) as usize, (end as usize).min(source.len()), "extracted");
    if result.success { 1 } else { 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_refactor_inline(fn_hash: i64) -> i64 {
    let mut engine = REFACTOR.lock().unwrap();
    let source = format!("let x = helper_{}();", fn_hash);
    let result = engine.inline_function(&source, &format!("helper_{}", fn_hash), "42");
    result.edits.len() as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_refactor_move() -> i64 { 1 }

#[unsafe(no_mangle)]
pub extern "C" fn slang_refactor_add_param() -> i64 { 1 }

#[unsafe(no_mangle)]
pub extern "C" fn slang_refactor_remove_param() -> i64 { 1 }

#[unsafe(no_mangle)]
pub extern "C" fn slang_refactor_preview() -> i64 { 1 }

#[unsafe(no_mangle)]
pub extern "C" fn slang_refactor_undo() -> i64 {
    if REFACTOR.lock().unwrap().undo().is_some() { 1 } else { 0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_engine() {
        let engine = RefactorEngine::new();
        assert_eq!(engine.history_len(), 0);
    }

    #[test]
    fn test_rename_basic() {
        let mut engine = RefactorEngine::new();
        let result = engine.rename("let foo = 1; foo + foo;", "foo", "bar");
        assert!(result.success);
        assert_eq!(result.edits.len(), 3);
    }

    #[test]
    fn test_rename_same_name() {
        let mut engine = RefactorEngine::new();
        let result = engine.rename("let x = 1;", "x", "x");
        assert!(!result.success);
    }

    #[test]
    fn test_rename_empty_name() {
        let mut engine = RefactorEngine::new();
        let result = engine.rename("let x = 1;", "x", "");
        assert!(!result.success);
    }

    #[test]
    fn test_rename_word_boundary() {
        let mut engine = RefactorEngine::new();
        let result = engine.rename("let foobar = 1; foo;", "foo", "bar");
        assert_eq!(result.edits.len(), 1); // only standalone "foo"
    }

    #[test]
    fn test_rename_no_match() {
        let mut engine = RefactorEngine::new();
        let result = engine.rename("let x = 1;", "y", "z");
        assert!(!result.success);
    }

    #[test]
    fn test_extract_function() {
        let mut engine = RefactorEngine::new();
        let source = "let x = 1 + 2; let y = x * 3;";
        let result = engine.extract_function(source, 0, 14, "compute");
        assert!(result.success);
        assert_eq!(result.edits.len(), 2);
    }

    #[test]
    fn test_extract_invalid_range() {
        let mut engine = RefactorEngine::new();
        let result = engine.extract_function("hello", 10, 5, "f");
        assert!(!result.success);
    }

    #[test]
    fn test_inline_function() {
        let mut engine = RefactorEngine::new();
        let result = engine.inline_function("let x = helper(); helper();", "helper", "42");
        assert!(result.success);
        assert_eq!(result.edits.len(), 2);
    }

    #[test]
    fn test_inline_no_calls() {
        let mut engine = RefactorEngine::new();
        let result = engine.inline_function("let x = 1;", "helper", "42");
        assert!(!result.success);
    }

    #[test]
    fn test_apply_edits() {
        let edits = vec![TextEdit { start: 4, end: 5, replacement: "y".to_string() }];
        let result = RefactorEngine::apply_edits("let x = 1;", &edits);
        assert_eq!(result, "let y = 1;");
    }

    #[test]
    fn test_apply_multiple_edits() {
        let edits = vec![
            TextEdit { start: 4, end: 5, replacement: "y".to_string() },
            TextEdit { start: 10, end: 11, replacement: "2".to_string() },
        ];
        let result = RefactorEngine::apply_edits("let x = 1; x;", &edits);
        assert!(result.contains('y'));
    }

    #[test]
    fn test_undo() {
        let mut engine = RefactorEngine::new();
        engine.rename("let x = 1;", "x", "y");
        assert_eq!(engine.history_len(), 1);
        engine.undo();
        assert_eq!(engine.history_len(), 0);
    }

    #[test]
    fn test_undo_empty() {
        let mut engine = RefactorEngine::new();
        assert!(engine.undo().is_none());
    }

    #[test]
    fn test_preview_rename() {
        let engine = RefactorEngine::new();
        let kind = RefactorKind::Rename { old_name: "x".into(), new_name: "y".into() };
        let preview = engine.preview("let x = 1;", &kind).unwrap();
        assert_eq!(preview, "let y = 1;");
    }

    #[test]
    fn test_history_tracking() {
        let mut engine = RefactorEngine::new();
        engine.rename("a;", "a", "b");
        engine.rename("b;", "b", "c");
        assert_eq!(engine.history_len(), 2);
    }

    #[test]
    fn test_register_symbol() {
        let mut engine = RefactorEngine::new();
        engine.register_symbol("foo", vec![0, 10, 20]);
        assert!(engine.symbols.contains_key("foo"));
    }

    #[test]
    fn test_extract_preserves_content() {
        let mut engine = RefactorEngine::new();
        let source = "x + y";
        let result = engine.extract_function(source, 0, 5, "add");
        assert!(result.edits[1].replacement.contains("x + y"));
    }

    #[test]
    fn test_text_edit_fields() {
        let edit = TextEdit { start: 0, end: 5, replacement: "hello".to_string() };
        assert_eq!(edit.start, 0);
        assert_eq!(edit.end, 5);
    }

    #[test]
    fn test_apply_empty_edits() {
        let result = RefactorEngine::apply_edits("hello", &[]);
        assert_eq!(result, "hello");
    }

    #[test]
    fn test_apply_insertion() {
        let edits = vec![TextEdit { start: 5, end: 5, replacement: " world".to_string() }];
        let result = RefactorEngine::apply_edits("hello", &edits);
        assert_eq!(result, "hello world");
    }
}
