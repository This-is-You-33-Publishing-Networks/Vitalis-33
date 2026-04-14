//! Semantic Graph Engine — Vitalis v601
//!
//! Builds rich semantic graphs from AST structures:
//! - Data flow edges (def-use, use-def chains)
//! - Control flow edges (branch, loop, exception)
//! - Effect flow edges (IO, mutation, allocation)
//! - Intent edges (inferred purpose annotations)
//! - Dominance frontiers and post-dominance
//! - Reaching definitions analysis
//! - Semantic similarity scoring between subgraphs

use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt;

// ── Node Types ───────────────────────────────────────────────────────

/// Semantic node kind in the graph.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SemanticNodeKind {
    /// Variable definition
    VarDef { name: String, mutable: bool },
    /// Variable use / read
    VarUse { name: String },
    /// Function call
    Call { target: String, arity: usize },
    /// Literal value
    Literal { type_name: String },
    /// Branch (if/match)
    Branch { arms: usize },
    /// Loop construct
    Loop { kind: LoopKind },
    /// Return from function
    Return,
    /// Effect (IO, mutation, etc.)
    Effect { effect: EffectKind },
    /// Allocation
    Alloc { type_name: String },
    /// Field access
    FieldAccess { field: String },
    /// Binary operation
    BinOp { op: String },
    /// Phi node (SSA merge)
    Phi { sources: usize },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum LoopKind {
    While,
    For,
    Loop,
    Iterator,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum EffectKind {
    IO,
    Mutation,
    Allocation,
    NetworkAccess,
    FileSystem,
    Unsafe,
    Pure,
}

impl fmt::Display for EffectKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EffectKind::IO => write!(f, "IO"),
            EffectKind::Mutation => write!(f, "Mutation"),
            EffectKind::Allocation => write!(f, "Allocation"),
            EffectKind::NetworkAccess => write!(f, "Net"),
            EffectKind::FileSystem => write!(f, "FS"),
            EffectKind::Unsafe => write!(f, "Unsafe"),
            EffectKind::Pure => write!(f, "Pure"),
        }
    }
}

// ── Edge Types ───────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SemanticEdgeKind {
    /// Data flows from source to target
    DataFlow,
    /// Control flows from source to target
    ControlFlow,
    /// Effect dependency
    EffectFlow { effect: EffectKind },
    /// Dominance edge
    Dominates,
    /// Post-dominance edge
    PostDominates,
    /// Intent/purpose edge
    Intent { label: String },
    /// Def-use chain
    DefUse,
    /// Use-def chain
    UseDef,
}

// ── Core Graph ───────────────────────────────────────────────────────

pub type NodeId = usize;

