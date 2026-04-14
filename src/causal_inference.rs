//! Causal Inference Engine — Vitalis v609
//!
//! Determines causality between program events and state changes:
//! - Causal graph construction from data/control dependencies
//! - Intervention analysis (do-calculus inspired)
//! - Counterfactual reasoning ("what if X didn't happen?")
//! - Root cause ranking via causal influence scores
//! - Causal chain extraction for debugging

use std::collections::{HashMap, HashSet, VecDeque};

// ── Causal Graph ─────────────────────────────────────────────────────

/// A node in the causal graph.
#[derive(Debug, Clone)]
pub struct CausalNode {
    pub id: u32,
    pub name: String,
    pub kind: CausalNodeKind,
    pub observed_value: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CausalNodeKind {
    Variable,
    Event,
    Condition,
    SideEffect,
    Error,
}

/// A directed causal edge.
#[derive(Debug, Clone)]
pub struct CausalEdge {
    pub from: u32,
    pub to: u32,
    pub strength: f64,    // 0.0-1.0 causal strength
    pub kind: CausalEdgeKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CausalEdgeKind {
    DataDependency,
    ControlDependency,
    EffectDependency,
    Correlation,
}

/// Directed acyclic causal graph.
pub struct CausalGraph {
    nodes: Vec<CausalNode>,
    edges: Vec<CausalEdge>,
    adj: HashMap<u32, Vec<usize>>,      // node → edge indices
    rev_adj: HashMap<u32, Vec<usize>>,   // node → incoming edge indices
}

impl CausalGraph {
    pub fn new() -> Self {
        Self { nodes: Vec::new(), edges: Vec::new(), adj: HashMap::new(), rev_adj: HashMap::new() }
    }

    pub fn add_node(&mut self, name: &str, kind: CausalNodeKind) -> u32 {
        let id = self.nodes.len() as u32;
        self.nodes.push(CausalNode { id, name: name.to_string(), kind, observed_value: None });
        id
    }

    pub fn set_observed(&mut self, id: u32, value: f64) {
        if let Some(node) = self.nodes.get_mut(id as usize) {
            node.observed_value = Some(value);
        }
    }

    pub fn add_edge(&mut self, from: u32, to: u32, strength: f64, kind: CausalEdgeKind) {
        let idx = self.edges.len();
        self.edges.push(CausalEdge { from, to, strength, kind });
        self.adj.entry(from).or_default().push(idx);
        self.rev_adj.entry(to).or_default().push(idx);
    }

    pub fn node_count(&self) -> usize { self.nodes.len() }
    pub fn edge_count(&self) -> usize { self.edges.len() }

    pub fn get_node(&self, id: u32) -> Option<&CausalNode> {
        self.nodes.get(id as usize)
    }

    /// Get direct causes of a node.
    pub fn causes_of(&self, id: u32) -> Vec<u32> {
        self.rev_adj.get(&id)
            .map(|indices| indices.iter().map(|&i| self.edges[i].from).collect())
            .unwrap_or_default()
    }

    /// Get direct effects of a node.
    pub fn effects_of(&self, id: u32) -> Vec<u32> {
        self.adj.get(&id)
            .map(|indices| indices.iter().map(|&i| self.edges[i].to).collect())
            .unwrap_or_default()
    }

    /// Compute causal influence score: how much does `source` influence `target`?
    /// Uses path-product along all causal chains.
    pub fn causal_influence(&self, source: u32, target: u32) -> f64 {
        if source == target { return 1.0; }

        // BFS finding all paths (bounded to avoid exponential blowup)
        let mut max_influence = 0.0f64;
        let mut queue: VecDeque<(u32, f64)> = VecDeque::new();
        queue.push_back((source, 1.0));
        let mut visited = HashSet::new();
        visited.insert(source);

        while let Some((current, path_strength)) = queue.pop_front() {
            if let Some(edge_indices) = self.adj.get(&current) {
                for &ei in edge_indices {
                    let edge = &self.edges[ei];
                    let new_strength = path_strength * edge.strength;
                    if edge.to == target {
                        max_influence = max_influence.max(new_strength);
                    } else if visited.insert(edge.to) && new_strength > 0.01 {
                        queue.push_back((edge.to, new_strength));
                    }
                }
            }
        }

        max_influence
    }

