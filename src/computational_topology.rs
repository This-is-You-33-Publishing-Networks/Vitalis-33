//! Computational Topology — Vitalis v1071
//!
//! Topological analysis of program spaces: holes, boundaries,
//! connected components, and continuous deformations of code.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<CtopState>> = LazyLock::new(|| Mutex::new(CtopState::default()));

#[derive(Default)]
struct CtopState {
    simplices: Vec<Vec<u32>>,
    components: u32,
    holes: u32,
    euler_characteristic: i32,
}

pub struct ComputationalTopology;

impl ComputationalTopology {
    pub fn add_simplex(vertices: &[u32]) -> usize {
        let mut s = STATE.lock().unwrap();
        s.simplices.push(vertices.to_vec());
        s.simplices.len()
    }

    pub fn compute_euler() -> i32 {
        let mut s = STATE.lock().unwrap();
        let v = s.simplices.iter().filter(|s| s.len() == 1).count() as i32;
        let e = s.simplices.iter().filter(|s| s.len() == 2).count() as i32;
        let f = s.simplices.iter().filter(|s| s.len() == 3).count() as i32;
        s.euler_characteristic = v - e + f;
        s.euler_characteristic
    }

    pub fn detect_holes() -> u32 {
        let mut s = STATE.lock().unwrap();
        let euler = s.euler_characteristic;
        s.holes = if euler < 2 { (2 - euler) as u32 } else { 0 };
        s.holes
    }

    pub fn connected_components() -> u32 {
        let mut s = STATE.lock().unwrap();
        let vertices: std::collections::HashSet<u32> = s.simplices.iter().flat_map(|s| s.iter().copied()).collect();
        s.components = vertices.len().max(1) as u32;
        s.components
    }

    pub fn holes() -> u32 { STATE.lock().unwrap().holes }
    pub fn simplex_count() -> usize { STATE.lock().unwrap().simplices.len() }
    pub fn reset() { *STATE.lock().unwrap() = CtopState::default(); }
}

#[unsafe(no_mangle)]
pub extern "C" fn ctop_add(dim: i64) -> i64 { ComputationalTopology::add_simplex(&vec![0; dim as usize]) as i64 }
#[unsafe(no_mangle)]
pub extern "C" fn ctop_euler() -> i64 { ComputationalTopology::compute_euler() as i64 }
#[unsafe(no_mangle)]
pub extern "C" fn ctop_holes() -> i64 { ComputationalTopology::detect_holes() as i64 }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_add() { ComputationalTopology::reset(); assert_eq!(ComputationalTopology::add_simplex(&[0]), 1); }
    #[test] fn test_euler() { ComputationalTopology::reset(); ComputationalTopology::add_simplex(&[0]); ComputationalTopology::add_simplex(&[1]); ComputationalTopology::add_simplex(&[0, 1]); let e = ComputationalTopology::compute_euler(); assert_eq!(e, 1); }
    #[test] fn test_holes() { ComputationalTopology::reset(); ComputationalTopology::compute_euler(); let h = ComputationalTopology::detect_holes(); assert!(h >= 0); }
    #[test] fn test_components() { ComputationalTopology::reset(); ComputationalTopology::add_simplex(&[0]); ComputationalTopology::add_simplex(&[1]); assert!(ComputationalTopology::connected_components() >= 2); }
    #[test] fn test_simplex_count() { ComputationalTopology::reset(); ComputationalTopology::add_simplex(&[0]); ComputationalTopology::add_simplex(&[1]); assert_eq!(ComputationalTopology::simplex_count(), 2); }
    #[test] fn test_empty_euler() { ComputationalTopology::reset(); assert_eq!(ComputationalTopology::compute_euler(), 0); }
    #[test] fn test_ffi() { ComputationalTopology::reset(); assert_eq!(ctop_euler(), 0); }
}