#[derive(Debug, Clone)]
pub struct SemanticNode {
    pub id: NodeId,
    pub kind: SemanticNodeKind,
    pub span_start: usize,
    pub span_end: usize,
    pub depth: usize,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct SemanticEdge {
    pub from: NodeId,
    pub to: NodeId,
    pub kind: SemanticEdgeKind,
    pub weight: f64,
}

#[derive(Debug, Clone)]
pub struct SemanticGraph {
    pub nodes: Vec<SemanticNode>,
    pub edges: Vec<SemanticEdge>,
    adjacency: HashMap<NodeId, Vec<usize>>,
    reverse_adj: HashMap<NodeId, Vec<usize>>,
    next_id: NodeId,
}

impl SemanticGraph {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            edges: Vec::new(),
            adjacency: HashMap::new(),
            reverse_adj: HashMap::new(),
            next_id: 0,
        }
    }

    pub fn add_node(&mut self, kind: SemanticNodeKind, span_start: usize, span_end: usize, depth: usize) -> NodeId {
        let id = self.next_id;
        self.next_id += 1;
        self.nodes.push(SemanticNode {
            id,
            kind,
            span_start,
            span_end,
            depth,
            metadata: HashMap::new(),
        });
        id
    }

    pub fn add_edge(&mut self, from: NodeId, to: NodeId, kind: SemanticEdgeKind, weight: f64) {
        let idx = self.edges.len();
        self.edges.push(SemanticEdge { from, to, kind, weight });
        self.adjacency.entry(from).or_default().push(idx);
        self.reverse_adj.entry(to).or_default().push(idx);
    }

    pub fn node(&self, id: NodeId) -> Option<&SemanticNode> {
        self.nodes.iter().find(|n| n.id == id)
    }

    pub fn successors(&self, id: NodeId) -> Vec<NodeId> {
        self.adjacency.get(&id)
            .map(|idxs| idxs.iter().map(|&i| self.edges[i].to).collect())
            .unwrap_or_default()
    }

    pub fn predecessors(&self, id: NodeId) -> Vec<NodeId> {
        self.reverse_adj.get(&id)
            .map(|idxs| idxs.iter().map(|&i| self.edges[i].from).collect())
            .unwrap_or_default()
    }

    pub fn edges_from(&self, id: NodeId) -> Vec<&SemanticEdge> {
        self.adjacency.get(&id)
            .map(|idxs| idxs.iter().map(|&i| &self.edges[i]).collect())
            .unwrap_or_default()
    }

    pub fn edges_to(&self, id: NodeId) -> Vec<&SemanticEdge> {
        self.reverse_adj.get(&id)
            .map(|idxs| idxs.iter().map(|&i| &self.edges[i]).collect())
            .unwrap_or_default()
    }

    // ── Reaching Definitions ─────────────────────────────────────────

    /// Compute reaching definitions using iterative data-flow analysis.
    /// Returns map: NodeId → set of definition NodeIds that reach it.
    pub fn reaching_definitions(&self) -> HashMap<NodeId, HashSet<NodeId>> {
        let mut reach_in: HashMap<NodeId, HashSet<NodeId>> = HashMap::new();
        let mut reach_out: HashMap<NodeId, HashSet<NodeId>> = HashMap::new();

        // Identify definition nodes
        let def_nodes: HashSet<NodeId> = self.nodes.iter()
            .filter(|n| matches!(n.kind, SemanticNodeKind::VarDef { .. }))
            .map(|n| n.id)
            .collect();

        // Initialize
        for n in &self.nodes {
            reach_in.insert(n.id, HashSet::new());
            reach_out.insert(n.id, HashSet::new());
        }

        // Iterative fixed-point
        let mut changed = true;
        let mut iterations = 0;
        while changed && iterations < 100 {
            changed = false;
            iterations += 1;

            for node in &self.nodes {
                // reach_in[n] = ∪ reach_out[p] for all predecessors p
                let preds = self.predecessors(node.id);
                let mut new_in = HashSet::new();
                for p in &preds {
                    if let Some(out) = reach_out.get(p) {
                        new_in.extend(out.iter().cloned());
                    }
                }

                // generate/kill
                let generated: HashSet<NodeId> = if def_nodes.contains(&node.id) {
                    let mut s = HashSet::new();
                    s.insert(node.id);
                    s
                } else {
                    HashSet::new()
                };

                // Kill: other defs of the same variable
                let kill: HashSet<NodeId> = if let SemanticNodeKind::VarDef { ref name, .. } = node.kind {
                    def_nodes.iter()
                        .filter(|&&d| d != node.id)
                        .filter(|&&d| {
                            if let Some(dn) = self.node(d) {
                                matches!(&dn.kind, SemanticNodeKind::VarDef { name: n, .. } if n == name)
                            } else {
                                false
                            }
                        })
                        .cloned()
                        .collect()
                } else {
                    HashSet::new()
                };

                // reach_out[n] = generated ∪ (reach_in[n] - kill)
                let new_out: HashSet<NodeId> = generated.union(
                    &new_in.difference(&kill).cloned().collect()
                ).cloned().collect();

                if new_in != *reach_in.get(&node.id).unwrap() {
                    changed = true;
                }
                if new_out != *reach_out.get(&node.id).unwrap() {
                    changed = true;
                }

                reach_in.insert(node.id, new_in);
                reach_out.insert(node.id, new_out);
            }
        }

        reach_in
    }

    // ── Dominance Frontier ───────────────────────────────────────────

    /// Compute immediate dominators using the Cooper-Harvey-Kennedy algorithm.
    /// Returns map: NodeId → immediate dominator NodeId.
    pub fn compute_idom(&self, entry: NodeId) -> HashMap<NodeId, NodeId> {
        let order = self.reverse_postorder(entry);
        let pos: HashMap<NodeId, usize> = order.iter().enumerate().map(|(i, &n)| (n, i)).collect();

        let mut idom: HashMap<NodeId, NodeId> = HashMap::new();
        idom.insert(entry, entry);

        let mut changed = true;
        while changed {
            changed = false;
            for &b in &order {
                if b == entry { continue; }
                let preds = self.predecessors(b);
                let processed: Vec<NodeId> = preds.iter().filter(|p| idom.contains_key(p)).cloned().collect();
                if processed.is_empty() { continue; }

                let mut new_idom = processed[0];
                for &p in &processed[1..] {
                    new_idom = self.intersect_dom(p, new_idom, &idom, &pos);
                }

                if idom.get(&b) != Some(&new_idom) {
                    idom.insert(b, new_idom);
                    changed = true;
                }
            }
        }

        idom
    }

    fn intersect_dom(&self, mut a: NodeId, mut b: NodeId, idom: &HashMap<NodeId, NodeId>, pos: &HashMap<NodeId, usize>) -> NodeId {
        while a != b {
            while pos.get(&a).unwrap_or(&0) > pos.get(&b).unwrap_or(&0) {
                a = *idom.get(&a).unwrap_or(&a);
            }
            while pos.get(&b).unwrap_or(&0) > pos.get(&a).unwrap_or(&0) {
                b = *idom.get(&b).unwrap_or(&b);
            }
        }
        a
    }

    /// Compute dominance frontier from immediate dominators.
    pub fn dominance_frontier(&self, entry: NodeId) -> HashMap<NodeId, HashSet<NodeId>> {
        let idom = self.compute_idom(entry);
        let mut df: HashMap<NodeId, HashSet<NodeId>> = HashMap::new();

        for n in &self.nodes {
            df.insert(n.id, HashSet::new());
        }

        for n in &self.nodes {
            let preds = self.predecessors(n.id);
            if preds.len() >= 2 {
                for p in &preds {
                    let mut runner = *p;
                    while runner != *idom.get(&n.id).unwrap_or(&n.id) {
                        df.entry(runner).or_default().insert(n.id);
                        if let Some(&dom) = idom.get(&runner) {
                            if dom == runner { break; }
                            runner = dom;
                        } else {
                            break;
                        }
                    }
                }
            }
        }

        df
    }

    // ── Topological & Reverse-Postorder ──────────────────────────────

    pub fn reverse_postorder(&self, entry: NodeId) -> Vec<NodeId> {
        let mut visited = HashSet::new();
        let mut order = Vec::new();
        self.rpo_dfs(entry, &mut visited, &mut order);
        order.reverse();
        order
    }

    fn rpo_dfs(&self, node: NodeId, visited: &mut HashSet<NodeId>, order: &mut Vec<NodeId>) {
        if !visited.insert(node) { return; }
        for succ in self.successors(node) {
            self.rpo_dfs(succ, visited, order);
        }
        order.push(node);
    }

    // ── Def-Use Chain Builder ────────────────────────────────────────

    /// Build def-use and use-def edges from existing data flow edges.
    pub fn build_def_use_chains(&mut self) {
        let mut defs: HashMap<String, Vec<NodeId>> = HashMap::new();
        let mut uses: HashMap<String, Vec<NodeId>> = HashMap::new();

        for n in &self.nodes {
            match &n.kind {
                SemanticNodeKind::VarDef { name, .. } => {
                    defs.entry(name.clone()).or_default().push(n.id);
                }
                SemanticNodeKind::VarUse { name } => {
                    uses.entry(name.clone()).or_default().push(n.id);
                }
                _ => {}
            }
        }

        let mut new_edges = Vec::new();
        for (name, def_ids) in &defs {
            if let Some(use_ids) = uses.get(name) {
                for &d in def_ids {
                    for &u in use_ids {
                        if d < u {
                            new_edges.push((d, u, SemanticEdgeKind::DefUse, 1.0));
                            new_edges.push((u, d, SemanticEdgeKind::UseDef, 1.0));
                        }
                    }
                }
            }
        }

        for (from, to, kind, weight) in new_edges {
            self.add_edge(from, to, kind, weight);
        }
    }

    // ── Effect Analysis ──────────────────────────────────────────────

    /// Collect all effects reachable from a node via transitive closure.
    pub fn transitive_effects(&self, start: NodeId) -> HashSet<EffectKind> {
        let mut effects = HashSet::new();
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        queue.push_back(start);

        while let Some(node) = queue.pop_front() {
            if !visited.insert(node) { continue; }
            if let Some(n) = self.node(node) {
                if let SemanticNodeKind::Effect { ref effect } = n.kind {
                    effects.insert(effect.clone());
                }
            }
            for succ in self.successors(node) {
                queue.push_back(succ);
            }
        }

        effects
    }

    // ── Semantic Similarity ──────────────────────────────────────────

    /// Compute structural similarity between two subgraphs rooted at given nodes.
    /// Returns 0.0 (completely different) to 1.0 (identical structure).
    pub fn subgraph_similarity(&self, root_a: NodeId, root_b: NodeId) -> f64 {
        let nodes_a = self.collect_reachable(root_a);
        let nodes_b = self.collect_reachable(root_b);

        if nodes_a.is_empty() && nodes_b.is_empty() { return 1.0; }
        if nodes_a.is_empty() || nodes_b.is_empty() { return 0.0; }

        // Compare node kind distributions
        let kinds_a = self.kind_histogram(&nodes_a);
        let kinds_b = self.kind_histogram(&nodes_b);

        let all_kinds: HashSet<&str> = kinds_a.keys().chain(kinds_b.keys()).cloned().collect();
        let mut intersection = 0.0;
        let mut union = 0.0;

        for k in &all_kinds {
            let a = *kinds_a.get(k).unwrap_or(&0) as f64;
            let b = *kinds_b.get(k).unwrap_or(&0) as f64;
            intersection += a.min(b);
            union += a.max(b);
        }

        if union == 0.0 { 1.0 } else { intersection / union }
    }

    fn collect_reachable(&self, root: NodeId) -> HashSet<NodeId> {
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        queue.push_back(root);
        while let Some(n) = queue.pop_front() {
            if visited.insert(n) {
                for s in self.successors(n) {
                    queue.push_back(s);
                }
            }
        }
        visited
    }

    fn kind_histogram<'a>(&'a self, nodes: &HashSet<NodeId>) -> HashMap<&'a str, usize> {
        let mut hist = HashMap::new();
        for &id in nodes {
            if let Some(n) = self.node(id) {
                let label = match &n.kind {
                    SemanticNodeKind::VarDef { .. } => "VarDef",
                    SemanticNodeKind::VarUse { .. } => "VarUse",
                    SemanticNodeKind::Call { .. } => "Call",
                    SemanticNodeKind::Literal { .. } => "Literal",
                    SemanticNodeKind::Branch { .. } => "Branch",
                    SemanticNodeKind::Loop { .. } => "Loop",
                    SemanticNodeKind::Return => "Return",
                    SemanticNodeKind::Effect { .. } => "Effect",
                    SemanticNodeKind::Alloc { .. } => "Alloc",
                    SemanticNodeKind::FieldAccess { .. } => "FieldAccess",
                    SemanticNodeKind::BinOp { .. } => "BinOp",
                    SemanticNodeKind::Phi { .. } => "Phi",
                };
                *hist.entry(label).or_insert(0) += 1;
            }
        }
        hist
    }

    // ── Graph Metrics ────────────────────────────────────────────────

    pub fn node_count(&self) -> usize { self.nodes.len() }
    pub fn edge_count(&self) -> usize { self.edges.len() }

    /// Cyclomatic complexity: E - N + 2P (P=1 for single component)
    pub fn cyclomatic_complexity(&self) -> usize {
        let e = self.edges.iter().filter(|e| matches!(e.kind, SemanticEdgeKind::ControlFlow)).count() as isize;
        let n = self.nodes.len() as isize;
        if n == 0 { return 1; }
        (e - n + 2).max(1) as usize
    }

    /// Compute data flow depth: longest chain of def-use edges.
    pub fn data_flow_depth(&self) -> usize {
        let mut max_depth = 0;
        for n in &self.nodes {
            if matches!(n.kind, SemanticNodeKind::VarDef { .. }) {
                let d = self.longest_def_use_chain(n.id, &mut HashSet::new());
                max_depth = max_depth.max(d);
            }
        }
        max_depth
    }

    fn longest_def_use_chain(&self, node: NodeId, visited: &mut HashSet<NodeId>) -> usize {
        if !visited.insert(node) { return 0; }
        let mut max_child = 0;
        for edge in self.edges_from(node) {
            if matches!(edge.kind, SemanticEdgeKind::DefUse) {
                let child_depth = self.longest_def_use_chain(edge.to, visited);
                max_child = max_child.max(child_depth);
            }
        }
        visited.remove(&node);
        max_child + 1
    }

    /// Check if the graph is a DAG (no cycles in data flow).
    pub fn is_acyclic_data_flow(&self) -> bool {
        let mut visited = HashSet::new();
        let mut stack = HashSet::new();
        for n in &self.nodes {
            if !visited.contains(&n.id) {
                if self.has_cycle_dfs(n.id, &mut visited, &mut stack) {
                    return false;
                }
            }
        }
        true
    }

    fn has_cycle_dfs(&self, node: NodeId, visited: &mut HashSet<NodeId>, stack: &mut HashSet<NodeId>) -> bool {
        visited.insert(node);
        stack.insert(node);
        for edge in self.edges_from(node) {
            if matches!(edge.kind, SemanticEdgeKind::DataFlow | SemanticEdgeKind::DefUse) {
                if !visited.contains(&edge.to) {
                    if self.has_cycle_dfs(edge.to, visited, stack) { return true; }
                } else if stack.contains(&edge.to) {
                    return true;
                }
            }
        }
        stack.remove(&node);
        false
    }
}