    /// Find all causal ancestors of a node (transitive causes).
    pub fn causal_ancestors(&self, id: u32) -> HashSet<u32> {
        let mut ancestors = HashSet::new();
        let mut queue = VecDeque::new();
        queue.push_back(id);
        while let Some(current) = queue.pop_front() {
            for cause in self.causes_of(current) {
                if ancestors.insert(cause) {
                    queue.push_back(cause);
                }
            }
        }
        ancestors
    }

    /// Find causal chain from source to target (BFS shortest).
    pub fn causal_chain(&self, source: u32, target: u32) -> Option<Vec<u32>> {
        if source == target { return Some(vec![source]); }

        let mut parent: HashMap<u32, u32> = HashMap::new();
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        queue.push_back(source);
        visited.insert(source);

        while let Some(current) = queue.pop_front() {
            for effect in self.effects_of(current) {
                if !visited.insert(effect) { continue; }
                parent.insert(effect, current);
                if effect == target {
                    // Reconstruct path
                    let mut path = vec![target];
                    let mut node = target;
                    while let Some(&p) = parent.get(&node) {
                        path.push(p);
                        node = p;
                    }
                    path.reverse();
                    return Some(path);
                }
                queue.push_back(effect);
            }
        }
        None
    }

    /// Rank nodes by causal influence on a target (root cause ranking).
    pub fn rank_root_causes(&self, target: u32) -> Vec<(u32, f64)> {
        let ancestors = self.causal_ancestors(target);
        let mut ranked: Vec<(u32, f64)> = ancestors.iter()
            .map(|&a| (a, self.causal_influence(a, target)))
            .filter(|(_, inf)| *inf > 0.0)
            .collect();
        ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        ranked
    }
}

impl Default for CausalGraph {
    fn default() -> Self { Self::new() }
}

// ── Intervention Analysis ────────────────────────────────────────────

/// Simulate an intervention: remove all incoming edges to `node` and set its value.
pub fn intervene(graph: &CausalGraph, node: u32, _value: f64) -> Vec<u32> {
    // Return the set of nodes that would be affected downstream
    let mut affected = HashSet::new();
    let mut queue = VecDeque::new();
    queue.push_back(node);
    while let Some(current) = queue.pop_front() {
        for effect in graph.effects_of(current) {
            if affected.insert(effect) {
                queue.push_back(effect);
            }
        }
    }
    let mut result: Vec<u32> = affected.into_iter().collect();
    result.sort();
    result
}

/// Counterfactual: "what if `node` hadn't occurred?"
/// Returns the set of nodes that would be exclusively caused by `node`.
pub fn counterfactual_removal(graph: &CausalGraph, node: u32) -> Vec<u32> {
    // Nodes exclusively reachable through `node`
    let downstream = intervene(graph, node, 0.0);

    // For each downstream node, check if it has alternative causal paths
    let mut exclusively_caused = Vec::new();
    for &d in &downstream {
        let causes = graph.causes_of(d);
        let only_through_node = causes.iter().all(|&c| c == node || downstream.contains(&c));
        if only_through_node {
            exclusively_caused.push(d);
        }
    }
    exclusively_caused
}

// ── FFI Exports ──────────────────────────────────────────────────────

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_causal_influence(
    edges_from: *const i64, edges_to: *const i64, strengths: *const f64,
    edge_count: i64, source: i64, target: i64,
) -> f64 {
    if edges_from.is_null() || edges_to.is_null() || strengths.is_null() || edge_count <= 0 {
        return 0.0;
    }
    let from = unsafe { std::slice::from_raw_parts(edges_from, edge_count as usize) };
    let to = unsafe { std::slice::from_raw_parts(edges_to, edge_count as usize) };
    let str_s = unsafe { std::slice::from_raw_parts(strengths, edge_count as usize) };

    let mut graph = CausalGraph::new();
    let max_node = from.iter().chain(to.iter()).copied().max().unwrap_or(0) as u32;
    for i in 0..=max_node { graph.add_node(&format!("n{}", i), CausalNodeKind::Variable); }
    for i in 0..edge_count as usize {
        graph.add_edge(from[i] as u32, to[i] as u32, str_s[i], CausalEdgeKind::DataDependency);
    }
    graph.causal_influence(source as u32, target as u32)
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_causal_chain_length(
    edges_from: *const i64, edges_to: *const i64, edge_count: i64,
    source: i64, target: i64,
) -> i64 {
    if edges_from.is_null() || edges_to.is_null() || edge_count <= 0 { return -1; }
    let from = unsafe { std::slice::from_raw_parts(edges_from, edge_count as usize) };
    let to = unsafe { std::slice::from_raw_parts(edges_to, edge_count as usize) };

    let mut graph = CausalGraph::new();
    let max_node = from.iter().chain(to.iter()).copied().max().unwrap_or(0) as u32;
    for i in 0..=max_node { graph.add_node(&format!("n{}", i), CausalNodeKind::Variable); }
    for i in 0..edge_count as usize {
        graph.add_edge(from[i] as u32, to[i] as u32, 1.0, CausalEdgeKind::DataDependency);
    }
    graph.causal_chain(source as u32, target as u32)
        .map(|chain| chain.len() as i64)
        .unwrap_or(-1)
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_causal_ancestor_count(
    edges_from: *const i64, edges_to: *const i64, edge_count: i64, node: i64,
) -> i64 {
    if edges_from.is_null() || edges_to.is_null() || edge_count <= 0 { return 0; }
    let from = unsafe { std::slice::from_raw_parts(edges_from, edge_count as usize) };
    let to = unsafe { std::slice::from_raw_parts(edges_to, edge_count as usize) };

    let mut graph = CausalGraph::new();
    let max_node = from.iter().chain(to.iter()).copied().max().unwrap_or(0) as u32;
    for i in 0..=max_node { graph.add_node(&format!("n{}", i), CausalNodeKind::Variable); }
    for i in 0..edge_count as usize {
        graph.add_edge(from[i] as u32, to[i] as u32, 1.0, CausalEdgeKind::DataDependency);
    }
    graph.causal_ancestors(node as u32).len() as i64
}

// ═══════════════════════════════════════════════════════════════════════
// Tests
// ═══════════════════════════════════════════════════════════════════════
#[cfg(test)]
mod tests {
    use super::*;

    fn build_test_graph() -> CausalGraph {
        let mut g = CausalGraph::new();
        let a = g.add_node("input", CausalNodeKind::Variable);
        let b = g.add_node("process", CausalNodeKind::Event);
        let c = g.add_node("validate", CausalNodeKind::Condition);
        let d = g.add_node("output", CausalNodeKind::Variable);
        let e = g.add_node("error", CausalNodeKind::Error);
        g.add_edge(a, b, 0.9, CausalEdgeKind::DataDependency);
        g.add_edge(b, c, 0.8, CausalEdgeKind::ControlDependency);
        g.add_edge(c, d, 0.95, CausalEdgeKind::DataDependency);
        g.add_edge(c, e, 0.3, CausalEdgeKind::ControlDependency);
        g
    }

    #[test]
    fn test_graph_construction() {
        let g = build_test_graph();
        assert_eq!(g.node_count(), 5);
        assert_eq!(g.edge_count(), 4);
    }

    #[test]
    fn test_causes_of() {
        let g = build_test_graph();
        let causes = g.causes_of(3); // output
        assert_eq!(causes, vec![2]); // validate
    }

    #[test]
    fn test_effects_of() {
        let g = build_test_graph();
        let effects = g.effects_of(2); // validate
        assert!(effects.contains(&3)); // output
        assert!(effects.contains(&4)); // error
    }

    #[test]
    fn test_causal_influence_direct() {
        let g = build_test_graph();
        let inf = g.causal_influence(0, 1); // input → process
        assert!((inf - 0.9).abs() < 1e-10);
    }

    #[test]
    fn test_causal_influence_transitive() {
        let g = build_test_graph();
        let inf = g.causal_influence(0, 3); // input → ... → output
        assert!(inf > 0.5); // 0.9 * 0.8 * 0.95 = 0.684
    }

    #[test]
    fn test_causal_influence_none() {
        let g = build_test_graph();
        let inf = g.causal_influence(3, 0); // output → input (no path)
        assert_eq!(inf, 0.0);
    }

    #[test]
    fn test_causal_influence_self() {
        let g = build_test_graph();
        assert_eq!(g.causal_influence(0, 0), 1.0);
    }

    #[test]
    fn test_causal_ancestors() {
        let g = build_test_graph();
        let ancestors = g.causal_ancestors(3); // output
        assert!(ancestors.contains(&0)); // input
        assert!(ancestors.contains(&1)); // process
        assert!(ancestors.contains(&2)); // validate
        assert!(!ancestors.contains(&4)); // not error
    }

    #[test]
    fn test_causal_chain() {
        let g = build_test_graph();
        let chain = g.causal_chain(0, 3).unwrap();
        assert_eq!(chain[0], 0); // starts at input
        assert_eq!(*chain.last().unwrap(), 3); // ends at output
        assert!(chain.len() >= 3);
    }

    #[test]
    fn test_causal_chain_no_path() {
        let g = build_test_graph();
        assert!(g.causal_chain(3, 0).is_none());
    }

    #[test]
    fn test_rank_root_causes() {
        let g = build_test_graph();
        let ranked = g.rank_root_causes(3);
        assert!(!ranked.is_empty());
        // validate (direct cause) should rank high
        assert!(ranked.iter().any(|(id, _)| *id == 2));
    }

    #[test]
    fn test_intervene() {
        let g = build_test_graph();
        let affected = intervene(&g, 1, 0.0); // intervene on process
        assert!(affected.contains(&2)); // validate
        assert!(affected.contains(&3)); // output
    }

    #[test]
    fn test_counterfactual_removal() {
        let mut g = CausalGraph::new();
        let a = g.add_node("a", CausalNodeKind::Variable);
        let b = g.add_node("b", CausalNodeKind::Event);
        let c = g.add_node("c", CausalNodeKind::Variable);
        g.add_edge(a, b, 1.0, CausalEdgeKind::DataDependency);
        g.add_edge(b, c, 1.0, CausalEdgeKind::DataDependency);
        let removed = counterfactual_removal(&g, b);
        assert!(removed.contains(&c));
    }

    #[test]
    fn test_set_observed() {
        let mut g = CausalGraph::new();
        let id = g.add_node("x", CausalNodeKind::Variable);
        g.set_observed(id, 42.0);
        assert_eq!(g.get_node(id).unwrap().observed_value, Some(42.0));
    }

    #[test]
    fn test_node_kind_eq() {
        assert_eq!(CausalNodeKind::Variable, CausalNodeKind::Variable);
        assert_ne!(CausalNodeKind::Variable, CausalNodeKind::Event);
    }

    #[test]
    fn test_edge_kind_eq() {
        assert_eq!(CausalEdgeKind::DataDependency, CausalEdgeKind::DataDependency);
        assert_ne!(CausalEdgeKind::DataDependency, CausalEdgeKind::Correlation);
    }

    #[test]
    fn test_ffi_causal_influence() {
        let from = [0i64, 1, 2];
        let to = [1i64, 2, 3];
        let strengths = [0.9, 0.8, 0.7];
        let inf = vitalis_causal_influence(from.as_ptr(), to.as_ptr(), strengths.as_ptr(), 3, 0, 3);
        assert!(inf > 0.4); // 0.9 * 0.8 * 0.7 = 0.504
    }

    #[test]
    fn test_ffi_chain_length() {
        let from = [0i64, 1, 2];
        let to = [1i64, 2, 3];
        let len = vitalis_causal_chain_length(from.as_ptr(), to.as_ptr(), 3, 0, 3);
        assert_eq!(len, 4); // [0,1,2,3]
    }

    #[test]
    fn test_ffi_ancestor_count() {
        let from = [0i64, 1, 2];
        let to = [1i64, 2, 3];
        let count = vitalis_causal_ancestor_count(from.as_ptr(), to.as_ptr(), 3, 3);
        assert_eq!(count, 3); // 0, 1, 2
    }
}
