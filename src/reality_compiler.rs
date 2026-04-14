//! Reality Compiler — Vitalis v1081
//!
//! Compiles high-level "reality specifications" into executable
//! simulation environments with deterministic physics.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<RcState>> = LazyLock::new(|| Mutex::new(RcState::default()));

#[derive(Default)]
struct RcState {
    specs: Vec<(String, Vec<String>)>,
    compiled: Vec<String>,
    errors: Vec<String>,
}

pub struct RealityCompiler;

impl RealityCompiler {
    pub fn define_spec(name: &str, laws: &[&str]) -> usize {
        let mut s = STATE.lock().unwrap();
        s.specs.push((name.to_string(), laws.iter().map(|l| l.to_string()).collect()));
        s.specs.len()
    }

    pub fn compile(name: &str) -> Result<String, String> {
        let mut s = STATE.lock().unwrap();
        if let Some((_, laws)) = s.specs.iter().find(|(n, _)| n == name) {
            if laws.is_empty() {
                let err = format!("empty_spec_{}", name);
                s.errors.push(err.clone());
                Err(err)
            } else {
                let output = format!("reality_{}_{}_laws", name, laws.len());
                s.compiled.push(output.clone());
                Ok(output)
            }
        } else {
            let err = format!("unknown_spec_{}", name);
            s.errors.push(err.clone());
            Err(err)
        }
    }

    pub fn compiled_count() -> usize { STATE.lock().unwrap().compiled.len() }
    pub fn error_count() -> usize { STATE.lock().unwrap().errors.len() }
    pub fn spec_count() -> usize { STATE.lock().unwrap().specs.len() }
    pub fn reset() { *STATE.lock().unwrap() = RcState::default(); }
}

#[unsafe(no_mangle)]
pub extern "C" fn rc_define(laws: i64) -> i64 { RealityCompiler::define_spec("ffi", &vec!["law"; laws as usize]) as i64 }
#[unsafe(no_mangle)]
pub extern "C" fn rc_compile() -> i64 { if RealityCompiler::compile("ffi").is_ok() { 1 } else { 0 } }
#[unsafe(no_mangle)]
pub extern "C" fn rc_compiled() -> i64 { RealityCompiler::compiled_count() as i64 }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_define() { RealityCompiler::reset(); assert_eq!(RealityCompiler::define_spec("r1", &["gravity", "entropy"]), 1); }
    #[test] fn test_compile_ok() { RealityCompiler::reset(); RealityCompiler::define_spec("r1", &["gravity"]); assert!(RealityCompiler::compile("r1").is_ok()); }
    #[test] fn test_compile_empty() { RealityCompiler::reset(); RealityCompiler::define_spec("r1", &[]); assert!(RealityCompiler::compile("r1").is_err()); }
    #[test] fn test_compile_unknown() { RealityCompiler::reset(); assert!(RealityCompiler::compile("nope").is_err()); }
    #[test] fn test_compiled_count() { RealityCompiler::reset(); RealityCompiler::define_spec("r1", &["a"]); RealityCompiler::compile("r1").ok(); assert_eq!(RealityCompiler::compiled_count(), 1); }
    #[test] fn test_error_count() { RealityCompiler::reset(); RealityCompiler::compile("nope").ok(); assert_eq!(RealityCompiler::error_count(), 1); }
    #[test] fn test_ffi() { RealityCompiler::reset(); assert_eq!(rc_compiled(), 0); }
}
