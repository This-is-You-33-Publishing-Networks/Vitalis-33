//! Neural Instruction Scheduling — v725 ERA II Phase 30
//!
//! Attention-based instruction scheduling with pipeline modeling,
//! stall prediction, port balancing, and superscalar issue optimization.

/// Pipeline stages in a classical 5-stage pipeline.
#[derive(Debug, Clone, Copy, PartialEq)]
enum PipelineStage {
    Fetch = 0,
    Decode = 1,
    Execute = 2,
    Memory = 3,
    Writeback = 4,
}

/// Instruction window entry for out-of-order scheduling.
struct WindowEntry {
    id: usize,
    latency: i64,
    deps_remaining: i64,
    total_deps: i64,
    port: i64,
    priority: f64,
}

/// Compute scaled dot-product attention score between two instructions.
/// Uses latency as the feature dimension. Returns score × 1000.
fn attention_score(query_latency: i64, key_latency: i64, dim: i64) -> f64 {
    if dim <= 0 {
        return 0.0;
    }
    // Scaled dot-product: (Q · K) / sqrt(d)
    let dot = query_latency as f64 * key_latency as f64;
    let scale = (dim as f64).sqrt();
    dot / scale
}

/// Softmax-like normalization for a pair of scores.
fn softmax_pair(a: f64, b: f64) -> (f64, f64) {
    let max = if a > b { a } else { b };
    let ea = (a - max).exp();
    let eb = (b - max).exp();
    let sum = ea + eb;
    (ea / sum, eb / sum)
}

/// Determine pipeline stage from cycle count and number of stages.
fn pipeline_stage(cycle: i64, stages: i64) -> i64 {
    if stages <= 0 {
        return 0;
    }
    cycle % stages
}

/// Predict stall cycles from dependency chain length and issue width.
/// Longer chains with narrow issue width cause more stalls.
fn predict_stalls(dep_chain_len: i64, issue_width: i64) -> i64 {
    if issue_width <= 0 || dep_chain_len <= 0 {
        return 0;
    }
    // Stalls occur when dep chain exceeds overlap capacity
    // With wider issue, we can overlap more independent work
    let overlap_capacity = issue_width; // Can overlap this many ops per cycle
    let serialized_portion = dep_chain_len;
    let parallel_savings = (dep_chain_len / overlap_capacity).max(1);
    let stalls = serialized_portion - parallel_savings;
    stalls.max(0)
}

/// Compute port balance score. Perfectly balanced = 1000, all on one port = low.
fn port_balance(ports_used: i64, total_ports: i64) -> i64 {
    if total_ports <= 0 {
        return 0;
    }
    if ports_used <= 0 {
        return 1000; // No instructions, perfectly balanced
    }
    // Ideal distribution: each port gets equal share
    // Score decreases with port concentration
    let utilization = (ports_used * 1000) / total_ports;
    // Balance = how evenly distributed. If ports_used == total_ports, perfect
    let balance = (ports_used * 1000) / total_ports;
    balance.min(1000)
}

/// Check if an instruction is ready to issue.
fn is_ready(deps_satisfied: i64, total_deps: i64) -> bool {
    if total_deps <= 0 {
        return true; // No dependencies
    }
    deps_satisfied >= total_deps
}

/// Compute instructions per cycle (IPC) × 1000.
fn throughput_ipc(instructions: i64, cycles: i64) -> i64 {
    if cycles <= 0 {
        return 0;
    }
    (instructions * 1000) / cycles
}

/// Multi-instruction attention scoring for a window of ready instructions.
/// Returns priority scores for each instruction.
fn attention_window_scores(entries: &[WindowEntry], dim: i64) -> Vec<f64> {
    let n = entries.len();
    if n == 0 || dim <= 0 {
        return vec![];
    }
    let scale = (dim as f64).sqrt();
    let mut scores = vec![0.0; n];

    // Each instruction attends to all others
    for i in 0..n {
        let mut total_attn = 0.0;
        for j in 0..n {
            if i != j {
                let dot = entries[i].latency as f64 * entries[j].latency as f64;
                let attn = (dot / scale).exp();
                total_attn += attn;
            }
        }
        // Priority combines attention weight with urgency (dependency count)
        let dep_urgency = if entries[i].total_deps > 0 {
            1.0 + (entries[i].total_deps as f64 * 0.1)
        } else {
            1.0
        };
        scores[i] = total_attn * dep_urgency;
    }

    // Normalize
    let max_score = scores.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    if max_score > 0.0 {
        for s in &mut scores {
            *s /= max_score;
        }
    }
    scores
}

