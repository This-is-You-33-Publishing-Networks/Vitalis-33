//! Interprocedural Analysis — Compiler Core Hardening (v304)
//!
//! Call graph construction, interprocedural constant propagation,
//! side-effect inference, and purity analysis. Enables cross-function
//! optimizations by building a global view of function interactions.

use std::sync::{LazyLock, Mutex, atomic::{AtomicI64, Ordering}};
use std::collections::{HashMap, HashSet};

// ── Call graph state ─────────────────────────────────────────────────────────

struct CallGraph {
    /// Adjacency list: caller → set of callees
    edges: HashMap<i64, HashSet<i64>>,
    /// Functions determined to be pure (no side effects)
    pure_fns: HashSet<i64>,
    /// Functions with constant-propagated arguments
    const_args: HashMap<i64, i64>,
}

impl CallGraph {
    fn new() -> Self {
        Self {
            edges: HashMap::new(),
            pure_fns: HashSet::new(),
            const_args: HashMap::new(),
        }
    }
}

static CALL_GRAPH: LazyLock<Mutex<CallGraph>> =
    LazyLock::new(|| Mutex::new(CallGraph::new()));
static IPA_PURE_COUNT: AtomicI64 = AtomicI64::new(0);

/// Add a call edge from caller to callee. Returns total edge count.
#[unsafe(no_mangle)]
pub extern "C" fn slang_ipa_add_edge(caller: i64, callee: i64) -> i64 {
    let mut cg = CALL_GRAPH.lock().unwrap();
    cg.edges.entry(caller).or_default().insert(callee);
    cg.edges.values().map(|s| s.len() as i64).sum()
}

/// Return the number of unique functions (nodes) in the call graph.
#[unsafe(no_mangle)]
pub extern "C" fn slang_ipa_call_graph_size() -> i64 {
    let cg = CALL_GRAPH.lock().unwrap();
    let mut nodes = HashSet::new();
    for (caller, callees) in &cg.edges {
        nodes.insert(*caller);
        for c in callees {
            nodes.insert(*c);
        }
    }
    nodes.len() as i64
}

/// Mark a function as pure (no side effects). Returns total pure count.
#[unsafe(no_mangle)]
pub extern "C" fn slang_ipa_mark_pure(fn_id: i64) -> i64 {
    let mut cg = CALL_GRAPH.lock().unwrap();
    cg.pure_fns.insert(fn_id);
    let count = cg.pure_fns.len() as i64;
    IPA_PURE_COUNT.store(count, Ordering::SeqCst);
    count
}

/// Return the number of functions identified as pure.
#[unsafe(no_mangle)]
pub extern "C" fn slang_ipa_pure_functions() -> i64 {
    IPA_PURE_COUNT.load(Ordering::SeqCst)
}

/// Record that fn_id has n_const constant-propagated arguments.
/// Returns total functions with const args.
#[unsafe(no_mangle)]
pub extern "C" fn slang_ipa_record_const_args(fn_id: i64, n_const: i64) -> i64 {
    let mut cg = CALL_GRAPH.lock().unwrap();
    cg.const_args.insert(fn_id, n_const);
    cg.const_args.len() as i64
}

/// Return total number of functions with constant-propagated args.
#[unsafe(no_mangle)]
pub extern "C" fn slang_ipa_const_args() -> i64 {
    CALL_GRAPH.lock().unwrap().const_args.len() as i64
}

/// Return a summary: (nodes << 32) | edges, packed into i64.
#[unsafe(no_mangle)]
pub extern "C" fn slang_ipa_summary() -> i64 {
    let cg = CALL_GRAPH.lock().unwrap();
    let mut nodes = HashSet::new();
    let mut edge_count = 0i64;
    for (caller, callees) in &cg.edges {
        nodes.insert(*caller);
        for c in callees {
            nodes.insert(*c);
        }
        edge_count += callees.len() as i64;
    }
    (nodes.len() as i64) << 16 | (edge_count & 0xFFFF)
}

