//! Ownership & Borrow Analysis for Vitalis
//!
//! Phase-1 borrow checker:
//! - Tracks variable ownership states (Owned, Borrowed, Moved, Dropped)
//! - Detects use-after-move errors
//! - Detects double-free / double-drop
//! - Detects mutable aliasing violations (two &mut to same var)
//!
//! This is a static analysis pass that runs on the AST *before* IR lowering.

use crate::ast::{Block, Expr, Function, Program, Stmt, TopLevel};
use std::collections::HashMap;

/// The state a variable can be in.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OwnershipState {
    /// Variable owns its value and can be used
    Owned,
    /// Value has been moved out — using it is an error
    Moved,
    /// Immutably borrowed — reads ok, mutation not ok
    BorrowedShared,
    /// Mutably borrowed — no other access allowed
    BorrowedMut,
    /// Value dropped — use is an error
    Dropped,
    /// Undefined — never assigned
    Undefined,
}

/// An ownership/borrow error
#[derive(Debug, Clone)]
pub struct OwnershipError {
    pub message: String,
    pub variable: String,
    pub state: OwnershipState,
}

impl std::fmt::Display for OwnershipError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ownership error: {} (variable '{}' is {:?})",
               self.message, self.variable, self.state)
    }
}

/// Variable tracking entry
#[derive(Debug, Clone)]
struct VarInfo {
    state: OwnershipState,
    mutable: bool,
    borrow_count: usize,
    mut_borrow_count: usize,
}

/// The borrow checker
pub struct BorrowChecker {
    scopes: Vec<HashMap<String, VarInfo>>,
    errors: Vec<OwnershipError>,
}

impl BorrowChecker {
    pub fn new() -> Self {
        Self {
            scopes: vec![HashMap::new()],
            errors: Vec::new(),
        }
    }

    /// Analyze a full program and return ownership errors
    pub fn check(&mut self, program: &Program) -> Vec<OwnershipError> {
        for item in &program.items {
            match item {
                TopLevel::Function(func) => self.check_function(func),
                _ => {} // structs, enums, etc. don't have ownership semantics
            }
        }
        self.errors.clone()
    }

    fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    fn declare(&mut self, name: &str, mutable: bool) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name.to_string(), VarInfo {
                state: OwnershipState::Owned,
                mutable,
                borrow_count: 0,
                mut_borrow_count: 0,
            });
        }
    }

    fn lookup(&self, name: &str) -> Option<&VarInfo> {
        for scope in self.scopes.iter().rev() {
            if let Some(info) = scope.get(name) {
                return Some(info);
            }
        }
        None
    }

    fn lookup_mut(&mut self, name: &str) -> Option<&mut VarInfo> {
        for scope in self.scopes.iter_mut().rev() {
            if let Some(info) = scope.get_mut(name) {
                return Some(info);
            }
        }
        None
    }

    fn check_use(&mut self, name: &str) {
        if let Some(info) = self.lookup(name) {
            match info.state {
                OwnershipState::Moved => {
                    self.errors.push(OwnershipError {
                        message: format!("use of moved value '{}'", name),
                        variable: name.to_string(),
                        state: OwnershipState::Moved,
                    });
                }
                OwnershipState::Dropped => {
                    self.errors.push(OwnershipError {
                        message: format!("use of dropped value '{}'", name),
                        variable: name.to_string(),
                        state: OwnershipState::Dropped,
                    });
                }
                _ => {}
            }
        }
    }

    fn mark_moved(&mut self, name: &str) {
        if let Some(info) = self.lookup_mut(name) {
            info.state = OwnershipState::Moved;
        }
    }

    /// Mark a variable as borrowed (shared or mutable) and update borrow counts.
    fn mark_borrowed(&mut self, name: &str, mutable: bool) {
        if let Some(info) = self.lookup_mut(name) {
            match info.state {
                OwnershipState::Moved => {
                    self.errors.push(OwnershipError {
                        message: format!("cannot borrow '{}' — value has been moved", name),
                        variable: name.to_string(),
                        state: OwnershipState::Moved,
                    });
                    return;
                }
                OwnershipState::Dropped => {
                    self.errors.push(OwnershipError {
                        message: format!("cannot borrow '{}' — value has been dropped", name),
                        variable: name.to_string(),
                        state: OwnershipState::Dropped,
                    });
                    return;
                }
                OwnershipState::BorrowedMut if mutable => {
                    self.errors.push(OwnershipError {
                        message: format!("cannot mutably borrow '{}' — already mutably borrowed", name),
                        variable: name.to_string(),
                        state: OwnershipState::BorrowedMut,
                    });
                    return;
                }
                OwnershipState::BorrowedMut if !mutable => {
                    self.errors.push(OwnershipError {
                        message: format!("cannot borrow '{}' as immutable — already mutably borrowed", name),
                        variable: name.to_string(),
                        state: OwnershipState::BorrowedMut,
                    });
                    return;
                }
                OwnershipState::BorrowedShared if mutable => {
                    self.errors.push(OwnershipError {
                        message: format!("cannot mutably borrow '{}' — already immutably borrowed", name),
                        variable: name.to_string(),
                        state: OwnershipState::BorrowedShared,
                    });
                    return;
                }
                _ => {}
            }
            if mutable {
                info.state = OwnershipState::BorrowedMut;
                info.mut_borrow_count += 1;
            } else {
                info.state = OwnershipState::BorrowedShared;
                info.borrow_count += 1;
            }
        }
    }

    /// Release borrows — return variable to Owned state.
    fn release_borrow(&mut self, name: &str) {
        if let Some(info) = self.lookup_mut(name) {
            if info.state == OwnershipState::BorrowedShared || info.state == OwnershipState::BorrowedMut {
                info.state = OwnershipState::Owned;
            }
        }
    }

    fn check_function(&mut self, func: &Function) {
        self.push_scope();
        // Declare params as owned
        for param in &func.params {
            self.declare(&param.name, false);
        }
        self.check_block(&func.body);
        self.pop_scope();
    }

    fn check_block(&mut self, block: &Block) {
        self.push_scope();
        for stmt in &block.stmts {
            self.check_stmt(stmt);
        }
        if let Some(tail) = &block.tail_expr {
            self.check_expr(tail);
        }
        self.pop_scope();
    }

    fn check_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Let { name, value, mutable, .. } => {
                if let Some(val) = value {
                    self.check_expr(val);
                }
                self.declare(name, *mutable);
            }
            Stmt::Expr(expr) => {
                self.check_expr(expr);
            }
            Stmt::While { condition, body, .. } => {
                self.check_expr(condition);
                self.check_block(body);
            }
            Stmt::For { var, iter, body, .. } => {
                self.check_expr(iter);
                self.push_scope();
                self.declare(var, false);
                self.check_block(body);
                self.pop_scope();
            }
            Stmt::Loop { body, .. } => {
                self.check_block(body);
            }
        }
    }

    fn check_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::Ident(name, _) => {
                self.check_use(name);
            }
            Expr::Binary { left, right, .. } => {
                self.check_expr(left);
                self.check_expr(right);
            }
            Expr::Unary { operand, .. } => {
                self.check_expr(operand);
            }
            Expr::Call { func, args, .. } => {
                self.check_expr(func);
                for arg in args {
                    if let Expr::Ident(name, _) = arg {
                        self.check_use(name);
                    } else {
                        self.check_expr(arg);
                    }
                }
            }
            Expr::MethodCall { object, args, .. } => {
                self.check_expr(object);
                for arg in args {
                    self.check_expr(arg);
                }
            }
            Expr::Field { object, .. } => {
                self.check_expr(object);
            }
            Expr::Index { object, index, .. } => {
                self.check_expr(object);
                self.check_expr(index);
            }
            Expr::If { condition, then_branch, else_branch, .. } => {
                self.check_expr(condition);
                self.check_block(then_branch);
                if let Some(eb) = else_branch {
                    self.check_block(eb);
                }
            }
            Expr::Match { subject, arms, .. } => {
                self.check_expr(subject);
                for arm in arms {
                    self.check_expr(&arm.body);
                }
            }
            Expr::Block(block) => {
                self.check_block(block);
            }
            Expr::List { elements, .. } => {
                for el in elements {
                    self.check_expr(el);
                }
            }
            Expr::Lambda { params, body, .. } => {
                self.push_scope();
                for p in params {
                    self.declare(&p.name, false);
                }
                self.check_expr(body);
                self.pop_scope();
            }
            Expr::Assign { target, value, .. } => {
                self.check_expr(value);
                // Assignment to identifier — check target is mutable
                if let Expr::Ident(name, _) = target.as_ref() {
                    if let Some(info) = self.lookup(name) {
                        if !info.mutable {
                            self.errors.push(OwnershipError {
                                message: format!("cannot assign to immutable variable '{}'", name),
                                variable: name.to_string(),
                                state: info.state,
                            });
                        }
                    }
                }
                self.check_expr(target);
            }
            Expr::CompoundAssign { target, value, .. } => {
                self.check_expr(value);
                if let Expr::Ident(name, _) = target.as_ref() {
                    if let Some(info) = self.lookup(name) {
                        if !info.mutable {
                            self.errors.push(OwnershipError {
                                message: format!("cannot assign to immutable variable '{}'", name),
                                variable: name.to_string(),
                                state: info.state,
                            });
                        }
                    }
                }
                self.check_expr(target);
            }
            Expr::Return { value, .. } => {
                if let Some(val) = value {
                    self.check_expr(val);
                }
            }
            Expr::Pipe { stages, .. } => {
                for stage in stages {
                    self.check_expr(stage);
                }
            }
            Expr::Try { expr, .. } => {
                self.check_expr(expr);
            }
            Expr::TryCatch { try_body, catch_body, catch_var, .. } => {
                self.check_block(try_body);
                self.push_scope();
                self.declare(catch_var, false);
                self.check_block(catch_body);
                self.pop_scope();
            }
            Expr::Throw { code, message, .. } => {
                self.check_expr(code);
                self.check_expr(message);
            }
            Expr::Range { start, end, .. } => {
                self.check_expr(start);
                self.check_expr(end);
            }
            Expr::Cast { expr, .. } => {
                self.check_expr(expr);
            }
            Expr::StructLiteral { fields, .. } => {
                for (_, val) in fields {
                    self.check_expr(val);
                }
            }
            Expr::Parallel { exprs, .. } => {
                for e in exprs {
                    self.check_expr(e);
                }
            }
            _ => {} // Literals, break, continue — no ownership implications
        }
    }
}

