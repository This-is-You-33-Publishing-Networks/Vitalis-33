//! Architecture Advisor — Vitalis v630
//!
//! Provides architectural recommendations for code structure:
//! - Coupling/cohesion analysis
//! - Dependency graph metrics (fan-in, fan-out, instability)
//! - Module organization suggestions
//! - Design pattern detection and recommendations
//! - Architecture quality scoring

use std::collections::HashMap;

// ── Dependency Metrics ───────────────────────────────────────────────

/// Module dependency information.
#[derive(Debug, Clone)]
pub struct ModuleMetrics {
    pub name: String,
    pub fan_in: usize,     // Incoming dependencies (afferent coupling)
    pub fan_out: usize,    // Outgoing dependencies (efferent coupling)
    pub internal_syms: usize,  // Symbols defined in module
    pub exported_syms: usize,  // Symbols exported
    pub lines: usize,
}

impl ModuleMetrics {
    pub fn new(name: &str) -> Self {
        Self { name: name.to_string(), fan_in: 0, fan_out: 0, internal_syms: 0, exported_syms: 0, lines: 0 }
    }

    /// Instability: I = Ce / (Ca + Ce), where Ca = fan_in, Ce = fan_out.
    pub fn instability(&self) -> f64 {
        let total = self.fan_in + self.fan_out;
        if total == 0 { return 0.5; }
        self.fan_out as f64 / total as f64
    }

    /// Abstractness: ratio of abstract (exported) to total symbols.
    pub fn abstractness(&self) -> f64 {
        if self.internal_syms == 0 { return 0.0; }
        self.exported_syms as f64 / self.internal_syms as f64
    }

    /// Distance from main sequence: |A + I - 1|.
    pub fn distance_from_main_sequence(&self) -> f64 {
        (self.abstractness() + self.instability() - 1.0).abs()
    }

    /// Module complexity score (higher = more complex).
    pub fn complexity_score(&self) -> f64 {
        let coupling = (self.fan_in + self.fan_out) as f64;
        let size = (self.lines as f64).sqrt();
        coupling * 0.3 + size * 0.7
    }
}

// ── Architecture Graph ───────────────────────────────────────────────

/// Architecture dependency graph for analysis.
pub struct ArchGraph {
    modules: Vec<ModuleMetrics>,
    module_index: HashMap<String, usize>,
    dependencies: Vec<(usize, usize)>,  // (from_module, to_module)
}

impl ArchGraph {
    pub fn new() -> Self {
        Self { modules: Vec::new(), module_index: HashMap::new(), dependencies: Vec::new() }
    }

    pub fn add_module(&mut self, metrics: ModuleMetrics) -> usize {
        let idx = self.modules.len();
        self.module_index.insert(metrics.name.clone(), idx);
        self.modules.push(metrics);
        idx
    }

    pub fn add_dependency(&mut self, from: &str, to: &str) {
        if let (Some(&f), Some(&t)) = (self.module_index.get(from), self.module_index.get(to)) {
            self.dependencies.push((f, t));
            self.modules[f].fan_out += 1;
            self.modules[t].fan_in += 1;
        }
    }

    pub fn module_count(&self) -> usize { self.modules.len() }
    pub fn dependency_count(&self) -> usize { self.dependencies.len() }

    /// Detect circular dependencies.
    pub fn detect_cycles(&self) -> Vec<Vec<String>> {
        let n = self.modules.len();
        let mut adj: Vec<Vec<usize>> = vec![vec![]; n];
        for &(f, t) in &self.dependencies {
            adj[f].push(t);
        }

        let mut cycles = Vec::new();
        let mut visited = vec![false; n];
        let mut on_stack = vec![false; n];
        let mut stack = Vec::new();

        for start in 0..n {
            if visited[start] { continue; }
            self.dfs_cycle(start, &adj, &mut visited, &mut on_stack, &mut stack, &mut cycles);
        }
        cycles
    }

    fn dfs_cycle(
        &self, node: usize, adj: &[Vec<usize>],
        visited: &mut [bool], on_stack: &mut [bool],
        stack: &mut Vec<usize>, cycles: &mut Vec<Vec<String>>,
    ) {
        visited[node] = true;
        on_stack[node] = true;
        stack.push(node);

        for &next in &adj[node] {
            if !visited[next] {
                self.dfs_cycle(next, adj, visited, on_stack, stack, cycles);
            } else if on_stack[next] {
                // Found cycle: extract from stack
                let pos = stack.iter().position(|&n| n == next).unwrap_or(0);
                let cycle: Vec<String> = stack[pos..].iter()
                    .map(|&i| self.modules[i].name.clone())
                    .collect();
                if cycle.len() >= 2 {
                    cycles.push(cycle);
                }
            }
        }

        stack.pop();
        on_stack[node] = false;
    }

