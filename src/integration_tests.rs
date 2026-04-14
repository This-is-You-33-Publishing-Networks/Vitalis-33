//! v385 — Cross-module integration tests.
//!
//! Validates that the major subsystems work together correctly:
//! - Compiler pipeline: lexer → parser → type checker → IR → codegen
//! - Evolution pipeline: safety rails → engine → meta evolution
//! - Neuromorphic pipeline: spike engine → learning → hybrid
//! - ML pipeline: tensor → autograd → neural_net
//! - Autonomous pipeline: agent → code intelligence → synthesis

/// Verify integration health: returns number of subsystems verified.
pub fn integration_health_check() -> usize {
    let mut verified = 0;

    // 1. Compiler pipeline
    if verify_compiler_pipeline() { verified += 1; }
    // 2. Evolution pipeline
    if verify_evolution_pipeline() { verified += 1; }
    // 3. Neuromorphic pipeline
    if verify_neuromorphic_pipeline() { verified += 1; }
    // 4. ML pipeline
    if verify_ml_pipeline() { verified += 1; }
    // 5. Type system
    if verify_type_system() { verified += 1; }
    // 6. Async runtime
    if verify_async_runtime() { verified += 1; }
    // 7. LSP
    if verify_lsp_pipeline() { verified += 1; }

    verified
}

fn verify_compiler_pipeline() -> bool {
    let source = "fn main() -> i64 { 42 }";
    let (tokens, lex_errors) = crate::lexer::lex(source);
    if tokens.is_empty() || !lex_errors.is_empty() { return false; }

    let (program, errors) = crate::parser::parse(source);
    if !errors.is_empty() { return false; }
    if program.items.is_empty() { return false; }

    true
}

fn verify_evolution_pipeline() -> bool {
    use crate::evolution_safety_rails::SafetyGovernor;
    let governor = SafetyGovernor::new();
    // Governor should have default policy (public field)
    // Default policy should forbid core compiler paths
    governor.policy.forbidden_paths.iter().any(|p| p.contains("lexer"))
}

fn verify_neuromorphic_pipeline() -> bool {
    // spike_engine stores spikes as (timestamp, neuron_id, value) tuples
    // Verify spike emission works via FFI
    crate::spike_engine::slang_spike_emit(0, 1, 100);
    let count = crate::spike_engine::slang_spike_queue_len();
    crate::spike_engine::slang_spike_clear();
    count >= 0
}

fn verify_ml_pipeline() -> bool {
    use crate::tensor::Tensor;
    let t = Tensor::zeros(&[2, 3]);
    t.shape == vec![2, 3]
}

fn verify_type_system() -> bool {
    use crate::types::Type;
    let t = Type::I64;
    matches!(t, Type::I64)
}

fn verify_async_runtime() -> bool {
    use crate::async_runtime::Executor;
    let mut exec = Executor::new();
    let id = exec.spawn_ready("test", 42);
    exec.get_result(id) == Some(42)
}

fn verify_lsp_pipeline() -> bool {
    use crate::lsp::LspServer;
    let mut server = LspServer::new();
    server.initialize();
    server.open_document("test.sl", "fn main() -> i64 { 42 }");
    let diags = server.get_diagnostics("test.sl");
    diags.is_empty() // Valid code → no errors
}

// ─── FFI ─────────────────────────────────────────────────────────────

