//! Neural Register Allocation — v715 ERA II Phase 30
//!
//! Register allocation via graph coloring with neural heuristics. Builds
//! interference graphs from live intervals, applies simplify-select-coalesce
//! with a neural coloring heuristic that scores variables by degree, spill cost,
//! and loop nesting. Spill decisions weighted by usage frequency.

use std::collections::{HashMap, HashSet, BTreeSet};

/// A virtual register / variable to be allocated.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct VReg {
    pub id: u32,
    pub name: String,
}

/// Live interval: [start, end) for a virtual register.
#[derive(Debug, Clone)]
pub struct LiveInterval {
    pub vreg: u32,
    pub start: u32,
    pub end: u32,
    pub loop_depth: u32,
    pub use_count: u32,
}

impl LiveInterval {
    pub fn overlaps(&self, other: &LiveInterval) -> bool {
        self.start < other.end && other.start < self.end
    }

    pub fn length(&self) -> u32 {
        self.end.saturating_sub(self.start)
    }

    /// Spill cost heuristic: high use_count in deep loops → expensive spill.
    pub fn spill_cost(&self) -> f64 {
        let depth_factor = 10.0_f64.powi(self.loop_depth as i32);
        self.use_count as f64 * depth_factor / self.length().max(1) as f64
    }
}

/// A physical register with optional constraints.
#[derive(Debug, Clone)]
pub struct PhysReg {
    pub id: u32,
    pub name: String,
    pub is_callee_saved: bool,
}

/// Pool of available physical registers.
#[derive(Debug)]
pub struct RegisterPool {
    pub registers: Vec<PhysReg>,
}

impl RegisterPool {
    pub fn new(registers: Vec<PhysReg>) -> Self {
        Self { registers }
    }

    pub fn count(&self) -> usize {
        self.registers.len()
    }

    /// Create a standard x86-64-like register set.
    pub fn x86_64_gp() -> Self {
        let names = ["rax", "rbx", "rcx", "rdx", "rsi", "rdi", "r8", "r9",
                     "r10", "r11", "r12", "r13", "r14", "r15"];
        let callee_saved = ["rbx", "r12", "r13", "r14", "r15"];
        let regs = names
            .iter()
            .enumerate()
            .map(|(i, name)| PhysReg {
                id: i as u32,
                name: name.to_string(),
                is_callee_saved: callee_saved.contains(name),
            })
            .collect();
        Self::new(regs)
    }
}

/// Interference graph: nodes are vregs, edges mean live ranges overlap.
#[derive(Debug, Default)]
pub struct InterferenceGraph {
    /// Adjacency list: vreg_id → set of interfering vreg_ids.
    adjacency: HashMap<u32, HashSet<u32>>,
    /// Move-related pairs (for coalescing).
    move_pairs: Vec<(u32, u32)>,
}

impl InterferenceGraph {
    pub fn new() -> Self {
        Self::default()
    }

    /// Build from live intervals: O(n²) pairwise overlap check.
    pub fn build(intervals: &[LiveInterval]) -> Self {
        let mut graph = Self::new();
        for iv in intervals {
            graph.adjacency.entry(iv.vreg).or_default();
        }
        for i in 0..intervals.len() {
            for j in (i + 1)..intervals.len() {
                if intervals[i].overlaps(&intervals[j]) {
                    graph.add_edge(intervals[i].vreg, intervals[j].vreg);
                }
            }
        }
        graph
    }

    pub fn add_edge(&mut self, a: u32, b: u32) {
        self.adjacency.entry(a).or_default().insert(b);
        self.adjacency.entry(b).or_default().insert(a);
    }

    pub fn add_move(&mut self, a: u32, b: u32) {
        self.move_pairs.push((a, b));
    }

    pub fn degree(&self, vreg: u32) -> usize {
        self.adjacency.get(&vreg).map_or(0, |s| s.len())
    }

