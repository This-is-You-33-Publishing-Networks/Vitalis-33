//! Multi-stage meta-programming for Vitalis.
//!
//! - **Quasi-quotation**: `quote { let x = $(expr) }` — construct AST fragments with splicing
//! - **Splice**: `$(...)` — insert computed AST nodes into quoted templates
//! - **Cross-stage persistence**: Values computed at stage N available at stage N+1
//! - **Staging annotations**: `@stage(0)` / `@stage(1)` — explicit multi-stage structure
//! - **Compiler-compiler**: Vitalis generates its own parser from a grammar specification
//! - **Self-modifying compilation**: Compiler plugins written in Vitalis, loaded at compile time

use std::collections::HashMap;

// ── AST Fragment Representation ─────────────────────────────────────

/// A fragment of quoted AST with splice holes.
#[derive(Debug, Clone, PartialEq)]
pub enum AstFragment {
    /// A literal integer.
    IntLit(i64),
    /// A literal float.
    FloatLit(f64),
    /// A literal string.
    StringLit(String),
    /// A literal boolean.
    BoolLit(bool),
    /// An identifier reference.
    Ident(String),
    /// A splice hole: `$(name)` — filled at expansion time.
    Splice(String),
    /// A binary operation.
    BinOp { op: String, left: Box<AstFragment>, right: Box<AstFragment> },
    /// A function call.
    Call { func: Box<AstFragment>, args: Vec<AstFragment> },
    /// A let binding.
    Let { name: String, value: Box<AstFragment> },
    /// A block of fragments.
    Block(Vec<AstFragment>),
    /// A function definition.
    FnDef { name: String, params: Vec<(String, String)>, body: Box<AstFragment> },
    /// An if expression.
    If { cond: Box<AstFragment>, then_branch: Box<AstFragment>, else_branch: Option<Box<AstFragment>> },
    /// A return expression.
    Return(Box<AstFragment>),
}

impl AstFragment {
    /// Count splice holes in this fragment.
    pub fn splice_count(&self) -> usize {
        match self {
            AstFragment::Splice(_) => 1,
            AstFragment::BinOp { left, right, .. } => left.splice_count() + right.splice_count(),
            AstFragment::Call { func, args } => {
                func.splice_count() + args.iter().map(|a| a.splice_count()).sum::<usize>()
            }
            AstFragment::Let { value, .. } => value.splice_count(),
            AstFragment::Block(stmts) => stmts.iter().map(|s| s.splice_count()).sum(),
            AstFragment::FnDef { body, .. } => body.splice_count(),
            AstFragment::If { cond, then_branch, else_branch } => {
                cond.splice_count() + then_branch.splice_count()
                    + else_branch.as_ref().map(|e| e.splice_count()).unwrap_or(0)
            }
            AstFragment::Return(e) => e.splice_count(),
            _ => 0,
        }
    }

    /// Collect all splice hole names.
    pub fn splice_names(&self) -> Vec<String> {
        let mut names = Vec::new();
        self.collect_splices(&mut names);
        names
    }

    fn collect_splices(&self, names: &mut Vec<String>) {
        match self {
            AstFragment::Splice(name) => names.push(name.clone()),
            AstFragment::BinOp { left, right, .. } => {
                left.collect_splices(names);
                right.collect_splices(names);
            }
            AstFragment::Call { func, args } => {
                func.collect_splices(names);
                for a in args { a.collect_splices(names); }
            }
            AstFragment::Let { value, .. } => value.collect_splices(names),
            AstFragment::Block(stmts) => {
                for s in stmts { s.collect_splices(names); }
            }
            AstFragment::FnDef { body, .. } => body.collect_splices(names),
            AstFragment::If { cond, then_branch, else_branch } => {
                cond.collect_splices(names);
                then_branch.collect_splices(names);
                if let Some(e) = else_branch { e.collect_splices(names); }
            }
            AstFragment::Return(e) => e.collect_splices(names),
            _ => {}
        }
    }

