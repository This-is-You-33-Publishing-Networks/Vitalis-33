//! Code Analogy Engine — Vitalis v607
//!
//! Finds structural similarities between code fragments:
//! - AST structural fingerprinting (shape hash)
//! - Control flow pattern matching
//! - Variable role analysis (iterator, accumulator, guard, pivot)
//! - Code clone detection (Type-1/2/3 clones)
//! - Analogy-based code suggestion

use std::collections::HashMap;

// ── Structural Fingerprint ───────────────────────────────────────────

/// A structural fingerprint of a code fragment.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StructFingerprint {
    pub shape: Vec<NodeShape>,
    pub depth: usize,
    pub hash: u64,
}

/// Abstracted AST node shape (type without content).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NodeShape {
    Literal,
    Variable,
    BinaryOp,
    UnaryOp,
    Call,
    Index,
    Field,
    If,
    Loop,
    Match,
    Return,
    Assign,
    Block,
    Lambda,
}

impl StructFingerprint {
    pub fn new(shapes: Vec<NodeShape>, depth: usize) -> Self {
        let hash = Self::compute_hash(&shapes, depth);
        Self { shape: shapes, depth, hash }
    }

    fn compute_hash(shapes: &[NodeShape], depth: usize) -> u64 {
        let mut h: u64 = 0xcbf29ce484222325; // FNV-1a offset basis
        h ^= depth as u64;
        h = h.wrapping_mul(0x100000001b3);
        for s in shapes {
            h ^= *s as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        h
    }

    /// Structural similarity: Jaccard on shape multisets.
    pub fn similarity(&self, other: &StructFingerprint) -> f64 {
        if self.shape.is_empty() && other.shape.is_empty() { return 1.0; }
        if self.shape.is_empty() || other.shape.is_empty() { return 0.0; }

        let mut count_a: HashMap<NodeShape, usize> = HashMap::new();
        let mut count_b: HashMap<NodeShape, usize> = HashMap::new();
        for &s in &self.shape { *count_a.entry(s).or_insert(0) += 1; }
        for &s in &other.shape { *count_b.entry(s).or_insert(0) += 1; }

        let mut intersection = 0usize;
        let mut union = 0usize;
        let all_keys: std::collections::HashSet<NodeShape> =
            count_a.keys().chain(count_b.keys()).copied().collect();
        for k in all_keys {
            let a = count_a.get(&k).copied().unwrap_or(0);
            let b = count_b.get(&k).copied().unwrap_or(0);
            intersection += a.min(b);
            union += a.max(b);
        }
        if union == 0 { 0.0 } else { intersection as f64 / union as f64 }
    }
}

// ── Variable Role Analysis ───────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum VarRole {
    Iterator,       // Loop index / iteration variable
    Accumulator,    // Running total / collector
    Guard,          // Condition variable for early exit
    Pivot,          // Divide-and-conquer split point
    Temporary,      // Short-lived intermediate
    Parameter,      // Function input
    Result,         // Value being computed for return
    Counter,        // Counting occurrences
    Flag,           // Boolean state flag
    Bound,          // Loop bound / limit
}

impl VarRole {
    pub fn label(&self) -> &'static str {
        match self {
            VarRole::Iterator => "iterator",
            VarRole::Accumulator => "accumulator",
            VarRole::Guard => "guard",
            VarRole::Pivot => "pivot",
            VarRole::Temporary => "temporary",
            VarRole::Parameter => "parameter",
            VarRole::Result => "result",
            VarRole::Counter => "counter",
            VarRole::Flag => "flag",
            VarRole::Bound => "bound",
        }
    }
}

/// Heuristic variable role classifier.
pub fn classify_variable_role(
    name: &str,
    is_loop_var: bool,
    is_mutated_in_loop: bool,
    is_param: bool,
    is_bool: bool,
    is_returned: bool,
) -> VarRole {
    let lower = name.to_lowercase();

    if is_param { return VarRole::Parameter; }
    if is_returned { return VarRole::Result; }
    if is_loop_var && !is_mutated_in_loop { return VarRole::Iterator; }

    if is_mutated_in_loop {
        if lower.contains("sum") || lower.contains("total") || lower.contains("acc") {
            return VarRole::Accumulator;
        }
        if lower.contains("count") || lower.contains("cnt") {
            return VarRole::Counter;
        }
        return VarRole::Accumulator; // default for mutated-in-loop
    }

    if is_bool || lower.contains("flag") || lower.contains("found") || lower.contains("done") {
        return VarRole::Flag;
    }

    if lower.contains("mid") || lower.contains("pivot") || lower.contains("split") {
        return VarRole::Pivot;
    }

    if lower.contains("max") || lower.contains("min") || lower.contains("limit") || lower.contains("bound") || lower.contains("len") {
        return VarRole::Bound;
    }

    if lower.contains("tmp") || lower.contains("temp") || lower.len() <= 2 {
        return VarRole::Temporary;
    }

    VarRole::Temporary
}

