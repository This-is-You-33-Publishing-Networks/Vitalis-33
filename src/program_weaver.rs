//! Program Weaver — Vitalis v1160
//!
//! Weaves multi-paradigm program fragments into coherent wholes.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<PwState>> = LazyLock::new(|| Mutex::new(PwState::default()));

#[derive(Default)]
struct PwState { fragments: Vec<(String, String)>, woven: Vec<String>, seams: u64 }

pub struct ProgramWeaver;

impl ProgramWeaver {
    pub fn add_fragment(name: &str, paradigm: &str) -> usize { let mut s = STATE.lock().unwrap(); s.fragments.push((name.to_string(), paradigm.to_string())); s.fragments.len() }
    pub fn weave() -> String {
        let mut s = STATE.lock().unwrap();
        let result = s.fragments.iter().map(|(n, _)| n.as_str()).collect::<Vec<_>>().join("+");
        s.seams += s.fragments.len().saturating_sub(1) as u64;
        s.woven.push(result.clone());
        result
    }
    pub fn fragment_count() -> usize { STATE.lock().unwrap().fragments.len() }
    pub fn woven_count() -> usize { STATE.lock().unwrap().woven.len() }
    pub fn seams() -> u64 { STATE.lock().unwrap().seams }
    pub fn reset() { *STATE.lock().unwrap() = PwState::default(); }
}

#[unsafe(no_mangle)]
pub extern "C" fn pw_add(id: i64) -> i64 { ProgramWeaver::add_fragment(&format!("f{}", id), "mixed") as i64 }
#[unsafe(no_mangle)]
pub extern "C" fn pw_weave() -> i64 { ProgramWeaver::weave(); 1 }
#[unsafe(no_mangle)]
pub extern "C" fn pw_fragments() -> i64 { ProgramWeaver::fragment_count() as i64 }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_add() { ProgramWeaver::reset(); assert_eq!(ProgramWeaver::add_fragment("f1", "func"), 1); }
    #[test] fn test_weave() { ProgramWeaver::reset(); ProgramWeaver::add_fragment("a", "func"); ProgramWeaver::add_fragment("b", "imp"); let w = ProgramWeaver::weave(); assert!(w.contains("a") && w.contains("b")); }
    #[test] fn test_woven_count() { ProgramWeaver::reset(); ProgramWeaver::add_fragment("a", "f"); ProgramWeaver::weave(); assert_eq!(ProgramWeaver::woven_count(), 1); }
    #[test] fn test_seams() { ProgramWeaver::reset(); ProgramWeaver::add_fragment("a", "f"); ProgramWeaver::add_fragment("b", "f"); ProgramWeaver::weave(); assert_eq!(ProgramWeaver::seams(), 1); }
    #[test] fn test_empty_weave() { ProgramWeaver::reset(); let w = ProgramWeaver::weave(); assert!(w.is_empty()); }
    #[test] fn test_fragment_count() { ProgramWeaver::reset(); ProgramWeaver::add_fragment("a", "f"); assert_eq!(ProgramWeaver::fragment_count(), 1); }
    #[test] fn test_ffi() { ProgramWeaver::reset(); assert_eq!(pw_fragments(), 0); }
    #[test] fn test_ffi_weave() { ProgramWeaver::reset(); assert_eq!(pw_weave(), 1); }
}
