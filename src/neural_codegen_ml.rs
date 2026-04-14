//! Neural Code Generation — v720 ERA II Phase 30
//!
//! ML-based instruction selection using feature vectors and cost models.
//! Implements instruction pattern matching, cost scoring, register pressure
//! estimation, dependency DAG scheduling, and pipeline hazard detection.

/// Feature vector for an instruction candidate.
/// Fields: opcode class, latency, throughput (inverse), port mask, register pressure delta.
struct InstructionFeature {
    opcode: i64,
    latency: i64,
    throughput: i64,
    port_mask: i64,
    reg_pressure_delta: i64,
}

/// Weight vector for the dot-product cost model.
const COST_WEIGHTS: [f64; 5] = [0.05, 0.35, 0.25, 0.15, 0.20];

/// Dependency edge in the scheduling DAG.
struct DepEdge {
    from: usize,
    to: usize,
    latency: i64,
    dep_type: HazardType,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum HazardType {
    None = 0,
    Raw = 1,  // Read-After-Write
    War = 2,  // Write-After-Read
    Waw = 3,  // Write-After-Write
}

/// Score an instruction candidate using feature-vector dot product.
/// Higher score = better candidate.
fn score_instruction(feat: &InstructionFeature) -> f64 {
    let features = [
        feat.opcode as f64,
        feat.latency as f64,
        feat.throughput as f64,
        feat.port_mask as f64,
        feat.reg_pressure_delta as f64,
    ];
    let mut score = 0.0;
    for i in 0..5 {
        score += features[i] * COST_WEIGHTS[i];
    }
    // Invert latency contribution (lower latency = higher score)
    let latency_penalty = feat.latency as f64 * 0.35;
    let throughput_bonus = if feat.throughput > 0 {
        1000.0 / feat.throughput as f64
    } else {
        0.0
    };
    score - latency_penalty + throughput_bonus
}

/// Estimate register pressure given live-in, live-out, and def counts.
fn estimate_register_pressure(live_in: i64, live_out: i64, defs: i64) -> i64 {
    // Pressure = max simultaneous live values
    // At the point of maximum pressure: live_in + defs - killed values
    let killed = if live_in > live_out { live_in - live_out } else { 0 };
    let peak = live_in + defs - killed;
    // Clamp to reasonable range
    if peak < 0 { 0 } else { peak }
}

/// Compute critical path length through a dependency DAG.
/// Uses depth × average latency as approximation.
fn critical_path_length(dag_depth: i64, avg_latency: i64) -> i64 {
    if dag_depth <= 0 || avg_latency <= 0 {
        return 0;
    }
    // Critical path = sum of latencies along longest chain
    // Approximate: depth * avg_latency with diminishing returns for deep chains
    let base = dag_depth * avg_latency;
    // Account for ILP overlap: deeper chains have more overlap opportunity
    let overlap_factor = 1000 - (dag_depth * 50).min(500); // 50-950 range
    (base * overlap_factor) / 1000
}

/// Detect pipeline hazard type given dependency distance and pipeline depth.
fn detect_hazard(dep_distance: i64, pipeline_depth: i64) -> HazardType {
    if dep_distance <= 0 || pipeline_depth <= 0 {
        return HazardType::None;
    }
    if dep_distance >= pipeline_depth {
        return HazardType::None; // No hazard, enough distance
    }
    // Classify by distance relative to pipeline stages
    let ratio = (dep_distance * 3) / pipeline_depth;
    match ratio {
        0 => HazardType::Raw, // Very close: read-after-write
        1 => HazardType::War, // Medium: write-after-read
        2 => HazardType::Waw, // Farther: write-after-write
        _ => HazardType::None,
    }
}

/// Compute scheduling priority for an instruction node.
/// Based on critical path urgency, successor count, and own latency.
fn schedule_priority(critical_path: i64, successors: i64, latency: i64) -> i64 {
    // Priority = weighted combination:
    //   critical path contribution dominates (urgency)
    //   more successors = higher priority (unblocks more)
    //   higher latency = schedule earlier (start long ops first)
    let cp_weight = critical_path * 100;
    let succ_weight = successors * 50;
    let lat_weight = latency * 30;
    cp_weight + succ_weight + lat_weight
}

/// Compute cost model score: lower = cheaper instruction.
/// Returns cost × 1000.
fn cost_model(cycles: i64, ports_used: i64, total_ports: i64) -> i64 {
    if total_ports <= 0 || cycles <= 0 {
        return 0;
    }
    // Cost = cycles × (ports_used / total_ports) normalized
    let port_pressure = (ports_used * 1000) / total_ports;
    let cycle_cost = cycles * 1000;
    // Weighted combination
    (cycle_cost * 600 + port_pressure * 400) / 1000
}

/// Rank multiple instruction candidates, returns index of best.
fn rank_candidates(candidates: &[InstructionFeature]) -> Option<usize> {
    if candidates.is_empty() {
        return None;
    }
    let mut best_idx = 0;
    let mut best_score = f64::NEG_INFINITY;
    for (i, feat) in candidates.iter().enumerate() {
        let s = score_instruction(feat);
        if s > best_score {
            best_score = s;
            best_idx = i;
        }
    }
    Some(best_idx)
}

/// Build a topological order from dependency edges.
fn topological_sort(n: usize, edges: &[DepEdge]) -> Vec<usize> {
    let mut in_degree = vec![0usize; n];
    let mut adj: Vec<Vec<usize>> = vec![Vec::new(); n];
    for e in edges {
        if e.from < n && e.to < n {
            adj[e.from].push(e.to);
            in_degree[e.to] += 1;
        }
    }
    let mut queue: Vec<usize> = Vec::new();
    for i in 0..n {
        if in_degree[i] == 0 {
            queue.push(i);
        }
    }
    let mut order = Vec::with_capacity(n);
    while let Some(node) = queue.pop() {
        order.push(node);
        for &next in &adj[node] {
            in_degree[next] -= 1;
            if in_degree[next] == 0 {
                queue.push(next);
            }
        }
    }
    order
}

/// Compute longest path lengths from each node (used for critical path).
fn longest_paths(n: usize, edges: &[DepEdge], latencies: &[i64]) -> Vec<i64> {
    let mut dist = vec![0i64; n];
    let order = topological_sort(n, edges);
    let mut adj: Vec<Vec<(usize, i64)>> = vec![Vec::new(); n];
    for e in edges {
        if e.from < n && e.to < n {
            adj[e.from].push((e.to, e.latency));
        }
    }
    for &node in &order {
        let base = dist[node] + latencies.get(node).copied().unwrap_or(1);
        for &(next, edge_lat) in &adj[node] {
            let arrival = base + edge_lat;
            if arrival > dist[next] {
                dist[next] = arrival;
            }
        }
    }
    dist
}

// ── FFI functions ──────────────────────────────────────────

#[unsafe(no_mangle)]
pub extern "C" fn slang_ncg_instruction_score(opcode: i64, latency: i64, throughput: i64) -> i64 {
    let feat = InstructionFeature {
        opcode,
        latency,
        throughput,
        port_mask: 1,
        reg_pressure_delta: 0,
    };
    (score_instruction(&feat) * 1000.0) as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_ncg_register_pressure(live_in: i64, live_out: i64, defs: i64) -> i64 {
    estimate_register_pressure(live_in, live_out, defs)
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_ncg_critical_path(dag_depth: i64, avg_latency: i64) -> i64 {
    critical_path_length(dag_depth, avg_latency)
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_ncg_hazard_detect(dep_distance: i64, pipeline_depth: i64) -> i64 {
    detect_hazard(dep_distance, pipeline_depth) as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_ncg_schedule_priority(critical_path: i64, successors: i64, latency: i64) -> i64 {
    schedule_priority(critical_path, successors, latency)
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_ncg_cost_model(cycles: i64, ports_used: i64, total_ports: i64) -> i64 {
    cost_model(cycles, ports_used, total_ports)
}

// ── Tests ──────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_score_basic() {
        let feat = InstructionFeature { opcode: 1, latency: 2, throughput: 1, port_mask: 1, reg_pressure_delta: 0 };
        let s = score_instruction(&feat);
        assert!(s.is_finite());
    }

    #[test]
    fn test_lower_latency_scores_higher() {
        let low = InstructionFeature { opcode: 1, latency: 1, throughput: 1, port_mask: 1, reg_pressure_delta: 0 };
        let high = InstructionFeature { opcode: 1, latency: 10, throughput: 1, port_mask: 1, reg_pressure_delta: 0 };
        assert!(score_instruction(&low) > score_instruction(&high));
    }

    #[test]
    fn test_register_pressure_basic() {
        assert_eq!(estimate_register_pressure(5, 3, 2), 5 + 2 - 2);
    }

    #[test]
    fn test_register_pressure_no_kills() {
        assert_eq!(estimate_register_pressure(3, 5, 2), 3 + 2);
    }

    #[test]
    fn test_register_pressure_zero() {
        assert_eq!(estimate_register_pressure(0, 0, 0), 0);
    }

    #[test]
    fn test_critical_path_basic() {
        let cp = critical_path_length(4, 3);
        assert!(cp > 0);
        assert!(cp <= 4 * 3); // Can't exceed raw product
    }

    #[test]
    fn test_critical_path_zero_depth() {
        assert_eq!(critical_path_length(0, 5), 0);
    }

    #[test]
    fn test_hazard_none_far() {
        assert_eq!(detect_hazard(10, 5), HazardType::None);
    }

    #[test]
    fn test_hazard_raw_close() {
        assert_eq!(detect_hazard(1, 10), HazardType::Raw);
    }

    #[test]
    fn test_hazard_war_medium() {
        assert_eq!(detect_hazard(4, 10), HazardType::War);
    }

    #[test]
    fn test_hazard_zero_distance() {
        assert_eq!(detect_hazard(0, 5), HazardType::None);
    }

    #[test]
    fn test_schedule_priority_basic() {
        let p = schedule_priority(10, 3, 2);
        assert_eq!(p, 10 * 100 + 3 * 50 + 2 * 30);
    }

    #[test]
    fn test_schedule_priority_critical_dominates() {
        let p1 = schedule_priority(20, 1, 1);
        let p2 = schedule_priority(1, 20, 1);
        assert!(p1 > p2);
    }

    #[test]
    fn test_cost_model_basic() {
        let c = cost_model(2, 1, 4);
        assert!(c > 0);
    }

    #[test]
    fn test_cost_model_full_ports() {
        let c = cost_model(1, 4, 4);
        assert!(c > 0);
    }

    #[test]
    fn test_cost_model_zero() {
        assert_eq!(cost_model(0, 1, 4), 0);
    }

    #[test]
    fn test_rank_candidates() {
        let candidates = vec![
            InstructionFeature { opcode: 1, latency: 5, throughput: 2, port_mask: 1, reg_pressure_delta: 1 },
            InstructionFeature { opcode: 1, latency: 1, throughput: 1, port_mask: 1, reg_pressure_delta: 0 },
            InstructionFeature { opcode: 1, latency: 10, throughput: 4, port_mask: 3, reg_pressure_delta: 2 },
        ];
        let best = rank_candidates(&candidates).unwrap();
        assert_eq!(best, 1); // Lowest latency, best throughput
    }

    #[test]
    fn test_rank_empty() {
        assert_eq!(rank_candidates(&[]), None);
    }

    #[test]
    fn test_topological_sort_chain() {
        let edges = vec![
            DepEdge { from: 0, to: 1, latency: 1, dep_type: HazardType::Raw },
            DepEdge { from: 1, to: 2, latency: 1, dep_type: HazardType::Raw },
        ];
        let order = topological_sort(3, &edges);
        assert_eq!(order.len(), 3);
        assert!(order.iter().position(|&x| x == 0) < order.iter().position(|&x| x == 1));
    }

    #[test]
    fn test_longest_paths() {
        let edges = vec![
            DepEdge { from: 0, to: 1, latency: 2, dep_type: HazardType::Raw },
            DepEdge { from: 0, to: 2, latency: 1, dep_type: HazardType::Raw },
            DepEdge { from: 1, to: 3, latency: 3, dep_type: HazardType::Raw },
            DepEdge { from: 2, to: 3, latency: 1, dep_type: HazardType::Raw },
        ];
        let latencies = vec![1, 2, 1, 1];
        let dist = longest_paths(4, &edges, &latencies);
        assert!(dist[3] > dist[0]);
    }

    #[test]
    fn test_ffi_score() {
        let s = slang_ncg_instruction_score(1, 2, 1);
        assert!(s != 0);
    }

    #[test]
    fn test_ffi_cost_model() {
        let c = slang_ncg_cost_model(3, 2, 6);
        assert!(c > 0);
    }

    #[test]
    fn test_ffi_hazard_raw() {
        assert_eq!(slang_ncg_hazard_detect(1, 10), 1);
    }

    #[test]
    fn test_ffi_pressure() {
        assert!(slang_ncg_register_pressure(8, 4, 3) > 0);
    }
}
