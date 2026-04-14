//! Loop Vectorizer — v378
//! Auto-vectorization analysis: SLP (superword-level parallelism), cost model, legality.

#[derive(Debug, Clone, PartialEq)]
pub enum VecLegality {
    Vectorizable,
    NotVectorizable(String),
    ConditionallyVectorizable,
}

#[derive(Debug, Clone)]
pub struct LoopInfo {
    pub id: i64,
    pub trip_count: Option<i64>,
    pub body_ops: Vec<VecOp>,
    pub has_side_effects: bool,
    pub has_function_calls: bool,
    pub has_loop_carried_dep: bool,
    pub memory_accesses: Vec<MemAccess>,
}

#[derive(Debug, Clone)]
pub struct VecOp {
    pub kind: OpKind,
    pub width: i64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum OpKind {
    Add,
    Mul,
    Load,
    Store,
    FAdd,
    FMul,
    Compare,
    Select,
}

#[derive(Debug, Clone)]
pub struct MemAccess {
    pub base: i64,
    pub stride: i64,
    pub is_write: bool,
}

impl MemAccess {
    pub fn is_contiguous(&self) -> bool {
        self.stride == 1 || self.stride == -1
    }

    pub fn is_stride(&self) -> bool {
        self.stride.abs() > 1
    }
}

#[derive(Debug, Clone)]
pub struct VecCost {
    pub scalar_cost: f64,
    pub vector_cost: f64,
    pub speedup: f64,
}

pub struct LoopVectorizer {
    pub target_width: i64,
}

impl LoopVectorizer {
    pub fn new(target_width: i64) -> Self {
        Self { target_width: target_width.max(2) }
    }

    pub fn check_legality(&self, loop_info: &LoopInfo) -> VecLegality {
        if loop_info.has_loop_carried_dep {
            return VecLegality::NotVectorizable("loop-carried dependency".to_string());
        }
        if loop_info.has_side_effects {
            return VecLegality::NotVectorizable("side effects".to_string());
        }
        if loop_info.has_function_calls {
            return VecLegality::ConditionallyVectorizable;
        }
        if let Some(tc) = loop_info.trip_count {
            if tc < self.target_width {
                return VecLegality::NotVectorizable("trip count too small".to_string());
            }
        }
        // Check memory alignment
        for acc in &loop_info.memory_accesses {
            if acc.stride == 0 {
                return VecLegality::NotVectorizable("zero stride".to_string());
            }
        }
        VecLegality::Vectorizable
    }

    pub fn dependence_check(&self, accesses: &[MemAccess]) -> bool {
        for i in 0..accesses.len() {
            for j in (i+1)..accesses.len() {
                if accesses[i].base == accesses[j].base
                    && (accesses[i].is_write || accesses[j].is_write)
                    && accesses[i].stride == accesses[j].stride
                {
                    return false; // dependency found
                }
            }
        }
        true // safe
    }

    pub fn estimate_cost(&self, loop_info: &LoopInfo) -> VecCost {
        let num_ops = loop_info.body_ops.len() as f64;
        let trip = loop_info.trip_count.unwrap_or(100) as f64;
        let scalar_cost = num_ops * trip;
        let vec_width = self.target_width as f64;
        // Vectorization reduces trip count but adds setup/teardown
        let vec_iterations = (trip / vec_width).ceil();
        let setup_cost = 2.0;
        let vector_cost = num_ops * vec_iterations + setup_cost;
        let speedup = if vector_cost > 0.0 { scalar_cost / vector_cost } else { 1.0 };
        VecCost { scalar_cost, vector_cost, speedup }
    }

