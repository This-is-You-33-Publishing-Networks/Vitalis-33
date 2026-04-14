//! Attractor Landscape — Vitalis v1239
//!
//! Maps attractor landscape of evolutionary dynamics.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<AttrState>> = LazyLock::new(|| Mutex::new(AttrState::default()));

#[derive(Default)]
struct AttrState {
    attractors: Vec<(String, f64, f64)>,
    basins: Vec<String>,
    mapped_area: f64,
}

pub struct AttractorLandscape;

impl AttractorLandscape {
    pub fn add_attractor(name: &str, x: f64, y: f64) {
        let mut s = STATE.lock().unwrap();
        s.attractors.push((name.to_string(), x, y));
        s.mapped_area += (x.abs() + y.abs()) * 0.1;
    }

    pub fn add_basin(name: &str) {
        let mut s = STATE.lock().unwrap();
        s.basins.push(name.to_string());
    }

    pub fn attractor_count() -> usize { STATE.lock().unwrap().attractors.len() }

    pub fn basin_count() -> usize { STATE.lock().unwrap().basins.len() }

    pub fn mapped_area() -> f64 { STATE.lock().unwrap().mapped_area }

    pub fn reset() { *STATE.lock().unwrap() = AttrState::default(); }
}

#[unsafe(no_mangle)]
pub extern "C" fn attr_add(x: f64, y: f64) -> i64 { AttractorLandscape::add_attractor("ffi", x, y); AttractorLandscape::attractor_count() as i64 }
#[unsafe(no_mangle)]
pub extern "C" fn attr_count() -> i64 { AttractorLandscape::attractor_count() as i64 }
#[unsafe(no_mangle)]
pub extern "C" fn attr_area() -> f64 { AttractorLandscape::mapped_area() }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_add_attractor() { AttractorLandscape::reset(); AttractorLandscape::add_attractor("fixed", 1.0, 2.0); assert_eq!(AttractorLandscape::attractor_count(), 1); }
    #[test] fn test_basin() { AttractorLandscape::reset(); AttractorLandscape::add_basin("basin1"); assert_eq!(AttractorLandscape::basin_count(), 1); }
    #[test] fn test_mapped_area() { AttractorLandscape::reset(); AttractorLandscape::add_attractor("a", 5.0, 5.0); assert!(AttractorLandscape::mapped_area() > 0.0); }
    #[test] fn test_multiple_attractors() { AttractorLandscape::reset(); AttractorLandscape::add_attractor("a", 1.0, 1.0); AttractorLandscape::add_attractor("b", 2.0, 2.0); assert_eq!(AttractorLandscape::attractor_count(), 2); }
    #[test] fn test_area_accumulates() { AttractorLandscape::reset(); AttractorLandscape::add_attractor("a", 1.0, 1.0); let a1 = AttractorLandscape::mapped_area(); AttractorLandscape::add_attractor("b", 1.0, 1.0); assert!(AttractorLandscape::mapped_area() > a1); }
    #[test] fn test_reset() { AttractorLandscape::reset(); AttractorLandscape::add_attractor("x", 1.0, 1.0); AttractorLandscape::reset(); assert_eq!(AttractorLandscape::attractor_count(), 0); }
    #[test] fn test_ffi() { AttractorLandscape::reset(); attr_add(3.0, 4.0); assert_eq!(attr_count(), 1); }
}
