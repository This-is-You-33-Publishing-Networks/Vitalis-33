//! Omega Arithmetic — Vitalis v1231
//!
//! Arithmetic on transfinite quantities for resource analysis.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<OAState>> = LazyLock::new(|| Mutex::new(OAState::default()));

#[derive(Default)]
struct OAState {
    values: Vec<(String, f64)>,
    operations: u64,
}

pub struct OmegaArithmetic;

impl OmegaArithmetic {
    pub fn define(name: &str, value: f64) {
        let mut s = STATE.lock().unwrap();
        s.values.push((name.to_string(), value));
    }

    pub fn add(a: &str, b: &str) -> f64 {
        let mut s = STATE.lock().unwrap();
        s.operations += 1;
        let va = s.values.iter().find(|(n, _)| n == a).map(|(_, v)| *v).unwrap_or(0.0);
        let vb = s.values.iter().find(|(n, _)| n == b).map(|(_, v)| *v).unwrap_or(0.0);
        va + vb
    }

    pub fn multiply(a: &str, b: &str) -> f64 {
        let mut s = STATE.lock().unwrap();
        s.operations += 1;
        let va = s.values.iter().find(|(n, _)| n == a).map(|(_, v)| *v).unwrap_or(0.0);
        let vb = s.values.iter().find(|(n, _)| n == b).map(|(_, v)| *v).unwrap_or(0.0);
        va * vb
    }

    pub fn value_count() -> usize { STATE.lock().unwrap().values.len() }

    pub fn operations() -> u64 { STATE.lock().unwrap().operations }

    pub fn reset() { *STATE.lock().unwrap() = OAState::default(); }
}

#[unsafe(no_mangle)]
pub extern "C" fn oa_define(val: f64) -> i64 { OmegaArithmetic::define("ffi", val); OmegaArithmetic::value_count() as i64 }
#[unsafe(no_mangle)]
pub extern "C" fn oa_ops() -> i64 { OmegaArithmetic::operations() as i64 }
#[unsafe(no_mangle)]
pub extern "C" fn oa_count() -> i64 { OmegaArithmetic::value_count() as i64 }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_define() { OmegaArithmetic::reset(); OmegaArithmetic::define("omega", f64::INFINITY); assert_eq!(OmegaArithmetic::value_count(), 1); }
    #[test] fn test_add() { OmegaArithmetic::reset(); OmegaArithmetic::define("a", 3.0); OmegaArithmetic::define("b", 5.0); assert!((OmegaArithmetic::add("a", "b") - 8.0).abs() < f64::EPSILON); }
    #[test] fn test_multiply() { OmegaArithmetic::reset(); OmegaArithmetic::define("x", 4.0); OmegaArithmetic::define("y", 7.0); assert!((OmegaArithmetic::multiply("x", "y") - 28.0).abs() < f64::EPSILON); }
    #[test] fn test_operations() { OmegaArithmetic::reset(); OmegaArithmetic::define("a", 1.0); OmegaArithmetic::define("b", 2.0); OmegaArithmetic::add("a", "b"); assert_eq!(OmegaArithmetic::operations(), 1); }
    #[test] fn test_reset() { OmegaArithmetic::reset(); OmegaArithmetic::define("z", 1.0); OmegaArithmetic::reset(); assert_eq!(OmegaArithmetic::value_count(), 0); }
    #[test] fn test_ffi() { OmegaArithmetic::reset(); oa_define(42.0); assert_eq!(oa_count(), 1); }
}