    /// Get modules sorted by distance from main sequence (worst first).
    pub fn pain_points(&self) -> Vec<(String, f64)> {
        let mut points: Vec<_> = self.modules.iter()
            .map(|m| (m.name.clone(), m.distance_from_main_sequence()))
            .collect();
        points.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        points
    }

    /// Overall architecture quality score (0-1).
    pub fn quality_score(&self) -> f64 {
        if self.modules.is_empty() { return 1.0; }

        // Penalize cycles
        let cycle_penalty = if self.detect_cycles().is_empty() { 0.0 } else { 0.3 };

        // Average distance from main sequence
        let avg_dist = self.modules.iter()
            .map(|m| m.distance_from_main_sequence())
            .sum::<f64>() / self.modules.len() as f64;

        // High fan-out penalty
        let max_fan_out = self.modules.iter().map(|m| m.fan_out).max().unwrap_or(0);
        let fan_out_penalty = if max_fan_out > 10 { 0.1 } else { 0.0 };

        (1.0 - avg_dist - cycle_penalty - fan_out_penalty).clamp(0.0, 1.0)
    }

    /// Generate architectural recommendations.
    pub fn recommendations(&self) -> Vec<String> {
        let mut recs = Vec::new();

        let cycles = self.detect_cycles();
        if !cycles.is_empty() {
            recs.push(format!("CRITICAL: {} circular dependency cycles detected", cycles.len()));
        }

        for m in &self.modules {
            if m.fan_out > 10 {
                recs.push(format!("HIGH_COUPLING: {} has fan-out of {} — consider splitting", m.name, m.fan_out));
            }
            if m.fan_in > 15 {
                recs.push(format!("GOD_MODULE: {} has fan-in of {} — too many dependents", m.name, m.fan_in));
            }
            if m.distance_from_main_sequence() > 0.7 {
                recs.push(format!("ZONE_OF_PAIN: {} has D={:.2} — needs restructuring", m.name, m.distance_from_main_sequence()));
            }
        }

        recs
    }

    /// Find strongly connected components (Tarjan's algorithm).
    pub fn strongly_connected_components(&self) -> Vec<Vec<usize>> {
        let n = self.modules.len();
        let mut adj: Vec<Vec<usize>> = vec![vec![]; n];
        for &(f, t) in &self.dependencies {
            adj[f].push(t);
        }

        let mut index_counter = 0u32;
        let mut stack = Vec::new();
        let mut on_stack = vec![false; n];
        let mut indices = vec![u32::MAX; n];
        let mut lowlinks = vec![u32::MAX; n];
        let mut sccs = Vec::new();

        for v in 0..n {
            if indices[v] == u32::MAX {
                self.tarjan_dfs(v, &adj, &mut index_counter, &mut stack, &mut on_stack,
                              &mut indices, &mut lowlinks, &mut sccs);
            }
        }
        sccs
    }

    fn tarjan_dfs(
        &self, v: usize, adj: &[Vec<usize>],
        index_counter: &mut u32, stack: &mut Vec<usize>,
        on_stack: &mut [bool], indices: &mut [u32], lowlinks: &mut [u32],
        sccs: &mut Vec<Vec<usize>>,
    ) {
        indices[v] = *index_counter;
        lowlinks[v] = *index_counter;
        *index_counter += 1;
        stack.push(v);
        on_stack[v] = true;

        for &w in &adj[v] {
            if indices[w] == u32::MAX {
                self.tarjan_dfs(w, adj, index_counter, stack, on_stack, indices, lowlinks, sccs);
                lowlinks[v] = lowlinks[v].min(lowlinks[w]);
            } else if on_stack[w] {
                lowlinks[v] = lowlinks[v].min(indices[w]);
            }
        }

        if lowlinks[v] == indices[v] {
            let mut scc = Vec::new();
            while let Some(w) = stack.pop() {
                on_stack[w] = false;
                scc.push(w);
                if w == v { break; }
            }
            sccs.push(scc);
        }
    }
}

impl Default for ArchGraph {
    fn default() -> Self { Self::new() }
}