    /// Substitute all splice holes with the given bindings.
    pub fn expand(&self, bindings: &HashMap<String, AstFragment>) -> AstFragment {
        match self {
            AstFragment::Splice(name) => {
                bindings.get(name).cloned().unwrap_or(AstFragment::Splice(name.clone()))
            }
            AstFragment::BinOp { op, left, right } => AstFragment::BinOp {
                op: op.clone(),
                left: Box::new(left.expand(bindings)),
                right: Box::new(right.expand(bindings)),
            },
            AstFragment::Call { func, args } => AstFragment::Call {
                func: Box::new(func.expand(bindings)),
                args: args.iter().map(|a| a.expand(bindings)).collect(),
            },
            AstFragment::Let { name, value } => AstFragment::Let {
                name: name.clone(),
                value: Box::new(value.expand(bindings)),
            },
            AstFragment::Block(stmts) => AstFragment::Block(
                stmts.iter().map(|s| s.expand(bindings)).collect(),
            ),
            AstFragment::FnDef { name, params, body } => AstFragment::FnDef {
                name: name.clone(),
                params: params.clone(),
                body: Box::new(body.expand(bindings)),
            },
            AstFragment::If { cond, then_branch, else_branch } => AstFragment::If {
                cond: Box::new(cond.expand(bindings)),
                then_branch: Box::new(then_branch.expand(bindings)),
                else_branch: else_branch.as_ref().map(|e| Box::new(e.expand(bindings))),
            },
            AstFragment::Return(e) => AstFragment::Return(Box::new(e.expand(bindings))),
            other => other.clone(),
        }
    }

    /// Pretty-print the fragment as Vitalis source code.
    pub fn to_source(&self) -> String {
        match self {
            AstFragment::IntLit(v) => v.to_string(),
            AstFragment::FloatLit(v) => format!("{:.6}", v),
            AstFragment::StringLit(s) => format!("\"{}\"", s),
            AstFragment::BoolLit(b) => if *b { "true" } else { "false" }.to_string(),
            AstFragment::Ident(name) => name.clone(),
            AstFragment::Splice(name) => format!("$({name})"),
            AstFragment::BinOp { op, left, right } => {
                format!("({} {} {})", left.to_source(), op, right.to_source())
            }
            AstFragment::Call { func, args } => {
                let arg_str: Vec<String> = args.iter().map(|a| a.to_source()).collect();
                format!("{}({})", func.to_source(), arg_str.join(", "))
            }
            AstFragment::Let { name, value } => {
                format!("let {} = {};", name, value.to_source())
            }
            AstFragment::Block(stmts) => {
                let body: Vec<String> = stmts.iter().map(|s| s.to_source()).collect();
                format!("{{\n{}\n}}", body.join("\n"))
            }
            AstFragment::FnDef { name, params, body } => {
                let param_str: Vec<String> = params.iter()
                    .map(|(n, t)| format!("{}: {}", n, t))
                    .collect();
                format!("fn {}({}) {}", name, param_str.join(", "), body.to_source())
            }
            AstFragment::If { cond, then_branch, else_branch } => {
                let base = format!("if {} {}", cond.to_source(), then_branch.to_source());
                match else_branch {
                    Some(e) => format!("{} else {}", base, e.to_source()),
                    None => base,
                }
            }
            AstFragment::Return(e) => format!("return {};", e.to_source()),
        }
    }
}

// ── Staging System ──────────────────────────────────────────────────

/// A staging annotation on a declaration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StagingLevel {
    /// Available at compile time (stage 0).
    CompileTime,
    /// Available at runtime (stage 1).
    Runtime,
    /// Available at both stages.
    Both,
}

/// A staged value that persists across compilation stages.
#[derive(Debug, Clone)]
pub struct StagedValue {
    pub name: String,
    pub stage: StagingLevel,
    pub value: AstFragment,
    pub type_name: String,
}

/// Cross-stage persistence store.
pub struct StageStore {
    values: HashMap<String, StagedValue>,
}

impl StageStore {
    pub fn new() -> Self { Self { values: HashMap::new() } }

    pub fn persist(&mut self, name: &str, stage: StagingLevel, value: AstFragment, type_name: &str) {
        self.values.insert(name.to_string(), StagedValue {
            name: name.to_string(),
            stage,
            value,
            type_name: type_name.to_string(),
        });
    }

    pub fn get(&self, name: &str) -> Option<&StagedValue> {
        self.values.get(name)
    }

    /// Get all values available at a given stage.
    pub fn available_at(&self, stage: StagingLevel) -> Vec<&StagedValue> {
        self.values.values().filter(|v| {
            v.stage == stage || v.stage == StagingLevel::Both
        }).collect()
    }

    pub fn count(&self) -> usize { self.values.len() }
}

// ── Grammar Specification ───────────────────────────────────────────