/// Clear all interprocedural analysis state.
#[unsafe(no_mangle)]
pub extern "C" fn slang_ipa_clear() -> i64 {
    let mut cg = CALL_GRAPH.lock().unwrap();
    let n = cg.edges.len() as i64;
    cg.edges.clear();
    cg.pure_fns.clear();
    cg.const_args.clear();
    IPA_PURE_COUNT.store(0, Ordering::SeqCst);
    n
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn reset() { slang_ipa_clear(); }

    #[test]
    fn test_add_edge() {
        reset();
        let count = slang_ipa_add_edge(1, 2);
        assert_eq!(count, 1);
    }

    #[test]
    fn test_call_graph_size() {
        reset();
        slang_ipa_add_edge(1, 2);
        slang_ipa_add_edge(1, 3);
        slang_ipa_add_edge(2, 3);
        assert_eq!(slang_ipa_call_graph_size(), 3); // nodes: 1, 2, 3
    }

    #[test]
    fn test_duplicate_edge() {
        reset();
        slang_ipa_add_edge(1, 2);
        slang_ipa_add_edge(1, 2);
        assert_eq!(slang_ipa_call_graph_size(), 2);
    }

    #[test]
    fn test_mark_pure() {
        reset();
        slang_ipa_mark_pure(1);
        slang_ipa_mark_pure(2);
        assert_eq!(slang_ipa_pure_functions(), 2);
    }

    #[test]
    fn test_pure_idempotent() {
        reset();
        slang_ipa_mark_pure(1);
        slang_ipa_mark_pure(1);
        assert_eq!(slang_ipa_pure_functions(), 1);
    }

    #[test]
    fn test_const_args() {
        reset();
        slang_ipa_record_const_args(1, 3);
        slang_ipa_record_const_args(2, 1);
        assert_eq!(slang_ipa_const_args(), 2);
    }

    #[test]
    fn test_summary_packed() {
        reset();
        slang_ipa_add_edge(1, 2);
        slang_ipa_add_edge(3, 4);
        let s = slang_ipa_summary();
        let nodes = s >> 16;
        let edges = s & 0xFFFF;
        assert_eq!(nodes, 4);
        assert_eq!(edges, 2);
    }

    #[test]
    fn test_clear() {
        reset();
        slang_ipa_add_edge(1, 2);
        slang_ipa_mark_pure(1);
        slang_ipa_record_const_args(1, 2);
        slang_ipa_clear();
        assert_eq!(slang_ipa_call_graph_size(), 0);
        assert_eq!(slang_ipa_pure_functions(), 0);
        assert_eq!(slang_ipa_const_args(), 0);
    }

    #[test]
    fn test_empty_graph() {
        reset();
        assert_eq!(slang_ipa_call_graph_size(), 0);
        assert_eq!(slang_ipa_summary(), 0);
    }

    #[test]
    fn test_self_call() {
        reset();
        slang_ipa_add_edge(1, 1); // recursive
        assert_eq!(slang_ipa_call_graph_size(), 1);
    }

    #[test]
    fn test_chain_call() {
        reset();
        for i in 0..10 {
            slang_ipa_add_edge(i, i + 1);
        }
        assert_eq!(slang_ipa_call_graph_size(), 11);
    }

    #[test]
    fn test_star_topology() {
        reset();
        for i in 1..=5 {
            slang_ipa_add_edge(0, i);
        }
        assert_eq!(slang_ipa_call_graph_size(), 6);
    }

    #[test]
    fn test_const_args_update() {
        reset();
        slang_ipa_record_const_args(1, 2);
        slang_ipa_record_const_args(1, 5); // update
        assert_eq!(slang_ipa_const_args(), 1);
    }

    #[test]
    fn test_mixed_operations() {
        reset();
        slang_ipa_add_edge(1, 2);
        slang_ipa_add_edge(2, 3);
        slang_ipa_mark_pure(2);
        slang_ipa_mark_pure(3);
        slang_ipa_record_const_args(1, 1);
        assert_eq!(slang_ipa_call_graph_size(), 3);
        assert_eq!(slang_ipa_pure_functions(), 2);
        assert_eq!(slang_ipa_const_args(), 1);
    }

    #[test]
    fn test_negative_fn_id() {
        reset();
        slang_ipa_add_edge(-1, -2);
        assert_eq!(slang_ipa_call_graph_size(), 2);
    }

    #[test]
    fn test_clear_returns_caller_count() {
        reset();
        slang_ipa_add_edge(1, 2);
        slang_ipa_add_edge(3, 4);
        let n = slang_ipa_clear();
        assert_eq!(n, 2); // 2 callers
    }

    #[test]
    fn test_large_graph() {
        reset();
        for i in 0..50 {
            slang_ipa_add_edge(i, i + 1);
            if i % 5 == 0 {
                slang_ipa_mark_pure(i);
            }
        }
        assert_eq!(slang_ipa_call_graph_size(), 51);
        assert_eq!(slang_ipa_pure_functions(), 10);
    }

    #[test]
    fn test_bidirectional_edges() {
        reset();
        slang_ipa_add_edge(1, 2);
        slang_ipa_add_edge(2, 1); // mutual recursion
        assert_eq!(slang_ipa_call_graph_size(), 2);
    }

    #[test]
    fn test_zero_fn_id() {
        reset();
        slang_ipa_add_edge(0, 1);
        slang_ipa_mark_pure(0);
        assert_eq!(slang_ipa_pure_functions(), 1);
    }
}
