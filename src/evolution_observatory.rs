//! v440 — Evolution Observatory.
//!
//! Real-time monitoring of all self-evolution activity. Lineage tracking,
//! rollback tree, and evolution metrics. JSON export for Void Studio integration.

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

/// A recorded evolution event.
#[derive(Debug, Clone)]
pub struct EvolutionEvent {
    pub id: u64,
    pub timestamp_ms: u64,
    pub event_type: EventType,
    pub function_name: String,
    pub generation: u64,
    pub fitness_before: f64,
    pub fitness_after: f64,
    pub description: String,
}

/// Types of evolution events.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventType {
    Mutation,
    Crossover,
    Selection,
    Rollback,
    Improvement,
    Regression,
    PassEvolution,
    HealAction,
}

impl EventType {
    pub fn name(&self) -> &'static str {
        match self {
            EventType::Mutation => "mutation",
            EventType::Crossover => "crossover",
            EventType::Selection => "selection",
            EventType::Rollback => "rollback",
            EventType::Improvement => "improvement",
            EventType::Regression => "regression",
            EventType::PassEvolution => "pass_evolution",
            EventType::HealAction => "heal",
        }
    }
}

/// Lineage node for tracking evolutionary ancestry.
#[derive(Debug, Clone)]
pub struct LineageNode {
    pub generation: u64,
    pub fitness: f64,
    pub parent_generation: Option<u64>,
    pub mutation_kind: String,
    pub children: Vec<u64>,
}

/// The Evolution Observatory.
pub struct Observatory {
    pub events: Vec<EvolutionEvent>,
    pub lineages: HashMap<String, Vec<LineageNode>>,
    event_counter: u64,
    /// Clock for timestamps.
    clock_ms: u64,
    /// Max events to retain.
    max_events: usize,
}

impl Observatory {
    pub fn new() -> Self {
        Self {
            events: Vec::new(),
            lineages: HashMap::new(),
            event_counter: 0,
            clock_ms: 0,
            max_events: 10000,
        }
    }

    /// Record an evolution event.
    pub fn record_event(
        &mut self,
        event_type: EventType,
        function_name: &str,
        generation: u64,
        fitness_before: f64,
        fitness_after: f64,
        description: &str,
    ) -> u64 {
        self.event_counter += 1;
        self.clock_ms += 1;
        let event = EvolutionEvent {
            id: self.event_counter,
            timestamp_ms: self.clock_ms,
            event_type,
            function_name: function_name.to_string(),
            generation,
            fitness_before,
            fitness_after,
            description: description.to_string(),
        };
        self.events.push(event);
        if self.events.len() > self.max_events {
            self.events.remove(0);
        }
        self.event_counter
    }

    /// Record a lineage entry.
    pub fn record_lineage(
        &mut self,
        function_name: &str,
        generation: u64,
        fitness: f64,
        parent_generation: Option<u64>,
        mutation_kind: &str,
    ) {
        let nodes = self.lineages.entry(function_name.to_string()).or_default();
        // Set parent-child relationship
        if let Some(parent_gen) = parent_generation {
            for node in nodes.iter_mut() {
                if node.generation == parent_gen {
                    node.children.push(generation);
                    break;
                }
            }
        }
        nodes.push(LineageNode {
            generation,
            fitness,
            parent_generation,
            mutation_kind: mutation_kind.to_string(),
            children: Vec::new(),
        });
    }

    /// Get events by type.
    pub fn events_by_type(&self, event_type: EventType) -> Vec<&EvolutionEvent> {
        self.events.iter().filter(|e| e.event_type == event_type).collect()
    }

    /// Get events for a function.
    pub fn events_for_function(&self, name: &str) -> Vec<&EvolutionEvent> {
        self.events.iter().filter(|e| e.function_name == name).collect()
    }

    /// Get lineage depth for a function.
    pub fn lineage_depth(&self, name: &str) -> usize {
        self.lineages.get(name).map(|l| l.len()).unwrap_or(0)
    }

