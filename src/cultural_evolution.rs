//! Cultural Evolution — Vitalis v1037
//!
//! Tracks and propagates successful coding idioms across projects
//! like cultural memes — patterns that survive because they work.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<CeState>> = LazyLock::new(|| Mutex::new(CeState::default()));

#[derive(Default)]
struct CeState {
    memes: Vec<(String, f64, u64)>,
    generations: u64,
    extinction_count: u64,
}

pub struct CulturalEvolution;

impl CulturalEvolution {
    pub fn introduce_meme(pattern: &str, fitness: f64) -> usize {
        let mut s = STATE.lock().unwrap();
        s.memes.push((pattern.to_string(), fitness, 0));
        s.memes.len()
    }

    pub fn propagate() -> usize {
        let mut s = STATE.lock().unwrap();
        s.generations += 1;
        for m in s.memes.iter_mut() {
            if m.1 > 0.5 { m.2 += 1; m.1 *= 1.01; }
            else { m.1 *= 0.9; }
        }
        let before = s.memes.len();
        s.memes.retain(|(_, f, _)| *f > 0.01);
        let extinct = before - s.memes.len();
        s.extinction_count += extinct as u64;
        s.memes.len()
    }

    pub fn dominant_meme() -> Option<String> {
        let s = STATE.lock().unwrap();
        s.memes.iter().max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal)).map(|(p, _, _)| p.clone())
    }

    pub fn meme_count() -> usize { STATE.lock().unwrap().memes.len() }
    pub fn generations() -> u64 { STATE.lock().unwrap().generations }
    pub fn extinctions() -> u64 { STATE.lock().unwrap().extinction_count }
    pub fn reset() { *STATE.lock().unwrap() = CeState::default(); }
}

#[unsafe(no_mangle)]
pub extern "C" fn ce_introduce(fitness: f64) -> i64 { CulturalEvolution::introduce_meme("ffi_meme", fitness) as i64 }
#[unsafe(no_mangle)]
pub extern "C" fn ce_propagate() -> i64 { CulturalEvolution::propagate() as i64 }
#[unsafe(no_mangle)]
pub extern "C" fn ce_generations() -> i64 { CulturalEvolution::generations() as i64 }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_introduce() { CulturalEvolution::reset(); assert_eq!(CulturalEvolution::introduce_meme("pattern_match", 0.8), 1); }
    #[test] fn test_propagate() { CulturalEvolution::reset(); CulturalEvolution::introduce_meme("a", 0.9); let n = CulturalEvolution::propagate(); assert_eq!(n, 1); }
    #[test] fn test_extinction() { CulturalEvolution::reset(); CulturalEvolution::introduce_meme("weak", 0.02); for _ in 0..20 { CulturalEvolution::propagate(); } assert_eq!(CulturalEvolution::meme_count(), 0); }
    #[test] fn test_dominant() { CulturalEvolution::reset(); CulturalEvolution::introduce_meme("a", 0.3); CulturalEvolution::introduce_meme("b", 0.9); assert_eq!(CulturalEvolution::dominant_meme(), Some("b".to_string())); }
    #[test] fn test_generations() { CulturalEvolution::reset(); CulturalEvolution::propagate(); assert_eq!(CulturalEvolution::generations(), 1); }
    #[test] fn test_ffi() { CulturalEvolution::reset(); assert_eq!(ce_generations(), 0); }
}