impl Default for SemanticGraph {
    fn default() -> Self { Self::new() }
}

// ── FFI Exports ──────────────────────────────────────────────────────

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_semantic_graph_new() -> i64 {
    let g = Box::new(SemanticGraph::new());
    Box::into_raw(g) as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_semantic_graph_add_node(kind_id: i64, span_start: i64, span_end: i64, depth: i64) -> i64 {
    let kind = match kind_id {
        0 => SemanticNodeKind::VarDef { name: "x".into(), mutable: false },
        1 => SemanticNodeKind::VarUse { name: "x".into() },
        2 => SemanticNodeKind::Call { target: "f".into(), arity: 1 },
        3 => SemanticNodeKind::Literal { type_name: "i64".into() },
        4 => SemanticNodeKind::Branch { arms: 2 },
        5 => SemanticNodeKind::Loop { kind: LoopKind::While },
        6 => SemanticNodeKind::Return,
        7 => SemanticNodeKind::Effect { effect: EffectKind::IO },
        8 => SemanticNodeKind::Alloc { type_name: "Vec".into() },
        9 => SemanticNodeKind::BinOp { op: "+".into() },
        _ => SemanticNodeKind::Literal { type_name: "unknown".into() },
    };
    let mut g = SemanticGraph::new();
    g.add_node(kind, span_start as usize, span_end as usize, depth as usize) as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_semantic_graph_cyclomatic(nodes: i64, cf_edges: i64) -> i64 {
    if nodes == 0 { return 1; }
    (cf_edges - nodes + 2).max(1)
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_semantic_similarity_jaccard(
    set_a_len: i64, set_b_len: i64, intersection_len: i64,
) -> f64 {
    let union = set_a_len + set_b_len - intersection_len;
    if union <= 0 { 1.0 } else { intersection_len as f64 / union as f64 }
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_semantic_data_flow_depth(chain_lengths: *const i64, count: i64) -> i64 {
    if chain_lengths.is_null() || count <= 0 { return 0; }
    let lengths = unsafe { std::slice::from_raw_parts(chain_lengths, count as usize) };
    *lengths.iter().max().unwrap_or(&0)
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_semantic_reaching_def_count(gen_count: i64, kill_count: i64, in_count: i64) -> i64 {
    gen_count + (in_count - kill_count).max(0)
}

// ═══════════════════════════════════════════════════════════════════════
// Tests
// ═══════════════════════════════════════════════════════════════════════
#[cfg(test)]
mod tests {
    use super::*;

    // ── Basic Graph Construction ─────────────────────────────────────

    #[test]
    fn test_empty_graph() {
        let g = SemanticGraph::new();
        assert_eq!(g.node_count(), 0);
        assert_eq!(g.edge_count(), 0);
    }

    #[test]
    fn test_add_nodes() {
        let mut g = SemanticGraph::new();
        let n0 = g.add_node(SemanticNodeKind::VarDef { name: "x".into(), mutable: false }, 0, 5, 0);
        let n1 = g.add_node(SemanticNodeKind::VarUse { name: "x".into() }, 10, 11, 0);
        assert_eq!(n0, 0);
        assert_eq!(n1, 1);
        assert_eq!(g.node_count(), 2);
    }

    #[test]
    fn test_add_edges() {
        let mut g = SemanticGraph::new();
        let a = g.add_node(SemanticNodeKind::VarDef { name: "a".into(), mutable: false }, 0, 5, 0);
        let b = g.add_node(SemanticNodeKind::VarUse { name: "a".into() }, 10, 11, 0);
        g.add_edge(a, b, SemanticEdgeKind::DataFlow, 1.0);
        assert_eq!(g.edge_count(), 1);
        assert_eq!(g.successors(a), vec![b]);
        assert_eq!(g.predecessors(b), vec![a]);
    }

    #[test]
    fn test_edges_from_to() {
        let mut g = SemanticGraph::new();
        let a = g.add_node(SemanticNodeKind::VarDef { name: "x".into(), mutable: true }, 0, 3, 0);
        let b = g.add_node(SemanticNodeKind::VarUse { name: "x".into() }, 5, 6, 0);
        let c = g.add_node(SemanticNodeKind::VarUse { name: "x".into() }, 8, 9, 0);
        g.add_edge(a, b, SemanticEdgeKind::DefUse, 1.0);
        g.add_edge(a, c, SemanticEdgeKind::DefUse, 1.0);
        assert_eq!(g.edges_from(a).len(), 2);
        assert_eq!(g.edges_to(b).len(), 1);
    }

    // ── Def-Use Chains ───────────────────────────────────────────────

    #[test]
    fn test_def_use_chain_builder() {
        let mut g = SemanticGraph::new();
        let d = g.add_node(SemanticNodeKind::VarDef { name: "x".into(), mutable: false }, 0, 5, 0);
        let u1 = g.add_node(SemanticNodeKind::VarUse { name: "x".into() }, 10, 11, 0);
        let u2 = g.add_node(SemanticNodeKind::VarUse { name: "x".into() }, 15, 16, 0);
        g.add_edge(d, u1, SemanticEdgeKind::ControlFlow, 1.0);
        g.add_edge(u1, u2, SemanticEdgeKind::ControlFlow, 1.0);
        g.build_def_use_chains();
        let def_use_edges: Vec<_> = g.edges.iter()
            .filter(|e| matches!(e.kind, SemanticEdgeKind::DefUse))
            .collect();
        assert_eq!(def_use_edges.len(), 2);
    }

    #[test]
    fn test_def_use_no_false_links() {
        let mut g = SemanticGraph::new();
        g.add_node(SemanticNodeKind::VarDef { name: "x".into(), mutable: false }, 0, 5, 0);
        g.add_node(SemanticNodeKind::VarUse { name: "y".into() }, 10, 11, 0);
        g.build_def_use_chains();
        let def_use_edges: Vec<_> = g.edges.iter()
            .filter(|e| matches!(e.kind, SemanticEdgeKind::DefUse))
            .collect();
        assert_eq!(def_use_edges.len(), 0);
    }

    // ── Reaching Definitions ─────────────────────────────────────────

    #[test]
    fn test_reaching_definitions_simple() {
        let mut g = SemanticGraph::new();
        let d1 = g.add_node(SemanticNodeKind::VarDef { name: "x".into(), mutable: false }, 0, 5, 0);
        let u1 = g.add_node(SemanticNodeKind::VarUse { name: "x".into() }, 10, 11, 0);
        g.add_edge(d1, u1, SemanticEdgeKind::ControlFlow, 1.0);
        let reaching = g.reaching_definitions();
        assert!(reaching.get(&u1).unwrap().contains(&d1));
    }

    #[test]
    fn test_reaching_definitions_kill() {
        let mut g = SemanticGraph::new();
        let d1 = g.add_node(SemanticNodeKind::VarDef { name: "x".into(), mutable: true }, 0, 5, 0);
        let d2 = g.add_node(SemanticNodeKind::VarDef { name: "x".into(), mutable: true }, 6, 10, 0);
        let u1 = g.add_node(SemanticNodeKind::VarUse { name: "x".into() }, 15, 16, 0);
        g.add_edge(d1, d2, SemanticEdgeKind::ControlFlow, 1.0);
        g.add_edge(d2, u1, SemanticEdgeKind::ControlFlow, 1.0);
        let reaching = g.reaching_definitions();
        // d2 kills d1, so only d2 reaches u1
        let reach_u1 = reaching.get(&u1).unwrap();
        assert!(reach_u1.contains(&d2));
    }

    // ── Dominance ────────────────────────────────────────────────────

    #[test]
    fn test_dominance_linear() {
        let mut g = SemanticGraph::new();
        let a = g.add_node(SemanticNodeKind::Literal { type_name: "entry".into() }, 0, 1, 0);
        let b = g.add_node(SemanticNodeKind::Literal { type_name: "mid".into() }, 2, 3, 0);
        let c = g.add_node(SemanticNodeKind::Return, 4, 5, 0);
        g.add_edge(a, b, SemanticEdgeKind::ControlFlow, 1.0);
        g.add_edge(b, c, SemanticEdgeKind::ControlFlow, 1.0);
        let idom = g.compute_idom(a);
        assert_eq!(idom[&b], a);
        assert_eq!(idom[&c], b);
    }

    #[test]
    fn test_dominance_diamond() {
        let mut g = SemanticGraph::new();
        let entry = g.add_node(SemanticNodeKind::Branch { arms: 2 }, 0, 1, 0);
        let left = g.add_node(SemanticNodeKind::Literal { type_name: "left".into() }, 2, 3, 1);
        let right = g.add_node(SemanticNodeKind::Literal { type_name: "right".into() }, 4, 5, 1);
        let merge = g.add_node(SemanticNodeKind::Phi { sources: 2 }, 6, 7, 0);
        g.add_edge(entry, left, SemanticEdgeKind::ControlFlow, 1.0);
        g.add_edge(entry, right, SemanticEdgeKind::ControlFlow, 1.0);
        g.add_edge(left, merge, SemanticEdgeKind::ControlFlow, 1.0);
        g.add_edge(right, merge, SemanticEdgeKind::ControlFlow, 1.0);
        let idom = g.compute_idom(entry);
        assert_eq!(idom[&left], entry);
        assert_eq!(idom[&right], entry);
        assert_eq!(idom[&merge], entry);
    }

    #[test]
    fn test_dominance_frontier_diamond() {
        let mut g = SemanticGraph::new();
        let entry = g.add_node(SemanticNodeKind::Branch { arms: 2 }, 0, 1, 0);
        let left = g.add_node(SemanticNodeKind::Literal { type_name: "L".into() }, 2, 3, 1);
        let right = g.add_node(SemanticNodeKind::Literal { type_name: "R".into() }, 4, 5, 1);
        let merge = g.add_node(SemanticNodeKind::Phi { sources: 2 }, 6, 7, 0);
        g.add_edge(entry, left, SemanticEdgeKind::ControlFlow, 1.0);
        g.add_edge(entry, right, SemanticEdgeKind::ControlFlow, 1.0);
        g.add_edge(left, merge, SemanticEdgeKind::ControlFlow, 1.0);
        g.add_edge(right, merge, SemanticEdgeKind::ControlFlow, 1.0);
        let df = g.dominance_frontier(entry);
        assert!(df[&left].contains(&merge));
        assert!(df[&right].contains(&merge));
    }

    // ── Effect Analysis ──────────────────────────────────────────────

    #[test]
    fn test_transitive_effects() {
        let mut g = SemanticGraph::new();
        let a = g.add_node(SemanticNodeKind::Call { target: "open".into(), arity: 1 }, 0, 4, 0);
        let b = g.add_node(SemanticNodeKind::Effect { effect: EffectKind::FileSystem }, 5, 10, 0);
        let c = g.add_node(SemanticNodeKind::Effect { effect: EffectKind::IO }, 11, 15, 0);
        g.add_edge(a, b, SemanticEdgeKind::ControlFlow, 1.0);
        g.add_edge(b, c, SemanticEdgeKind::ControlFlow, 1.0);
        let effects = g.transitive_effects(a);
        assert!(effects.contains(&EffectKind::FileSystem));
        assert!(effects.contains(&EffectKind::IO));
    }

    #[test]
    fn test_pure_function_no_effects() {
        let mut g = SemanticGraph::new();
        let a = g.add_node(SemanticNodeKind::BinOp { op: "+".into() }, 0, 3, 0);
        let b = g.add_node(SemanticNodeKind::Return, 4, 5, 0);
        g.add_edge(a, b, SemanticEdgeKind::ControlFlow, 1.0);
        let effects = g.transitive_effects(a);
        assert!(effects.is_empty());
    }

    // ── Similarity ───────────────────────────────────────────────────

    #[test]
    fn test_identical_subgraph_similarity() {
        let mut g = SemanticGraph::new();
        let a = g.add_node(SemanticNodeKind::BinOp { op: "+".into() }, 0, 1, 0);
        let b = g.add_node(SemanticNodeKind::BinOp { op: "+".into() }, 2, 3, 0);
        assert_eq!(g.subgraph_similarity(a, b), 1.0);
    }

    #[test]
    fn test_different_subgraph_similarity() {
        let mut g = SemanticGraph::new();
        let a = g.add_node(SemanticNodeKind::BinOp { op: "+".into() }, 0, 1, 0);
        let b = g.add_node(SemanticNodeKind::Loop { kind: LoopKind::While }, 2, 3, 0);
        let sim = g.subgraph_similarity(a, b);
        assert!(sim < 1.0);
    }

    // ── Cyclomatic / Metrics ─────────────────────────────────────────

    #[test]
    fn test_cyclomatic_linear() {
        let mut g = SemanticGraph::new();
        let a = g.add_node(SemanticNodeKind::Literal { type_name: "i".into() }, 0, 1, 0);
        let b = g.add_node(SemanticNodeKind::Return, 2, 3, 0);
        g.add_edge(a, b, SemanticEdgeKind::ControlFlow, 1.0);
        assert_eq!(g.cyclomatic_complexity(), 1);
    }

    #[test]
    fn test_cyclomatic_branch() {
        let mut g = SemanticGraph::new();
        let entry = g.add_node(SemanticNodeKind::Branch { arms: 2 }, 0, 1, 0);
        let left = g.add_node(SemanticNodeKind::Literal { type_name: "L".into() }, 2, 3, 1);
        let right = g.add_node(SemanticNodeKind::Literal { type_name: "R".into() }, 4, 5, 1);
        let merge = g.add_node(SemanticNodeKind::Return, 6, 7, 0);
        g.add_edge(entry, left, SemanticEdgeKind::ControlFlow, 1.0);
        g.add_edge(entry, right, SemanticEdgeKind::ControlFlow, 1.0);
        g.add_edge(left, merge, SemanticEdgeKind::ControlFlow, 1.0);
        g.add_edge(right, merge, SemanticEdgeKind::ControlFlow, 1.0);
        assert_eq!(g.cyclomatic_complexity(), 2);
    }

    #[test]
    fn test_data_flow_depth() {
        let mut g = SemanticGraph::new();
        let d = g.add_node(SemanticNodeKind::VarDef { name: "x".into(), mutable: false }, 0, 3, 0);
        let u1 = g.add_node(SemanticNodeKind::VarDef { name: "y".into(), mutable: false }, 4, 7, 0);
        let u2 = g.add_node(SemanticNodeKind::VarDef { name: "z".into(), mutable: false }, 8, 11, 0);
        g.add_edge(d, u1, SemanticEdgeKind::DefUse, 1.0);
        g.add_edge(u1, u2, SemanticEdgeKind::DefUse, 1.0);
        assert_eq!(g.data_flow_depth(), 3);
    }

    // ── DAG Check ────────────────────────────────────────────────────

    #[test]
    fn test_is_acyclic() {
        let mut g = SemanticGraph::new();
        let a = g.add_node(SemanticNodeKind::VarDef { name: "a".into(), mutable: false }, 0, 1, 0);
        let b = g.add_node(SemanticNodeKind::VarUse { name: "a".into() }, 2, 3, 0);
        g.add_edge(a, b, SemanticEdgeKind::DataFlow, 1.0);
        assert!(g.is_acyclic_data_flow());
    }

    #[test]
    fn test_reverse_postorder() {
        let mut g = SemanticGraph::new();
        let a = g.add_node(SemanticNodeKind::Literal { type_name: "A".into() }, 0, 1, 0);
        let b = g.add_node(SemanticNodeKind::Literal { type_name: "B".into() }, 2, 3, 0);
        let c = g.add_node(SemanticNodeKind::Literal { type_name: "C".into() }, 4, 5, 0);
        g.add_edge(a, b, SemanticEdgeKind::ControlFlow, 1.0);
        g.add_edge(b, c, SemanticEdgeKind::ControlFlow, 1.0);
        let rpo = g.reverse_postorder(a);
        assert_eq!(rpo, vec![a, b, c]);
    }

    // ── FFI Smoke Tests ──────────────────────────────────────────────

    #[test]
    fn test_ffi_graph_new() {
        let ptr = vitalis_semantic_graph_new();
        assert_ne!(ptr, 0);
        unsafe { drop(Box::from_raw(ptr as *mut SemanticGraph)); }
    }

    #[test]
    fn test_ffi_cyclomatic() {
        assert_eq!(vitalis_semantic_graph_cyclomatic(4, 5), 3);
        assert_eq!(vitalis_semantic_graph_cyclomatic(2, 1), 1);
    }

    #[test]
    fn test_ffi_similarity_jaccard() {
        let sim = vitalis_semantic_similarity_jaccard(5, 5, 3);
        assert!((sim - 3.0 / 7.0).abs() < 1e-10);
    }

    #[test]
    fn test_ffi_data_flow_depth() {
        let chains = [3i64, 5, 2, 7, 1];
        let d = vitalis_semantic_data_flow_depth(chains.as_ptr(), 5);
        assert_eq!(d, 7);
    }

    #[test]
    fn test_ffi_reaching_def_count() {
        assert_eq!(vitalis_semantic_reaching_def_count(1, 0, 3), 4);
        assert_eq!(vitalis_semantic_reaching_def_count(1, 2, 5), 4);
    }
}
