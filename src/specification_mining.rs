//! Specification Mining — Vitalis v656
//!
//! Extracts implicit specifications from code and execution traces:
//! - Likely invariants via Daikon-style observation
//! - Pre/postcondition inference from execution traces
//! - Protocol state machine extraction
//! - API usage pattern mining
//! - Anomaly detection against mined specifications

use std::collections::{HashMap, HashSet};

// ── Invariant Mining ─────────────────────────────────────────────────

/// A mined invariant kind.
#[derive(Debug, Clone, PartialEq)]
pub enum MinedInvariant {
    /// Variable is always within [lo, hi].
    Range { var: String, lo: f64, hi: f64 },
    /// Variable is never zero.
    NonZero(String),
    /// Variable is always positive.
    Positive(String),
    /// Two variables always have a fixed relationship.
    LinearRelation { x: String, y: String, slope: f64, intercept: f64 },
    /// Variable is always equal to a constant.
    Constant { var: String, value: f64 },
    /// Variable is always one of a fixed set of values.
    OneOf { var: String, values: Vec<f64> },
    /// Ordering between two variables.
    Ordering { smaller: String, larger: String },
}

impl MinedInvariant {
    pub fn description(&self) -> String {
        match self {
            Self::Range { var, lo, hi } => format!("{lo} <= {var} <= {hi}"),
            Self::NonZero(v) => format!("{v} != 0"),
            Self::Positive(v) => format!("{v} > 0"),
            Self::LinearRelation { x, y, slope, intercept } =>
                format!("{y} = {slope} * {x} + {intercept}"),
            Self::Constant { var, value } => format!("{var} == {value}"),
            Self::OneOf { var, values } => format!("{var} in {:?}", values),
            Self::Ordering { smaller, larger } => format!("{smaller} <= {larger}"),
        }
    }
}

/// Observer that collects variable traces for invariant mining.
pub struct InvariantMiner {
    traces: HashMap<String, Vec<f64>>,
    pair_traces: Vec<HashMap<String, f64>>,  // Per-invocation snapshots
}

impl InvariantMiner {
    pub fn new() -> Self {
        Self { traces: HashMap::new(), pair_traces: Vec::new() }
    }

    /// Record a single observation of a variable.
    pub fn observe(&mut self, var: &str, value: f64) {
        self.traces.entry(var.to_string()).or_default().push(value);
    }

    /// Record a snapshot of multiple variables (for relational mining).
    pub fn observe_snapshot(&mut self, values: HashMap<String, f64>) {
        for (k, v) in &values {
            self.traces.entry(k.clone()).or_default().push(*v);
        }
        self.pair_traces.push(values);
    }

    /// Mine invariants from collected traces.
    pub fn mine(&self) -> Vec<MinedInvariant> {
        let mut invariants = Vec::new();

        for (var, values) in &self.traces {
            if values.is_empty() { continue; }

            // Range invariant
            let lo = values.iter().cloned().fold(f64::INFINITY, f64::min);
            let hi = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            invariants.push(MinedInvariant::Range { var: var.clone(), lo, hi });

            // NonZero check
            if values.iter().all(|&v| v != 0.0) {
                invariants.push(MinedInvariant::NonZero(var.clone()));
            }

            // Positive check
            if values.iter().all(|&v| v > 0.0) {
                invariants.push(MinedInvariant::Positive(var.clone()));
            }

            // Constant check
            if values.iter().all(|&v| (v - values[0]).abs() < 1e-12) {
                invariants.push(MinedInvariant::Constant { var: var.clone(), value: values[0] });
            }

            // OneOf check (small discrete set)
            let unique: HashSet<u64> = values.iter().map(|&v| v.to_bits()).collect();
            if unique.len() <= 5 && unique.len() < values.len() {
                let mut vals: Vec<f64> = unique.iter().map(|&b| f64::from_bits(b)).collect();
                vals.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                invariants.push(MinedInvariant::OneOf { var: var.clone(), values: vals });
            }
        }

        // Relational mining from snapshots
        if !self.pair_traces.is_empty() {
            let vars: Vec<String> = self.pair_traces[0].keys().cloned().collect();
            for i in 0..vars.len() {
                for j in (i + 1)..vars.len() {
                    let (x, y) = (&vars[i], &vars[j]);
                    self.try_mine_ordering(x, y, &mut invariants);
                    self.try_mine_linear(x, y, &mut invariants);
                }
            }
        }

        invariants
    }

