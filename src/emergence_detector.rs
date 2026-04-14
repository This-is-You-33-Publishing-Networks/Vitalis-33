//! Emergence Detector — Vitalis v1031
//!
//! Identifies unexpected emergent properties in compiled programs—
//! behaviors that arise from component interactions but aren't in any single part.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<EmdState>> = LazyLock::new(|| Mutex::new(EmdState::default()));

#[derive(Default)]
struct EmdState {
    components: Vec<String>,
    interactions: Vec<(String, String, f64)>,
    emergent_properties: Vec<(String, f64)>,
}

pub struct EmergenceDetector;

impl EmergenceDetector {
    pub fn register_component(name: &str) -> usize {
        let mut s = STATE.lock().unwrap();
        s.components.push(name.to_string());
        s.components.len()
    }

    pub fn record_interaction(a: &str, b: &str, strength: f64) -> usize {
        let mut s = STATE.lock().unwrap();
        s.interactions.push((a.to_string(), b.to_string(), strength));
        s.interactions.len()
    }

    pub fn detect() -> Vec<String> {
        let mut s = STATE.lock().unwrap();
        let threshold = 0.5;
        let new_props: Vec<(String, f64)> = s.interactions.iter()
            .filter(|(_, _, strength)| *strength > threshold)
            .map(|(a, b, strength)| (format!("emergent_{}_{}", a, b), *strength))
            .collect();
        let found: Vec<String> = new_props.iter().map(|(p, _)| p.clone()).collect();
        s.emergent_properties.extend(new_props);
        found
    }

    pub fn emergent_count() -> usize { STATE.lock().unwrap().emergent_properties.len() }
    pub fn component_count() -> usize { STATE.lock().unwrap().components.len() }
    pub fn strongest_emergence() -> f64 {
        STATE.lock().unwrap().emergent_properties.iter().map(|(_, s)| *s).fold(0.0_f64, f64::max)
    }
    pub fn reset() { *STATE.lock().unwrap() = EmdState::default(); }
}

#[unsafe(no_mangle)]
pub extern "C" fn emd_register(id: i64) -> i64 { EmergenceDetector::register_component(&format!("c{}", id)) as i64 }
#[unsafe(no_mangle)]
pub extern "C" fn emd_detect() -> i64 { EmergenceDetector::detect().len() as i64 }
#[unsafe(no_mangle)]
pub extern "C" fn emd_count() -> i64 { EmergenceDetector::emergent_count() as i64 }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_register() { EmergenceDetector::reset(); assert_eq!(EmergenceDetector::register_component("a"), 1); }
    #[test] fn test_interaction() { EmergenceDetector::reset(); assert_eq!(EmergenceDetector::record_interaction("a", "b", 0.8), 1); }
    #[test] fn test_detect() { EmergenceDetector::reset(); EmergenceDetector::record_interaction("a", "b", 0.9); let props = EmergenceDetector::detect(); assert!(!props.is_empty()); }
    #[test] fn test_no_emergence() { EmergenceDetector::reset(); EmergenceDetector::record_interaction("a", "b", 0.1); let props = EmergenceDetector::detect(); assert!(props.is_empty()); }
    #[test] fn test_strongest() { EmergenceDetector::reset(); EmergenceDetector::record_interaction("a", "b", 0.9); EmergenceDetector::detect(); assert!(EmergenceDetector::strongest_emergence() > 0.8); }
    #[test] fn test_component_count() { EmergenceDetector::reset(); EmergenceDetector::register_component("x"); assert_eq!(EmergenceDetector::component_count(), 1); }
    #[test] fn test_ffi() { EmergenceDetector::reset(); assert_eq!(emd_count(), 0); }
}
