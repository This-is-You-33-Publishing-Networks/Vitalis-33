//! Virtual Physics Engine — Vitalis v1079
//!
//! Simulates physical laws within the compilation environment
//! for resource modeling, energy budgets, and constraint satisfaction.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<VpeState>> = LazyLock::new(|| Mutex::new(VpeState::default()));

#[derive(Default)]
struct VpeState {
    bodies: Vec<(String, f64, f64, f64)>,
    time: f64,
    gravity: f64,
}

pub struct VirtualPhysicsEngine;

impl VirtualPhysicsEngine {
    pub fn set_gravity(g: f64) { STATE.lock().unwrap().gravity = g; }

    pub fn add_body(name: &str, mass: f64, position: f64, velocity: f64) -> usize {
        let mut s = STATE.lock().unwrap();
        s.bodies.push((name.to_string(), mass, position, velocity));
        s.bodies.len()
    }

    pub fn step(dt: f64) -> f64 {
        let mut s = STATE.lock().unwrap();
        s.time += dt;
        let g = s.gravity;
        for b in s.bodies.iter_mut() {
            b.3 += g * dt;
            b.2 += b.3 * dt;
        }
        s.time
    }

    pub fn body_position(name: &str) -> Option<f64> {
        STATE.lock().unwrap().bodies.iter().find(|(n, _, _, _)| n == name).map(|(_, _, p, _)| *p)
    }

    pub fn body_count() -> usize { STATE.lock().unwrap().bodies.len() }
    pub fn time() -> f64 { STATE.lock().unwrap().time }
    pub fn reset() { *STATE.lock().unwrap() = VpeState::default(); }
}

#[unsafe(no_mangle)]
pub extern "C" fn vpe_add(mass: f64) -> i64 { VirtualPhysicsEngine::add_body("ffi", mass, 0.0, 0.0) as i64 }
#[unsafe(no_mangle)]
pub extern "C" fn vpe_step(dt: f64) -> f64 { VirtualPhysicsEngine::step(dt) }
#[unsafe(no_mangle)]
pub extern "C" fn vpe_time() -> f64 { VirtualPhysicsEngine::time() }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_add() { VirtualPhysicsEngine::reset(); assert_eq!(VirtualPhysicsEngine::add_body("a", 1.0, 0.0, 0.0), 1); }
    #[test] fn test_step() { VirtualPhysicsEngine::reset(); VirtualPhysicsEngine::add_body("a", 1.0, 0.0, 1.0); VirtualPhysicsEngine::step(1.0); assert!(VirtualPhysicsEngine::body_position("a").unwrap() > 0.0); }
    #[test] fn test_gravity() { VirtualPhysicsEngine::reset(); VirtualPhysicsEngine::set_gravity(-9.8); VirtualPhysicsEngine::add_body("a", 1.0, 100.0, 0.0); VirtualPhysicsEngine::step(1.0); assert!(VirtualPhysicsEngine::body_position("a").unwrap() < 100.0); }
    #[test] fn test_time() { VirtualPhysicsEngine::reset(); VirtualPhysicsEngine::step(0.5); VirtualPhysicsEngine::step(0.5); assert!((VirtualPhysicsEngine::time() - 1.0).abs() < 0.01); }
    #[test] fn test_missing_body() { VirtualPhysicsEngine::reset(); assert!(VirtualPhysicsEngine::body_position("nope").is_none()); }
    #[test] fn test_ffi() { VirtualPhysicsEngine::reset(); assert!((vpe_time() - 0.0).abs() < 0.01); }
}
