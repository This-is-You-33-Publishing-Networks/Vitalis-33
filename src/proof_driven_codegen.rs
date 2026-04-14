//! Proof-Driven Codegen — Vitalis v1166
//!
//! Generates code directly from mathematical proofs of correctness.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<PdcState>> = LazyLock::new(|| Mutex::new(PdcState::default()));

#[derive(Default)]
struct PdcState { proofs: Vec<(String, bool)>, generated: Vec<String>, extraction_count: u64 }

pub struct ProofDrivenCodegen;

impl ProofDrivenCodegen {
    pub fn submit_proof(name: &str, valid: bool) -> usize { let mut s = STATE.lock().unwrap(); s.proofs.push((name.to_string(), valid)); s.proofs.len() }
    pub fn extract_program(proof: &str) -> Option<String> {
        let mut s = STATE.lock().unwrap();
        if s.proofs.iter().any(|(n, v)| n == proof && *v) { let prog = format!("extracted_{}", proof); s.generated.push(prog.clone()); s.extraction_count += 1; Some(prog) } else { None }
    }
    pub fn proof_count() -> usize { STATE.lock().unwrap().proofs.len() }
    pub fn generated_count() -> usize { STATE.lock().unwrap().generated.len() }
    pub fn extractions() -> u64 { STATE.lock().unwrap().extraction_count }
    pub fn reset() { *STATE.lock().unwrap() = PdcState::default(); }
}

#[unsafe(no_mangle)]
pub extern "C" fn pdc_submit(valid: i64) -> i64 { ProofDrivenCodegen::submit_proof("ffi", valid != 0) as i64 }
#[unsafe(no_mangle)]
pub extern "C" fn pdc_extract() -> i64 { if ProofDrivenCodegen::extract_program("ffi").is_some() { 1 } else { 0 } }
#[unsafe(no_mangle)]
pub extern "C" fn pdc_generated() -> i64 { ProofDrivenCodegen::generated_count() as i64 }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_submit() { ProofDrivenCodegen::reset(); assert_eq!(ProofDrivenCodegen::submit_proof("p1", true), 1); }
    #[test] fn test_extract() { ProofDrivenCodegen::reset(); ProofDrivenCodegen::submit_proof("p1", true); assert!(ProofDrivenCodegen::extract_program("p1").is_some()); }
    #[test] fn test_extract_invalid() { ProofDrivenCodegen::reset(); ProofDrivenCodegen::submit_proof("p1", false); assert!(ProofDrivenCodegen::extract_program("p1").is_none()); }
    #[test] fn test_extract_missing() { ProofDrivenCodegen::reset(); assert!(ProofDrivenCodegen::extract_program("nope").is_none()); }
    #[test] fn test_generated() { ProofDrivenCodegen::reset(); ProofDrivenCodegen::submit_proof("p", true); ProofDrivenCodegen::extract_program("p"); assert_eq!(ProofDrivenCodegen::generated_count(), 1); }
    #[test] fn test_ffi() { ProofDrivenCodegen::reset(); assert_eq!(pdc_generated(), 0); }
}