    /// Get best fitness for a function from lineage.
    pub fn best_fitness(&self, name: &str) -> Option<f64> {
        self.lineages.get(name).and_then(|nodes| {
            nodes.iter().map(|n| n.fitness)
                .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        })
    }

    /// Total events recorded.
    pub fn total_events(&self) -> usize {
        self.events.len()
    }

    /// Improvement rate: events where fitness improved / total mutation+crossover events.
    pub fn improvement_rate(&self) -> f64 {
        let total = self.events.iter()
            .filter(|e| e.event_type == EventType::Mutation || e.event_type == EventType::Crossover)
            .count();
        if total == 0 { return 0.0; }
        let improved = self.events.iter()
            .filter(|e| e.event_type == EventType::Improvement)
            .count();
        improved as f64 / total as f64
    }

    /// Rollback count.
    pub fn rollback_count(&self) -> usize {
        self.events_by_type(EventType::Rollback).len()
    }

    /// Functions being tracked.
    pub fn tracked_functions(&self) -> Vec<String> {
        self.lineages.keys().cloned().collect()
    }

    /// Summary as JSON.
    pub fn summary_json(&self) -> String {
        format!(
            r#"{{"total_events":{},"tracked_functions":{},"improvement_rate":{:.4},"rollbacks":{}}}"#,
            self.total_events(),
            self.lineages.len(),
            self.improvement_rate(),
            self.rollback_count(),
        )
    }

    /// Export events as JSON array (last N events).
    pub fn events_json(&self, limit: usize) -> String {
        let events: Vec<String> = self.events.iter()
            .rev()
            .take(limit)
            .map(|e| format!(
                r#"{{"id":{},"type":"{}","function":"{}","gen":{},"fitness_delta":{:.6}}}"#,
                e.id, e.event_type.name(), e.function_name, e.generation, e.fitness_after - e.fitness_before
            ))
            .collect();
        format!("[{}]", events.join(","))
    }
}

// ─── FFI ──────────────────────────────────────────────────────────────

static GLOBAL_OBSERVATORY: LazyLock<Mutex<Observatory>> =
    LazyLock::new(|| Mutex::new(Observatory::new()));