    pub fn neighbors(&self, vreg: u32) -> impl Iterator<Item = u32> + '_ {
        self.adjacency.get(&vreg).into_iter().flat_map(|s| s.iter().copied())
    }

    pub fn nodes(&self) -> impl Iterator<Item = u32> + '_ {
        self.adjacency.keys().copied()
    }

    pub fn node_count(&self) -> usize {
        self.adjacency.len()
    }

    pub fn remove_node(&mut self, vreg: u32) {
        if let Some(neighbors) = self.adjacency.remove(&vreg) {
            for n in neighbors {
                if let Some(adj) = self.adjacency.get_mut(&n) {
                    adj.remove(&vreg);
                }
            }
        }
    }

    pub fn interferes(&self, a: u32, b: u32) -> bool {
        self.adjacency.get(&a).map_or(false, |s| s.contains(&b))
    }
}

/// Neural coloring heuristic: scores which vreg to process next.
#[derive(Debug)]
pub struct NeuralColoringHeuristic {
    /// Weight for degree in scoring.
    pub degree_weight: f64,
    /// Weight for spill cost in scoring.
    pub spill_cost_weight: f64,
    /// Weight for loop nesting depth.
    pub loop_depth_weight: f64,
}

impl NeuralColoringHeuristic {
    pub fn default_weights() -> Self {
        Self {
            degree_weight: 1.0,
            spill_cost_weight: -2.0, // High spill cost → avoid spilling → allocate first
            loop_depth_weight: -1.5,
        }
    }

    /// Score a vreg for ordering: higher score → process first.
    pub fn score(&self, degree: usize, interval: &LiveInterval) -> f64 {
        self.degree_weight * degree as f64
            + self.spill_cost_weight * interval.spill_cost()
            + self.loop_depth_weight * interval.loop_depth as f64
    }
}

/// Result of register allocation for a single vreg.
#[derive(Debug, Clone, PartialEq)]
pub enum AllocResult {
    Register(u32),
    Spilled,
}

/// The neural register allocator.
pub struct NeuralRegAlloc {
    pub heuristic: NeuralColoringHeuristic,
    pub pool: RegisterPool,
}

impl NeuralRegAlloc {
    pub fn new(pool: RegisterPool, heuristic: NeuralColoringHeuristic) -> Self {
        Self { heuristic, pool }
    }

    /// Allocate registers using simplify-select with neural ordering.
    pub fn allocate(&self, intervals: &[LiveInterval]) -> HashMap<u32, AllocResult> {
        let mut graph = InterferenceGraph::build(intervals);
        let interval_map: HashMap<u32, &LiveInterval> =
            intervals.iter().map(|iv| (iv.vreg, iv)).collect();
        let k = self.pool.count();

        // Phase 1: Simplify — iteratively remove nodes with degree < k
        let mut stack: Vec<u32> = Vec::new();
        let mut remaining: BTreeSet<u32> = graph.nodes().collect();

        loop {
            // Find a node with degree < k (or use heuristic to pick spill candidate)
            let candidate = remaining
                .iter()
                .filter(|&&v| graph.degree(v) < k)
                .copied()
                .next();
            if let Some(v) = candidate {
                remaining.remove(&v);
                stack.push(v);
                graph.remove_node(v);
            } else if let Some(&spill_candidate) = remaining.iter().min_by(|&&a, &&b| {
                let sa = interval_map.get(&a).map_or(0.0, |iv| self.heuristic.score(graph.degree(a), iv));
                let sb = interval_map.get(&b).map_or(0.0, |iv| self.heuristic.score(graph.degree(b), iv));
                // Higher score = harder to spill, so pick lowest score to spill
                sa.partial_cmp(&sb).unwrap_or(std::cmp::Ordering::Equal)
            }) {
                remaining.remove(&spill_candidate);
                stack.push(spill_candidate);
                graph.remove_node(spill_candidate);
            } else {
                break;
            }
        }

        // Phase 2: Select — pop stack and assign colors
        let orig_graph = InterferenceGraph::build(intervals);
        let mut allocation: HashMap<u32, AllocResult> = HashMap::new();

        while let Some(vreg) = stack.pop() {
            let used_colors: HashSet<u32> = orig_graph
                .neighbors(vreg)
                .filter_map(|n| match allocation.get(&n) {
                    Some(AllocResult::Register(c)) => Some(*c),
                    _ => None,
                })
                .collect();

            let color = self
                .pool
                .registers
                .iter()
                .map(|r| r.id)
                .find(|c| !used_colors.contains(c));

            match color {
                Some(c) => { allocation.insert(vreg, AllocResult::Register(c)); }
                None => { allocation.insert(vreg, AllocResult::Spilled); }
            }
        }

        allocation
    }