    pub fn optimal_width(&self, loop_info: &LoopInfo) -> i64 {
        let tc = loop_info.trip_count.unwrap_or(256);
        let mut best_width = 2i64;
        let mut best_speedup = 0.0f64;
        for w in [2, 4, 8, 16] {
            if w > tc {
                break;
            }
            let vec_iters = (tc as f64 / w as f64).ceil();
            let ops = loop_info.body_ops.len() as f64;
            let scalar = ops * tc as f64;
            let vector = ops * vec_iters + 2.0;
            let speedup = scalar / vector;
            if speedup > best_speedup {
                best_speedup = speedup;
                best_width = w;
            }
        }
        best_width
    }

    pub fn unroll_factor(&self, loop_info: &LoopInfo) -> i64 {
        let tc = loop_info.trip_count.unwrap_or(100);
        if tc >= 64 { 4 }
        else if tc >= 16 { 2 }
        else { 1 }
    }

    pub fn slp_analyze(&self, ops: &[VecOp]) -> Vec<Vec<usize>> {
        let mut groups: Vec<Vec<usize>> = Vec::new();
        let mut used = vec![false; ops.len()];
        for i in 0..ops.len() {
            if used[i] { continue; }
            let mut group = vec![i];
            for j in (i+1)..ops.len() {
                if !used[j] && ops[j].kind == ops[i].kind {
                    group.push(j);
                    if group.len() as i64 >= self.target_width {
                        break;
                    }
                }
            }
            if group.len() >= 2 {
                for &idx in &group {
                    used[idx] = true;
                }
                groups.push(group);
            }
        }
        groups
    }
}

use std::sync::Mutex;
use std::sync::LazyLock;
static VECTORIZER: LazyLock<Mutex<LoopVectorizer>> = LazyLock::new(|| Mutex::new(LoopVectorizer::new(4)));

#[unsafe(no_mangle)]
pub extern "C" fn slang_vec_analyze(trip_count: i64, num_ops: i64) -> i64 {
    let info = LoopInfo {
        id: 1, trip_count: Some(trip_count),
        body_ops: (0..num_ops).map(|_| VecOp { kind: OpKind::Add, width: 4 }).collect(),
        has_side_effects: false, has_function_calls: false,
        has_loop_carried_dep: false, memory_accesses: vec![],
    };
    let v = VECTORIZER.lock().unwrap();
    match v.check_legality(&info) {
        VecLegality::Vectorizable => 1,
        VecLegality::ConditionallyVectorizable => 2,
        VecLegality::NotVectorizable(_) => 0,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_vec_is_vectorizable(trip_count: i64) -> i64 {
    let info = LoopInfo {
        id: 1, trip_count: Some(trip_count), body_ops: vec![VecOp { kind: OpKind::Add, width: 4 }],
        has_side_effects: false, has_function_calls: false,
        has_loop_carried_dep: false, memory_accesses: vec![MemAccess { base: 0, stride: 1, is_write: false }],
    };
    let v = VECTORIZER.lock().unwrap();
    if v.check_legality(&info) == VecLegality::Vectorizable { 1 } else { 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_vec_cost_model(trip_count: i64, num_ops: i64) -> i64 {
    let info = LoopInfo {
        id: 1, trip_count: Some(trip_count),
        body_ops: (0..num_ops).map(|_| VecOp { kind: OpKind::Add, width: 4 }).collect(),
        has_side_effects: false, has_function_calls: false,
        has_loop_carried_dep: false, memory_accesses: vec![],
    };
    let cost = VECTORIZER.lock().unwrap().estimate_cost(&info);
    (cost.speedup * 100.0) as i64
}

pub extern "C" fn slang_vec_width(trip_count: i64) -> i64 {
    let info = LoopInfo {
        id: 1, trip_count: Some(trip_count), body_ops: vec![VecOp { kind: OpKind::Add, width: 4 }],
        has_side_effects: false, has_function_calls: false,
        has_loop_carried_dep: false, memory_accesses: vec![],
    };
    VECTORIZER.lock().unwrap().optimal_width(&info)
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_vec_unroll_factor(trip_count: i64) -> i64 {
    let info = LoopInfo {
        id: 1, trip_count: Some(trip_count), body_ops: vec![],
        has_side_effects: false, has_function_calls: false,
        has_loop_carried_dep: false, memory_accesses: vec![],
    };
    VECTORIZER.lock().unwrap().unroll_factor(&info)
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_vec_dependence_check(base_a: i64, stride_a: i64, base_b: i64, stride_b: i64) -> i64 {
    let accesses = vec![
        MemAccess { base: base_a, stride: stride_a, is_write: false },
        MemAccess { base: base_b, stride: stride_b, is_write: true },
    ];
    if VECTORIZER.lock().unwrap().dependence_check(&accesses) { 1 } else { 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_vec_slp_analyze(num_ops: i64) -> i64 {
    let ops: Vec<_> = (0..num_ops).map(|_| VecOp { kind: OpKind::Add, width: 4 }).collect();
    let groups = VECTORIZER.lock().unwrap().slp_analyze(&ops);
    groups.len() as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_vec_transform(trip_count: i64) -> i64 {
    let info = LoopInfo {
        id: 1, trip_count: Some(trip_count), body_ops: vec![VecOp { kind: OpKind::Add, width: 4 }],
        has_side_effects: false, has_function_calls: false,
        has_loop_carried_dep: false, memory_accesses: vec![MemAccess { base: 0, stride: 1, is_write: false }],
    };
    let v = VECTORIZER.lock().unwrap();
    if v.check_legality(&info) == VecLegality::Vectorizable { 1 } else { 0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn simple_loop(trip_count: i64) -> LoopInfo {
        LoopInfo {
            id: 1, trip_count: Some(trip_count),
            body_ops: vec![VecOp { kind: OpKind::Add, width: 4 }, VecOp { kind: OpKind::Mul, width: 4 }],
            has_side_effects: false, has_function_calls: false,
            has_loop_carried_dep: false,
            memory_accesses: vec![MemAccess { base: 0, stride: 1, is_write: false }],
        }
    }

    #[test]
    fn test_new_vectorizer() {
        let v = LoopVectorizer::new(4);
        assert_eq!(v.target_width, 4);
    }

    #[test]
    fn test_min_width() {
        let v = LoopVectorizer::new(0);
        assert_eq!(v.target_width, 2);
    }

    #[test]
    fn test_vectorizable_loop() {
        let v = LoopVectorizer::new(4);
        let info = simple_loop(100);
        assert_eq!(v.check_legality(&info), VecLegality::Vectorizable);
    }

    #[test]
    fn test_loop_carried_dep() {
        let v = LoopVectorizer::new(4);
        let mut info = simple_loop(100);
        info.has_loop_carried_dep = true;
        assert!(matches!(v.check_legality(&info), VecLegality::NotVectorizable(_)));
    }

    #[test]
    fn test_side_effects() {
        let v = LoopVectorizer::new(4);
        let mut info = simple_loop(100);
        info.has_side_effects = true;
        assert!(matches!(v.check_legality(&info), VecLegality::NotVectorizable(_)));
    }

    #[test]
    fn test_function_calls() {
        let v = LoopVectorizer::new(4);
        let mut info = simple_loop(100);
        info.has_function_calls = true;
        assert_eq!(v.check_legality(&info), VecLegality::ConditionallyVectorizable);
    }

    #[test]
    fn test_small_trip_count() {
        let v = LoopVectorizer::new(4);
        let info = simple_loop(2);
        assert!(matches!(v.check_legality(&info), VecLegality::NotVectorizable(_)));
    }

    #[test]
    fn test_cost_model() {
        let v = LoopVectorizer::new(4);
        let info = simple_loop(100);
        let cost = v.estimate_cost(&info);
        assert!(cost.speedup > 1.0);
        assert!(cost.scalar_cost > cost.vector_cost);
    }

    #[test]
    fn test_optimal_width() {
        let v = LoopVectorizer::new(4);
        let info = simple_loop(256);
        let w = v.optimal_width(&info);
        assert!(w >= 2);
    }

    #[test]
    fn test_unroll_factor_large() {
        let v = LoopVectorizer::new(4);
        let info = simple_loop(100);
        assert_eq!(v.unroll_factor(&info), 4);
    }

    #[test]
    fn test_unroll_factor_small() {
        let v = LoopVectorizer::new(4);
        let info = simple_loop(10);
        assert_eq!(v.unroll_factor(&info), 1);
    }

    #[test]
    fn test_dependence_safe() {
        let v = LoopVectorizer::new(4);
        let accesses = vec![
            MemAccess { base: 0, stride: 1, is_write: false },
            MemAccess { base: 100, stride: 1, is_write: true },
        ];
        assert!(v.dependence_check(&accesses));
    }

    #[test]
    fn test_dependence_conflict() {
        let v = LoopVectorizer::new(4);
        let accesses = vec![
            MemAccess { base: 0, stride: 1, is_write: false },
            MemAccess { base: 0, stride: 1, is_write: true },
        ];
        assert!(!v.dependence_check(&accesses));
    }

    #[test]
    fn test_mem_contiguous() {
        let acc = MemAccess { base: 0, stride: 1, is_write: false };
        assert!(acc.is_contiguous());
    }

    #[test]
    fn test_mem_stride() {
        let acc = MemAccess { base: 0, stride: 4, is_write: false };
        assert!(acc.is_stride());
        assert!(!acc.is_contiguous());
    }

    #[test]
    fn test_slp_groups() {
        let v = LoopVectorizer::new(4);
        let ops = vec![
            VecOp { kind: OpKind::Add, width: 4 },
            VecOp { kind: OpKind::Add, width: 4 },
            VecOp { kind: OpKind::Add, width: 4 },
            VecOp { kind: OpKind::Add, width: 4 },
            VecOp { kind: OpKind::Mul, width: 4 },
            VecOp { kind: OpKind::Mul, width: 4 },
        ];
        let groups = v.slp_analyze(&ops);
        assert_eq!(groups.len(), 2);
    }

    #[test]
    fn test_slp_no_groups() {
        let v = LoopVectorizer::new(4);
        let ops = vec![
            VecOp { kind: OpKind::Add, width: 4 },
        ];
        let groups = v.slp_analyze(&ops);
        assert!(groups.is_empty());
    }

    #[test]
    fn test_zero_stride_illegal() {
        let v = LoopVectorizer::new(4);
        let info = LoopInfo {
            id: 1, trip_count: Some(100),
            body_ops: vec![VecOp { kind: OpKind::Add, width: 4 }],
            has_side_effects: false, has_function_calls: false,
            has_loop_carried_dep: false,
            memory_accesses: vec![MemAccess { base: 0, stride: 0, is_write: false }],
        };
        assert!(matches!(v.check_legality(&info), VecLegality::NotVectorizable(_)));
    }

    #[test]
    fn test_cost_model_single_op() {
        let v = LoopVectorizer::new(4);
        let info = LoopInfo {
            id: 1, trip_count: Some(1000),
            body_ops: vec![VecOp { kind: OpKind::FAdd, width: 4 }],
            has_side_effects: false, has_function_calls: false,
            has_loop_carried_dep: false, memory_accesses: vec![],
        };
        let cost = v.estimate_cost(&info);
        assert!(cost.speedup > 3.0);
    }

    #[test]
    fn test_unknown_trip_count() {
        let v = LoopVectorizer::new(4);
        let info = LoopInfo {
            id: 1, trip_count: None,
            body_ops: vec![VecOp { kind: OpKind::Add, width: 4 }],
            has_side_effects: false, has_function_calls: false,
            has_loop_carried_dep: false, memory_accesses: vec![],
        };
        assert_eq!(v.check_legality(&info), VecLegality::Vectorizable);
    }
}