/// A grammar rule for the compiler-compiler.
#[derive(Debug, Clone)]
pub struct GrammarRule {
    pub name: String,
    pub alternatives: Vec<Production>,
}

/// A production alternative.
#[derive(Debug, Clone)]
pub struct Production {
    pub symbols: Vec<Symbol>,
    pub action: Option<String>, // Vitalis code to execute on match
}

/// A symbol in a production.
#[derive(Debug, Clone, PartialEq)]
pub enum Symbol {
    /// A terminal (token type).
    Terminal(String),
    /// A non-terminal (rule name).
    NonTerminal(String),
    /// An optional symbol.
    Optional(Box<Symbol>),
    /// Zero or more repetitions.
    Repeat(Box<Symbol>),
    /// One or more repetitions.
    RepeatOne(Box<Symbol>),
}

impl Symbol {
    pub fn is_terminal(&self) -> bool {
        matches!(self, Symbol::Terminal(_))
    }

    pub fn is_nonterminal(&self) -> bool {
        matches!(self, Symbol::NonTerminal(_))
    }

    pub fn name(&self) -> &str {
        match self {
            Symbol::Terminal(n) | Symbol::NonTerminal(n) => n,
            Symbol::Optional(inner) | Symbol::Repeat(inner) | Symbol::RepeatOne(inner) => inner.name(),
        }
    }
}

/// A grammar specification.
pub struct Grammar {
    rules: Vec<GrammarRule>,
    start_symbol: String,
}

impl Grammar {
    pub fn new(start: &str) -> Self {
        Self { rules: Vec::new(), start_symbol: start.to_string() }
    }

    pub fn add_rule(&mut self, rule: GrammarRule) {
        self.rules.push(rule);
    }

    pub fn start_symbol(&self) -> &str { &self.start_symbol }
    pub fn rule_count(&self) -> usize { self.rules.len() }

    pub fn get_rule(&self, name: &str) -> Option<&GrammarRule> {
        self.rules.iter().find(|r| r.name == name)
    }

    /// Collect all terminal symbols.
    pub fn terminals(&self) -> Vec<String> {
        let mut terms = Vec::new();
        for rule in &self.rules {
            for prod in &rule.alternatives {
                for sym in &prod.symbols {
                    Self::collect_terminals(sym, &mut terms);
                }
            }
        }
        terms.sort();
        terms.dedup();
        terms
    }

    fn collect_terminals(sym: &Symbol, out: &mut Vec<String>) {
        match sym {
            Symbol::Terminal(t) => out.push(t.clone()),
            Symbol::Optional(inner) | Symbol::Repeat(inner) | Symbol::RepeatOne(inner) => {
                Self::collect_terminals(inner, out);
            }
            Symbol::NonTerminal(_) => {}
        }
    }

    /// Collect all non-terminal symbols.
    pub fn non_terminals(&self) -> Vec<String> {
        self.rules.iter().map(|r| r.name.clone()).collect()
    }
}

// ── Compiler Plugin System ──────────────────────────────────────────

/// A compiler plugin that runs during compilation.
#[derive(Debug, Clone)]
pub struct CompilerPlugin {
    pub name: String,
    pub version: String,
    pub stage: StagingLevel,
    pub hooks: Vec<PluginHook>,
}

/// A point in the pipeline where a plugin can hook.
#[derive(Debug, Clone, PartialEq)]
pub enum PluginHook {
    AfterParse,
    AfterTypeCheck,
    AfterIrBuild,
    AfterOptimize,
    BeforeCodegen,
    Custom(String),
}

impl CompilerPlugin {
    pub fn new(name: &str, version: &str) -> Self {
        Self {
            name: name.to_string(),
            version: version.to_string(),
            stage: StagingLevel::CompileTime,
            hooks: Vec::new(),
        }
    }

    pub fn add_hook(&mut self, hook: PluginHook) {
        self.hooks.push(hook);
    }

    pub fn has_hook(&self, hook: &PluginHook) -> bool {
        self.hooks.contains(hook)
    }
}

/// Plugin registry.
pub struct PluginRegistry {
    plugins: Vec<CompilerPlugin>,
}

impl PluginRegistry {
    pub fn new() -> Self { Self { plugins: Vec::new() } }

    pub fn register(&mut self, plugin: CompilerPlugin) {
        self.plugins.push(plugin);
    }

    pub fn plugins_for_hook(&self, hook: &PluginHook) -> Vec<&CompilerPlugin> {
        self.plugins.iter().filter(|p| p.has_hook(hook)).collect()
    }

