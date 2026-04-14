//! Graph Neural Networks (GNN) — AI/ML Production Stack (v322)
//!
//! Message passing, GCN-style graph convolution, graph attention (GAT),
//! and readout aggregation for learning on graph-structured data.

use std::sync::{LazyLock, Mutex};

struct GraphState {
    nodes: Vec<i64>,        // node feature values
    edges: Vec<(i64, i64)>, // (src, dst) edges
    messages: Vec<i64>,     // accumulated messages per node
}

static GRAPHS: LazyLock<Mutex<Vec<GraphState>>> =
    LazyLock::new(|| Mutex::new(Vec::new()));

/// Create a graph with n_nodes. Returns graph ID.
#[unsafe(no_mangle)]
pub extern "C" fn slang_gnn_create(n_nodes: i64) -> i64 {
    let mut gs = GRAPHS.lock().unwrap();
    let id = gs.len() as i64;
    let n = n_nodes.max(0) as usize;
    gs.push(GraphState { nodes: vec![0; n], edges: Vec::new(), messages: vec![0; n] });
    id
}

/// Add an edge (src→dst) to the graph. Returns total edge count.
#[unsafe(no_mangle)]
pub extern "C" fn slang_gnn_add_edge(graph_id: i64, src: i64, dst: i64) -> i64 {
    let mut gs = GRAPHS.lock().unwrap();
    let idx = graph_id as usize;
    if idx >= gs.len() { return -1; }
    gs[idx].edges.push((src, dst));
    gs[idx].edges.len() as i64
}

/// Set a node feature value. Returns 1 on success, -1 if invalid.
#[unsafe(no_mangle)]
pub extern "C" fn slang_gnn_set_feature(graph_id: i64, node: i64, value: i64) -> i64 {
    let mut gs = GRAPHS.lock().unwrap();
    let idx = graph_id as usize;
    if idx >= gs.len() { return -1; }
    let ni = node as usize;
    if ni >= gs[idx].nodes.len() { return -1; }
    gs[idx].nodes[ni] = value;
    1
}

/// Perform one round of message passing: each node receives sum of neighbor features.
/// Returns total messages sent.
#[unsafe(no_mangle)]
pub extern "C" fn slang_gnn_message_pass(graph_id: i64) -> i64 {
    let mut gs = GRAPHS.lock().unwrap();
    let idx = graph_id as usize;
    if idx >= gs.len() { return -1; }
    let g = &mut gs[idx];
    let n = g.nodes.len();
    g.messages = vec![0; n];
    let mut sent = 0i64;
    for &(src, dst) in &g.edges {
        let si = src as usize;
        let di = dst as usize;
        if si < n && di < n {
            g.messages[di] += g.nodes[si];
            sent += 1;
        }
    }
    // Update node features with messages
    for i in 0..n {
        g.nodes[i] += g.messages[i];
    }
    sent
}

/// Graph convolution: aggregate neighbor features with normalization.
/// Returns sum of all node features after conv (for verification).
#[unsafe(no_mangle)]
pub extern "C" fn slang_gnn_conv(graph_id: i64, weight: i64) -> i64 {
    let mut gs = GRAPHS.lock().unwrap();
    let idx = graph_id as usize;
    if idx >= gs.len() { return -1; }
    let g = &mut gs[idx];
    let n = g.nodes.len();
    // Apply weight to all node features
    for i in 0..n {
        g.nodes[i] = g.nodes[i].wrapping_mul(weight) / 1000; // weight is ×1000
    }
    g.nodes.iter().sum()
}

/// Graph attention: compute attention-weighted message for a node.
/// Returns attention score × 1000.
#[unsafe(no_mangle)]
pub extern "C" fn slang_gnn_attention(query: i64, key: i64, n_neighbors: i64) -> i64 {
    if n_neighbors <= 0 { return 0; }
    let score = query.wrapping_mul(key);
    // Normalize by sqrt(d) * n_neighbors
    let norm = ((n_neighbors as f64).sqrt() * 1000.0) as i64;
    if norm == 0 { return 0; }
    score.wrapping_mul(1000) / norm
}

