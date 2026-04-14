//! Vitalis Type System — structural typing with capability annotations.
//!
//! Types carry safety metadata (trust tiers, mutability permissions,
//! evolvability flags) that is enforced at compile time. This prevents
//! evolved code from violating safety invariants.

use crate::ast::*;
use std::collections::HashMap;
use std::fmt;

// ─── Internal Type Representation ───────────────────────────────────────
#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    I32,
    I64,
    F32,
    F64,
    Bool,
    Str,
    Void,
    /// Named user type (struct / enum)
    Named(String),
    /// List[T]
    List(Box<Type>),
    /// Map[K, V]
    Map(Box<Type>, Box<Type>),
    /// Option[T]
    Option(Box<Type>),
    /// Result[T, E]
    Result(Box<Type>, Box<Type>),
    /// Future[T]
    Future(Box<Type>),
    /// Function type: (params...) -> ret
    Function {
        params: Vec<Type>,
        ret: Box<Type>,
    },
    /// Array: [T; N]
    Array(Box<Type>, Option<usize>),
    /// Reference: &T or &mut T
    Ref {
        inner: Box<Type>,
        mutable: bool,
    },
    /// Tuple type: (T1, T2, ...)
    Tuple(Vec<Type>),
    /// Union type: T1 | T2 | ...
    Union(Vec<Type>),
    /// Intersection type: T1 & T2 & ...
    Intersection(Vec<Type>),
    /// Never type (bottom) — a function that never returns
    Never,
    /// Type variable (for generics, inference)
    Var(u32),
    /// Error sentinel — used during recovery
    Error,
    /// Not yet resolved
    Unknown,
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Type::I32 => write!(f, "i32"),
            Type::I64 => write!(f, "i64"),
            Type::F32 => write!(f, "f32"),
            Type::F64 => write!(f, "f64"),
            Type::Bool => write!(f, "bool"),
            Type::Str => write!(f, "str"),
            Type::Void => write!(f, "void"),
            Type::Named(n) => write!(f, "{}", n),
            Type::List(t) => write!(f, "list[{}]", t),
            Type::Map(k, v) => write!(f, "map[{}, {}]", k, v),
            Type::Option(t) => write!(f, "option[{}]", t),
            Type::Result(t, e) => write!(f, "result[{}, {}]", t, e),
            Type::Future(t) => write!(f, "future[{}]", t),
            Type::Function { params, ret } => {
                write!(f, "fn(")?;
                for (i, p) in params.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}", p)?;
                }
                write!(f, ") -> {}", ret)
            }
            Type::Array(t, Some(n)) => write!(f, "[{}; {}]", t, n),
            Type::Array(t, None) => write!(f, "[{}]", t),
            Type::Ref { inner, mutable } => {
                if *mutable { write!(f, "&mut {}", inner) }
                else { write!(f, "&{}", inner) }
            }
            Type::Tuple(ts) => {
                write!(f, "(")?;
                for (i, t) in ts.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}", t)?;
                }
                write!(f, ")")
            }
            Type::Union(ts) => {
                for (i, t) in ts.iter().enumerate() {
                    if i > 0 { write!(f, " | ")?; }
                    write!(f, "{}", t)?;
                }
                Ok(())
            }
            Type::Intersection(ts) => {
                for (i, t) in ts.iter().enumerate() {
                    if i > 0 { write!(f, " & ")?; }
                    write!(f, "{}", t)?;
                }
                Ok(())
            }
            Type::Never => write!(f, "never"),
            Type::Var(id) => write!(f, "?T{}", id),
            Type::Error => write!(f, "<error>"),
            Type::Unknown => write!(f, "<unknown>"),
        }
    }
}

// ─── Type Errors ────────────────────────────────────────────────────────
#[derive(Debug, Clone)]
pub struct TypeError {
    pub message: String,
    pub span: Span,
    /// Optional suggestion hint (e.g., "did you mean 'x'?")
    pub hint: Option<String>,
}

impl fmt::Display for TypeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "type error at {}..{}: {}", self.span.start, self.span.end, self.message)?;
        if let Some(hint) = &self.hint {
            write!(f, "\n  hint: {}", hint)?;
        }
        Ok(())
    }
}

// ─── Symbol Table ───────────────────────────────────────────────────────
#[derive(Debug, Clone)]
pub struct SymbolInfo {
    pub ty: Type,
    pub mutable: bool,
    pub evolvable: bool,
    pub trust_tier: Option<u8>,
}

#[derive(Debug)]
pub struct Scope {
    symbols: HashMap<String, SymbolInfo>,
    parent: Option<Box<Scope>>,
}

impl Scope {
    pub fn new() -> Self {
        Self {
            symbols: HashMap::new(),
            parent: None,
        }
    }

    pub fn child(parent: Scope) -> Self {
        Self {
            symbols: HashMap::new(),
            parent: Some(Box::new(parent)),
        }
    }

    pub fn define(&mut self, name: String, info: SymbolInfo) {
        self.symbols.insert(name, info);
    }

    pub fn lookup(&self, name: &str) -> Option<&SymbolInfo> {
        self.symbols
            .get(name)
            .or_else(|| self.parent.as_ref().and_then(|p| p.lookup(name)))
    }

    pub fn into_parent(self) -> Option<Scope> {
        self.parent.map(|b| *b)
    }

    /// Collect all visible symbol names (for "did you mean?" suggestions).
    pub fn all_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.symbols.keys().cloned().collect();
        if let Some(parent) = &self.parent {
            names.extend(parent.all_names());
        }
        names
    }
}

// ─── Type Checker ───────────────────────────────────────────────────────
pub struct TypeChecker {
    scope: Scope,
    errors: Vec<TypeError>,
    next_var: u32,
    /// Struct definitions: name -> fields
    struct_defs: HashMap<String, Vec<(String, Type)>>,
    /// Enum definitions: name -> variants
    enum_defs: HashMap<String, Vec<(String, Vec<Type>)>>,
    /// Function signatures for forward references
    fn_sigs: HashMap<String, (Vec<Type>, Type)>,
    /// v132: Hindley-Milner inference engine for Var resolution
    infer_engine: crate::type_inference::InferEngine,
    /// v134: Trait dispatch registry for trait/impl validation
    trait_dispatcher: crate::trait_dispatch::TraitDispatcher,
}

impl TypeChecker {
    pub fn new() -> Self {
        let mut checker = Self {
            scope: Scope::new(),
            errors: Vec::new(),
            next_var: 0,
            struct_defs: HashMap::new(),
            enum_defs: HashMap::new(),
            fn_sigs: HashMap::new(),
            infer_engine: crate::type_inference::InferEngine::new(),
            trait_dispatcher: crate::trait_dispatch::TraitDispatcher::new(),
        };
        // v129: Auto-derive type checker registrations from stdlib (single source of truth)
        crate::stdlib::register_builtins_for_typechecker(
            &mut |name, params, ret| checker.register_builtin(name, params, ret),
        );
        // Extras not in stdlib.rs (language-level builtins)
        checker.register_builtin("assert", vec![Type::Bool], Type::Void);
        checker.register_builtin("panic", vec![Type::Str], Type::Void);
        checker.register_builtin("to_string", vec![Type::I64], Type::Str);
        checker.register_builtin("len", vec![Type::Str], Type::I64);

        checker
    }

    fn register_builtin(&mut self, name: &str, params: Vec<Type>, ret: Type) {
        self.fn_sigs.insert(name.to_string(), (params.clone(), ret.clone()));
        self.scope.define(
            name.to_string(),
            SymbolInfo {
                ty: Type::Function { params, ret: Box::new(ret) },
                mutable: false,
                evolvable: false,
                trust_tier: None,
            },
        );
    }

    fn fresh_var(&mut self) -> Type {
        let v = self.next_var;
        self.next_var += 1;
        Type::Var(v)
    }