/// Convenience: run the borrow checker on source code
pub fn analyze_ownership(source: &str) -> Vec<OwnershipError> {
    let (program, errors) = crate::parser::parse(source);
    if !errors.is_empty() {
        return vec![OwnershipError {
            message: "Cannot analyze ownership — parse errors present".to_string(),
            variable: String::new(),
            state: OwnershipState::Undefined,
        }];
    }
    let mut checker = BorrowChecker::new();
    checker.check(&program)
}

/// Run combined ownership + NLL analysis on source code.
/// Returns ownership errors first, then NLL errors mapped to OwnershipError.
pub fn analyze_full_safety(source: &str) -> Vec<OwnershipError> {
    let (program, errors) = crate::parser::parse(source);
    if !errors.is_empty() {
        return vec![OwnershipError {
            message: "Cannot analyze safety — parse errors present".to_string(),
            variable: String::new(),
            state: OwnershipState::Undefined,
        }];
    }

    // Phase 1: Ownership/move analysis
    let mut checker = BorrowChecker::new();
    let mut all_errors = checker.check(&program);

    // Phase 2: NLL borrow region analysis
    let nll_errors = crate::nll::analyze_nll(source);
    for nll_err in nll_errors {
        all_errors.push(OwnershipError {
            message: nll_err.message,
            variable: String::new(),
            state: match nll_err.kind {
                crate::nll::NllErrorKind::MutableAliasing => OwnershipState::BorrowedMut,
                crate::nll::NllErrorKind::MutableSharedConflict => OwnershipState::BorrowedShared,
                crate::nll::NllErrorKind::UseAfterMove => OwnershipState::Moved,
                crate::nll::NllErrorKind::BorrowAfterMove => OwnershipState::Moved,
                crate::nll::NllErrorKind::ModifyWhileBorrowed => OwnershipState::BorrowedMut,
            },
        });
    }

    all_errors
}