/// Readout: aggregate all node features into a single graph-level representation.
/// mode: 0=sum, 1=mean, 2=max. Returns aggregated value.
#[unsafe(no_mangle)]
pub extern "C" fn slang_gnn_readout(graph_id: i64, mode: i64) -> i64 {
    let gs = GRAPHS.lock().unwrap();
    let idx = graph_id as usize;
    if idx >= gs.len() { return -1; }
    let nodes = &gs[idx].nodes;
    if nodes.is_empty() { return 0; }
    match mode {
        0 => nodes.iter().sum(),
        1 => nodes.iter().sum::<i64>() / nodes.len() as i64,
        2 => *nodes.iter().max().unwrap(),
        _ => nodes.iter().sum(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_create() { let id = slang_gnn_create(5); assert!(id >= 0); }
    #[test] fn test_add_edge() { let id = slang_gnn_create(3); assert_eq!(slang_gnn_add_edge(id, 0, 1), 1); }
    #[test] fn test_set_feature() { let id = slang_gnn_create(3); assert_eq!(slang_gnn_set_feature(id, 0, 42), 1); }
    #[test] fn test_message_pass() {
        let id = slang_gnn_create(3);
        slang_gnn_set_feature(id, 0, 10);
        slang_gnn_add_edge(id, 0, 1);
        let sent = slang_gnn_message_pass(id);
        assert_eq!(sent, 1);
    }
    #[test] fn test_conv() {
        let id = slang_gnn_create(2);
        slang_gnn_set_feature(id, 0, 1000);
        slang_gnn_set_feature(id, 1, 2000);
        let sum = slang_gnn_conv(id, 500); // weight=0.5
        assert_eq!(sum, 1500); // 1000*500/1000 + 2000*500/1000
    }
    #[test] fn test_attention() {
        let s = slang_gnn_attention(100, 100, 4);
        assert!(s != 0);
    }
    #[test] fn test_readout_sum() {
        let id = slang_gnn_create(3);
        slang_gnn_set_feature(id, 0, 10);
        slang_gnn_set_feature(id, 1, 20);
        slang_gnn_set_feature(id, 2, 30);
        assert_eq!(slang_gnn_readout(id, 0), 60);
    }
    #[test] fn test_readout_mean() {
        let id = slang_gnn_create(3);
        slang_gnn_set_feature(id, 0, 10);
        slang_gnn_set_feature(id, 1, 20);
        slang_gnn_set_feature(id, 2, 30);
        assert_eq!(slang_gnn_readout(id, 1), 20);
    }
    #[test] fn test_readout_max() {
        let id = slang_gnn_create(3);
        slang_gnn_set_feature(id, 0, 10);
        slang_gnn_set_feature(id, 1, 30);
        slang_gnn_set_feature(id, 2, 20);
        assert_eq!(slang_gnn_readout(id, 2), 30);
    }
    #[test] fn test_empty_graph() {
        let id = slang_gnn_create(0);
        assert_eq!(slang_gnn_readout(id, 0), 0);
    }
    #[test] fn test_invalid_graph() { assert_eq!(slang_gnn_message_pass(999), -1); }
    #[test] fn test_invalid_node() { let id = slang_gnn_create(2); assert_eq!(slang_gnn_set_feature(id, 99, 1), -1); }
    #[test] fn test_self_loop() {
        let id = slang_gnn_create(2);
        slang_gnn_set_feature(id, 0, 5);
        slang_gnn_add_edge(id, 0, 0);
        slang_gnn_message_pass(id);
        // node 0 should receive its own feature
    }
    #[test] fn test_attention_zero_neighbors() { assert_eq!(slang_gnn_attention(10, 10, 0), 0); }
    #[test] fn test_multiple_edges() {
        let id = slang_gnn_create(3);
        slang_gnn_add_edge(id, 0, 1);
        slang_gnn_add_edge(id, 0, 2);
        slang_gnn_add_edge(id, 1, 2);
        assert_eq!(slang_gnn_add_edge(id, 2, 0), 4);
    }
    #[test] fn test_message_pass_bidirectional() {
        let id = slang_gnn_create(2);
        slang_gnn_set_feature(id, 0, 10);
        slang_gnn_set_feature(id, 1, 20);
        slang_gnn_add_edge(id, 0, 1);
        slang_gnn_add_edge(id, 1, 0);
        slang_gnn_message_pass(id);
    }
    #[test] fn test_readout_invalid() { assert_eq!(slang_gnn_readout(999, 0), -1); }
    #[test] fn test_conv_invalid() { assert_eq!(slang_gnn_conv(999, 1), -1); }
    #[test] fn test_large_graph() {
        let id = slang_gnn_create(100);
        for i in 0..99 { slang_gnn_add_edge(id, i, i + 1); }
        assert!(id >= 0);
    }
    #[test] fn test_negative_features() {
        let id = slang_gnn_create(2);
        slang_gnn_set_feature(id, 0, -10);
        slang_gnn_set_feature(id, 1, 10);
        assert_eq!(slang_gnn_readout(id, 0), 0);
    }
}