// ── Clone Detection ──────────────────────────────────────────────────

/// Clone type classification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CloneType {
    Type1,  // Exact textual match (modulo whitespace)
    Type2,  // Same structure, different identifiers/literals
    Type3,  // Similar structure with modifications (>= threshold)
    None,   // Not a clone
}

/// Detect clone type between two structural fingerprints.
pub fn detect_clone(a: &StructFingerprint, b: &StructFingerprint, threshold: f64) -> CloneType {
    if a.hash == b.hash && a.shape == b.shape && a.depth == b.depth {
        return CloneType::Type1;
    }

    let sim = a.similarity(b);

    if sim >= 1.0 && a.shape.len() == b.shape.len() {
        return CloneType::Type2;
    }

    if sim >= threshold {
        return CloneType::Type3;
    }

    CloneType::None
}

// ── Analogy Mapping ──────────────────────────────────────────────────

/// A mapping between analogous elements in two code fragments.
#[derive(Debug, Clone)]
pub struct AnalogyMapping {
    pub source_elements: Vec<String>,
    pub target_elements: Vec<String>,
    pub mappings: Vec<(usize, usize, f64)>, // (src_idx, tgt_idx, confidence)
}

impl AnalogyMapping {
    pub fn new() -> Self {
        Self { source_elements: Vec::new(), target_elements: Vec::new(), mappings: Vec::new() }
    }

    pub fn add_source(&mut self, elem: &str) -> usize {
        self.source_elements.push(elem.to_string());
        self.source_elements.len() - 1
    }

    pub fn add_target(&mut self, elem: &str) -> usize {
        self.target_elements.push(elem.to_string());
        self.target_elements.len() - 1
    }

    pub fn map(&mut self, src: usize, tgt: usize, confidence: f64) {
        self.mappings.push((src, tgt, confidence));
    }

    pub fn overall_confidence(&self) -> f64 {
        if self.mappings.is_empty() { return 0.0; }
        let sum: f64 = self.mappings.iter().map(|(_, _, c)| c).sum();
        sum / self.mappings.len() as f64
    }

    pub fn mapped_count(&self) -> usize { self.mappings.len() }
}

impl Default for AnalogyMapping {
    fn default() -> Self { Self::new() }
}