    /// Attempt coalescing: merge move-related vregs if they don't interfere.
    pub fn coalesce(intervals: &mut Vec<LiveInterval>, graph: &InterferenceGraph) -> usize {
        let mut coalesced = 0;
        let pairs = graph.move_pairs.clone();
        for (a, b) in pairs {
            if !graph.interferes(a, b) {
                // Merge b into a
                if let Some(pos_b) = intervals.iter().position(|iv| iv.vreg == b) {
                    let iv_b = intervals[pos_b].clone();
                    if let Some(iv_a) = intervals.iter_mut().find(|iv| iv.vreg == a) {
                        iv_a.start = iv_a.start.min(iv_b.start);
                        iv_a.end = iv_a.end.max(iv_b.end);
                        iv_a.use_count += iv_b.use_count;
                    }
                    intervals.remove(pos_b);
                    coalesced += 1;
                }
            }
        }
        coalesced
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_interval(vreg: u32, start: u32, end: u32) -> LiveInterval {
        LiveInterval { vreg, start, end, loop_depth: 0, use_count: 1 }
    }

    #[test]
    fn test_interval_overlaps() {
        let a = make_interval(0, 0, 10);
        let b = make_interval(1, 5, 15);
        assert!(a.overlaps(&b));
    }

    #[test]
    fn test_interval_no_overlap() {
        let a = make_interval(0, 0, 5);
        let b = make_interval(1, 5, 10);
        assert!(!a.overlaps(&b));
    }

    #[test]
    fn test_interval_spill_cost() {
        let iv = LiveInterval { vreg: 0, start: 0, end: 10, loop_depth: 2, use_count: 5 };
        let cost = iv.spill_cost();
        assert!(cost > 0.0);
        // depth=2 → factor=100, cost = 5*100/10 = 50
        assert!((cost - 50.0).abs() < 1e-6);
    }

    #[test]
    fn test_graph_build() {
        let intervals = vec![
            make_interval(0, 0, 10),
            make_interval(1, 5, 15),
            make_interval(2, 20, 30),
        ];
        let graph = InterferenceGraph::build(&intervals);
        assert!(graph.interferes(0, 1));
        assert!(!graph.interferes(0, 2));
    }

    #[test]
    fn test_graph_degree() {
        let intervals = vec![
            make_interval(0, 0, 10),
            make_interval(1, 5, 15),
            make_interval(2, 8, 12),
        ];
        let graph = InterferenceGraph::build(&intervals);
        // All three overlap with each other
        assert_eq!(graph.degree(0), 2);
    }

    #[test]
    fn test_graph_remove_node() {
        let intervals = vec![
            make_interval(0, 0, 10),
            make_interval(1, 5, 15),
        ];
        let mut graph = InterferenceGraph::build(&intervals);
        graph.remove_node(0);
        assert_eq!(graph.degree(1), 0);
        assert_eq!(graph.node_count(), 1);
    }

    #[test]
    fn test_register_pool_x86() {
        let pool = RegisterPool::x86_64_gp();
        assert_eq!(pool.count(), 14);
    }

    #[test]
    fn test_heuristic_score() {
        let h = NeuralColoringHeuristic::default_weights();
        let iv = LiveInterval { vreg: 0, start: 0, end: 10, loop_depth: 1, use_count: 5 };
        let score = h.score(3, &iv);
        // degree_weight * 3 + spill_cost_weight * (5*10/10) + loop_depth_weight * 1
        // = 3 + (-2)*5 + (-1.5)*1 = 3 - 10 - 1.5 = -8.5
        assert!((score - (-8.5)).abs() < 1e-6);
    }

    #[test]
    fn test_allocate_no_interference() {
        let pool = RegisterPool::new(vec![
            PhysReg { id: 0, name: "r0".into(), is_callee_saved: false },
            PhysReg { id: 1, name: "r1".into(), is_callee_saved: false },
        ]);
        let intervals = vec![
            make_interval(0, 0, 5),
            make_interval(1, 10, 15),
        ];
        let alloc = NeuralRegAlloc::new(pool, NeuralColoringHeuristic::default_weights());
        let result = alloc.allocate(&intervals);
        assert!(matches!(result.get(&0), Some(AllocResult::Register(_))));
        assert!(matches!(result.get(&1), Some(AllocResult::Register(_))));
    }

    #[test]
    fn test_allocate_with_interference() {
        let pool = RegisterPool::new(vec![
            PhysReg { id: 0, name: "r0".into(), is_callee_saved: false },
            PhysReg { id: 1, name: "r1".into(), is_callee_saved: false },
        ]);
        let intervals = vec![
            make_interval(0, 0, 10),
            make_interval(1, 5, 15),
        ];
        let alloc = NeuralRegAlloc::new(pool, NeuralColoringHeuristic::default_weights());
        let result = alloc.allocate(&intervals);
        // Both should get different registers
        let r0 = result.get(&0).unwrap();
        let r1 = result.get(&1).unwrap();
        assert_ne!(r0, r1);
    }

    #[test]
    fn test_allocate_spill_needed() {
        let pool = RegisterPool::new(vec![
            PhysReg { id: 0, name: "r0".into(), is_callee_saved: false },
        ]);
        let intervals = vec![
            make_interval(0, 0, 10),
            make_interval(1, 5, 15),
        ];
        let alloc = NeuralRegAlloc::new(pool, NeuralColoringHeuristic::default_weights());
        let result = alloc.allocate(&intervals);
        // One must be spilled
        let spill_count = result.values().filter(|r| matches!(r, AllocResult::Spilled)).count();
        assert_eq!(spill_count, 1);
    }

    #[test]
    fn test_allocate_many_regs() {
        let pool = RegisterPool::x86_64_gp();
        let intervals: Vec<LiveInterval> = (0..14)
            .map(|i| make_interval(i, 0, 100)) // all interfere
            .collect();
        let alloc = NeuralRegAlloc::new(pool, NeuralColoringHeuristic::default_weights());
        let result = alloc.allocate(&intervals);
        assert_eq!(result.len(), 14);
        // All 14 interfere, 14 registers available: should color all
        let spilled = result.values().filter(|r| matches!(r, AllocResult::Spilled)).count();
        assert_eq!(spilled, 0);
    }

    #[test]
    fn test_coalesce_non_interfering() {
        let mut intervals = vec![
            make_interval(0, 0, 5),
            make_interval(1, 10, 15),
        ];
        let mut graph = InterferenceGraph::build(&intervals);
        graph.add_move(0, 1);
        let coalesced = NeuralRegAlloc::coalesce(&mut intervals, &graph);
        assert_eq!(coalesced, 1);
        assert_eq!(intervals.len(), 1);
        assert_eq!(intervals[0].start, 0);
        assert_eq!(intervals[0].end, 15);
    }

    #[test]
    fn test_coalesce_interfering_stays() {
        let mut intervals = vec![
            make_interval(0, 0, 10),
            make_interval(1, 5, 15),
        ];
        let mut graph = InterferenceGraph::build(&intervals);
        graph.add_move(0, 1);
        let coalesced = NeuralRegAlloc::coalesce(&mut intervals, &graph);
        assert_eq!(coalesced, 0); // can't coalesce: they interfere
        assert_eq!(intervals.len(), 2);
    }

    #[test]
    fn test_allocate_empty() {
        let pool = RegisterPool::x86_64_gp();
        let alloc = NeuralRegAlloc::new(pool, NeuralColoringHeuristic::default_weights());
        let result = alloc.allocate(&[]);
        assert!(result.is_empty());
    }
}
