//! Reality Model — Vitalis v1011
//!
//! Internal world model representing the program's execution environment.
//! Tracks state, side effects, and environmental constraints.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<RmState>> = LazyLock::new(|| Mutex::new(RmState::default()));

#[derive(Default)]
struct RmState {
    entities: Vec<(String, String)>,
    constraints: Vec<String>,
    predictions: Vec<(String, f64)>,
    fidelity: f64,
}

pub struct RealityModel;

impl RealityModel {
    pub fn add_entity(name: &str, kind: &str) -> usize {
        let mut s = STATE.lock().unwrap();
        s.entities.push((name.to_string(), kind.to_string()));
        s.fidelity = 1.0 - (1.0 / (s.entities.len() as f64 + 1.0));
        s.entities.len()
    }

    pub fn add_constraint(rule: &str) {
        let mut s = STATE.lock().unwrap();
        s.constraints.push(rule.to_string());
    }

    pub fn predict(event: &str) -> f64 {
        let mut s = STATE.lock().unwrap();
        let conf = s.fidelity * (s.entities.len() as f64 * 0.1).min(1.0);
        s.predictions.push((event.to_string(), conf));
        conf
    }

    pub fn fidelity() -> f64 { STATE.lock().unwrap().fidelity }
    pub fn entity_count() -> usize { STATE.lock().unwrap().entities.len() }
    pub fn prediction_count() -> usize { STATE.lock().unwrap().predictions.len() }
    pub fn reset() { *STATE.lock().unwrap() = RmState::default(); }
}

#[unsafe(no_mangle)]
pub extern "C" fn rm_add_entity(kind: i64) -> i64 { RealityModel::add_entity(&format!("e{}", kind), "object") as i64 }
#[unsafe(no_mangle)]
pub extern "C" fn rm_predict() -> f64 { RealityModel::predict("ffi_event") }
#[unsafe(no_mangle)]
pub extern "C" fn rm_fidelity() -> f64 { RealityModel::fidelity() }
#[unsafe(no_mangle)]
pub extern "C" fn rm_entities() -> i64 { RealityModel::entity_count() as i64 }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_add_entity() { RealityModel::reset(); assert_eq!(RealityModel::add_entity("x", "obj"), 1); }
    #[test] fn test_constraint() { RealityModel::reset(); RealityModel::add_constraint("no_null"); let s = STATE.lock().unwrap(); assert_eq!(s.constraints.len(), 1); }
    #[test] fn test_predict() { RealityModel::reset(); RealityModel::add_entity("a", "t"); let p = RealityModel::predict("crash"); assert!(p >= 0.0); }
    #[test] fn test_fidelity_grows() { RealityModel::reset(); RealityModel::add_entity("a", "t"); let f1 = RealityModel::fidelity(); RealityModel::add_entity("b", "t"); assert!(RealityModel::fidelity() > f1); }
    #[test] fn test_empty_fidelity() { RealityModel::reset(); assert!((RealityModel::fidelity() - 0.0).abs() < 0.01); }
    #[test] fn test_prediction_count() { RealityModel::reset(); RealityModel::predict("e"); assert_eq!(RealityModel::prediction_count(), 1); }
    #[test] fn test_entity_count() { RealityModel::reset(); RealityModel::add_entity("a", "t"); RealityModel::add_entity("b", "t"); assert_eq!(RealityModel::entity_count(), 2); }
    #[test] fn test_ffi() { RealityModel::reset(); assert_eq!(rm_entities(), 0); }
}
