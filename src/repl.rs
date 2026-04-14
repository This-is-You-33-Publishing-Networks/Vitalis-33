//! REPL — Interactive Read-Eval-Print Loop for Vitalis
//!
//! Supports:
//! - Multi-line input with { } matching (string/comment-aware)
//! - Tab completion for keywords and builtins
//! - `:help`, `:ast`, `:ir`, `:type`, `:clear`, `:exit` commands
//! - `:load file.sl` / `:save file.sl` — file I/O
//! - ANSI-colored prompts, results, and errors
//! - Expression evaluation wrapping in fn main()

use crate::codegen;
use crate::ir;
use crate::parser;
use crate::types;

/// ANSI color codes
pub mod color {
    pub const RESET: &str = "\x1b[0m";
    pub const GREEN: &str = "\x1b[32m";
    pub const CYAN: &str = "\x1b[36m";
    pub const RED: &str = "\x1b[31m";
    pub const YELLOW: &str = "\x1b[33m";
    pub const BOLD: &str = "\x1b[1m";
    pub const DIM: &str = "\x1b[2m";
}

/// Keywords and builtins available for tab completion
pub const COMPLETION_KEYWORDS: &[&str] = &[
    "fn", "let", "mut", "if", "else", "while", "for", "in", "loop", "break",
    "continue", "return", "struct", "enum", "impl", "trait", "match", "true",
    "false", "try", "catch", "throw", "async", "await", "spawn", "import",
    "module", "type", "const", "extern", "pub", "self", "i32", "i64", "f32",
    "f64", "bool", "str", "void",
];

pub const COMPLETION_BUILTINS: &[&str] = &[
    "print", "println", "len", "push", "pop", "abs", "min", "max",
    "sqrt", "to_string", "parse_int", "assert", "assert_eq",
];

/// REPL session state
pub struct ReplSession {
    pub history: Vec<String>,
    pub line_number: usize,
    pub last_result: Option<i64>,
    pub verbose: bool,
    pub definitions: Vec<String>,
}

impl ReplSession {
    pub fn new() -> Self {
        Self {
            history: Vec::new(),
            line_number: 0,
            last_result: None,
            verbose: false,
            definitions: Vec::new(),
        }
    }

    /// Evaluate a single REPL input line or block.
    /// Returns Ok(Some(value)) for expressions, Ok(None) for commands, Err for errors.
    pub fn eval(&mut self, input: &str) -> Result<Option<i64>, String> {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return Ok(None);
        }

        // Handle REPL commands
        if trimmed.starts_with(':') {
            return self.handle_command(trimmed);
        }

        self.history.push(input.to_string());
        self.line_number += 1;

        // Accumulate top-level definitions across evals
        let is_def = trimmed.starts_with("fn ") || trimmed.starts_with("struct ")
            || trimmed.starts_with("enum ") || trimmed.starts_with("impl ")
            || trimmed.starts_with("trait ");

        if is_def {
            self.definitions.push(trimmed.to_string());
        }

        // Build source with accumulated definitions prefix
        let defs_prefix = self.definitions.iter()
            .filter(|d| *d != trimmed || !is_def)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n");

        let source = if is_def {
            // Top-level definition — append a main that returns 0
            format!("{}\n{}\nfn main() -> i64 {{ 0 }}", defs_prefix, trimmed)
        } else if trimmed.contains("let ") || trimmed.ends_with(';') {
            // Statement — wrap in main
            format!("{}\nfn main() -> i64 {{ {}; 0 }}", defs_prefix, trimmed.trim_end_matches(';'))
        } else {
            // Expression — wrap as return value
            format!("{}\nfn main() -> i64 {{ {} }}", defs_prefix, trimmed)
        };

