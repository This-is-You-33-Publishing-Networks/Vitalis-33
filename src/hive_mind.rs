//! Hive Mind — Vitalis v1001
//!
//! Distributed consciousness across multiple compiler instances,
//! sharing state, insights, and optimization knowledge collectively.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<HiveState>> = LazyLock::new(|| Mutex::new(HiveState::default()));

#[derive(Default)]
struct HiveState {
    nodes: Vec<String>,
    shared_insights: Vec<(String, f64)>,
    sync_generation: u64,
    collective_fitness: f64,
}

pub struct HiveMind;

impl HiveMind {
    pub fn join_node(id: &str) -> usize {
        let mut s = STATE.lock().unwrap();
        s.nodes.push(id.to_string());
        s.nodes.len()
    }

    pub fn share_insight(insight: &str, confidence: f64) -> f64 {
        let mut s = STATE.lock().unwrap();
        s.shared_insights.push((insight.to_string(), confidence));
        let avg = s.shared_insights.iter().map(|(_, c)| c).sum::<f64>()
            / s.shared_insights.len().max(1) as f64;
        s.collective_fitness = avg;
        avg
    }

    pub fn synchronize() -> u64 {
        let mut s = STATE.lock().unwrap();
        s.sync_generation += 1;
        s.sync_generation
    }

    pub fn collective_fitness() -> f64 {
        STATE.lock().unwrap().collective_fitness
    }

    pub fn node_count() -> usize {
        STATE.lock().unwrap().nodes.len()
    }

    pub fn insight_count() -> usize {
        STATE.lock().unwrap().shared_insights.len()
    }

    pub fn reset() {
        *STATE.lock().unwrap() = HiveState::default();
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn hive_join(id_len: i64) -> i64 {
    HiveMind::join_node(&format!("node_{}", id_len)) as i64
}
#[unsafe(no_mangle)]
pub extern "C" fn hive_share(confidence: f64) -> f64 {
    HiveMind::share_insight("ffi_insight", confidence)
}
#[unsafe(no_mangle)]
pub extern "C" fn hive_sync() -> i64 {
    HiveMind::synchronize() as i64
}
#[unsafe(no_mangle)]
pub extern "C" fn hive_fitness() -> f64 {
    HiveMind::collective_fitness()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_join() { HiveMind::reset(); assert_eq!(HiveMind::join_node("a"), 1); assert_eq!(HiveMind::join_node("b"), 2); }
    #[test] fn test_share() { HiveMind::reset(); let f = HiveMind::share_insight("opt1", 0.8); assert!((f - 0.8).abs() < 0.01); }
    #[test] fn test_sync() { HiveMind::reset(); assert_eq!(HiveMind::synchronize(), 1); assert_eq!(HiveMind::synchronize(), 2); }
    #[test] fn test_fitness() { HiveMind::reset(); HiveMind::share_insight("a", 0.5); HiveMind::share_insight("b", 1.0); assert!((HiveMind::collective_fitness() - 0.75).abs() < 0.01); }
    #[test] fn test_node_count() { HiveMind::reset(); HiveMind::join_node("x"); assert_eq!(HiveMind::node_count(), 1); }
    #[test] fn test_insight_count() { HiveMind::reset(); HiveMind::share_insight("i", 0.5); assert_eq!(HiveMind::insight_count(), 1); }
    #[test] fn test_reset() { HiveMind::reset(); HiveMind::join_node("z"); HiveMind::reset(); assert_eq!(HiveMind::node_count(), 0); }
    #[test] fn test_ffi() { HiveMind::reset(); assert_eq!(hive_join(1), 1); }
}