// ── FFI Exports ──────────────────────────────────────────────────────

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_analogy_fingerprint_hash(shapes: *const i64, count: i64, depth: i64) -> i64 {
    if shapes.is_null() || count <= 0 { return 0; }
    let s = unsafe { std::slice::from_raw_parts(shapes, count as usize) };
    let mut h: u64 = 0xcbf29ce484222325;
    h ^= depth as u64;
    h = h.wrapping_mul(0x100000001b3);
    for &v in s {
        h ^= v as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_analogy_similarity(
    a: *const i64, a_len: i64,
    b: *const i64, b_len: i64,
) -> f64 {
    if a.is_null() || b.is_null() || a_len <= 0 || b_len <= 0 { return 0.0; }
    let a_s = unsafe { std::slice::from_raw_parts(a, a_len as usize) };
    let b_s = unsafe { std::slice::from_raw_parts(b, b_len as usize) };

    let mut count_a: HashMap<i64, usize> = HashMap::new();
    let mut count_b: HashMap<i64, usize> = HashMap::new();
    for &v in a_s { *count_a.entry(v).or_insert(0) += 1; }
    for &v in b_s { *count_b.entry(v).or_insert(0) += 1; }

    let all_keys: std::collections::HashSet<i64> = count_a.keys().chain(count_b.keys()).copied().collect();
    let mut inter = 0usize;
    let mut uni = 0usize;
    for k in all_keys {
        let ca = count_a.get(&k).copied().unwrap_or(0);
        let cb = count_b.get(&k).copied().unwrap_or(0);
        inter += ca.min(cb);
        uni += ca.max(cb);
    }
    if uni == 0 { 0.0 } else { inter as f64 / uni as f64 }
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_analogy_clone_type(similarity: f64, exact_match: i64, same_length: i64) -> i64 {
    if exact_match != 0 { return 1; }  // Type1
    if similarity >= 1.0 && same_length != 0 { return 2; }  // Type2
    if similarity >= 0.7 { return 3; }  // Type3
    0  // None
}

// ═══════════════════════════════════════════════════════════════════════
// Tests
// ═══════════════════════════════════════════════════════════════════════
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fingerprint_identical() {
        let a = StructFingerprint::new(vec![NodeShape::If, NodeShape::Call, NodeShape::Return], 2);
        let b = StructFingerprint::new(vec![NodeShape::If, NodeShape::Call, NodeShape::Return], 2);
        assert_eq!(a.hash, b.hash);
        assert!((a.similarity(&b) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_fingerprint_different() {
        let a = StructFingerprint::new(vec![NodeShape::If, NodeShape::Return], 1);
        let b = StructFingerprint::new(vec![NodeShape::Loop, NodeShape::Assign], 1);
        assert!(a.similarity(&b) < 0.01);
    }

    #[test]
    fn test_fingerprint_partial_overlap() {
        let a = StructFingerprint::new(vec![NodeShape::If, NodeShape::Call, NodeShape::Return], 2);
        let b = StructFingerprint::new(vec![NodeShape::If, NodeShape::Call, NodeShape::Assign], 2);
        let sim = a.similarity(&b);
        assert!(sim >= 0.5 && sim < 1.0);
    }

    #[test]
    fn test_fingerprint_empty() {
        let a = StructFingerprint::new(vec![], 0);
        let b = StructFingerprint::new(vec![], 0);
        assert!((a.similarity(&b) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_var_role_parameter() {
        assert_eq!(classify_variable_role("x", false, false, true, false, false), VarRole::Parameter);
    }

    #[test]
    fn test_var_role_result() {
        assert_eq!(classify_variable_role("res", false, false, false, false, true), VarRole::Result);
    }

    #[test]
    fn test_var_role_iterator() {
        assert_eq!(classify_variable_role("i", true, false, false, false, false), VarRole::Iterator);
    }

    #[test]
    fn test_var_role_accumulator() {
        assert_eq!(classify_variable_role("sum", false, true, false, false, false), VarRole::Accumulator);
    }

    #[test]
    fn test_var_role_counter() {
        assert_eq!(classify_variable_role("count", false, true, false, false, false), VarRole::Counter);
    }

    #[test]
    fn test_var_role_flag() {
        assert_eq!(classify_variable_role("found", false, false, false, true, false), VarRole::Flag);
    }

    #[test]
    fn test_var_role_pivot() {
        assert_eq!(classify_variable_role("mid_point", false, false, false, false, false), VarRole::Pivot);
    }

    #[test]
    fn test_var_role_bound() {
        assert_eq!(classify_variable_role("max_size", false, false, false, false, false), VarRole::Bound);
    }

    #[test]
    fn test_var_role_temporary() {
        assert_eq!(classify_variable_role("t", false, false, false, false, false), VarRole::Temporary);
    }

    #[test]
    fn test_clone_detect_type1() {
        let a = StructFingerprint::new(vec![NodeShape::If, NodeShape::Call], 2);
        let b = StructFingerprint::new(vec![NodeShape::If, NodeShape::Call], 2);
        assert_eq!(detect_clone(&a, &b, 0.7), CloneType::Type1);
    }

    #[test]
    fn test_clone_detect_type3() {
        let a = StructFingerprint::new(vec![NodeShape::If, NodeShape::Call, NodeShape::Return, NodeShape::Assign], 2);
        let b = StructFingerprint::new(vec![NodeShape::If, NodeShape::Call, NodeShape::Return, NodeShape::Loop], 3);
        let ct = detect_clone(&a, &b, 0.6);
        assert_eq!(ct, CloneType::Type3);
    }

    #[test]
    fn test_clone_detect_none() {
        let a = StructFingerprint::new(vec![NodeShape::If], 1);
        let b = StructFingerprint::new(vec![NodeShape::Loop, NodeShape::Match, NodeShape::Lambda], 3);
        assert_eq!(detect_clone(&a, &b, 0.7), CloneType::None);
    }

    #[test]
    fn test_analogy_mapping() {
        let mut am = AnalogyMapping::new();
        let s0 = am.add_source("arr");
        let s1 = am.add_source("len");
        let t0 = am.add_target("list");
        let t1 = am.add_target("size");
        am.map(s0, t0, 0.9);
        am.map(s1, t1, 0.8);
        assert_eq!(am.mapped_count(), 2);
        assert!((am.overall_confidence() - 0.85).abs() < 1e-10);
    }

    #[test]
    fn test_analogy_empty_confidence() {
        let am = AnalogyMapping::new();
        assert_eq!(am.overall_confidence(), 0.0);
    }

    #[test]
    fn test_var_role_labels() {
        assert_eq!(VarRole::Iterator.label(), "iterator");
        assert_eq!(VarRole::Accumulator.label(), "accumulator");
        assert_eq!(VarRole::Guard.label(), "guard");
    }

    #[test]
    fn test_ffi_fingerprint_hash() {
        let shapes = [1i64, 2, 3];
        let h = vitalis_analogy_fingerprint_hash(shapes.as_ptr(), 3, 2);
        assert_ne!(h, 0);
    }

    #[test]
    fn test_ffi_similarity() {
        let a = [1i64, 2, 3];
        let b = [1i64, 2, 3];
        let sim = vitalis_analogy_similarity(a.as_ptr(), 3, b.as_ptr(), 3);
        assert!((sim - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_ffi_clone_type() {
        assert_eq!(vitalis_analogy_clone_type(1.0, 1, 1), 1);  // Type1
        assert_eq!(vitalis_analogy_clone_type(0.5, 0, 1), 0);  // None
        assert_eq!(vitalis_analogy_clone_type(0.8, 0, 0), 3);  // Type3
    }
}
