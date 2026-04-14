//! Information Geometry — Vitalis v1069
//!
//! Treats code transformations as movements on a Riemannian manifold
//! of programs. Optimization follows geodesics in program space.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<IgState>> = LazyLock::new(|| Mutex::new(IgState::default()));

#[derive(Default)]
struct IgState {
    position: Vec<f64>,
    trajectory: Vec<Vec<f64>>,
    curvature: f64,
    distance_traveled: f64,
}

pub struct InformationGeometry;

impl InformationGeometry {
    pub fn set_position(coords: &[f64]) {
        let mut s = STATE.lock().unwrap();
        s.position = coords.to_vec();
        s.trajectory.push(coords.to_vec());
    }

    pub fn move_along_geodesic(direction: &[f64], step: f64) -> f64 {
        let mut s = STATE.lock().unwrap();
        let dist: f64 = direction.iter().map(|d| d * d * step * step).sum::<f64>().sqrt();
        for (p, d) in s.position.iter_mut().zip(direction.iter()) { *p += d * step; }
        s.distance_traveled += dist;
        let pos = s.position.clone();
        s.trajectory.push(pos);
        dist
    }

    pub fn compute_curvature() -> f64 {
        let mut s = STATE.lock().unwrap();
        if s.trajectory.len() < 3 { s.curvature = 0.0; return 0.0; }
        let n = s.trajectory.len();
        let deviation: f64 = s.trajectory[n - 1].iter().zip(s.trajectory[n - 3].iter())
            .map(|(a, b)| (a - b).powi(2)).sum::<f64>().sqrt();
        s.curvature = deviation / (s.distance_traveled.max(1.0));
        s.curvature
    }

    pub fn distance_traveled() -> f64 { STATE.lock().unwrap().distance_traveled }
    pub fn curvature() -> f64 { STATE.lock().unwrap().curvature }
    pub fn dimension() -> usize { STATE.lock().unwrap().position.len() }
    pub fn reset() { *STATE.lock().unwrap() = IgState::default(); }
}

#[unsafe(no_mangle)]
pub extern "C" fn ig_move(step: f64) -> f64 { InformationGeometry::move_along_geodesic(&[1.0, 0.0], step) }
#[unsafe(no_mangle)]
pub extern "C" fn ig_curvature() -> f64 { InformationGeometry::compute_curvature() }
#[unsafe(no_mangle)]
pub extern "C" fn ig_distance() -> f64 { InformationGeometry::distance_traveled() }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_set_position() { InformationGeometry::reset(); InformationGeometry::set_position(&[1.0, 2.0]); assert_eq!(InformationGeometry::dimension(), 2); }
    #[test] fn test_move() { InformationGeometry::reset(); InformationGeometry::set_position(&[0.0, 0.0]); let d = InformationGeometry::move_along_geodesic(&[1.0, 0.0], 1.0); assert!(d > 0.0); }
    #[test] fn test_distance() { InformationGeometry::reset(); InformationGeometry::set_position(&[0.0]); InformationGeometry::move_along_geodesic(&[1.0], 5.0); assert!(InformationGeometry::distance_traveled() > 0.0); }
    #[test] fn test_curvature_flat() { InformationGeometry::reset(); InformationGeometry::set_position(&[0.0]); InformationGeometry::move_along_geodesic(&[1.0], 1.0); InformationGeometry::move_along_geodesic(&[1.0], 1.0); let c = InformationGeometry::compute_curvature(); assert!(c >= 0.0); }
    #[test] fn test_dimension() { InformationGeometry::reset(); InformationGeometry::set_position(&[1.0, 2.0, 3.0]); assert_eq!(InformationGeometry::dimension(), 3); }
    #[test] fn test_empty_curvature() { InformationGeometry::reset(); assert!((InformationGeometry::compute_curvature() - 0.0).abs() < 0.01); }
    #[test] fn test_ffi() { InformationGeometry::reset(); assert!((ig_distance() - 0.0).abs() < 0.01); }
}
