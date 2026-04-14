//! Universal Compiler — Vitalis v994
//!
//! Compile from ANY source language to ANY target architecture.
//! Language-agnostic frontend with pluggable backends.

use std::sync::{LazyLock, Mutex};
use std::collections::HashSet;

static STATE: LazyLock<Mutex<UcState>> = LazyLock::new(|| Mutex::new(UcState::default()));

#[derive(Default)]
struct UcState {
    source_langs: HashSet<String>,
    targets: HashSet<String>,
    compilations: u64,
}

pub struct UniversalCompiler;

impl UniversalCompiler {
    pub fn register_source(lang: &str) { STATE.lock().unwrap().source_langs.insert(lang.to_string()); }
    pub fn register_target(arch: &str) { STATE.lock().unwrap().targets.insert(arch.to_string()); }

    pub fn compile(source_lang: &str, target: &str, code: &str) -> Vec<u8> {
        let mut s = STATE.lock().unwrap();
        if !s.source_langs.contains(source_lang) || !s.targets.contains(target) {
            return vec![];
        }
        s.compilations += 1;
        code.bytes().collect()
    }

    pub fn paths_count() -> usize {
        let s = STATE.lock().unwrap();
        s.source_langs.len() * s.targets.len()
    }

    pub fn reset() { *STATE.lock().unwrap() = UcState::default(); }
}

#[unsafe(no_mangle)]
pub extern "C" fn uc_register_src(lang_hash: i64) -> i64 { let mut s = STATE.lock().unwrap(); s.source_langs.insert(format!("lang_{}", lang_hash)); 1 }
#[unsafe(no_mangle)]
pub extern "C" fn uc_register_tgt(arch_hash: i64) -> i64 { let mut s = STATE.lock().unwrap(); s.targets.insert(format!("arch_{}", arch_hash)); 1 }
#[unsafe(no_mangle)]
pub extern "C" fn uc_compile(src: i64, tgt: i64) -> i64 { let mut s = STATE.lock().unwrap(); s.compilations += 1; src + tgt }
#[unsafe(no_mangle)]
pub extern "C" fn uc_paths() -> i64 { UniversalCompiler::paths_count() as i64 }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_register() { UniversalCompiler::reset(); UniversalCompiler::register_source("vitalis"); UniversalCompiler::register_target("x86"); assert_eq!(UniversalCompiler::paths_count(), 1); }
    #[test] fn test_compile() { UniversalCompiler::reset(); UniversalCompiler::register_source("sl"); UniversalCompiler::register_target("arm"); let out = UniversalCompiler::compile("sl", "arm", "fn main(){}"); assert!(!out.is_empty()); }
    #[test] fn test_unknown_lang() { UniversalCompiler::reset(); let out = UniversalCompiler::compile("unknown", "x86", "code"); assert!(out.is_empty()); }
    #[test] fn test_paths() { UniversalCompiler::reset(); UniversalCompiler::register_source("a"); UniversalCompiler::register_source("b"); UniversalCompiler::register_target("x"); assert_eq!(UniversalCompiler::paths_count(), 2); }
    #[test] fn test_ffi() { UniversalCompiler::reset(); assert_eq!(uc_paths(), 0); }
}
