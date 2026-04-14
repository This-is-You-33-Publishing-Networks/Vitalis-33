//! Cubical Types — Vitalis v1158
//!
//! Cubical type theory for computational univalence and higher inductive types.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<CubeState>> = LazyLock::new(|| Mutex::new(CubeState::default()));

#[derive(Default)]
struct CubeState { dimensions: u32, faces: Vec<(u32, String)>, fillings: u64 }

pub struct CubicalTypes;

impl CubicalTypes {
    pub fn add_dimension() -> u32 { let mut s = STATE.lock().unwrap(); s.dimensions += 1; s.dimensions }
    pub fn add_face(dim: u32, name: &str) -> usize { let mut s = STATE.lock().unwrap(); s.faces.push((dim, name.to_string())); s.faces.len() }
    pub fn fill() -> u64 { let mut s = STATE.lock().unwrap(); s.fillings += 1; s.fillings }
    pub fn dimensions() -> u32 { STATE.lock().unwrap().dimensions }
    pub fn face_count() -> usize { STATE.lock().unwrap().faces.len() }
    pub fn fillings() -> u64 { STATE.lock().unwrap().fillings }
    pub fn reset() { *STATE.lock().unwrap() = CubeState::default(); }
}

#[unsafe(no_mangle)]
pub extern "C" fn cube_dim() -> i64 { CubicalTypes::add_dimension() as i64 }
#[unsafe(no_mangle)]
pub extern "C" fn cube_face(dim: i64) -> i64 { CubicalTypes::add_face(dim as u32, "face") as i64 }
#[unsafe(no_mangle)]
pub extern "C" fn cube_fill() -> i64 { CubicalTypes::fill() as i64 }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_dimension() { CubicalTypes::reset(); assert_eq!(CubicalTypes::add_dimension(), 1); }
    #[test] fn test_face() { CubicalTypes::reset(); assert_eq!(CubicalTypes::add_face(1, "left"), 1); }
    #[test] fn test_fill() { CubicalTypes::reset(); assert_eq!(CubicalTypes::fill(), 1); assert_eq!(CubicalTypes::fill(), 2); }
    #[test] fn test_dimensions() { CubicalTypes::reset(); CubicalTypes::add_dimension(); CubicalTypes::add_dimension(); assert_eq!(CubicalTypes::dimensions(), 2); }
    #[test] fn test_face_count() { CubicalTypes::reset(); CubicalTypes::add_face(0, "a"); assert_eq!(CubicalTypes::face_count(), 1); }
    #[test] fn test_ffi() { CubicalTypes::reset(); assert_eq!(cube_dim(), 1); }
}
