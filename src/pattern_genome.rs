//! Pattern Genome — Vitalis v1146
//!
//! Design patterns as genetic material that can evolve and recombine.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<PgState>> = LazyLock::new(|| Mutex::new(PgState::default()));

#[derive(Default)]
struct PgState { genes: Vec<(String, Vec<u8>)>, mutations: u64, crossovers: u64 }

pub struct PatternGenome;

impl PatternGenome {
    pub fn encode_pattern(name: &str, dna: &[u8]) -> usize { let mut s = STATE.lock().unwrap(); s.genes.push((name.to_string(), dna.to_vec())); s.genes.len() }
    pub fn mutate(name: &str) -> bool {
        let mut s = STATE.lock().unwrap();
        if let Some(g) = s.genes.iter_mut().find(|(n, _)| n == name) { g.1.iter_mut().for_each(|b| *b = b.wrapping_add(1)); s.mutations += 1; true } else { false }
    }
    pub fn crossover(a: &str, b: &str) -> Option<Vec<u8>> {
        let mut s = STATE.lock().unwrap();
        let ga = s.genes.iter().find(|(n, _)| n == a).map(|(_, d)| d.clone());
        let gb = s.genes.iter().find(|(n, _)| n == b).map(|(_, d)| d.clone());
        if let (Some(da), Some(db)) = (ga, gb) { s.crossovers += 1; let mid = da.len().min(db.len()) / 2; Some([&da[..mid], &db[mid..]].concat()) } else { None }
    }
    pub fn gene_count() -> usize { STATE.lock().unwrap().genes.len() }
    pub fn mutations() -> u64 { STATE.lock().unwrap().mutations }
    pub fn reset() { *STATE.lock().unwrap() = PgState::default(); }
}

#[unsafe(no_mangle)]
pub extern "C" fn pg_encode(len: i64) -> i64 { PatternGenome::encode_pattern("ffi", &vec![0u8; len as usize]) as i64 }
#[unsafe(no_mangle)]
pub extern "C" fn pg_mutate() -> i64 { if PatternGenome::mutate("ffi") { 1 } else { 0 } }
#[unsafe(no_mangle)]
pub extern "C" fn pg_genes() -> i64 { PatternGenome::gene_count() as i64 }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_encode() { PatternGenome::reset(); assert_eq!(PatternGenome::encode_pattern("p1", &[1, 2, 3]), 1); }
    #[test] fn test_mutate() { PatternGenome::reset(); PatternGenome::encode_pattern("p1", &[1, 2]); assert!(PatternGenome::mutate("p1")); }
    #[test] fn test_mutate_miss() { PatternGenome::reset(); assert!(!PatternGenome::mutate("nope")); }
    #[test] fn test_crossover() { PatternGenome::reset(); PatternGenome::encode_pattern("a", &[1, 2, 3, 4]); PatternGenome::encode_pattern("b", &[5, 6, 7, 8]); let c = PatternGenome::crossover("a", "b"); assert!(c.is_some()); }
    #[test] fn test_crossover_miss() { PatternGenome::reset(); assert!(PatternGenome::crossover("a", "b").is_none()); }
    #[test] fn test_gene_count() { PatternGenome::reset(); PatternGenome::encode_pattern("a", &[1]); assert_eq!(PatternGenome::gene_count(), 1); }
    #[test] fn test_ffi() { PatternGenome::reset(); assert_eq!(pg_genes(), 0); }
}