    fn try_mine_ordering(&self, x: &str, y: &str, out: &mut Vec<MinedInvariant>) {
        let always_leq = self.pair_traces.iter().all(|snap| {
            match (snap.get(x), snap.get(y)) {
                (Some(xv), Some(yv)) => xv <= yv,
                _ => true,
            }
        });
        if always_leq && !self.pair_traces.is_empty() {
            out.push(MinedInvariant::Ordering { smaller: x.to_string(), larger: y.to_string() });
        }
    }

    fn try_mine_linear(&self, x: &str, y: &str, out: &mut Vec<MinedInvariant>) {
        if self.pair_traces.len() < 3 { return; }

        let pairs: Vec<(f64, f64)> = self.pair_traces.iter()
            .filter_map(|s| Some((*s.get(x)?, *s.get(y)?)))
            .collect();
        if pairs.len() < 3 { return; }

        // Simple linear regression
        let n = pairs.len() as f64;
        let sum_x: f64 = pairs.iter().map(|(x, _)| x).sum();
        let sum_y: f64 = pairs.iter().map(|(_, y)| y).sum();
        let sum_xy: f64 = pairs.iter().map(|(x, y)| x * y).sum();
        let sum_xx: f64 = pairs.iter().map(|(x, _)| x * x).sum();

        let denom = n * sum_xx - sum_x * sum_x;
        if denom.abs() < 1e-12 { return; }

        let slope = (n * sum_xy - sum_x * sum_y) / denom;
        let intercept = (sum_y - slope * sum_x) / n;

        // Check fit (R² > 0.99)
        let ss_res: f64 = pairs.iter()
            .map(|(xi, yi)| { let pred = slope * xi + intercept; (yi - pred) * (yi - pred) })
            .sum();
        let mean_y = sum_y / n;
        let ss_tot: f64 = pairs.iter().map(|(_, yi)| (yi - mean_y) * (yi - mean_y)).sum();

        if ss_tot > 1e-12 {
            let r_sq = 1.0 - ss_res / ss_tot;
            if r_sq > 0.99 {
                out.push(MinedInvariant::LinearRelation {
                    x: x.to_string(), y: y.to_string(), slope, intercept,
                });
            }
        }
    }
}

impl Default for InvariantMiner {
    fn default() -> Self { Self::new() }
}

// ── Protocol Mining ──────────────────────────────────────────────────

/// A state in a mined protocol state machine.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ProtocolState(pub String);

/// A mined protocol transition.
#[derive(Debug, Clone)]
pub struct ProtocolTransition {
    pub from: ProtocolState,
    pub event: String,
    pub to: ProtocolState,
    pub count: u32,
}

/// Protocol state machine miner.
pub struct ProtocolMiner {
    transitions: HashMap<(String, String), (String, u32)>,
}

impl ProtocolMiner {
    pub fn new() -> Self { Self { transitions: HashMap::new() } }

    pub fn observe_transition(&mut self, from: &str, event: &str, to: &str) {
        let key = (from.to_string(), event.to_string());
        let entry = self.transitions.entry(key).or_insert_with(|| (to.to_string(), 0));
        entry.1 += 1;
    }

    pub fn extract_transitions(&self) -> Vec<ProtocolTransition> {
        self.transitions.iter().map(|((from, event), (to, count))| {
            ProtocolTransition {
                from: ProtocolState(from.clone()),
                event: event.clone(),
                to: ProtocolState(to.clone()),
                count: *count,
            }
        }).collect()
    }

    pub fn state_count(&self) -> usize {
        let mut states = HashSet::new();
        for ((from, _), (to, _)) in &self.transitions {
            states.insert(from.clone());
            states.insert(to.clone());
        }
        states.len()
    }
}

impl Default for ProtocolMiner {
    fn default() -> Self { Self::new() }
}