// ── FFI Exports ──────────────────────────────────────────────────────

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_arch_instability(fan_in: i64, fan_out: i64) -> f64 {
    let total = fan_in + fan_out;
    if total <= 0 { 0.5 } else { fan_out as f64 / total as f64 }
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_arch_distance_main_sequence(abstractness: f64, instability: f64) -> f64 {
    (abstractness + instability - 1.0).abs()
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_arch_quality(
    avg_distance: f64, has_cycles: i64, max_fan_out: i64,
) -> f64 {
    let cycle_penalty = if has_cycles != 0 { 0.3 } else { 0.0 };
    let fan_penalty = if max_fan_out > 10 { 0.1 } else { 0.0 };
    (1.0 - avg_distance - cycle_penalty - fan_penalty).clamp(0.0, 1.0)
}

// ═══════════════════════════════════════════════════════════════════════
// Tests
// ═══════════════════════════════════════════════════════════════════════
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_instability() {
        let mut m = ModuleMetrics::new("core");
        m.fan_in = 5; m.fan_out = 5;
        assert!((m.instability() - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_module_abstractness() {
        let mut m = ModuleMetrics::new("api");
        m.internal_syms = 20; m.exported_syms = 15;
        assert!((m.abstractness() - 0.75).abs() < 1e-10);
    }

    #[test]
    fn test_main_sequence_distance() {
        let mut m = ModuleMetrics::new("util");
        m.fan_in = 0; m.fan_out = 10; // I = 1.0
        m.internal_syms = 10; m.exported_syms = 0; // A = 0.0
        // D = |0 + 1 - 1| = 0
        assert!(m.distance_from_main_sequence() < 0.01);
    }

    #[test]
    fn test_module_complexity() {
        let mut m = ModuleMetrics::new("complex");
        m.fan_in = 10; m.fan_out = 10; m.lines = 1000;
        assert!(m.complexity_score() > 0.0);
    }

    #[test]
    fn test_arch_graph_basic() {
        let mut g = ArchGraph::new();
        g.add_module(ModuleMetrics::new("a"));
        g.add_module(ModuleMetrics::new("b"));
        g.add_dependency("a", "b");
        assert_eq!(g.module_count(), 2);
        assert_eq!(g.dependency_count(), 1);
    }

    #[test]
    fn test_arch_no_cycles() {
        let mut g = ArchGraph::new();
        g.add_module(ModuleMetrics::new("a"));
        g.add_module(ModuleMetrics::new("b"));
        g.add_module(ModuleMetrics::new("c"));
        g.add_dependency("a", "b");
        g.add_dependency("b", "c");
        assert!(g.detect_cycles().is_empty());
    }

    #[test]
    fn test_arch_with_cycle() {
        let mut g = ArchGraph::new();
        g.add_module(ModuleMetrics::new("a"));
        g.add_module(ModuleMetrics::new("b"));
        g.add_module(ModuleMetrics::new("c"));
        g.add_dependency("a", "b");
        g.add_dependency("b", "c");
        g.add_dependency("c", "a");
        assert!(!g.detect_cycles().is_empty());
    }

    #[test]
    fn test_pain_points() {
        let mut g = ArchGraph::new();
        let mut m1 = ModuleMetrics::new("stable");
        m1.fan_in = 10; m1.fan_out = 0; m1.internal_syms = 10; m1.exported_syms = 10;
        let mut m2 = ModuleMetrics::new("painful");
        m2.fan_in = 0; m2.fan_out = 0; m2.internal_syms = 10; m2.exported_syms = 0;
        g.add_module(m1);
        g.add_module(m2);
        let points = g.pain_points();
        assert_eq!(points.len(), 2);
    }

    #[test]
    fn test_quality_score_good() {
        let mut g = ArchGraph::new();
        let mut m = ModuleMetrics::new("balanced");
        m.fan_in = 3; m.fan_out = 3;
        m.internal_syms = 10; m.exported_syms = 5;
        g.add_module(m);
        assert!(g.quality_score() > 0.5);
    }

    #[test]
    fn test_recommendations_high_coupling() {
        let mut g = ArchGraph::new();
        let mut m = ModuleMetrics::new("big_module");
        m.fan_out = 15;
        g.add_module(m);
        let recs = g.recommendations();
        assert!(recs.iter().any(|r| r.contains("HIGH_COUPLING")));
    }

    #[test]
    fn test_scc() {
        let mut g = ArchGraph::new();
        g.add_module(ModuleMetrics::new("a"));
        g.add_module(ModuleMetrics::new("b"));
        g.add_module(ModuleMetrics::new("c"));
        g.add_dependency("a", "b");
        g.add_dependency("b", "c");
        g.add_dependency("c", "a");
        let sccs = g.strongly_connected_components();
        assert!(sccs.iter().any(|scc| scc.len() == 3));
    }

    #[test]
    fn test_empty_graph() {
        let g = ArchGraph::new();
        assert_eq!(g.quality_score(), 1.0);
        assert!(g.detect_cycles().is_empty());
    }

    #[test]
    fn test_ffi_instability() {
        assert!((vitalis_arch_instability(5, 5) - 0.5).abs() < 1e-10);
        assert_eq!(vitalis_arch_instability(0, 0), 0.5);
    }

    #[test]
    fn test_ffi_distance() {
        assert!((vitalis_arch_distance_main_sequence(0.5, 0.5) - 0.0).abs() < 1e-10);
        assert!((vitalis_arch_distance_main_sequence(1.0, 1.0) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_ffi_quality() {
        assert!(vitalis_arch_quality(0.1, 0, 5) > 0.8);
        assert!(vitalis_arch_quality(0.1, 1, 15) < 0.6);
    }
}