#[unsafe(no_mangle)]
pub extern "C" fn slang_observatory_record(event_type: i64, generation: i64) -> i64 {
    let et = match event_type {
        0 => EventType::Mutation,
        1 => EventType::Crossover,
        2 => EventType::Selection,
        3 => EventType::Rollback,
        4 => EventType::Improvement,
        5 => EventType::Regression,
        _ => EventType::Mutation,
    };
    let mut obs = GLOBAL_OBSERVATORY.lock().unwrap();
    obs.record_event(et, "auto", generation as u64, 0.0, 0.0, "ffi") as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_observatory_total_events() -> i64 {
    let obs = GLOBAL_OBSERVATORY.lock().unwrap();
    obs.total_events() as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_observatory_rollback_count() -> i64 {
    let obs = GLOBAL_OBSERVATORY.lock().unwrap();
    obs.rollback_count() as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_observatory_tracked_functions() -> i64 {
    let obs = GLOBAL_OBSERVATORY.lock().unwrap();
    obs.tracked_functions().len() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fresh() -> Observatory {
        Observatory::new()
    }

    #[test]
    fn test_event_type_name() {
        assert_eq!(EventType::Mutation.name(), "mutation");
        assert_eq!(EventType::HealAction.name(), "heal");
    }

    #[test]
    fn test_observatory_new() {
        let o = fresh();
        assert_eq!(o.total_events(), 0);
    }

    #[test]
    fn test_record_event() {
        let mut o = fresh();
        let id = o.record_event(EventType::Mutation, "fn_a", 1, 0.5, 0.7, "mutated");
        assert_eq!(id, 1);
        assert_eq!(o.total_events(), 1);
    }

    #[test]
    fn test_events_by_type() {
        let mut o = fresh();
        o.record_event(EventType::Mutation, "a", 1, 0.0, 0.1, "");
        o.record_event(EventType::Crossover, "b", 1, 0.0, 0.2, "");
        o.record_event(EventType::Mutation, "c", 2, 0.0, 0.3, "");
        assert_eq!(o.events_by_type(EventType::Mutation).len(), 2);
        assert_eq!(o.events_by_type(EventType::Crossover).len(), 1);
    }

    #[test]
    fn test_events_for_function() {
        let mut o = fresh();
        o.record_event(EventType::Mutation, "fn_a", 1, 0.0, 0.1, "");
        o.record_event(EventType::Crossover, "fn_b", 1, 0.0, 0.2, "");
        o.record_event(EventType::Improvement, "fn_a", 2, 0.1, 0.3, "");
        assert_eq!(o.events_for_function("fn_a").len(), 2);
    }

    #[test]
    fn test_record_lineage() {
        let mut o = fresh();
        o.record_lineage("fn_a", 0, 0.5, None, "initial");
        o.record_lineage("fn_a", 1, 0.7, Some(0), "mutation");
        assert_eq!(o.lineage_depth("fn_a"), 2);
    }

    #[test]
    fn test_lineage_parent_child() {
        let mut o = fresh();
        o.record_lineage("f", 0, 0.5, None, "init");
        o.record_lineage("f", 1, 0.7, Some(0), "mut");
        let nodes = &o.lineages["f"];
        assert_eq!(nodes[0].children, vec![1]);
    }

    #[test]
    fn test_best_fitness() {
        let mut o = fresh();
        o.record_lineage("f", 0, 0.3, None, "init");
        o.record_lineage("f", 1, 0.9, Some(0), "mut");
        o.record_lineage("f", 2, 0.5, Some(1), "mut");
        assert!((o.best_fitness("f").unwrap() - 0.9).abs() < 1e-10);
    }

    #[test]
    fn test_best_fitness_none() {
        let o = fresh();
        assert!(o.best_fitness("nonexistent").is_none());
    }

    #[test]
    fn test_improvement_rate() {
        let mut o = fresh();
        o.record_event(EventType::Mutation, "a", 1, 0.0, 0.1, "");
        o.record_event(EventType::Mutation, "b", 1, 0.0, 0.2, "");
        o.record_event(EventType::Improvement, "a", 2, 0.1, 0.3, "");
        assert!((o.improvement_rate() - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_rollback_count() {
        let mut o = fresh();
        o.record_event(EventType::Rollback, "a", 1, 0.5, 0.3, "regression");
        assert_eq!(o.rollback_count(), 1);
    }

    #[test]
    fn test_tracked_functions() {
        let mut o = fresh();
        o.record_lineage("f1", 0, 0.5, None, "init");
        o.record_lineage("f2", 0, 0.3, None, "init");
        assert_eq!(o.tracked_functions().len(), 2);
    }

    #[test]
    fn test_summary_json() {
        let o = fresh();
        let json = o.summary_json();
        assert!(json.contains("total_events"));
        assert!(json.contains("improvement_rate"));
    }

    #[test]
    fn test_events_json() {
        let mut o = fresh();
        o.record_event(EventType::Mutation, "a", 1, 0.0, 0.1, "");
        let json = o.events_json(10);
        assert!(json.contains("mutation"));
    }

    #[test]
    fn test_max_events_cap() {
        let mut o = fresh();
        o.max_events = 5;
        for i in 0..10 {
            o.record_event(EventType::Mutation, "a", i, 0.0, 0.0, "");
        }
        assert_eq!(o.total_events(), 5);
    }

    #[test]
    fn test_ffi_record() {
        let id = slang_observatory_record(0, 1);
        assert!(id >= 1);
    }

    #[test]
    fn test_ffi_total_events() {
        let count = slang_observatory_total_events();
        assert!(count >= 0);
    }

    #[test]
    fn test_ffi_tracked() {
        let count = slang_observatory_tracked_functions();
        assert!(count >= 0);
    }
}
