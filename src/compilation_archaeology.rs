//! Compilation Archaeology — Vitalis v1150
//!
//! Mines historical compilations for buried optimization wisdom.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<CaState>> = LazyLock::new(|| Mutex::new(CaState::default()));

#[derive(Default)]
struct CaState { artifacts: Vec<(String, u64, f64)>, excavations: u64, discoveries: Vec<String> }

pub struct CompilationArchaeology;

impl CompilationArchaeology {
    pub fn archive(name: &str, epoch: u64, quality: f64) -> usize { let mut s = STATE.lock().unwrap(); s.artifacts.push((name.to_string(), epoch, quality)); s.artifacts.len() }
    pub fn excavate(min_quality: f64) -> Vec<String> {
        let mut s = STATE.lock().unwrap();
        s.excavations += 1;
        let found: Vec<String> = s.artifacts.iter().filter(|(_, _, q)| *q >= min_quality).map(|(n, _, _)| n.clone()).collect();
        s.discoveries.extend(found.clone());
        found
    }
    pub fn artifact_count() -> usize { STATE.lock().unwrap().artifacts.len() }
    pub fn discovery_count() -> usize { STATE.lock().unwrap().discoveries.len() }
    pub fn excavations() -> u64 { STATE.lock().unwrap().excavations }
    pub fn reset() { *STATE.lock().unwrap() = CaState::default(); }
}

#[unsafe(no_mangle)]
pub extern "C" fn ca_archive(quality: f64) -> i64 { CompilationArchaeology::archive("ffi", 0, quality) as i64 }
#[unsafe(no_mangle)]
pub extern "C" fn ca_excavate(min: f64) -> i64 { CompilationArchaeology::excavate(min).len() as i64 }
#[unsafe(no_mangle)]
pub extern "C" fn ca_artifacts() -> i64 { CompilationArchaeology::artifact_count() as i64 }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_archive() { CompilationArchaeology::reset(); assert_eq!(CompilationArchaeology::archive("a", 100, 0.9), 1); }
    #[test] fn test_excavate() { CompilationArchaeology::reset(); CompilationArchaeology::archive("a", 1, 0.9); let found = CompilationArchaeology::excavate(0.5); assert_eq!(found.len(), 1); }
    #[test] fn test_no_finds() { CompilationArchaeology::reset(); CompilationArchaeology::archive("a", 1, 0.1); let found = CompilationArchaeology::excavate(0.5); assert!(found.is_empty()); }
    #[test] fn test_discovery_count() { CompilationArchaeology::reset(); CompilationArchaeology::archive("a", 1, 0.9); CompilationArchaeology::excavate(0.5); assert_eq!(CompilationArchaeology::discovery_count(), 1); }
    #[test] fn test_excavations() { CompilationArchaeology::reset(); CompilationArchaeology::excavate(0.0); assert_eq!(CompilationArchaeology::excavations(), 1); }
    #[test] fn test_ffi() { CompilationArchaeology::reset(); assert_eq!(ca_artifacts(), 0); }
}