    fn error(&mut self, message: impl Into<String>, span: &Span) {
        self.errors.push(TypeError {
            message: message.into(),
            span: span.clone(),
            hint: None,
        });
    }

    fn error_with_hint(&mut self, message: impl Into<String>, span: &Span, hint: impl Into<String>) {
        self.errors.push(TypeError {
            message: message.into(),
            span: span.clone(),
            hint: Some(hint.into()),
        });
    }

    // ── Public Entry Point ──────────────────────────────────────────
    pub fn check(mut self, program: &Program) -> Vec<TypeError> {
        // First pass: collect signatures
        for item in &program.items {
            self.collect_signatures(item);
        }

        // Second pass: type-check bodies
        for item in &program.items {
            self.check_top_level(item);
        }

        self.errors
    }

    fn collect_signatures(&mut self, item: &TopLevel) {
        match item {
            TopLevel::Function(f) => {
                let params: Vec<Type> = f.params.iter().map(|p| self.resolve_type_expr(&p.ty)).collect();
                let ret = f.return_type.as_ref().map(|t| self.resolve_type_expr(t)).unwrap_or(Type::Void);
                self.fn_sigs.insert(f.name.clone(), (params.clone(), ret.clone()));
                self.scope.define(
                    f.name.clone(),
                    SymbolInfo {
                        ty: Type::Function { params, ret: Box::new(ret) },
                        mutable: false,
                        evolvable: false,
                        trust_tier: None,
                    },
                );
            }
            TopLevel::Struct(s) => {
                let fields: Vec<(String, Type)> = s.fields.iter()
                    .map(|f| (f.name.clone(), self.resolve_type_expr(&f.ty)))
                    .collect();
                self.struct_defs.insert(s.name.clone(), fields);
            }
            TopLevel::Enum(e) => {
                let variants: Vec<(String, Vec<Type>)> = e.variants.iter()
                    .map(|v| (v.name.clone(), v.fields.iter().map(|t| self.resolve_type_expr(t)).collect()))
                    .collect();
                self.enum_defs.insert(e.name.clone(), variants);
            }
            TopLevel::Annotated { item, .. } => {
                self.collect_signatures(item);
            }
            TopLevel::Module(m) => {
                for sub in &m.items {
                    self.collect_signatures(sub);
                }
            }
            TopLevel::Impl(imp) => {
                // Collect impl method signatures as TypeName_method
                for method in &imp.methods {
                    let mangled = format!("{}_{}", imp.type_name, method.name);
                    let params: Vec<Type> = method.params.iter().map(|p| self.resolve_type_expr(&p.ty)).collect();
                    let ret = method.return_type.as_ref().map(|t| self.resolve_type_expr(t)).unwrap_or(Type::Void);
                    self.fn_sigs.insert(mangled.clone(), (params.clone(), ret.clone()));
                }
                // v134: If this impl is for a trait, validate against registered trait
                if let Some(ref trait_name) = imp.trait_name {
                    let method_names: Vec<String> = imp.methods.iter().map(|m| m.name.clone()).collect();
                    if let Err(err) = self.trait_dispatcher.register_impl(
                        &imp.type_name, trait_name, method_names,
                    ) {
                        self.error(err, &imp.span);
                    }
                }
            }
            TopLevel::Trait(t) => {
                // v134: Register trait definition in dispatcher
                let methods: Vec<crate::trait_dispatch::TraitMethodSig> = t.methods.iter().map(|m| {
                    crate::trait_dispatch::TraitMethodSig {
                        name: m.name.clone(),
                        param_count: m.params.len(),
                        has_self: m.params.first().map_or(false, |p| p.name == "self"),
                        has_default: m.has_default,
                    }
                }).collect();
                self.trait_dispatcher.register_trait(&t.name, methods);
            }
            _ => {}
        }
    }

    // ── Resolve AST TypeExpr → internal Type ────────────────────────
    pub fn resolve_type_expr(&mut self, texpr: &TypeExpr) -> Type {
        match texpr {
            TypeExpr::Named(name, _) => match name.as_str() {
                "i32" => Type::I32,
                "i64" => Type::I64,
                "f32" => Type::F32,
                "f64" => Type::F64,
                "bool" => Type::Bool,
                "str" => Type::Str,
                "void" => Type::Void,
                "never" => Type::Never,
                _ => Type::Named(name.clone()),
            },
            TypeExpr::Generic { name, args, span } => {
                let resolved: Vec<Type> = args.iter().map(|a| self.resolve_type_expr(a)).collect();
                match name.as_str() {
                    "list" if resolved.len() == 1 => Type::List(Box::new(resolved[0].clone())),
                    "map" if resolved.len() == 2 => Type::Map(Box::new(resolved[0].clone()), Box::new(resolved[1].clone())),
                    "option" if resolved.len() == 1 => Type::Option(Box::new(resolved[0].clone())),
                    "result" if resolved.len() == 2 => Type::Result(Box::new(resolved[0].clone()), Box::new(resolved[1].clone())),
                    "future" if resolved.len() == 1 => Type::Future(Box::new(resolved[0].clone())),
                    "tuple" => Type::Tuple(resolved),
                    "union" if resolved.len() >= 2 => Type::Union(resolved),
                    _ => {
                        self.error(format!("unknown generic type '{}'", name), span);
                        Type::Error
                    }
                }
            }
            TypeExpr::Function { params, ret, .. } => {
                let ps: Vec<Type> = params.iter().map(|p| self.resolve_type_expr(p)).collect();
                let r = self.resolve_type_expr(ret);
                Type::Function { params: ps, ret: Box::new(r) }
            }
            TypeExpr::Array { elem, size, .. } => {
                let e = self.resolve_type_expr(elem);
                Type::Array(Box::new(e), *size)
            }
            TypeExpr::Ref { inner, mutable, .. } => {
                let i = self.resolve_type_expr(inner);
                Type::Ref { inner: Box::new(i), mutable: *mutable }
            }
            TypeExpr::Inferred(_) => self.fresh_var(),
        }
    }

    // ── Check Top-Level Items ───────────────────────────────────────
    fn check_top_level(&mut self, item: &TopLevel) {
        match item {
            TopLevel::Function(f) => self.check_function(f),
            TopLevel::Struct(_) => {} // Already collected in first pass
            TopLevel::Enum(_) => {}
            TopLevel::Annotated { item, .. } => self.check_top_level(item),
            TopLevel::Module(m) => {
                for sub in &m.items {
                    self.check_top_level(sub);
                }
            }
            TopLevel::Const(c) => {
                let val_ty = self.check_expr(&c.value);
                if let Some(ref declared) = c.ty {
                    let expected = self.resolve_type_expr(declared);
                    self.unify(&expected, &val_ty, &c.span);
                }
                self.scope.define(
                    c.name.clone(),
                    SymbolInfo {
                        ty: val_ty,
                        mutable: false,
                        evolvable: false,
                        trust_tier: None,
                    },
                );
            }
            TopLevel::Impl(imp) => {
                // v134: Type-check impl method bodies
                for method in &imp.methods {
                    self.check_function(method);
                }
            }
            _ => {}
        }
    }

    fn check_function(&mut self, f: &Function) {
        // Push a new scope for the function body
        let old_scope = std::mem::replace(&mut self.scope, Scope::new());
        self.scope = Scope::child(old_scope);

        // Bind parameters
        for param in &f.params {
            let ty = self.resolve_type_expr(&param.ty);
            self.scope.define(
                param.name.clone(),
                SymbolInfo {
                    ty,
                    mutable: false,
                    evolvable: false,
                    trust_tier: None,
                },
            );
        }

        // Check body
        let body_ty = self.check_block(&f.body);

        // Verify return type
        if let Some(ref ret_texpr) = f.return_type {
            let expected = self.resolve_type_expr(ret_texpr);
            self.unify(&expected, &body_ty, &f.span);
        }

        // Pop scope
        let old_scope = self.take_scope();
        if let Some(parent) = old_scope.into_parent() {
            self.scope = parent;
        }
    }