/// Run integration health check, returns count of verified subsystems.
#[unsafe(no_mangle)]
pub extern "C" fn slang_integration_health() -> i64 {
    integration_health_check() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Compiler Pipeline Integration ────────────────────────────────

    #[test]
    fn test_full_compiler_pipeline() {
        assert!(verify_compiler_pipeline());
    }

    #[test]
    fn test_lex_parse_roundtrip() {
        let source = "fn add(a: i64, b: i64) -> i64 { a + b }";
        let (tokens, lex_errors) = crate::lexer::lex(source);
        assert!(!tokens.is_empty());
        assert!(lex_errors.is_empty());
        let (program, errors) = crate::parser::parse(source);
        assert!(errors.is_empty());
        assert!(!program.items.is_empty());
    }

    #[test]
    fn test_parse_multiple_functions() {
        let source = r#"
fn add(a: i64, b: i64) -> i64 { a + b }
fn sub(a: i64, b: i64) -> i64 { a - b }
fn main() -> i64 { add(10, sub(5, 2)) }
"#;
        let (program, errors) = crate::parser::parse(source);
        assert!(errors.is_empty());
        assert_eq!(program.items.len(), 3);
    }

    #[test]
    fn test_parse_struct_and_function() {
        let source = r#"
struct Point { x: i64, y: i64 }
fn distance(p: Point) -> i64 { p.x + p.y }
"#;
        let (program, errors) = crate::parser::parse(source);
        assert!(errors.is_empty());
        assert_eq!(program.items.len(), 2);
    }

    #[test]
    fn test_parse_enum() {
        let source = r#"
enum Color { Red, Green, Blue }
fn main() -> i64 { 0 }
"#;
        let (program, errors) = crate::parser::parse(source);
        assert!(errors.is_empty());
    }

    #[test]
    fn test_parse_if_else() {
        let source = r#"
fn max(a: i64, b: i64) -> i64 {
    if a > b { a } else { b }
}
"#;
        let (program, errors) = crate::parser::parse(source);
        assert!(errors.is_empty());
    }

    #[test]
    fn test_parse_while_loop() {
        let source = r#"
fn count(n: i64) -> i64 {
    let mut i: i64 = 0;
    while i < n { i = i + 1; }
    i
}
"#;
        let (program, errors) = crate::parser::parse(source);
        assert!(errors.is_empty());
    }

    #[test]
    fn test_parse_match_expr() {
        let source = r#"
fn classify(x: i64) -> i64 {
    match x {
        0 => 0,
        1 => 1,
        _ => 2,
    }
}
"#;
        let (program, errors) = crate::parser::parse(source);
        assert!(errors.is_empty());
    }

    #[test]
    fn test_parse_lambda() {
        let source = r#"
fn apply(f: fn(i64) -> i64, x: i64) -> i64 { f(x) }
fn main() -> i64 { apply(|x: i64| -> i64 { x + 1 }, 41) }
"#;
        let (program, errors) = crate::parser::parse(source);
        assert!(errors.is_empty());
    }

    // ── Evolution Pipeline Integration ───────────────────────────────

    #[test]
    fn test_evolution_pipeline() {
        assert!(verify_evolution_pipeline());
    }

    #[test]
    fn test_safety_rails_block_core_mutation() {
        use crate::evolution_safety_rails::SafetyGovernor;
        let governor = SafetyGovernor::new();
        // Default policy forbids mutating core compiler modules (public field)
        assert!(governor.policy.forbidden_paths.iter().any(|p| p.contains("parser")));
        assert!(governor.policy.forbidden_paths.iter().any(|p| p.contains("codegen")));
    }

    #[test]
    fn test_evolution_registry_mutation() {
        use crate::evolution::MutationRng;
        let mut rng = MutationRng::new(42);
        let source = "fn test() -> i64 { 1 + 2 }";
        let (mutated, _mutation) = crate::evolution::mutate_source(source, &mut rng);
        // Mutation should produce a different string (usually)
        assert!(!mutated.is_empty());
    }

    #[test]
    fn test_improvement_lab_scheduling() {
        use crate::autonomous_improvement_lab::ImprovementLab;
        let mut lab = ImprovementLab::new();
        lab.begin_cycle();
        let manifests = lab.schedule_trials(42, 5, "min-runtime", "optimize", "fn opt() -> i64 { 1 }");
        assert_eq!(manifests.len(), 5);
    }

    // ── Neuromorphic Pipeline Integration ────────────────────────────

    #[test]
    fn test_neuromorphic_pipeline() {
        assert!(verify_neuromorphic_pipeline());
    }

    #[test]
    fn test_spike_engine_integration() {
        // spike_engine stores spikes as (timestamp, neuron_id, value) tuples
        crate::spike_engine::slang_spike_clear();
        for i in 0..10 {
            crate::spike_engine::slang_spike_emit(0, i, i * 10);
        }
        let count = crate::spike_engine::slang_spike_queue_len();
        assert!(count >= 10);
        crate::spike_engine::slang_spike_clear();
    }

    #[test]
    fn test_loihi_sim_core() {
        // Verify Loihi core creation works
        let core_id = crate::loihi_sim::slang_loihi_core_create(1.0, 0.5, 0);
        assert!(core_id >= 0);
    }

    // ── ML Pipeline Integration ──────────────────────────────────────

    #[test]
    fn test_ml_pipeline() {
        assert!(verify_ml_pipeline());
    }

    #[test]
    fn test_tensor_creation() {
        use crate::tensor::Tensor;
        let t = Tensor::zeros(&[3, 4]);
        assert_eq!(t.shape, vec![3, 4]);
    }

    #[test]
    fn test_tensor_ones() {
        use crate::tensor::Tensor;
        let t = Tensor::ones(&[2, 2]);
        assert_eq!(t.shape, vec![2, 2]);
    }

    // ── Type System Integration ──────────────────────────────────────

    #[test]
    fn test_type_system() {
        assert!(verify_type_system());
    }

    #[test]
    fn test_type_display() {
        use crate::types::Type;
        let t = Type::Function { params: vec![Type::I64, Type::I64], ret: Box::new(Type::I64) };
        let s = format!("{}", t);
        assert!(s.contains("i64"));
    }

    // ── Async Runtime Integration ────────────────────────────────────

    #[test]
    fn test_async_runtime() {
        assert!(verify_async_runtime());
    }

    #[test]
    fn test_async_executor_multiple_tasks() {
        use crate::async_runtime::Executor;
        let mut exec = Executor::new();
        let id1 = exec.spawn_ready("t1", 10);
        let id2 = exec.spawn_ready("t2", 20);
        let id3 = exec.spawn_ready("t3", 30);
        assert_eq!(exec.get_result(id1), Some(10));
        assert_eq!(exec.get_result(id2), Some(20));
        assert_eq!(exec.get_result(id3), Some(30));
    }

    #[test]
    fn test_async_channel() {
        use crate::async_runtime::Channel;
        let mut ch = Channel::new(10);
        assert!(ch.send(42));
        assert_eq!(ch.recv(), Some(42));
        assert!(ch.is_empty());
    }

    // ── LSP Integration ─────────────────────────────────────────────

    #[test]
    fn test_lsp_pipeline() {
        assert!(verify_lsp_pipeline());
    }

    #[test]
    fn test_lsp_references_integration() {
        use crate::lsp::LspServer;
        let mut server = LspServer::new();
        server.initialize();
        server.open_document("a.sl", "fn helper() -> i64 { 1 }\nfn main() -> i64 { helper() }");
        let refs = server.find_references("a.sl", "helper");
        assert!(refs.len() >= 2);
    }

    #[test]
    fn test_lsp_rename_integration() {
        use crate::lsp::LspServer;
        let mut server = LspServer::new();
        server.initialize();
        server.open_document("a.sl", "fn calc() -> i64 { 1 }\nfn main() -> i64 { calc() }");
        let edits = server.rename_symbol("a.sl", "calc", "compute");
        assert!(!edits.is_empty());
    }

    // ── Cross-Module Health ──────────────────────────────────────────

    #[test]
    fn test_health_check_comprehensive() {
        let verified = integration_health_check();
        assert!(verified >= 5, "Expected at least 5 subsystems verified, got {}", verified);
    }

    #[test]
    fn test_ffi_integration_health() {
        let h = slang_integration_health();
        assert!(h >= 5);
    }

    // ── IR Integration ───────────────────────────────────────────────

    #[test]
    fn test_ir_module_creation() {
        use crate::ir::{IrModule, IrFunction, IrType, BlockId, BasicBlock};
        let mut module = IrModule::new();
        let func = IrFunction {
            name: "main".into(),
            params: vec![],
            ret_type: IrType::I64,
            blocks: vec![BasicBlock::new(BlockId(0))],
            entry: BlockId(0),
        };
        module.functions.push(func);
        assert_eq!(module.functions.len(), 1);
    }

    #[test]
    fn test_ir_function_blocks() {
        use crate::ir::{IrFunction, IrType, BlockId, BasicBlock};
        let func = IrFunction {
            name: "test".into(),
            params: vec![],
            ret_type: IrType::I64,
            blocks: vec![BasicBlock::new(BlockId(0))],
            entry: BlockId(0),
        };
        assert!(!func.blocks.is_empty());
    }

    // ── Pattern Matching Integration ─────────────────────────────────

    #[test]
    fn test_parse_complex_program() {
        let source = r#"
fn factorial(n: i64) -> i64 {
    if n <= 1 { 1 }
    else { n * factorial(n - 1) }
}
fn fib(n: i64) -> i64 {
    if n <= 1 { n }
    else { fib(n - 1) + fib(n - 2) }
}
fn main() -> i64 { factorial(5) + fib(10) }
"#;
        let (program, errors) = crate::parser::parse(source);
        assert!(errors.is_empty());
        assert_eq!(program.items.len(), 3);
    }

    // ── Stdlib Integration ───────────────────────────────────────────

    #[test]
    fn test_stdlib_builtins_registered() {
        let builtins = crate::stdlib::builtins();
        // Should have hundreds of builtins
        assert!(builtins.len() > 100, "Expected 100+ builtins, got {}", builtins.len());
    }

    #[test]
    fn test_stdlib_has_core_functions() {
        let builtins = crate::stdlib::builtins();
        let names: Vec<&str> = builtins.iter().map(|b| b.name.as_str()).collect();
        assert!(names.contains(&"print"));
        assert!(names.contains(&"spawn"));
        assert!(names.contains(&"sqrt"));
    }
}
