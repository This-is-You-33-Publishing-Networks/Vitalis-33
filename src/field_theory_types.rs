//! Field Theory Types — Vitalis v1101
//!
//! Type system based on quantum field theory: types as fields,
//! values as excitations, type transformations as gauge symmetries.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<FttState>> = LazyLock::new(|| Mutex::new(FttState::default()));

#[derive(Default)]
struct FttState {
    fields: Vec<(String, f64)>,
    excitations: Vec<(String, String, f64)>,
    gauge_transforms: u64,
}

pub struct FieldTheoryTypes;

impl FieldTheoryTypes {
    pub fn define_field(name: &str, coupling: f64) -> usize {
        let mut s = STATE.lock().unwrap();
        s.fields.push((name.to_string(), coupling));
        s.fields.len()
    }

    pub fn excite(field: &str, particle: &str, energy: f64) -> f64 {
        let mut s = STATE.lock().unwrap();
        let coupling = s.fields.iter().find(|(f, _)| f == field).map(|(_, c)| *c).unwrap_or(1.0);
        let effective = energy * coupling;
        s.excitations.push((field.to_string(), particle.to_string(), effective));
        effective
    }

    pub fn gauge_transform(field: &str) -> f64 {
        let mut s = STATE.lock().unwrap();
        s.gauge_transforms += 1;
        s.excitations.iter().filter(|(f, _, _)| f == field).map(|(_, _, e)| e).sum()
    }

    pub fn field_count() -> usize { STATE.lock().unwrap().fields.len() }
    pub fn excitation_count() -> usize { STATE.lock().unwrap().excitations.len() }
    pub fn transforms() -> u64 { STATE.lock().unwrap().gauge_transforms }
    pub fn reset() { *STATE.lock().unwrap() = FttState::default(); }
}

#[unsafe(no_mangle)]
pub extern "C" fn ftt_field(coupling: f64) -> i64 { FieldTheoryTypes::define_field("ffi_field", coupling) as i64 }
#[unsafe(no_mangle)]
pub extern "C" fn ftt_excite(energy: f64) -> f64 { FieldTheoryTypes::excite("ffi_field", "photon", energy) }
#[unsafe(no_mangle)]
pub extern "C" fn ftt_fields() -> i64 { FieldTheoryTypes::field_count() as i64 }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_field() { FieldTheoryTypes::reset(); assert_eq!(FieldTheoryTypes::define_field("em", 0.5), 1); }
    #[test] fn test_excite() { FieldTheoryTypes::reset(); FieldTheoryTypes::define_field("em", 2.0); let e = FieldTheoryTypes::excite("em", "photon", 5.0); assert!((e - 10.0).abs() < 0.01); }
    #[test] fn test_gauge() { FieldTheoryTypes::reset(); FieldTheoryTypes::define_field("em", 1.0); FieldTheoryTypes::excite("em", "a", 3.0); let total = FieldTheoryTypes::gauge_transform("em"); assert!((total - 3.0).abs() < 0.01); }
    #[test] fn test_no_field() { FieldTheoryTypes::reset(); let e = FieldTheoryTypes::excite("none", "p", 5.0); assert!((e - 5.0).abs() < 0.01); }
    #[test] fn test_field_count() { FieldTheoryTypes::reset(); FieldTheoryTypes::define_field("a", 1.0); FieldTheoryTypes::define_field("b", 2.0); assert_eq!(FieldTheoryTypes::field_count(), 2); }
    #[test] fn test_excitation_count() { FieldTheoryTypes::reset(); FieldTheoryTypes::excite("f", "p", 1.0); assert_eq!(FieldTheoryTypes::excitation_count(), 1); }
    #[test] fn test_ffi() { FieldTheoryTypes::reset(); assert_eq!(ftt_fields(), 0); }
}