    fn take_scope(&mut self) -> Scope {
        std::mem::replace(&mut self.scope, Scope::new())
    }

    // ── Check Block ─────────────────────────────────────────────────
    fn check_block(&mut self, block: &Block) -> Type {
        for stmt in &block.stmts {
            self.check_stmt(stmt);
        }
        if let Some(ref tail) = block.tail_expr {
            self.check_expr(tail)
        } else {
            Type::Void
        }
    }

    // ── Check Statements ────────────────────────────────────────────
    fn check_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Let { name, ty, value, mutable, span } => {
                let val_ty = if let Some(v) = value {
                    self.check_expr(v)
                } else {
                    Type::Unknown
                };

                let declared = if let Some(texpr) = ty {
                    let expected = self.resolve_type_expr(texpr);
                    if !matches!(val_ty, Type::Unknown) {
                        self.unify(&expected, &val_ty, span);
                    }
                    expected
                } else {
                    val_ty
                };

                self.scope.define(
                    name.clone(),
                    SymbolInfo {
                        ty: declared,
                        mutable: *mutable,
                        evolvable: false,
                        trust_tier: None,
                    },
                );
            }
            Stmt::Expr(e) => {
                self.check_expr(e);
            }
            Stmt::While { condition, body, span } => {
                let cond_ty = self.check_expr(condition);
                self.unify(&Type::Bool, &cond_ty, span);
                self.check_block(body);
            }
            Stmt::For { var, iter, body, span } => {
                // Special-case: for x in start..end  → loop variable is i64
                let elem_ty = if matches!(iter, Expr::Range { .. }) {
                    self.check_expr(iter); // still type-check start/end
                    Type::I64
                } else {
                    let iter_ty = self.check_expr(iter);
                    match &iter_ty {
                        Type::List(inner) => *inner.clone(),
                        Type::Array(inner, _) => *inner.clone(),
                        _ => {
                            self.error(format!("cannot iterate over type '{}'", iter_ty), span);
                            Type::Error
                        }
                    }
                };

                let old_scope = self.take_scope();
                self.scope = Scope::child(old_scope);
                self.scope.define(
                    var.clone(),
                    SymbolInfo {
                        ty: elem_ty,
                        mutable: false,
                        evolvable: false,
                        trust_tier: None,
                    },
                );
                self.check_block(body);
                let old_scope = self.take_scope();
                if let Some(parent) = old_scope.into_parent() {
                    self.scope = parent;
                }
            }
            Stmt::Loop { body, .. } => {
                self.check_block(body);
            }
        }
    }

    // ── Check Expressions ───────────────────────────────────────────
    fn check_expr(&mut self, expr: &Expr) -> Type {
        match expr {
            Expr::IntLiteral(_, _) => Type::I64,
            Expr::FloatLiteral(_, _) => Type::F64,
            Expr::StringLiteral(_, _) => Type::Str,
            Expr::BoolLiteral(_, _) => Type::Bool,

            Expr::Ident(name, span) => {
                if let Some(info) = self.scope.lookup(name) {
                    info.ty.clone()
                } else {
                    // "Did you mean?" suggestion via edit distance
                    let names = self.scope.all_names();
                    let candidates: Vec<&str> = names.iter().map(|s| s.as_str()).collect();
                    let matches = crate::error_recovery::find_similar(name, &candidates, 2);
                    if let Some((suggestion, _)) = matches.first() {
                        self.error_with_hint(
                            format!("undefined variable '{}'", name),
                            span,
                            format!("did you mean '{}'?", suggestion),
                        );
                    } else {
                        self.error(format!("undefined variable '{}'", name), span);
                    }
                    Type::Error
                }
            }

            Expr::Binary { op, left, right, span } => {
                let lt = self.check_expr(left);
                let rt = self.check_expr(right);
                self.check_binary_op(*op, &lt, &rt, span)
            }

            Expr::Unary { op, operand, span } => {
                let t = self.check_expr(operand);
                match op {
                    UnaryOp::Neg => {
                        if !self.is_numeric(&t) {
                            self.error(format!("cannot negate type '{}'", t), span);
                        }
                        t
                    }
                    UnaryOp::Not => {
                        self.unify(&Type::Bool, &t, span);
                        Type::Bool
                    }
                }
            }

            Expr::Call { func, args, span } => {
                let func_ty = self.check_expr(func);
                match func_ty {
                    Type::Function { params, ret } => {
                        if args.len() != params.len() {
                            self.error(
                                format!("expected {} arguments, got {}", params.len(), args.len()),
                                span,
                            );
                        }
                        for (arg, expected) in args.iter().zip(params.iter()) {
                            let arg_ty = self.check_expr(arg);
                            self.unify(expected, &arg_ty, arg.span());
                        }
                        *ret
                    }
                    Type::Error => Type::Error,
                    _ => {
                        self.error(format!("type '{}' is not callable", func_ty), span);
                        Type::Error
                    }
                }
            }

            Expr::MethodCall { object, method, args, span } => {
                let _obj_ty = self.check_expr(object);
                // For Phase 0 — method calls are loosely typed
                for arg in args {
                    self.check_expr(arg);
                }
                let _ = method;
                let _ = span;
                Type::Unknown
            }

            Expr::Field { object, field, span } => {
                let obj_ty = self.check_expr(object);
                match &obj_ty {
                    Type::Named(name) => {
                        if let Some(fields) = self.struct_defs.get(name).cloned() {
                            if let Some((_, fty)) = fields.iter().find(|(n, _)| n == field) {
                                fty.clone()
                            } else {
                                self.error(format!("struct '{}' has no field '{}'", name, field), span);
                                Type::Error
                            }
                        } else {
                            Type::Unknown
                        }
                    }
                    _ => {
                        // Allow field access on unknown types for flexibility
                        Type::Unknown
                    }
                }
            }

            Expr::Index { object, index, span } => {
                let obj_ty = self.check_expr(object);
                let idx_ty = self.check_expr(index);
                match &obj_ty {
                    Type::List(inner) => {
                        self.unify(&Type::I64, &idx_ty, span);
                        *inner.clone()
                    }
                    Type::Array(inner, _) => {
                        self.unify(&Type::I64, &idx_ty, span);
                        *inner.clone()
                    }
                    Type::Map(k, v) => {
                        self.unify(k, &idx_ty, span);
                        *v.clone()
                    }
                    _ => {
                        self.error(format!("type '{}' is not indexable", obj_ty), span);
                        Type::Error
                    }
                }
            }

            Expr::If { condition, then_branch, else_branch, span } => {
                let cond_ty = self.check_expr(condition);
                self.unify(&Type::Bool, &cond_ty, span);
                let then_ty = self.check_block(then_branch);
                if let Some(else_b) = else_branch {
                    let else_ty = self.check_block(else_b);
                    self.unify(&then_ty, &else_ty, span);
                    then_ty
                } else {
                    Type::Void
                }
            }

            Expr::Match { subject, arms, span } => {
                let subj_ty = self.check_expr(subject);
                let mut result_ty = Type::Unknown;
                for arm in arms {
                    let arm_ty = self.check_expr(&arm.body);
                    if matches!(result_ty, Type::Unknown) {
                        result_ty = arm_ty;
                    } else {
                        self.unify(&result_ty, &arm_ty, span);
                    }
                }
                // v140: Pattern exhaustiveness check
                let type_desc = self.type_to_desc(&subj_ty);
                let result = crate::pattern_exhaustiveness::analyze_match_arms(arms, &type_desc);
                if !result.is_exhaustive {
                    let missing = result.missing_patterns.join(", ");
                    self.errors.push(TypeError {
                        message: format!(
                            "non-exhaustive match: missing patterns: {}",
                            missing
                        ),
                        span: *span,
                        hint: None,
                    });
                }
                for idx in &result.redundant_arms {
                    if *idx < arms.len() {
                        self.errors.push(TypeError {
                            message: format!("unreachable match arm #{}", idx + 1),
                            span: arms[*idx].span,
                            hint: None,
                        });
                    }
                }
                result_ty
            }

            Expr::Block(block) => self.check_block(block),

            Expr::List { elements, .. } => {
                if elements.is_empty() {
                    Type::List(Box::new(self.fresh_var()))
                } else {
                    let first = self.check_expr(&elements[0]);
                    for elem in &elements[1..] {
                        let t = self.check_expr(elem);
                        self.unify(&first, &t, elem.span());
                    }
                    Type::List(Box::new(first))
                }
            }

            Expr::StructLiteral { name, fields, span } => {
                if let Some(def_fields) = self.struct_defs.get(name).cloned() {
                    for (fname, fexpr) in fields {
                        let fty = self.check_expr(fexpr);
                        if let Some((_, expected)) = def_fields.iter().find(|(n, _)| n == fname) {
                            self.unify(expected, &fty, fexpr.span());
                        } else {
                            self.error(format!("struct '{}' has no field '{}'", name, fname), span);
                        }
                    }
                    Type::Named(name.clone())
                } else {
                    self.error(format!("unknown struct '{}'", name), span);
                    Type::Error
                }
            }

            Expr::Lambda { params, body, .. } => {
                let old_scope = self.take_scope();
                self.scope = Scope::child(old_scope);
                let param_types: Vec<Type> = params.iter().map(|p| {
                    let ty = self.resolve_type_expr(&p.ty);
                    self.scope.define(p.name.clone(), SymbolInfo {
                        ty: ty.clone(),
                        mutable: false,
                        evolvable: false,
                        trust_tier: None,
                    });
                    ty
                }).collect();
                let ret = self.check_expr(body);
                let old_scope = self.take_scope();
                if let Some(parent) = old_scope.into_parent() {
                    self.scope = parent;
                }
                Type::Function {
                    params: param_types,
                    ret: Box::new(ret),
                }
            }

            Expr::Pipe { stages, .. } => {
                // The last stage's return type is the pipe's type
                // (simplified — full version would thread types through)
                let mut last = Type::Unknown;
                for stage in stages {
                    last = self.check_expr(stage);
                }
                last
            }

            Expr::Parallel { exprs, .. } => {
                // Returns a tuple/list of all results — simplified as list
                let mut types = Vec::new();
                for e in exprs {
                    types.push(self.check_expr(e));
                }
                if types.is_empty() {
                    Type::Void
                } else {
                    // Simplified: return the type of the first for now
                    types.remove(0)
                }
            }

            Expr::Try { expr, .. } => {
                let t = self.check_expr(expr);
                match t {
                    Type::Result(ok, _) => *ok,
                    Type::Option(inner) => *inner,
                    _ => t, // Permissive for Phase 0
                }
            }

            Expr::TryCatch { try_body, catch_body, .. } => {
                let try_ty = self.check_block(try_body);
                let catch_ty = self.check_block(catch_body);
                // Both branches should have compatible types
                self.unify(&try_ty, &catch_ty, &try_body.span);
                try_ty
            }

            Expr::Throw { code, message, .. } => {
                self.check_expr(code);
                self.check_expr(message);
                Type::Void // throw never produces a value
            }

            Expr::Return { value, .. } => {
                if let Some(v) = value {
                    self.check_expr(v)
                } else {
                    Type::Void
                }
            }

            Expr::Break(_) | Expr::Continue(_) => Type::Void,

            Expr::Assign { target, value, span } => {
                let target_ty = self.check_expr(target);
                let value_ty = self.check_expr(value);
                // Check mutability
                if let Expr::Ident(name, _) = target.as_ref() {
                    if let Some(info) = self.scope.lookup(name) {
                        if !info.mutable {
                            self.error(format!("cannot assign to immutable variable '{}'", name), span);
                        }
                    }
                }
                self.unify(&target_ty, &value_ty, span);
                Type::Void
            }

            Expr::CompoundAssign { target, value, span, .. } => {
                let target_ty = self.check_expr(target);
                let value_ty = self.check_expr(value);
                self.unify(&target_ty, &value_ty, span);
                Type::Void
            }

            Expr::Cast { ty, .. } => {
                self.resolve_type_expr(ty)
            }

            Expr::Range { start, end, .. } => {
                // Range is only valid inside a for-loop iterator; check both sides
                self.check_expr(&**start);
                self.check_expr(&**end);
                Type::I64 // ranges are integer ranges
            }
        }
    }

    // ── Binary Op Type Rules ────────────────────────────────────────
    fn check_binary_op(&mut self, op: BinOp, left: &Type, right: &Type, span: &Span) -> Type {
        match op {
            BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Mod => {
                if self.is_numeric(left) && self.is_numeric(right) {
                    self.unify(left, right, span);
                    left.clone()
                } else if op == BinOp::Add && matches!((left, right), (Type::Str, Type::Str)) {
                    Type::Str
                } else {
                    self.error(format!("cannot apply '{}' to '{}' and '{}'", op, left, right), span);
                    Type::Error
                }
            }
            BinOp::Eq | BinOp::NotEq | BinOp::Lt | BinOp::Gt | BinOp::LtEq | BinOp::GtEq => {
                self.unify(left, right, span);
                Type::Bool
            }
            BinOp::And | BinOp::Or => {
                self.unify(&Type::Bool, left, span);
                self.unify(&Type::Bool, right, span);
                Type::Bool
            }
        }
    }

    fn is_numeric(&self, ty: &Type) -> bool {
        matches!(ty, Type::I32 | Type::I64 | Type::F32 | Type::F64 | Type::Var(_) | Type::Unknown)
    }

    // ── v140: Type→TypeDesc conversion for exhaustiveness ───────────
    fn type_to_desc(&self, ty: &Type) -> crate::pattern_exhaustiveness::TypeDesc {
        use crate::pattern_exhaustiveness::TypeDesc;
        match ty {
            Type::Bool => TypeDesc::Bool,
            Type::I32 | Type::I64 => TypeDesc::Int,
            Type::F32 | Type::F64 => TypeDesc::Float,
            Type::Str => TypeDesc::Str,
            Type::Option(_) => TypeDesc::Option,
            Type::Result(_, _) => TypeDesc::Result,
            Type::List(_) => TypeDesc::List,
            Type::Named(name) => {
                if let Some(variants) = self.enum_defs.get(name) {
                    TypeDesc::Enum {
                        name: name.clone(),
                        variants: variants.iter()
                            .map(|(vname, fields)| (vname.clone(), fields.len()))
                            .collect(),
                    }
                } else if let Some(fields) = self.struct_defs.get(name) {
                    TypeDesc::Struct {
                        name: name.clone(),
                        fields: fields.iter().map(|(f, _)| f.clone()).collect(),
                    }
                } else {
                    TypeDesc::Unknown
                }
            }
            _ => TypeDesc::Unknown,
        }
    }

    // ── Unification ─────────────────────────────────────────────────
    fn unify(&mut self, expected: &Type, actual: &Type, span: &Span) {
        if self.types_compatible(expected, actual) {
            return;
        }
        // v132: Try inference engine unification for Var types
        if matches!(expected, Type::Var(_)) || matches!(actual, Type::Var(_)) {
            let a = Self::to_infer_type(expected);
            let b = Self::to_infer_type(actual);
            if crate::type_inference::unify(&a, &b).is_ok() {
                return;
            }
        }
        self.error(
            format!("type mismatch: expected '{}', found '{}'", expected, actual),
            span,
        );
    }

    /// v132: Attempt to resolve a Var/Unknown type via the inference engine.
    /// If the type contains unresolved variables, tries to infer a concrete type.
    fn try_resolve_type(&mut self, ty: &Type, value_expr: Option<&Expr>) -> Type {
        match ty {
            Type::Var(_) | Type::Unknown => {
                if let Some(expr) = value_expr {
                    // Re-check the expression to get a concrete type
                    let resolved = self.check_expr(expr);
                    if !matches!(resolved, Type::Var(_) | Type::Unknown) {
                        return resolved;
                    }
                }
                ty.clone()
            }
            _ => ty.clone(),
        }
    }

    /// v134: Check if a type implements a trait
    pub fn type_implements_trait(&self, type_name: &str, trait_name: &str) -> bool {
        self.trait_dispatcher.implements(type_name, trait_name)
    }

    /// v134: Resolve a method call on a type via trait dispatch
    pub fn resolve_trait_method(&self, type_name: &str, method_name: &str) -> Option<String> {
        self.trait_dispatcher.resolve_method(type_name, method_name)
            .map(|(_trait_name, _slot)| format!("{}_{}", type_name, method_name))
    }

    /// Structural compatibility check with union/intersection/tuple/never support.
    fn types_compatible(&self, a: &Type, b: &Type) -> bool {
        // Error / Unknown / Var always compatible (gradual typing)
        if matches!(a, Type::Error | Type::Unknown | Type::Var(_)) { return true; }
        if matches!(b, Type::Error | Type::Unknown | Type::Var(_)) { return true; }
        // Never is bottom — compatible with everything
        if matches!(a, Type::Never) || matches!(b, Type::Never) { return true; }
        // Union: actual is compatible with union if it matches any variant
        if let Type::Union(variants) = a {
            return variants.iter().any(|v| self.types_compatible(v, b));
        }
        if let Type::Union(variants) = b {
            return variants.iter().any(|v| self.types_compatible(a, v));
        }
        // Intersection: must be compatible with all constituent types
        if let Type::Intersection(parts) = a {
            return parts.iter().all(|p| self.types_compatible(p, b));
        }
        if let Type::Intersection(parts) = b {
            return parts.iter().all(|p| self.types_compatible(a, p));
        }
        // Tuple: element-wise compatibility
        if let (Type::Tuple(ts1), Type::Tuple(ts2)) = (a, b) {
            return ts1.len() == ts2.len()
                && ts1.iter().zip(ts2.iter()).all(|(t1, t2)| self.types_compatible(t1, t2));
        }
        // Structural recursion
        match (a, b) {
            (Type::List(a_inner), Type::List(b_inner)) =>
                self.types_compatible(a_inner, b_inner),
            (Type::Map(ak, av), Type::Map(bk, bv)) =>
                self.types_compatible(ak, bk) && self.types_compatible(av, bv),
            (Type::Option(a_inner), Type::Option(b_inner)) =>
                self.types_compatible(a_inner, b_inner),
            (Type::Result(aok, aerr), Type::Result(bok, berr)) =>
                self.types_compatible(aok, bok) && self.types_compatible(aerr, berr),
            (Type::Future(a_inner), Type::Future(b_inner)) =>
                self.types_compatible(a_inner, b_inner),
            (Type::Array(at, an), Type::Array(bt, bn)) =>
                self.types_compatible(at, bt) && an == bn,
            (Type::Ref { inner: ai, mutable: am }, Type::Ref { inner: bi, mutable: bm }) =>
                am == bm && self.types_compatible(ai, bi),
            (Type::Function { params: ap, ret: ar }, Type::Function { params: bp, ret: br }) =>
                ap.len() == bp.len()
                    && ap.iter().zip(bp.iter()).all(|(p1, p2)| self.types_compatible(p1, p2))
                    && self.types_compatible(ar, br),
            _ => a == b,
        }
    }

    // ── Inference Engine Bridge ──────────────────────────────────────
    /// Convert a core Type to the inference engine's InferType.
    pub fn to_infer_type(ty: &Type) -> crate::type_inference::InferType {
        use crate::type_inference::InferType;
        match ty {
            Type::I32 | Type::I64 => InferType::Int,
            Type::F32 | Type::F64 => InferType::Float,
            Type::Bool => InferType::Bool,
            Type::Str => InferType::Str,
            Type::Void => InferType::Void,
            Type::Never => InferType::Never,
            Type::Named(n) => InferType::Named(n.clone()),
            Type::List(inner) => InferType::List(Box::new(Self::to_infer_type(inner))),
            Type::Option(inner) => InferType::Option(Box::new(Self::to_infer_type(inner))),
            Type::Result(ok, err) => InferType::Result(
                Box::new(Self::to_infer_type(ok)),
                Box::new(Self::to_infer_type(err)),
            ),
            Type::Tuple(ts) => InferType::Tuple(ts.iter().map(Self::to_infer_type).collect()),
            Type::Union(ts) => InferType::Union(ts.iter().map(Self::to_infer_type).collect()),
            Type::Intersection(ts) => InferType::Intersection(ts.iter().map(Self::to_infer_type).collect()),
            Type::Function { params, ret } => InferType::Function(
                params.iter().map(Self::to_infer_type).collect(),
                Box::new(Self::to_infer_type(ret)),
            ),
            Type::Var(id) => InferType::Var(*id),
            _ => InferType::Void,
        }
    }

    /// Convert an InferType back to a core Type.
    pub fn from_infer_type(ty: &crate::type_inference::InferType) -> Type {
        use crate::type_inference::InferType;
        match ty {
            InferType::Int => Type::I64,
            InferType::Float => Type::F64,
            InferType::Bool => Type::Bool,
            InferType::Str => Type::Str,
            InferType::Void => Type::Void,
            InferType::Never => Type::Never,
            InferType::Named(n) => Type::Named(n.clone()),
            InferType::List(inner) => Type::List(Box::new(Self::from_infer_type(inner))),
            InferType::Option(inner) => Type::Option(Box::new(Self::from_infer_type(inner))),
            InferType::Result(ok, err) => Type::Result(
                Box::new(Self::from_infer_type(ok)),
                Box::new(Self::from_infer_type(err)),
            ),
            InferType::Tuple(ts) => Type::Tuple(ts.iter().map(Self::from_infer_type).collect()),
            InferType::Union(ts) => Type::Union(ts.iter().map(Self::from_infer_type).collect()),
            InferType::Intersection(ts) => Type::Intersection(ts.iter().map(Self::from_infer_type).collect()),
            InferType::Function(params, ret) => Type::Function {
                params: params.iter().map(Self::from_infer_type).collect(),
                ret: Box::new(Self::from_infer_type(ret)),
            },
            InferType::Var(id) => Type::Var(*id),
            InferType::Applied(name, _) => Type::Named(name.clone()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser;

    fn check_src(source: &str) -> Vec<TypeError> {
        let (program, parse_errors) = parser::parse(source);
        assert!(parse_errors.is_empty(), "Parse errors: {:?}", parse_errors);
        let checker = TypeChecker::new();
        checker.check(&program)
    }

    #[test]
    fn test_simple_function() {
        let errors = check_src("fn main() -> i64 { 42 }");
        assert!(errors.is_empty(), "Errors: {:?}", errors);
    }

    #[test]
    fn test_type_mismatch() {
        let errors = check_src("fn main() -> i64 { true }");
        assert!(!errors.is_empty(), "Expected a type error");
    }

    #[test]
    fn test_undefined_variable() {
        let errors = check_src("fn main() { let x: i64 = y; }");
        assert!(!errors.is_empty());
        assert!(errors[0].message.contains("undefined"));
    }

    #[test]
    fn test_let_binding() {
        let errors = check_src("fn main() { let x: i64 = 42; }");
        assert!(errors.is_empty(), "Errors: {:?}", errors);
    }

    #[test]
    fn test_binary_ops() {
        let errors = check_src("fn test() -> i64 { 1 + 2 }");
        assert!(errors.is_empty(), "Errors: {:?}", errors);
    }

    #[test]
    fn test_immutable_assign() {
        let errors = check_src("fn test() { let x: i64 = 1; x = 2; }");
        assert!(!errors.is_empty());
        assert!(errors[0].message.contains("immutable"));
    }

    #[test]
    fn test_mutable_assign() {
        let errors = check_src("fn test() { let mut x: i64 = 1; x = 2; }");
        assert!(errors.is_empty(), "Errors: {:?}", errors);
    }

    #[test]
    fn test_struct_field_check() {
        let errors = check_src(r#"
            struct Point { x: f64, y: f64 }
            fn test() -> f64 {
                let p: Point = Point { x: 1.0, y: 2.0 };
                p.x
            }
        "#);
        assert!(errors.is_empty(), "Errors: {:?}", errors);
    }

    #[test]
    fn test_condition_must_be_bool() {
        let errors = check_src("fn test() { if 42 { } }");
        assert!(!errors.is_empty());
    }

    // ── v61: Union, Intersection, Tuple, Never type tests ────────────

    #[test]
    fn test_union_type_display() {
        let t = Type::Union(vec![Type::I64, Type::Str]);
        assert_eq!(format!("{}", t), "i64 | str");
    }

    #[test]
    fn test_intersection_type_display() {
        let t = Type::Intersection(vec![Type::Named("Printable".into()), Type::Named("Serializable".into())]);
        assert_eq!(format!("{}", t), "Printable & Serializable");
    }

    #[test]
    fn test_tuple_type_display() {
        let t = Type::Tuple(vec![Type::I64, Type::Str, Type::Bool]);
        assert_eq!(format!("{}", t), "(i64, str, bool)");
    }

    #[test]
    fn test_never_type_display() {
        assert_eq!(format!("{}", Type::Never), "never");
    }

    #[test]
    fn test_union_compatible_with_member() {
        let checker = TypeChecker::new();
        let union = Type::Union(vec![Type::I64, Type::Str]);
        assert!(checker.types_compatible(&union, &Type::I64));
        assert!(checker.types_compatible(&union, &Type::Str));
        assert!(!checker.types_compatible(&union, &Type::Bool));
    }

    #[test]
    fn test_never_compatible_with_any() {
        let checker = TypeChecker::new();
        assert!(checker.types_compatible(&Type::Never, &Type::I64));
        assert!(checker.types_compatible(&Type::Never, &Type::Str));
        assert!(checker.types_compatible(&Type::I64, &Type::Never));
    }

    #[test]
    fn test_tuple_compatibility() {
        let checker = TypeChecker::new();
        let t1 = Type::Tuple(vec![Type::I64, Type::Str]);
        let t2 = Type::Tuple(vec![Type::I64, Type::Str]);
        let t3 = Type::Tuple(vec![Type::I64, Type::Bool]);
        assert!(checker.types_compatible(&t1, &t2));
        assert!(!checker.types_compatible(&t1, &t3));
    }

    #[test]
    fn test_nested_union_tuple() {
        let checker = TypeChecker::new();
        let union = Type::Union(vec![
            Type::Tuple(vec![Type::I64, Type::Str]),
            Type::I64,
        ]);
        assert!(checker.types_compatible(&union, &Type::I64));
        assert!(checker.types_compatible(&union, &Type::Tuple(vec![Type::I64, Type::Str])));
    }

    #[test]
    fn test_function_type_compatibility() {
        let checker = TypeChecker::new();
        let f1 = Type::Function { params: vec![Type::I64], ret: Box::new(Type::Str) };
        let f2 = Type::Function { params: vec![Type::I64], ret: Box::new(Type::Str) };
        let f3 = Type::Function { params: vec![Type::Str], ret: Box::new(Type::Str) };
        assert!(checker.types_compatible(&f1, &f2));
        assert!(!checker.types_compatible(&f1, &f3));
    }

    #[test]
    fn test_infer_type_bridge_roundtrip() {
        // Test conversion from Type → InferType → Type
        let original = Type::Function {
            params: vec![Type::I64, Type::Str],
            ret: Box::new(Type::Bool),
        };
        let infer_ty = TypeChecker::to_infer_type(&original);
        let back = TypeChecker::from_infer_type(&infer_ty);
        assert_eq!(original, back);
    }

    #[test]
    fn test_infer_type_bridge_union() {
        let union_ty = Type::Union(vec![Type::I64, Type::Str]);
        let infer = TypeChecker::to_infer_type(&union_ty);
        let back = TypeChecker::from_infer_type(&infer);
        assert_eq!(union_ty, back);
    }

    #[test]
    fn test_infer_type_bridge_tuple() {
        let tuple_ty = Type::Tuple(vec![Type::I64, Type::Bool, Type::Str]);
        let infer = TypeChecker::to_infer_type(&tuple_ty);
        let back = TypeChecker::from_infer_type(&infer);
        assert_eq!(tuple_ty, back);
    }

    #[test]
    fn test_infer_type_bridge_never() {
        let never = Type::Never;
        let infer = TypeChecker::to_infer_type(&never);
        let back = TypeChecker::from_infer_type(&infer);
        assert_eq!(never, back);
    }

    #[test]
    fn test_resolve_never_type() {
        let errors = check_src(r#"
            fn diverge() -> never {
                panic("unreachable")
            }
        "#);
        assert!(errors.is_empty(), "Errors: {:?}", errors);
    }

    #[test]
    fn test_structural_list_compatibility() {
        let checker = TypeChecker::new();
        let l1 = Type::List(Box::new(Type::I64));
        let l2 = Type::List(Box::new(Type::I64));
        let l3 = Type::List(Box::new(Type::Str));
        assert!(checker.types_compatible(&l1, &l2));
        assert!(!checker.types_compatible(&l1, &l3));
    }

    #[test]
    fn test_structural_result_compatibility() {
        let checker = TypeChecker::new();
        let r1 = Type::Result(Box::new(Type::I64), Box::new(Type::Str));
        let r2 = Type::Result(Box::new(Type::I64), Box::new(Type::Str));
        let r3 = Type::Result(Box::new(Type::Bool), Box::new(Type::Str));
        assert!(checker.types_compatible(&r1, &r2));
        assert!(!checker.types_compatible(&r1, &r3));
    }

    #[test]
    fn test_union_with_never() {
        let checker = TypeChecker::new();
        // Union with Never should still work
        let union = Type::Union(vec![Type::I64, Type::Never]);
        assert!(checker.types_compatible(&union, &Type::I64));
        assert!(checker.types_compatible(&union, &Type::Str)); // Never member matches anything
    }

    // ── v119: Error message overhaul tests ──────────────────────────

    #[test]
    fn test_type_error_has_hint_field() {
        let err = TypeError {
            message: "test error".into(),
            span: Span::new(0, 5),
            hint: Some("try this".into()),
        };
        assert_eq!(err.hint.as_deref(), Some("try this"));
        let display = format!("{}", err);
        assert!(display.contains("hint: try this"));
    }

    #[test]
    fn test_type_error_no_hint() {
        let err = TypeError {
            message: "test error".into(),
            span: Span::new(0, 5),
            hint: None,
        };
        assert!(!format!("{}", err).contains("hint"));
    }

    #[test]
    fn test_undefined_variable_suggests_similar() {
        // Define a variable 'count', then reference 'coun' (typo)
        let src = "fn test() { let count: i64 = 0; let x: i64 = coun; }";
        let (program, _) = crate::parser::parse(src);
        let errors = TypeChecker::new().check(&program);
        let undef = errors.iter().find(|e| e.message.contains("undefined"));
        assert!(undef.is_some(), "should have undefined variable error");
        assert!(undef.unwrap().hint.is_some(), "should have hint for 'coun' → 'count'");
        assert!(undef.unwrap().hint.as_ref().unwrap().contains("count"),
            "hint should suggest 'count': {:?}", undef.unwrap().hint);
    }

    #[test]
    fn test_undefined_variable_no_suggestion_for_unrelated() {
        // Define 'x', reference 'zzzzzzz' — too different for suggestion
        let src = "fn test() { let x: i64 = 0; let y: i64 = zzzzzzz; }";
        let (program, _) = crate::parser::parse(src);
        let errors = TypeChecker::new().check(&program);
        let undef = errors.iter().find(|e| e.message.contains("undefined"));
        assert!(undef.is_some(), "should have undefined variable error");
        // No hint because 'zzzzzzz' is too far from 'x'
        assert!(undef.unwrap().hint.is_none(),
            "should NOT have hint for completely unrelated name");
    }

    #[test]
    fn test_scope_all_names() {
        let mut scope = Scope::new();
        scope.define("alpha".into(), SymbolInfo { ty: Type::I64, mutable: false, evolvable: false, trust_tier: None });
        scope.define("beta".into(), SymbolInfo { ty: Type::I64, mutable: false, evolvable: false, trust_tier: None });
        let child = Scope::child(scope);
        let names = child.all_names();
        assert!(names.contains(&"alpha".to_string()));
        assert!(names.contains(&"beta".to_string()));
    }

    #[test]
    fn test_error_with_hint_method() {
        let mut checker = TypeChecker::new();
        checker.error_with_hint("test error", &Span::new(0, 5), "try this instead");
        let errors = checker.check(&crate::ast::Program { items: vec![], span: Span::default() });
        // The error we pushed should still be in the list
        // (check() adds to existing errors but returns all)
        // Actually check() consumes self, so we can't call error_with_hint then check.
        // Instead, just verify directly.
        drop(errors);

        let mut checker2 = TypeChecker::new();
        checker2.error_with_hint("msg", &Span::new(0, 1), "helpful hint");
        // Access errors directly is not possible (private), but we can test by
        // running check on empty program — errors accumulate.
        let program = crate::ast::Program { items: vec![], span: Span::default() };
        let errors = checker2.check(&program);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].hint.as_deref(), Some("helpful hint"));
    }

    // ═══ v132: Inference Engine Integration Tests ═══

    #[test]
    fn test_v132_infer_engine_wired() {
        // Verify TypeChecker has an InferEngine field (starts at var 100)
        let checker = TypeChecker::new();
        assert_eq!(checker.infer_engine.var_count(), 100);
    }

    #[test]
    fn test_v132_let_without_annotation() {
        // let x = 42 should infer x as i64 without error
        let errors = check_src("fn main() -> i64 { let x = 42; x }");
        assert!(errors.is_empty(), "let without annotation should work: {:?}", errors);
    }

    #[test]
    fn test_v132_let_with_annotation() {
        // let x: i64 = 42 should still work
        let errors = check_src("fn main() -> i64 { let x: i64 = 42; x }");
        assert!(errors.is_empty(), "let with annotation should work: {:?}", errors);
    }

    #[test]
    fn test_v132_unify_var_with_concrete() {
        // Var types should unify with concrete types via inference engine
        use crate::type_inference::{unify, InferType};
        let result = unify(&InferType::Var(0), &InferType::Int);
        assert!(result.is_ok(), "Var should unify with Int");
    }

    #[test]
    fn test_v132_unify_concrete_types() {
        use crate::type_inference::{unify, InferType};
        let result = unify(&InferType::Int, &InferType::Int);
        assert!(result.is_ok(), "Int should unify with Int");
        let result = unify(&InferType::Int, &InferType::Bool);
        assert!(result.is_err(), "Int should not unify with Bool");
    }

    #[test]
    fn test_v132_try_resolve_type() {
        let errors = check_src("fn foo() -> i64 { let x = 10; let y = x; y }");
        assert!(errors.is_empty(), "chained let inference should work: {:?}", errors);
    }

    #[test]
    fn test_v132_infer_bridge_consistency() {
        // Verify that to_infer_type and from_infer_type are consistent for all base types
        for ty in [Type::I32, Type::I64, Type::F32, Type::F64, Type::Bool, Type::Str, Type::Void, Type::Never] {
            let infer = TypeChecker::to_infer_type(&ty);
            let back = TypeChecker::from_infer_type(&infer);
            // I32 maps to Int which maps back to I64 — expected
            // F32 maps to Float which maps back to F64 — expected
            match ty {
                Type::I32 => assert_eq!(back, Type::I64),
                Type::F32 => assert_eq!(back, Type::F64),
                _ => assert_eq!(back, ty),
            }
        }
    }

    #[test]
    fn test_v132_complex_inference() {
        // Nested expressions should infer correctly
        let errors = check_src("fn main() -> i64 { let a = 1; let b = 2; let c = a + b; c }");
        assert!(errors.is_empty(), "arithmetic inference should work: {:?}", errors);
    }

    #[test]
    fn test_v132_function_return_inference() {
        // Function with explicit return type should check body against it
        let errors = check_src("fn add(a: i64, b: i64) -> i64 { a + b }");
        assert!(errors.is_empty(), "return type checking should work: {:?}", errors);
    }

    #[test]
    fn test_v132_type_mismatch_detected() {
        // Type mismatch should still be detected
        let errors = check_src("fn main() -> i64 { let x: i64 = true; x }");
        assert!(!errors.is_empty(), "should detect i64 vs bool mismatch");
    }

    // ─── v134 Tests: Trait Dispatch Integration ─────────────────────

    #[test]
    fn test_v134_trait_dispatcher_wired() {
        let checker = TypeChecker::new();
        // Dispatcher should be initialized and empty
        assert!(checker.trait_dispatcher.all_traits().is_empty());
    }

    #[test]
    fn test_v134_trait_registered_from_ast() {
        // Parse a trait definition, verify it gets registered
        let mut checker = TypeChecker::new();
        let (program, errors) = crate::parser::parse(
            "trait Display { fn to_string(self) -> str; }\nfn main() -> i64 { 0 }"
        );
        assert!(errors.is_empty());
        for item in &program.items {
            checker.collect_signatures(item);
        }
        assert!(!checker.trait_dispatcher.all_traits().is_empty());
    }

    #[test]
    fn test_v134_impl_validated_against_trait() {
        // Parse trait + valid impl — no errors
        let errors = check_src(
            "trait Greet { fn greet(self) -> i64; }\nstruct Dog { name: str }\nimpl Greet for Dog { fn greet(self) -> i64 { 42 } }\nfn main() -> i64 { 0 }"
        );
        let trait_errors: Vec<_> = errors.iter()
            .filter(|e| e.message.contains("trait") || e.message.contains("method"))
            .collect();
        assert!(trait_errors.is_empty(), "Valid impl should not produce trait errors: {:?}", trait_errors);
    }

    #[test]
    fn test_v134_impl_missing_method_detected() {
        // Parse trait + incomplete impl — should get error
        let errors = check_src(
            "trait Greet { fn greet(self) -> i64; }\nstruct Cat { name: str }\nimpl Greet for Cat { }\nfn main() -> i64 { 0 }"
        );
        let missing: Vec<_> = errors.iter()
            .filter(|e| e.message.contains("greet") || e.message.contains("implement"))
            .collect();
        assert!(!missing.is_empty(), "Missing required method should be detected: {:?}", errors);
    }

    #[test]
    fn test_v134_type_implements_trait() {
        let mut checker = TypeChecker::new();
        let (program, errors) = crate::parser::parse(
            "trait Show { fn show(self) -> i64; }\nstruct Point { x: i64 }\nimpl Show for Point { fn show(self) -> i64 { 1 } }\nfn main() -> i64 { 0 }"
        );
        assert!(errors.is_empty());
        for item in &program.items {
            checker.collect_signatures(item);
        }
        assert!(checker.type_implements_trait("Point", "Show"));
        assert!(!checker.type_implements_trait("Point", "Debug"));
    }

    #[test]
    fn test_v134_resolve_trait_method() {
        let mut checker = TypeChecker::new();
        let (program, errors) = crate::parser::parse(
            "trait Render { fn draw(self) -> i64; }\nstruct Circle { r: i64 }\nimpl Render for Circle { fn draw(self) -> i64 { 0 } }\nfn main() -> i64 { 0 }"
        );
        assert!(errors.is_empty());
        for item in &program.items {
            checker.collect_signatures(item);
        }
        let resolved = checker.resolve_trait_method("Circle", "draw");
        assert!(resolved.is_some());
        assert_eq!(resolved.unwrap(), "Circle_draw");
    }

    #[test]
    fn test_v134_unknown_trait_impl_error() {
        // Impl for a trait that doesn't exist should produce error
        let errors = check_src(
            "struct Foo { x: i64 }\nimpl NonExistent for Foo { fn bar(self) -> i64 { 0 } }\nfn main() -> i64 { 0 }"
        );
        let trait_errors: Vec<_> = errors.iter()
            .filter(|e| e.message.contains("Unknown trait") || e.message.contains("NonExistent"))
            .collect();
        assert!(!trait_errors.is_empty(), "Unknown trait impl should produce error: {:?}", errors);
    }

    #[test]
    fn test_v134_multiple_impls() {
        let errors = check_src(
            "trait A { fn fa(self) -> i64; }\ntrait B { fn fb(self) -> i64; }\nstruct Widget { v: i64 }\nimpl A for Widget { fn fa(self) -> i64 { 1 } }\nimpl B for Widget { fn fb(self) -> i64 { 2 } }\nfn main() -> i64 { 0 }"
        );
        let trait_errors: Vec<_> = errors.iter()
            .filter(|e| e.message.contains("trait") || e.message.contains("implement"))
            .collect();
        assert!(trait_errors.is_empty(), "Multiple valid impls should not error: {:?}", trait_errors);
    }

    #[test]
    fn test_v134_impl_method_body_checked() {
        // Method body should be type-checked
        let errors = check_src(
            "trait Math { fn compute(self) -> i64; }\nstruct Calc { v: i64 }\nimpl Math for Calc { fn compute(self) -> i64 { true } }\nfn main() -> i64 { 0 }"
        );
        // The method returns bool but declares i64 — type mismatch
        let type_errors: Vec<_> = errors.iter()
            .filter(|e| e.message.contains("mismatch") || e.message.contains("expected"))
            .collect();
        assert!(!type_errors.is_empty(), "Method body type mismatch should be detected: {:?}", errors);
    }

    #[test]
    fn test_v134_default_method_not_required() {
        // Trait with default method — impl can omit it
        let errors = check_src(
            "trait WithOpt { fn required(self) -> i64; fn optional(self) -> i64 { 0 } }\nstruct Bar { x: i64 }\nimpl WithOpt for Bar { fn required(self) -> i64 { 1 } }\nfn main() -> i64 { 0 }"
        );
        let trait_errors: Vec<_> = errors.iter()
            .filter(|e| e.message.contains("implement") || e.message.contains("method"))
            .collect();
        assert!(trait_errors.is_empty(), "Default method should not be required: {:?}", trait_errors);
    }

    #[test]
    fn test_v134_inherent_impl_no_trait_check() {
        // Inherent impl (no trait) should work without trait validation
        let errors = check_src(
            "struct Pair { x: i64, y: i64 }\nimpl Pair { fn sum(self) -> i64 { 0 } }\nfn main() -> i64 { 0 }"
        );
        let trait_errors: Vec<_> = errors.iter()
            .filter(|e| e.message.contains("trait"))
            .collect();
        assert!(trait_errors.is_empty(), "Inherent impl should not produce trait errors: {:?}", trait_errors);
    }

    // ─── v140 Tests: Pattern Exhaustiveness ──────────────────────────

    #[test]
    fn test_v140_type_to_desc_bool() {
        let tc = TypeChecker::new();
        let desc = tc.type_to_desc(&Type::Bool);
        assert_eq!(desc, crate::pattern_exhaustiveness::TypeDesc::Bool);
    }

    #[test]
    fn test_v140_type_to_desc_int() {
        let tc = TypeChecker::new();
        let desc = tc.type_to_desc(&Type::I64);
        assert_eq!(desc, crate::pattern_exhaustiveness::TypeDesc::Int);
    }

    #[test]
    fn test_v140_type_to_desc_str() {
        let tc = TypeChecker::new();
        let desc = tc.type_to_desc(&Type::Str);
        assert_eq!(desc, crate::pattern_exhaustiveness::TypeDesc::Str);
    }

    #[test]
    fn test_v140_type_to_desc_unknown() {
        let tc = TypeChecker::new();
        let desc = tc.type_to_desc(&Type::Void);
        assert_eq!(desc, crate::pattern_exhaustiveness::TypeDesc::Unknown);
    }

    #[test]
    fn test_v140_exhaustive_bool_match() {
        // Match on bool with true and false should be exhaustive — no errors
        let errors = check_src("fn main() -> i64 { let x: bool = true; match x { true => 1, false => 0 } }");
        let exh_errors: Vec<_> = errors.iter().filter(|e| e.message.contains("exhaustive")).collect();
        assert!(exh_errors.is_empty(), "Bool match with true+false should be exhaustive: {:?}", exh_errors);
    }

    #[test]
    fn test_v140_non_exhaustive_bool_match() {
        // Match on bool with only true should be non-exhaustive
        let errors = check_src("fn main() -> i64 { let x: bool = true; match x { true => 1 } }");
        let exh_errors: Vec<_> = errors.iter().filter(|e| e.message.contains("exhaustive")).collect();
        assert!(!exh_errors.is_empty(), "Bool match with only true should be non-exhaustive");
    }

    #[test]
    fn test_v140_wildcard_match_exhaustive() {
        // Match with wildcard catch-all should always be exhaustive
        let errors = check_src("fn main() -> i64 { let x: i64 = 5; match x { 1 => 10, _ => 20 } }");
        let exh_errors: Vec<_> = errors.iter().filter(|e| e.message.contains("exhaustive")).collect();
        assert!(exh_errors.is_empty(), "Match with wildcard should be exhaustive: {:?}", exh_errors);
    }

    #[test]
    fn test_v140_int_match_without_wildcard() {
        // Match on integer without wildcard should be non-exhaustive
        let errors = check_src("fn main() -> i64 { let x: i64 = 5; match x { 1 => 10, 2 => 20 } }");
        let exh_errors: Vec<_> = errors.iter().filter(|e| e.message.contains("exhaustive")).collect();
        assert!(!exh_errors.is_empty(), "Int match without wildcard should be non-exhaustive");
    }

    #[test]
    fn test_v140_exhaustiveness_produces_missing_info() {
        // Non-exhaustive match error should mention missing patterns
        let errors = check_src("fn main() -> i64 { let x: bool = true; match x { true => 1 } }");
        let exh_errors: Vec<_> = errors.iter().filter(|e| e.message.contains("exhaustive")).collect();
        assert!(exh_errors[0].message.contains("missing"), "Error should mention missing patterns: {}", exh_errors[0].message);
    }
}