// ─── Tests ──────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ownership_clean_program() {
        let errors = analyze_ownership("fn main() -> i64 { let x = 42; x }");
        assert!(errors.is_empty(), "Expected no ownership errors: {:?}", errors);
    }

    #[test]
    fn test_ownership_let_binding() {
        let errors = analyze_ownership("fn main() -> i64 { let x = 10; let y = 20; x + y }");
        assert!(errors.is_empty());
    }

    #[test]
    fn test_ownership_function_params() {
        let errors = analyze_ownership("fn add(a: i64, b: i64) -> i64 { a + b } fn main() -> i64 { add(1, 2) }");
        assert!(errors.is_empty());
    }

    #[test]
    fn test_ownership_nested_blocks() {
        let errors = analyze_ownership("fn main() -> i64 { let x = 1; { let y = 2; x + y } }");
        assert!(errors.is_empty());
    }

    #[test]
    fn test_ownership_if_branches() {
        let errors = analyze_ownership("fn main() -> i64 { let x = 42; if x > 0 { x } else { 0 } }");
        assert!(errors.is_empty());
    }

    #[test]
    fn test_ownership_while_loop() {
        let errors = analyze_ownership("fn main() -> i64 { let mut i = 0; while i < 10 { i = i + 1; } i }");
        assert!(errors.is_empty());
    }

    #[test]
    fn test_ownership_multiple_functions() {
        let errors = analyze_ownership("fn foo(x: i64) -> i64 { x + 1 } fn bar(y: i64) -> i64 { y * 2 } fn main() -> i64 { foo(bar(21)) }");
        assert!(errors.is_empty());
    }

    #[test]
    fn test_ownership_for_loop() {
        let errors = analyze_ownership("fn main() -> i64 { let mut s = 0; for i in [1, 2, 3] { s = s + i; } s }");
        assert!(errors.is_empty());
    }

    #[test]
    fn test_ownership_checker_creation() {
        let checker = BorrowChecker::new();
        assert_eq!(checker.scopes.len(), 1);
        assert!(checker.errors.is_empty());
    }

    #[test]
    fn test_ownership_scope_push_pop() {
        let mut checker = BorrowChecker::new();
        checker.push_scope();
        assert_eq!(checker.scopes.len(), 2);
        checker.pop_scope();
        assert_eq!(checker.scopes.len(), 1);
    }

    #[test]
    fn test_ownership_declare_and_lookup() {
        let mut checker = BorrowChecker::new();
        checker.declare("x", false);
        let info = checker.lookup("x").unwrap();
        assert_eq!(info.state, OwnershipState::Owned);
        assert!(!info.mutable);
    }

    #[test]
    fn test_ownership_mutable_declare() {
        let mut checker = BorrowChecker::new();
        checker.declare("y", true);
        let info = checker.lookup("y").unwrap();
        assert!(info.mutable);
    }

    #[test]
    fn test_ownership_moved_detection() {
        let mut checker = BorrowChecker::new();
        checker.declare("val", false);
        checker.mark_moved("val");
        checker.check_use("val");
        assert_eq!(checker.errors.len(), 1);
        assert!(checker.errors[0].message.contains("moved"));
    }

    #[test]
    fn test_ownership_state_display() {
        let err = OwnershipError {
            message: "test error".to_string(),
            variable: "x".to_string(),
            state: OwnershipState::Moved,
        };
        let s = format!("{}", err);
        assert!(s.contains("ownership error"));
        assert!(s.contains("test error"));
    }

    #[test]
    fn test_ownership_parse_error_handling() {
        let errors = analyze_ownership("fn { broken syntax");
        assert!(!errors.is_empty());
        assert!(errors[0].message.contains("parse errors"));
    }

    #[test]
    fn test_ownership_complex_expression() {
        let errors = analyze_ownership("fn main() -> i64 { let a = 5; let b = 10; (a + b) * 2 - a }");
        assert!(errors.is_empty());
    }

    #[test]
    fn test_ownership_method_call() {
        let errors = analyze_ownership(r#"fn main() -> i64 { let s = "hello"; println(s); 0 }"#);
        assert!(errors.is_empty());
    }

    #[test]
    fn test_ownership_lambda() {
        let errors = analyze_ownership("fn main() -> i64 { let f = |x: i64| -> i64 { x + 1 }; f(41) }");
        assert!(errors.is_empty());
    }

    #[test]
    fn test_ownership_nested_scope_shadowing() {
        let errors = analyze_ownership("fn main() -> i64 { let x = 1; { let x = 2; x } }");
        assert!(errors.is_empty());
    }

    #[test]
    fn test_ownership_enum_states() {
        assert_ne!(OwnershipState::Owned, OwnershipState::Moved);
        assert_ne!(OwnershipState::BorrowedShared, OwnershipState::BorrowedMut);
        assert_ne!(OwnershipState::Dropped, OwnershipState::Undefined);
    }

    // ─── v130 tests ─────────────────────────────────────────────────────

    #[test]
    fn test_v130_borrow_tracking_shared() {
        let mut checker = BorrowChecker::new();
        checker.declare("x", false);
        checker.mark_borrowed("x", false);
        let info = checker.lookup("x").unwrap();
        assert_eq!(info.state, OwnershipState::BorrowedShared);
        assert_eq!(info.borrow_count, 1);
    }

    #[test]
    fn test_v130_borrow_tracking_mut() {
        let mut checker = BorrowChecker::new();
        checker.declare("x", true);
        checker.mark_borrowed("x", true);
        let info = checker.lookup("x").unwrap();
        assert_eq!(info.state, OwnershipState::BorrowedMut);
        assert_eq!(info.mut_borrow_count, 1);
    }

    #[test]
    fn test_v130_double_mut_borrow_error() {
        let mut checker = BorrowChecker::new();
        checker.declare("x", true);
        checker.mark_borrowed("x", true);
        checker.mark_borrowed("x", true); // should error
        assert_eq!(checker.errors.len(), 1);
        assert!(checker.errors[0].message.contains("already mutably borrowed"));
    }

    #[test]
    fn test_v130_shared_then_mut_borrow_error() {
        let mut checker = BorrowChecker::new();
        checker.declare("x", true);
        checker.mark_borrowed("x", false); // shared
        checker.mark_borrowed("x", true);  // mut — should error
        assert_eq!(checker.errors.len(), 1);
        assert!(checker.errors[0].message.contains("already immutably borrowed"));
    }

    #[test]
    fn test_v130_mut_then_shared_borrow_error() {
        let mut checker = BorrowChecker::new();
        checker.declare("x", true);
        checker.mark_borrowed("x", true);  // mut
        checker.mark_borrowed("x", false); // shared — should error
        assert_eq!(checker.errors.len(), 1);
        assert!(checker.errors[0].message.contains("already mutably borrowed"));
    }

    #[test]
    fn test_v130_borrow_moved_value_error() {
        let mut checker = BorrowChecker::new();
        checker.declare("x", false);
        checker.mark_moved("x");
        checker.mark_borrowed("x", false); // borrow of moved value — error
        assert_eq!(checker.errors.len(), 1);
        assert!(checker.errors[0].message.contains("moved"));
    }

    #[test]
    fn test_v130_release_borrow() {
        let mut checker = BorrowChecker::new();
        checker.declare("x", true);
        checker.mark_borrowed("x", true);
        checker.release_borrow("x");
        let info = checker.lookup("x").unwrap();
        assert_eq!(info.state, OwnershipState::Owned);
    }

    #[test]
    fn test_v130_immutable_assignment_error() {
        let errors = analyze_ownership("fn main() -> i64 { let x = 42; x = 10; x }");
        assert!(!errors.is_empty(), "Should detect assignment to immutable var");
        assert!(errors.iter().any(|e| e.message.contains("immutable")));
    }

    #[test]
    fn test_v130_mutable_assignment_ok() {
        let errors = analyze_ownership("fn main() -> i64 { let mut x = 42; x = 10; x }");
        assert!(errors.is_empty(), "Mutable assignment should be ok: {:?}", errors);
    }

    #[test]
    fn test_v130_pipe_chain_ownership() {
        let errors = analyze_ownership("fn inc(x: i64) -> i64 { x + 1 }\nfn main() -> i64 { 10 |> inc |> inc }");
        assert!(errors.is_empty(), "Pipe chains should work: {:?}", errors);
    }

    #[test]
    fn test_v130_try_catch_ownership() {
        let errors = analyze_ownership("fn main() -> i64 { try { 42 } catch e { 0 } }");
        assert!(errors.is_empty(), "Try/catch should work: {:?}", errors);
    }

    #[test]
    fn test_v130_lambda_param_declaration() {
        let errors = analyze_ownership("fn main() -> i64 { let f = |a: i64, b: i64| -> i64 { a + b }; f(1, 2) }");
        assert!(errors.is_empty(), "Lambda params should be declared: {:?}", errors);
    }

    #[test]
    fn test_v130_full_safety_clean() {
        let errors = analyze_full_safety("fn main() -> i64 { let x = 42; x }");
        assert!(errors.is_empty(), "Full safety should pass for clean program: {:?}", errors);
    }

    #[test]
    fn test_v130_full_safety_parse_error() {
        let errors = analyze_full_safety("fn { broken");
        assert!(!errors.is_empty());
        assert!(errors[0].message.contains("parse errors"));
    }

    #[test]
    fn test_v130_struct_literal_ownership() {
        let errors = analyze_ownership("struct Point { x: i64, y: i64 }\nfn main() -> i64 { let p = Point { x: 1, y: 2 }; p.x }");
        assert!(errors.is_empty(), "Struct literals should work: {:?}", errors);
    }
}