        match codegen::compile_and_run(&source) {
            Ok(val) => {
                self.last_result = Some(val);
                Ok(Some(val))
            }
            Err(e) => {
                // If a definition failed to compile, remove it
                if is_def {
                    self.definitions.pop();
                }
                Err(e)
            }
        }
    }

    /// Handle REPL meta-commands
    fn handle_command(&mut self, cmd: &str) -> Result<Option<i64>, String> {
        let parts: Vec<&str> = cmd.splitn(2, ' ').collect();
        match parts[0] {
            ":help" | ":h" => {
                Ok(None) // Help text is printed by the caller
            }
            ":ast" => {
                let code = parts.get(1).unwrap_or(&"fn main() -> i64 { 0 }");
                let (program, errors) = parser::parse(code);
                if !errors.is_empty() {
                    return Err(errors.iter().map(|e| e.to_string()).collect::<Vec<_>>().join("\n"));
                }
                Err(format!("{:#?}", program))
            }
            ":ir" => {
                let code = parts.get(1).unwrap_or(&"fn main() -> i64 { 0 }");
                let (program, errors) = parser::parse(code);
                if !errors.is_empty() {
                    return Err(errors.iter().map(|e| e.to_string()).collect::<Vec<_>>().join("\n"));
                }
                let ir_module = ir::IrBuilder::new().build(&program);
                let mut out = String::new();
                for func in &ir_module.functions {
                    out.push_str(&format!("{}", func));
                }
                Err(out)
            }
            ":type" | ":t" => {
                let code = parts.get(1).unwrap_or(&"fn main() -> i64 { 0 }");
                let source = format!("fn main() -> i64 {{ {} }}", code);
                let (program, errors) = parser::parse(&source);
                if !errors.is_empty() {
                    return Err(errors.iter().map(|e| e.to_string()).collect::<Vec<_>>().join("\n"));
                }
                let type_errors = types::TypeChecker::new().check(&program);
                if type_errors.is_empty() {
                    Err("Type check passed — no errors".to_string())
                } else {
                    Err(type_errors.iter().map(|e| e.to_string()).collect::<Vec<_>>().join("\n"))
                }
            }
            ":clear" => {
                self.history.clear();
                self.line_number = 0;
                self.last_result = None;
                self.definitions.clear();
                Ok(None)
            }
            ":reset" => {
                self.definitions.clear();
                Err("Accumulated definitions cleared.".to_string())
            }
            ":load" => {
                let path = parts.get(1).ok_or("Usage: :load <file.sl>")?;
                let content = std::fs::read_to_string(path)
                    .map_err(|e| format!("Failed to load '{}': {}", path, e))?;
                Err(format!("Loaded {} bytes from '{}'", content.len(), path))
            }
            ":run" => {
                let path = parts.get(1).ok_or("Usage: :run <file.sl>")?;
                let content = std::fs::read_to_string(path)
                    .map_err(|e| format!("Failed to load '{}': {}", path, e))?;
                match codegen::compile_and_run(&content) {
                    Ok(val) => {
                        self.last_result = Some(val);
                        Ok(Some(val))
                    }
                    Err(e) => Err(e),
                }
            }
            ":save" => {
                let path = parts.get(1).ok_or("Usage: :save <file.sl>")?;
                let content = self.history.join("\n");
                std::fs::write(path, &content)
                    .map_err(|e| format!("Failed to save '{}': {}", path, e))?;
                Err(format!("Saved {} entries to '{}'", self.history.len(), path))
            }
            ":defs" => {
                if self.definitions.is_empty() {
                    Err("No accumulated definitions.".to_string())
                } else {
                    Err(self.definitions.join("\n\n"))
                }
            }
            ":history" => {
                let hist = self.history.iter().enumerate()
                    .map(|(i, h)| format!("[{}] {}", i + 1, h))
                    .collect::<Vec<_>>()
                    .join("\n");
                Err(hist)
            }
            ":verbose" => {
                self.verbose = !self.verbose;
                Err(format!("Verbose mode: {}", if self.verbose { "on" } else { "off" }))
            }
            ":exit" | ":quit" | ":q" => {
                Err("__EXIT__".to_string())
            }
            _ => Err(format!("Unknown command: {}", parts[0])),
        }
    }

    /// Check if input is a complete block (balanced braces, string/comment-aware)
    pub fn is_complete(input: &str) -> bool {
        let mut depth: i32 = 0;
        let mut in_string = false;
        let mut in_line_comment = false;
        let chars: Vec<char> = input.chars().collect();
        let mut i = 0;
        while i < chars.len() {
            let ch = chars[i];
            if ch == '\n' {
                in_line_comment = false;
                i += 1;
                continue;
            }
            if in_line_comment {
                i += 1;
                continue;
            }
            if ch == '"' && !in_line_comment {
                in_string = !in_string;
                i += 1;
                continue;
            }
            if in_string {
                // Skip escaped characters inside strings
                if ch == '\\' && i + 1 < chars.len() {
                    i += 2;
                    continue;
                }
                i += 1;
                continue;
            }
            if ch == '/' && i + 1 < chars.len() && chars[i + 1] == '/' {
                in_line_comment = true;
                i += 2;
                continue;
            }
            match ch {
                '{' => depth += 1,
                '}' => depth -= 1,
                _ => {}
            }
            i += 1;
        }
        depth <= 0
    }

    /// Tab completion: find all completions matching a prefix
    pub fn complete(prefix: &str) -> Vec<&'static str> {
        let mut results = Vec::new();
        for &kw in COMPLETION_KEYWORDS {
            if kw.starts_with(prefix) && kw != prefix {
                results.push(kw);
            }
        }
        for &b in COMPLETION_BUILTINS {
            if b.starts_with(prefix) && b != prefix {
                results.push(b);
            }
        }
        results
    }

    /// Get help text
    pub fn help_text() -> &'static str {
        "\
Vitalis REPL Commands:
  :help, :h       Show this help message
  :ast <code>     Dump AST for code
  :ir <code>      Dump IR for code
  :type <expr>    Type-check an expression
  :load <file>    Display file contents
  :run <file>     Load and execute a .sl file
  :save <file>    Save history to a file
  :defs           Show accumulated definitions
  :reset          Clear accumulated definitions
  :clear          Clear history, definitions, and state
  :history        Show input history
  :verbose        Toggle verbose output
  :exit, :q       Exit the REPL