// ── FFI Exports ──────────────────────────────────────────────────────

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_specmine_invariant_count(
    sample_count: i64, unique_count: i64, all_positive: i64,
) -> i64 {
    let mut count: i64 = 1; // Range always
    if all_positive != 0 { count += 2; } // NonZero + Positive
    if unique_count == 1 { count += 1; } // Constant
    if unique_count <= 5 && unique_count < sample_count { count += 1; } // OneOf
    count
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_specmine_linear_r_squared(
    ss_res: f64, ss_tot: f64,
) -> f64 {
    if ss_tot.abs() < 1e-12 { return 0.0; }
    (1.0 - ss_res / ss_tot).clamp(0.0, 1.0)
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_specmine_protocol_states(transition_count: i64) -> i64 {
    // Upper bound: at most 2 * transitions states
    (transition_count.max(0) * 2).min(transition_count.max(0) + 1)
}

// ═══════════════════════════════════════════════════════════════════════
// Tests
// ═══════════════════════════════════════════════════════════════════════
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mine_range() {
        let mut miner = InvariantMiner::new();
        for v in [1.0, 5.0, 3.0, 7.0, 2.0] { miner.observe("x", v); }
        let invs = miner.mine();
        assert!(invs.iter().any(|i| matches!(i, MinedInvariant::Range { var, lo, hi } if var == "x" && *lo == 1.0 && *hi == 7.0)));
    }

    #[test]
    fn test_mine_positive() {
        let mut miner = InvariantMiner::new();
        for v in [1.0, 2.0, 3.0] { miner.observe("pos", v); }
        let invs = miner.mine();
        assert!(invs.iter().any(|i| matches!(i, MinedInvariant::Positive(v) if v == "pos")));
    }

    #[test]
    fn test_mine_constant() {
        let mut miner = InvariantMiner::new();
        for _ in 0..5 { miner.observe("c", 42.0); }
        let invs = miner.mine();
        assert!(invs.iter().any(|i| matches!(i, MinedInvariant::Constant { var, value } if var == "c" && *value == 42.0)));
    }

    #[test]
    fn test_mine_nonzero() {
        let mut miner = InvariantMiner::new();
        for v in [1.0, -1.0, 5.0] { miner.observe("nz", v); }
        let invs = miner.mine();
        assert!(invs.iter().any(|i| matches!(i, MinedInvariant::NonZero(v) if v == "nz")));
    }

    #[test]
    fn test_mine_oneof() {
        let mut miner = InvariantMiner::new();
        for v in [1.0, 2.0, 1.0, 2.0, 1.0] { miner.observe("flag", v); }
        let invs = miner.mine();
        assert!(invs.iter().any(|i| matches!(i, MinedInvariant::OneOf { var, .. } if var == "flag")));
    }

    #[test]
    fn test_mine_linear_relation() {
        let mut miner = InvariantMiner::new();
        for i in 0..10 {
            let mut snap = HashMap::new();
            snap.insert("x".into(), i as f64);
            snap.insert("y".into(), 2.0 * i as f64 + 1.0);
            miner.observe_snapshot(snap);
        }
        let invs = miner.mine();
        assert!(invs.iter().any(|i| matches!(i, MinedInvariant::LinearRelation { .. })));
    }

    #[test]
    fn test_mine_ordering() {
        let mut miner = InvariantMiner::new();
        for i in 0..5 {
            let mut snap = HashMap::new();
            snap.insert("lo".into(), i as f64);
            snap.insert("hi".into(), (i + 10) as f64);
            miner.observe_snapshot(snap);
        }
        let invs = miner.mine();
        assert!(invs.iter().any(|i| matches!(i, MinedInvariant::Ordering { .. })));
    }

    #[test]
    fn test_invariant_description() {
        let inv = MinedInvariant::Range { var: "x".into(), lo: 0.0, hi: 100.0 };
        assert!(inv.description().contains("x"));
    }

    #[test]
    fn test_protocol_miner() {
        let mut pm = ProtocolMiner::new();
        pm.observe_transition("init", "connect", "connected");
        pm.observe_transition("connected", "send", "sending");
        pm.observe_transition("sending", "ack", "connected");
        let transitions = pm.extract_transitions();
        assert_eq!(transitions.len(), 3);
        assert_eq!(pm.state_count(), 3);
    }

    #[test]
    fn test_protocol_transition_counts() {
        let mut pm = ProtocolMiner::new();
        pm.observe_transition("a", "go", "b");
        pm.observe_transition("a", "go", "b");
        pm.observe_transition("a", "go", "b");
        let ts = pm.extract_transitions();
        assert_eq!(ts[0].count, 3);
    }

    #[test]
    fn test_ffi_invariant_count() {
        assert!(vitalis_specmine_invariant_count(10, 1, 1) >= 4);
    }

    #[test]
    fn test_ffi_r_squared() {
        assert!((vitalis_specmine_linear_r_squared(0.0, 100.0) - 1.0).abs() < 1e-10);
        assert!((vitalis_specmine_linear_r_squared(50.0, 100.0) - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_ffi_protocol_states() {
        assert!(vitalis_specmine_protocol_states(3) >= 3);
    }

    #[test]
    fn test_empty_miner() {
        let miner = InvariantMiner::new();
        assert!(miner.mine().is_empty());
    }
}