    pub fn plugin_count(&self) -> usize { self.plugins.len() }

    pub fn get_plugin(&self, name: &str) -> Option<&CompilerPlugin> {
        self.plugins.iter().find(|p| p.name == name)
    }
}

// ═══════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ast_fragment_int() {
        let frag = AstFragment::IntLit(42);
        assert_eq!(frag.to_source(), "42");
        assert_eq!(frag.splice_count(), 0);
    }

    #[test]
    fn test_ast_fragment_string() {
        let frag = AstFragment::StringLit("hello".into());
        assert_eq!(frag.to_source(), "\"hello\"");
    }

    #[test]
    fn test_ast_fragment_splice() {
        let frag = AstFragment::Splice("expr".into());
        assert_eq!(frag.to_source(), "$(expr)");
        assert_eq!(frag.splice_count(), 1);
    }

    #[test]
    fn test_splice_names() {
        let frag = AstFragment::BinOp {
            op: "+".into(),
            left: Box::new(AstFragment::Splice("a".into())),
            right: Box::new(AstFragment::Splice("b".into())),
        };
        let names = frag.splice_names();
        assert_eq!(names, vec!["a".to_string(), "b".to_string()]);
    }

    #[test]
    fn test_expand_splices() {
        let template = AstFragment::Let {
            name: "x".into(),
            value: Box::new(AstFragment::Splice("val".into())),
        };
        let mut bindings = HashMap::new();
        bindings.insert("val".to_string(), AstFragment::IntLit(42));
        let expanded = template.expand(&bindings);
        assert_eq!(expanded.to_source(), "let x = 42;");
    }

    #[test]
    fn test_nested_expansion() {
        let template = AstFragment::Block(vec![
            AstFragment::Let {
                name: "x".into(),
                value: Box::new(AstFragment::Splice("init".into())),
            },
            AstFragment::Return(Box::new(AstFragment::Splice("result".into()))),
        ]);
        let mut bindings = HashMap::new();
        bindings.insert("init".into(), AstFragment::IntLit(0));
        bindings.insert("result".into(), AstFragment::Ident("x".into()));
        let expanded = template.expand(&bindings);
        assert_eq!(expanded.splice_count(), 0);
    }

    #[test]
    fn test_fn_def_source() {
        let frag = AstFragment::FnDef {
            name: "add".into(),
            params: vec![("a".into(), "i32".into()), ("b".into(), "i32".into())],
            body: Box::new(AstFragment::BinOp {
                op: "+".into(),
                left: Box::new(AstFragment::Ident("a".into())),
                right: Box::new(AstFragment::Ident("b".into())),
            }),
        };
        let src = frag.to_source();
        assert!(src.contains("fn add(a: i32, b: i32)"));
    }

    #[test]
    fn test_if_fragment() {
        let frag = AstFragment::If {
            cond: Box::new(AstFragment::BoolLit(true)),
            then_branch: Box::new(AstFragment::IntLit(1)),
            else_branch: Some(Box::new(AstFragment::IntLit(0))),
        };
        let src = frag.to_source();
        assert!(src.contains("if true"));
        assert!(src.contains("else"));
    }

    #[test]
    fn test_call_source() {
        let frag = AstFragment::Call {
            func: Box::new(AstFragment::Ident("print".into())),
            args: vec![AstFragment::StringLit("hello".into())],
        };
        assert_eq!(frag.to_source(), "print(\"hello\")");
    }

    #[test]
    fn test_stage_store() {
        let mut store = StageStore::new();
        store.persist("TABLE_SIZE", StagingLevel::CompileTime, AstFragment::IntLit(256), "i32");
        assert_eq!(store.count(), 1);
        assert!(store.get("TABLE_SIZE").is_some());
    }

    #[test]
    fn test_available_at_stage() {
        let mut store = StageStore::new();
        store.persist("ct", StagingLevel::CompileTime, AstFragment::IntLit(1), "i32");
        store.persist("rt", StagingLevel::Runtime, AstFragment::IntLit(2), "i32");
        store.persist("both", StagingLevel::Both, AstFragment::IntLit(3), "i32");
        assert_eq!(store.available_at(StagingLevel::CompileTime).len(), 2); // ct + both
        assert_eq!(store.available_at(StagingLevel::Runtime).len(), 2); // rt + both
    }

    #[test]
    fn test_grammar_rule() {
        let mut grammar = Grammar::new("program");
        grammar.add_rule(GrammarRule {
            name: "program".into(),
            alternatives: vec![Production {
                symbols: vec![Symbol::Repeat(Box::new(Symbol::NonTerminal("stmt".into())))],
                action: None,
            }],
        });
        grammar.add_rule(GrammarRule {
            name: "stmt".into(),
            alternatives: vec![Production {
                symbols: vec![Symbol::Terminal("LET".into()), Symbol::Terminal("IDENT".into())],
                action: Some("make_let()".into()),
            }],
        });
        assert_eq!(grammar.rule_count(), 2);
        assert_eq!(grammar.start_symbol(), "program");
    }

    #[test]
    fn test_grammar_terminals() {
        let mut grammar = Grammar::new("expr");
        grammar.add_rule(GrammarRule {
            name: "expr".into(),
            alternatives: vec![Production {
                symbols: vec![
                    Symbol::Terminal("INT".into()),
                    Symbol::Terminal("PLUS".into()),
                    Symbol::Terminal("INT".into()),
                ],
                action: None,
            }],
        });
        let terms = grammar.terminals();
        assert!(terms.contains(&"INT".to_string()));
        assert!(terms.contains(&"PLUS".to_string()));
    }

    #[test]
    fn test_symbol_types() {
        assert!(Symbol::Terminal("IF".into()).is_terminal());
        assert!(Symbol::NonTerminal("expr".into()).is_nonterminal());
        assert_eq!(Symbol::Terminal("IF".into()).name(), "IF");
    }

    #[test]
    fn test_optional_symbol() {
        let opt = Symbol::Optional(Box::new(Symbol::Terminal("SEMI".into())));
        assert!(!opt.is_terminal());
        assert_eq!(opt.name(), "SEMI");
    }

    #[test]
    fn test_compiler_plugin() {
        let mut plugin = CompilerPlugin::new("my-lint", "0.1.0");
        plugin.add_hook(PluginHook::AfterParse);
        plugin.add_hook(PluginHook::AfterTypeCheck);
        assert!(plugin.has_hook(&PluginHook::AfterParse));
        assert!(!plugin.has_hook(&PluginHook::BeforeCodegen));
    }

    #[test]
    fn test_plugin_registry() {
        let mut registry = PluginRegistry::new();
        let mut p1 = CompilerPlugin::new("lint", "1.0");
        p1.add_hook(PluginHook::AfterParse);
        let mut p2 = CompilerPlugin::new("opt", "1.0");
        p2.add_hook(PluginHook::AfterOptimize);
        registry.register(p1);
        registry.register(p2);
        assert_eq!(registry.plugin_count(), 2);
        assert_eq!(registry.plugins_for_hook(&PluginHook::AfterParse).len(), 1);
        assert!(registry.get_plugin("lint").is_some());
    }

    #[test]
    fn test_block_to_source() {
        let block = AstFragment::Block(vec![
            AstFragment::Let { name: "x".into(), value: Box::new(AstFragment::IntLit(1)) },
            AstFragment::Return(Box::new(AstFragment::Ident("x".into()))),
        ]);
        let src = block.to_source();
        assert!(src.contains("let x = 1;"));
        assert!(src.contains("return x;"));
    }

    #[test]
    fn test_expand_preserves_non_splices() {
        let frag = AstFragment::IntLit(99);
        let expanded = frag.expand(&HashMap::new());
        assert_eq!(expanded, AstFragment::IntLit(99));
    }

    #[test]
    fn test_splice_missing_binding() {
        let frag = AstFragment::Splice("missing".into());
        let expanded = frag.expand(&HashMap::new());
        assert_eq!(expanded, AstFragment::Splice("missing".into()));
    }

    #[test]
    fn test_grammar_non_terminals() {
        let mut grammar = Grammar::new("prog");
        grammar.add_rule(GrammarRule { name: "prog".into(), alternatives: vec![] });
        grammar.add_rule(GrammarRule { name: "expr".into(), alternatives: vec![] });
        let nts = grammar.non_terminals();
        assert_eq!(nts.len(), 2);
        assert!(nts.contains(&"prog".to_string()));
    }

    #[test]
    fn test_custom_plugin_hook() {
        let mut plugin = CompilerPlugin::new("custom", "0.1");
        plugin.add_hook(PluginHook::Custom("my_phase".into()));
        assert!(plugin.has_hook(&PluginHook::Custom("my_phase".into())));
    }
}