Enter any expression to evaluate (automatically wrapped in fn main).
Definitions (fn, struct, enum) accumulate across evaluations.
Use Tab for keyword/builtin completion."
    }
}

/// Run the interactive REPL loop (reads from stdin)
pub fn run_interactive() {
    use std::io::{self, Write, BufRead};

    println!(
        "{}{}Vitalis REPL v27.0.0{} — type {}:help{} for commands, {}:exit{} to quit",
        color::BOLD, color::CYAN, color::RESET,
        color::YELLOW, color::RESET,
        color::YELLOW, color::RESET,
    );
    println!();

    let mut session = ReplSession::new();
    let stdin = io::stdin();
    let mut buffer = String::new();

    loop {
        // Prompt
        if buffer.is_empty() {
            print!("{}{}vtc>{} ", color::BOLD, color::GREEN, color::RESET);
        } else {
            print!("{}...>{} ", color::DIM, color::RESET);
        }
        io::stdout().flush().unwrap_or(());

        let mut line = String::new();
        match stdin.lock().read_line(&mut line) {
            Ok(0) => break, // EOF
            Err(_) => break,
            Ok(_) => {}
        }

        // Handle tab completion (when line ends with \t before newline)
        let trimmed_line = line.trim_end_matches('\n').trim_end_matches('\r');
        if trimmed_line.ends_with('\t') {
            let word = trimmed_line.trim_end_matches('\t').rsplit(|c: char| c.is_whitespace() || c == '(' || c == '{').next().unwrap_or("");
            if !word.is_empty() {
                let completions = ReplSession::complete(word);
                if completions.len() == 1 {
                    println!("{}{}{}", color::CYAN, completions[0], color::RESET);
                } else if !completions.is_empty() {
                    println!("{}{}{}", color::DIM, completions.join("  "), color::RESET);
                }
            }
            continue;
        }

        buffer.push_str(&line);

        // Check if input is complete
        if !ReplSession::is_complete(&buffer) {
            continue;
        }

        let input = buffer.trim().to_string();
        buffer.clear();

        if input.is_empty() {
            continue;
        }

        match session.eval(&input) {
            Ok(Some(val)) => println!("{}=> {}{}", color::GREEN, val, color::RESET),
            Ok(None) => {
                if input == ":help" || input == ":h" {
                    println!("{}", ReplSession::help_text());
                }
            }
            Err(msg) => {
                if msg == "__EXIT__" {
                    println!("{}Goodbye.{}", color::DIM, color::RESET);
                    break;
                }
                // Colorize based on content
                if msg.starts_with("error") || msg.starts_with("parse error")
                    || msg.starts_with("type error") || msg.starts_with("Failed")
                    || msg.starts_with("Usage:")
                {
                    println!("{}{}{}", color::RED, msg, color::RESET);
                } else {
                    println!("{}", msg);
                }
            }
        }
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_repl_eval_expression() {
        let mut session = ReplSession::new();
        let result = session.eval("42");
        assert_eq!(result.unwrap(), Some(42));
    }

    #[test]
    fn test_repl_eval_arithmetic() {
        let mut session = ReplSession::new();
        let result = session.eval("10 + 32");
        assert_eq!(result.unwrap(), Some(42));
    }

    #[test]
    fn test_repl_eval_complex() {
        let mut session = ReplSession::new();
        let result = session.eval("if true { 99 } else { 0 }");
        assert_eq!(result.unwrap(), Some(99));
    }

    #[test]
    fn test_repl_help_command() {
        let mut session = ReplSession::new();
        let result = session.eval(":help");
        assert_eq!(result.unwrap(), None);
    }

    #[test]
    fn test_repl_clear_command() {
        let mut session = ReplSession::new();
        session.eval("42").unwrap();
        assert_eq!(session.history.len(), 1);
        session.eval(":clear").unwrap();
        assert_eq!(session.history.len(), 0);
    }

    #[test]
    fn test_repl_exit_command() {
        let mut session = ReplSession::new();
        let result = session.eval(":exit");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "__EXIT__");
    }

    #[test]
    fn test_repl_is_complete() {
        assert!(ReplSession::is_complete("42"));
        assert!(ReplSession::is_complete("fn foo() { 42 }"));
        assert!(!ReplSession::is_complete("fn foo() {"));
        assert!(ReplSession::is_complete("fn foo() { { } }"));
    }

    #[test]
    fn test_repl_history() {
        let mut session = ReplSession::new();
        session.eval("1").unwrap();
        session.eval("2").unwrap();
        session.eval("3").unwrap();
        assert_eq!(session.history.len(), 3);
    }

    #[test]
    fn test_repl_verbose_toggle() {
        let mut session = ReplSession::new();
        assert!(!session.verbose);
        let _ = session.eval(":verbose");
        assert!(session.verbose);
        let _ = session.eval(":verbose");
        assert!(!session.verbose);
    }

    #[test]
    fn test_repl_empty_input() {
        let mut session = ReplSession::new();
        let result = session.eval("");
        assert_eq!(result.unwrap(), None);
    }

    #[test]
    fn test_repl_unknown_command() {
        let mut session = ReplSession::new();
        let result = session.eval(":nonsense");
        assert!(result.is_err());
    }

    #[test]
    fn test_repl_multiplication() {
        let mut session = ReplSession::new();
        let result = session.eval("6 * 7");
        assert_eq!(result.unwrap(), Some(42));
    }

    #[test]
    fn test_repl_nested_expression() {
        let mut session = ReplSession::new();
        let result = session.eval("(10 + 5) * 2 + 12");
        assert_eq!(result.unwrap(), Some(42));
    }

    #[test]
    fn test_repl_let_binding() {
        let mut session = ReplSession::new();
        // Let binding returns 0 since it's a statement
        let result = session.eval("let x = 10;");
        assert_eq!(result.unwrap(), Some(0));
    }

    #[test]
    fn test_repl_negative() {
        let mut session = ReplSession::new();
        let result = session.eval("-5 + 47");
        assert_eq!(result.unwrap(), Some(42));
    }

    #[test]
    fn test_repl_last_result() {
        let mut session = ReplSession::new();
        session.eval("42").unwrap();
        assert_eq!(session.last_result, Some(42));
        session.eval("99").unwrap();
        assert_eq!(session.last_result, Some(99));
    }

    #[test]
    fn test_repl_function_def() {
        let mut session = ReplSession::new();
        // Defining a function returns 0 (the appended main)
        let result = session.eval("fn add(a: i64, b: i64) -> i64 { a + b }");
        assert_eq!(result.unwrap(), Some(0));
    }

    #[test]
    fn test_repl_while_statement() {
        let mut session = ReplSession::new();
        // Test simple loop-count via the codegen path
        let source = "fn main() -> i64 { let mut i = 0; while i < 10 { i = i + 1 } i }";
        let result = crate::codegen::compile_and_run(source);
        assert_eq!(result.unwrap(), 10);
        // Also verify REPL session tracks history
        session.eval("5 + 5").unwrap();
        assert_eq!(session.history.len(), 1);
    }

    // ─── v122: REPL Enhancement Tests ───────────────────────────────────

    #[test]
    fn test_repl_tab_completion_keyword() {
        let results = ReplSession::complete("whi");
        assert_eq!(results, vec!["while"]);
    }

    #[test]
    fn test_repl_tab_completion_multiple() {
        let results = ReplSession::complete("fo");
        assert!(results.contains(&"for"));
    }

    #[test]
    fn test_repl_tab_completion_builtin() {
        let results = ReplSession::complete("pri");
        assert!(results.contains(&"print"));
        assert!(results.contains(&"println"));
    }

    #[test]
    fn test_repl_tab_completion_no_match() {
        let results = ReplSession::complete("zzz");
        assert!(results.is_empty());
    }

    #[test]
    fn test_repl_tab_completion_exact_no_dup() {
        // If prefix exactly matches a keyword, don't return it
        let results = ReplSession::complete("while");
        assert!(!results.contains(&"while"));
    }

    #[test]
    fn test_repl_is_complete_with_string() {
        // Braces inside strings should not affect brace depth
        assert!(ReplSession::is_complete(r#"let x = "{";"#));
        assert!(ReplSession::is_complete(r#"let x = "{ } {";"#));
    }

    #[test]
    fn test_repl_is_complete_with_comment() {
        // Braces in comments should not affect brace depth
        assert!(ReplSession::is_complete("fn foo() { 42 } // {"));
        assert!(ReplSession::is_complete("// {\n42"));
    }

    #[test]
    fn test_repl_is_complete_escaped_quote() {
        // Escaped quotes in strings should not end the string
        assert!(ReplSession::is_complete(r#"let x = "hello \"world\"";"#));
    }

    #[test]
    fn test_repl_definition_accumulation() {
        let mut session = ReplSession::new();
        // Define a function
        let r = session.eval("fn square(x: i64) -> i64 { x * x }");
        assert_eq!(r.unwrap(), Some(0));
        assert_eq!(session.definitions.len(), 1);
        // Now use it — should have access to accumulated definition
        // (The accumulated defs get prepended to the source)
        assert!(session.definitions[0].contains("fn square"));
    }

    #[test]
    fn test_repl_reset_command() {
        let mut session = ReplSession::new();
        session.eval("fn foo() -> i64 { 1 }").unwrap();
        assert_eq!(session.definitions.len(), 1);
        let result = session.eval(":reset");
        assert!(result.is_err()); // Output via Err channel
        assert!(result.unwrap_err().contains("cleared"));
        assert_eq!(session.definitions.len(), 0);
    }

    #[test]
    fn test_repl_clear_clears_definitions() {
        let mut session = ReplSession::new();
        session.eval("fn foo() -> i64 { 1 }").unwrap();
        assert_eq!(session.definitions.len(), 1);
        session.eval(":clear").unwrap();
        assert_eq!(session.definitions.len(), 0);
        assert_eq!(session.history.len(), 0);
    }

    #[test]
    fn test_repl_defs_command_empty() {
        let mut session = ReplSession::new();
        let result = session.eval(":defs");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("No accumulated"));
    }

    #[test]
    fn test_repl_defs_command_with_defs() {
        let mut session = ReplSession::new();
        session.eval("fn double(x: i64) -> i64 { x * 2 }").unwrap();
        let result = session.eval(":defs");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("fn double"));
    }

    #[test]
    fn test_repl_save_and_load_roundtrip() {
        let mut session = ReplSession::new();
        session.eval("42").unwrap();
        session.eval("10 + 5").unwrap();

        let path = std::env::temp_dir().join("vitalis_repl_test.sl");
        let path_str = path.to_string_lossy().to_string();

        // Save
        let save_result = session.eval(&format!(":save {}", path_str));
        assert!(save_result.is_err());
        assert!(save_result.unwrap_err().contains("Saved 2 entries"));

        // Load
        let load_result = session.eval(&format!(":load {}", path_str));
        assert!(load_result.is_err());
        assert!(load_result.unwrap_err().contains("Loaded"));

        // Cleanup
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_repl_load_nonexistent() {
        let mut session = ReplSession::new();
        let result = session.eval(":load /nonexistent/file.sl");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Failed to load"));
    }

    #[test]
    fn test_repl_run_command() {
        let mut session = ReplSession::new();
        let path = std::env::temp_dir().join("vitalis_repl_run_test.sl");
        std::fs::write(&path, "fn main() -> i64 { 42 }").unwrap();

        let path_str = path.to_string_lossy().to_string();
        let result = session.eval(&format!(":run {}", path_str));
        assert_eq!(result.unwrap(), Some(42));

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_repl_color_constants() {
        // Verify all color codes are valid ANSI escapes
        assert!(color::RESET.starts_with("\x1b["));
        assert!(color::GREEN.starts_with("\x1b["));
        assert!(color::CYAN.starts_with("\x1b["));
        assert!(color::RED.starts_with("\x1b["));
        assert!(color::YELLOW.starts_with("\x1b["));
        assert!(color::BOLD.starts_with("\x1b["));
        assert!(color::DIM.starts_with("\x1b["));
    }

    #[test]
    fn test_repl_help_text_contains_new_commands() {
        let help = ReplSession::help_text();
        assert!(help.contains(":load"));
        assert!(help.contains(":save"));
        assert!(help.contains(":run"));
        assert!(help.contains(":defs"));
        assert!(help.contains(":reset"));
        assert!(help.contains("Tab"));
    }

    #[test]
    fn test_repl_completion_types() {
        let results = ReplSession::complete("i6");
        assert!(results.contains(&"i64"));
        let results = ReplSession::complete("bo");
        assert!(results.contains(&"bool"));
    }
}