/// Simulate superscalar issue: select up to `width` instructions from ready set.
fn superscalar_select(entries: &mut [WindowEntry], width: usize) -> Vec<usize> {
    let mut ready: Vec<usize> = entries
        .iter()
        .enumerate()
        .filter(|(_, e)| e.deps_remaining <= 0)
        .map(|(i, _)| i)
        .collect();

    // Sort by priority descending
    ready.sort_by(|&a, &b| {
        entries[b]
            .priority
            .partial_cmp(&entries[a].priority)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    ready.truncate(width);
    ready
}

/// Compute schedule slot accounting for port conflicts.
fn schedule_slot(cycle: i64, port: i64, port_busy_until: &[i64]) -> i64 {
    if port < 0 || port as usize >= port_busy_until.len() {
        return cycle;
    }
    let ready_at = port_busy_until[port as usize];
    if cycle >= ready_at {
        cycle
    } else {
        ready_at
    }
}

// ── FFI functions ──────────────────────────────────────────

#[unsafe(no_mangle)]
pub extern "C" fn slang_nsched_attention_score(query_latency: i64, key_latency: i64, dim: i64) -> i64 {
    (attention_score(query_latency, key_latency, dim) * 1000.0) as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_nsched_pipeline_stage(cycle: i64, stages: i64) -> i64 {
    pipeline_stage(cycle, stages)
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_nsched_stall_predict(dep_chain_len: i64, issue_width: i64) -> i64 {
    predict_stalls(dep_chain_len, issue_width)
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_nsched_port_balance(ports_used: i64, total_ports: i64) -> i64 {
    port_balance(ports_used, total_ports)
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_nsched_issue_ready(deps_satisfied: i64, total_deps: i64) -> i64 {
    if is_ready(deps_satisfied, total_deps) { 1 } else { 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_nsched_throughput(instructions: i64, cycles: i64) -> i64 {
    throughput_ipc(instructions, cycles)
}

// ── Tests ──────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_attention_score_basic() {
        let s = attention_score(4, 3, 16);
        assert!(s > 0.0);
    }

    #[test]
    fn test_attention_score_scaled() {
        let s1 = attention_score(4, 3, 4);
        let s2 = attention_score(4, 3, 64);
        assert!(s1 > s2); // Larger dim = more scaling = lower score
    }

    #[test]
    fn test_attention_zero_dim() {
        assert_eq!(attention_score(4, 3, 0), 0.0);
    }

    #[test]
    fn test_softmax_pair_equal() {
        let (a, b) = softmax_pair(1.0, 1.0);
        assert!((a - 0.5).abs() < 1e-6);
        assert!((b - 0.5).abs() < 1e-6);
    }

    #[test]
    fn test_softmax_pair_skewed() {
        let (a, b) = softmax_pair(10.0, 0.0);
        assert!(a > 0.9);
        assert!(b < 0.1);
    }

    #[test]
    fn test_pipeline_stage_wrap() {
        assert_eq!(pipeline_stage(0, 5), 0);
        assert_eq!(pipeline_stage(3, 5), 3);
        assert_eq!(pipeline_stage(7, 5), 2);
    }

    #[test]
    fn test_pipeline_stage_zero() {
        assert_eq!(pipeline_stage(5, 0), 0);
    }

    #[test]
    fn test_stall_predict_no_deps() {
        assert_eq!(predict_stalls(0, 4), 0);
    }

    #[test]
    fn test_stall_predict_serial() {
        let stalls = predict_stalls(8, 1);
        assert!(stalls > 0);
    }

    #[test]
    fn test_stall_predict_wide_issue() {
        let narrow = predict_stalls(10, 1);
        let wide = predict_stalls(10, 4);
        assert!(narrow >= wide);
    }

    #[test]
    fn test_port_balance_full() {
        assert_eq!(port_balance(4, 4), 1000);
    }

    #[test]
    fn test_port_balance_half() {
        assert_eq!(port_balance(2, 4), 500);
    }

    #[test]
    fn test_is_ready_all_satisfied() {
        assert!(is_ready(3, 3));
    }

    #[test]
    fn test_is_ready_not_satisfied() {
        assert!(!is_ready(1, 3));
    }

    #[test]
    fn test_is_ready_no_deps() {
        assert!(is_ready(0, 0));
    }

    #[test]
    fn test_ipc_basic() {
        assert_eq!(throughput_ipc(100, 50), 2000); // IPC = 2.0
    }

    #[test]
    fn test_ipc_subunit() {
        assert_eq!(throughput_ipc(1, 4), 250); // IPC = 0.25
    }

    #[test]
    fn test_ipc_zero_cycles() {
        assert_eq!(throughput_ipc(10, 0), 0);
    }

    #[test]
    fn test_attention_window() {
        let entries = vec![
            WindowEntry { id: 0, latency: 3, deps_remaining: 0, total_deps: 0, port: 0, priority: 0.0 },
            WindowEntry { id: 1, latency: 5, deps_remaining: 0, total_deps: 1, port: 1, priority: 0.0 },
        ];
        let scores = attention_window_scores(&entries, 8);
        assert_eq!(scores.len(), 2);
        assert!(scores[0] > 0.0 || scores[1] > 0.0);
    }

    #[test]
    fn test_superscalar_select() {
        let mut entries = vec![
            WindowEntry { id: 0, latency: 3, deps_remaining: 0, total_deps: 0, port: 0, priority: 5.0 },
            WindowEntry { id: 1, latency: 5, deps_remaining: 1, total_deps: 1, port: 1, priority: 10.0 },
            WindowEntry { id: 2, latency: 2, deps_remaining: 0, total_deps: 2, port: 0, priority: 8.0 },
        ];
        let selected = superscalar_select(&mut entries, 2);
        assert!(selected.len() <= 2);
        assert!(selected.contains(&0) || selected.contains(&2)); // Only ready ones
    }

    #[test]
    fn test_schedule_slot_no_conflict() {
        let busy = vec![0, 0, 0, 0];
        assert_eq!(schedule_slot(5, 1, &busy), 5);
    }

    #[test]
    fn test_schedule_slot_conflict() {
        let busy = vec![0, 10, 0, 0];
        assert_eq!(schedule_slot(5, 1, &busy), 10);
    }

    #[test]
    fn test_ffi_attention() {
        let s = slang_nsched_attention_score(4, 3, 16);
        assert!(s > 0);
    }

    #[test]
    fn test_ffi_throughput() {
        assert_eq!(slang_nsched_throughput(8, 4), 2000);
    }

    #[test]
    fn test_ffi_ready() {
        assert_eq!(slang_nsched_issue_ready(3, 3), 1);
        assert_eq!(slang_nsched_issue_ready(1, 3), 0);
    }
}
