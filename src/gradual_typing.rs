//! Gradual Typing — v385
//! Dynamic-static type boundary management with type guards, casts, and consistency checking.

use std::sync::{LazyLock, Mutex};
use std::collections::HashMap;

/// Type representation for gradual typing.
#[derive(Debug, Clone, PartialEq)]
pub enum GradualType {
    Dynamic,
    Int,
    Float,
    Bool,
    Str,
    Void,
    Function(Vec<GradualType>, Box<GradualType>),
    List(Box<GradualType>),
    Union(Vec<GradualType>),
    Intersection(Vec<GradualType>),
    Named(String),
}

impl GradualType {
    /// Check consistency (~ relation): two types are consistent if they
    /// could be equal after filling in dynamic types.
    pub fn is_consistent(&self, other: &GradualType) -> bool {
        match (self, other) {
            (GradualType::Dynamic, _) | (_, GradualType::Dynamic) => true,
            (GradualType::Int, GradualType::Int) => true,
            (GradualType::Float, GradualType::Float) => true,
            (GradualType::Bool, GradualType::Bool) => true,
            (GradualType::Str, GradualType::Str) => true,
            (GradualType::Void, GradualType::Void) => true,
            (GradualType::Named(a), GradualType::Named(b)) => a == b,
            (GradualType::List(a), GradualType::List(b)) => a.is_consistent(b),
            (GradualType::Function(args1, ret1), GradualType::Function(args2, ret2)) => {
                if args1.len() != args2.len() {
                    return false;
                }
                args1.iter().zip(args2.iter()).all(|(a, b)| a.is_consistent(b))
                    && ret1.is_consistent(ret2)
            }
            (GradualType::Union(types), other) | (other, GradualType::Union(types)) => {
                types.iter().any(|t| t.is_consistent(other))
            }
            _ => false,
        }
    }

    /// Check if this type is a static (fully-known) type.
    pub fn is_static(&self) -> bool {
        match self {
            GradualType::Dynamic => false,
            GradualType::Function(args, ret) => {
                args.iter().all(|a| a.is_static()) && ret.is_static()
            }
            GradualType::List(inner) => inner.is_static(),
            GradualType::Union(types) | GradualType::Intersection(types) => {
                types.iter().all(|t| t.is_static())
            }
            _ => true,
        }
    }

    /// Check if this type is fully dynamic.
    pub fn is_dynamic(&self) -> bool {
        matches!(self, GradualType::Dynamic)
    }

    /// Apply a type guard: narrow a type given evidence.
    pub fn narrow(&self, evidence: &GradualType) -> GradualType {
        match (self, evidence) {
            (GradualType::Dynamic, _) => evidence.clone(),
            (GradualType::Union(types), guard) => {
                let narrowed: Vec<GradualType> = types.iter()
                    .filter(|t| t.is_consistent(guard))
                    .cloned()
                    .collect();
                match narrowed.len() {
                    0 => GradualType::Void,
                    1 => narrowed.into_iter().next().unwrap(),
                    _ => GradualType::Union(narrowed),
                }
            }
            (t, guard) if t.is_consistent(guard) => guard.clone(),
            _ => GradualType::Void,
        }
    }

    /// Widen to dynamic type.
    pub fn widen(&self) -> GradualType {
        GradualType::Dynamic
    }

    /// Meet: greatest lower bound (intersection).
    pub fn meet(&self, other: &GradualType) -> GradualType {
        match (self, other) {
            (GradualType::Dynamic, t) | (t, GradualType::Dynamic) => t.clone(),
            (a, b) if a == b => a.clone(),
            (a, b) => GradualType::Intersection(vec![a.clone(), b.clone()]),
        }
    }

    /// Join: least upper bound (union).
    pub fn join(&self, other: &GradualType) -> GradualType {
        match (self, other) {
            (GradualType::Dynamic, _) | (_, GradualType::Dynamic) => GradualType::Dynamic,
            (a, b) if a == b => a.clone(),
            (a, b) => GradualType::Union(vec![a.clone(), b.clone()]),
        }
    }

    /// Check if a runtime cast from this type to target type would succeed.
    pub fn can_cast(&self, target: &GradualType) -> CastResult {
        if self == target {
            return CastResult::Safe;
        }
        if self.is_consistent(target) {
            if self.is_dynamic() || target.is_dynamic() {
                return CastResult::RuntimeCheck;
            }
            return CastResult::Safe;
        }
        CastResult::Impossible
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum CastResult {
    Safe,
    RuntimeCheck,
    Impossible,
}

/// Type environment for gradual type checking.
#[derive(Debug)]
pub struct GradualChecker {
    bindings: HashMap<String, GradualType>,
    cast_count: usize,
    guard_count: usize,
}

impl GradualChecker {
    pub fn new() -> Self {
        Self { bindings: HashMap::new(), cast_count: 0, guard_count: 0 }
    }

    pub fn bind(&mut self, name: &str, ty: GradualType) {
        self.bindings.insert(name.to_string(), ty);
    }

    pub fn lookup(&self, name: &str) -> Option<&GradualType> {
        self.bindings.get(name)
    }

    pub fn check_assignment(&self, target: &GradualType, value: &GradualType) -> bool {
        target.is_consistent(value)
    }

    pub fn cast(&mut self, from: &GradualType, to: &GradualType) -> CastResult {
        self.cast_count += 1;
        from.can_cast(to)
    }

    pub fn guard(&mut self, name: &str, evidence: &GradualType) -> GradualType {
        self.guard_count += 1;
        if let Some(current) = self.bindings.get(name) {
            let narrowed = current.narrow(evidence);
            self.bindings.insert(name.to_string(), narrowed.clone());
            narrowed
        } else {
            evidence.clone()
        }
    }

    pub fn cast_count(&self) -> usize { self.cast_count }
    pub fn guard_count(&self) -> usize { self.guard_count }
}

static GRAD_CHECKER: LazyLock<Mutex<GradualChecker>> =
    LazyLock::new(|| Mutex::new(GradualChecker::new()));

#[unsafe(no_mangle)]
pub extern "C" fn slang_grad_check(type_a: i64, type_b: i64) -> i64 {
    let a = id_to_type(type_a);
    let b = id_to_type(type_b);
    if a.is_consistent(&b) { 1 } else { 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_grad_cast(from: i64, to: i64) -> i64 {
    let f = id_to_type(from);
    let t = id_to_type(to);
    let mut checker = GRAD_CHECKER.lock().unwrap();
    match checker.cast(&f, &t) {
        CastResult::Safe => 1,
        CastResult::RuntimeCheck => 2,
        CastResult::Impossible => 0,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_grad_is_dynamic(type_id: i64) -> i64 {
    if id_to_type(type_id).is_dynamic() { 1 } else { 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_grad_guard(type_id: i64, evidence_id: i64) -> i64 {
    let current = id_to_type(type_id);
    let evidence = id_to_type(evidence_id);
    let narrowed = current.narrow(&evidence);
    type_to_id(&narrowed)
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_grad_narrow(type_id: i64, evidence_id: i64) -> i64 {
    let current = id_to_type(type_id);
    let evidence = id_to_type(evidence_id);
    let narrowed = current.narrow(&evidence);
    type_to_id(&narrowed)
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_grad_widen(_type_id: i64) -> i64 {
    0 // Dynamic = 0
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_grad_boundary(type_id: i64) -> i64 {
    let t = id_to_type(type_id);
    if t.is_static() { 1 } else { 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_grad_consistency(a: i64, b: i64) -> i64 {
    let ta = id_to_type(a);
    let tb = id_to_type(b);
    if ta.is_consistent(&tb) { 1 } else { 0 }
}

fn id_to_type(id: i64) -> GradualType {
    match id {
        0 => GradualType::Dynamic,
        1 => GradualType::Int,
        2 => GradualType::Float,
        3 => GradualType::Bool,
        4 => GradualType::Str,
        5 => GradualType::Void,
        _ => GradualType::Dynamic,
    }
}

fn type_to_id(ty: &GradualType) -> i64 {
    match ty {
        GradualType::Dynamic => 0,
        GradualType::Int => 1,
        GradualType::Float => 2,
        GradualType::Bool => 3,
        GradualType::Str => 4,
        GradualType::Void => 5,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dynamic_consistent_with_all() {
        assert!(GradualType::Dynamic.is_consistent(&GradualType::Int));
        assert!(GradualType::Dynamic.is_consistent(&GradualType::Str));
        assert!(GradualType::Dynamic.is_consistent(&GradualType::Dynamic));
    }

    #[test]
    fn test_same_type_consistent() {
        assert!(GradualType::Int.is_consistent(&GradualType::Int));
        assert!(GradualType::Bool.is_consistent(&GradualType::Bool));
    }

    #[test]
    fn test_different_types_inconsistent() {
        assert!(!GradualType::Int.is_consistent(&GradualType::Str));
        assert!(!GradualType::Bool.is_consistent(&GradualType::Float));
    }

    #[test]
    fn test_is_static() {
        assert!(GradualType::Int.is_static());
        assert!(!GradualType::Dynamic.is_static());
    }

    #[test]
    fn test_function_consistency() {
        let f1 = GradualType::Function(vec![GradualType::Int], Box::new(GradualType::Bool));
        let f2 = GradualType::Function(vec![GradualType::Dynamic], Box::new(GradualType::Bool));
        assert!(f1.is_consistent(&f2));
    }

    #[test]
    fn test_function_arity_mismatch() {
        let f1 = GradualType::Function(vec![GradualType::Int], Box::new(GradualType::Bool));
        let f2 = GradualType::Function(vec![GradualType::Int, GradualType::Int], Box::new(GradualType::Bool));
        assert!(!f1.is_consistent(&f2));
    }

    #[test]
    fn test_list_consistency() {
        let l1 = GradualType::List(Box::new(GradualType::Int));
        let l2 = GradualType::List(Box::new(GradualType::Dynamic));
        assert!(l1.is_consistent(&l2));
    }

    #[test]
    fn test_narrow_dynamic() {
        let t = GradualType::Dynamic;
        let narrowed = t.narrow(&GradualType::Int);
        assert_eq!(narrowed, GradualType::Int);
    }

    #[test]
    fn test_narrow_union() {
        let t = GradualType::Union(vec![GradualType::Int, GradualType::Str, GradualType::Bool]);
        let narrowed = t.narrow(&GradualType::Int);
        assert_eq!(narrowed, GradualType::Int);
    }

    #[test]
    fn test_widen() {
        assert_eq!(GradualType::Int.widen(), GradualType::Dynamic);
    }

    #[test]
    fn test_cast_safe() {
        let result = GradualType::Int.can_cast(&GradualType::Int);
        assert_eq!(result, CastResult::Safe);
    }

    #[test]
    fn test_cast_runtime_check() {
        let result = GradualType::Dynamic.can_cast(&GradualType::Int);
        assert_eq!(result, CastResult::RuntimeCheck);
    }

    #[test]
    fn test_cast_impossible() {
        let result = GradualType::Int.can_cast(&GradualType::Str);
        assert_eq!(result, CastResult::Impossible);
    }

    #[test]
    fn test_meet_with_dynamic() {
        let m = GradualType::Dynamic.meet(&GradualType::Int);
        assert_eq!(m, GradualType::Int);
    }

    #[test]
    fn test_meet_same() {
        let m = GradualType::Int.meet(&GradualType::Int);
        assert_eq!(m, GradualType::Int);
    }

    #[test]
    fn test_join_with_dynamic() {
        let j = GradualType::Int.join(&GradualType::Dynamic);
        assert_eq!(j, GradualType::Dynamic);
    }

    #[test]
    fn test_join_different() {
        let j = GradualType::Int.join(&GradualType::Str);
        assert_eq!(j, GradualType::Union(vec![GradualType::Int, GradualType::Str]));
    }

    #[test]
    fn test_checker_bind_lookup() {
        let mut checker = GradualChecker::new();
        checker.bind("x", GradualType::Int);
        assert_eq!(checker.lookup("x"), Some(&GradualType::Int));
    }

    #[test]
    fn test_checker_assignment() {
        let checker = GradualChecker::new();
        assert!(checker.check_assignment(&GradualType::Dynamic, &GradualType::Int));
        assert!(!checker.check_assignment(&GradualType::Int, &GradualType::Str));
    }

    #[test]
    fn test_checker_guard() {
        let mut checker = GradualChecker::new();
        checker.bind("x", GradualType::Dynamic);
        let narrowed = checker.guard("x", &GradualType::Int);
        assert_eq!(narrowed, GradualType::Int);
        assert_eq!(checker.guard_count(), 1);
    }

    #[test]
    fn test_union_consistency() {
        let u = GradualType::Union(vec![GradualType::Int, GradualType::Str]);
        assert!(u.is_consistent(&GradualType::Int));
        assert!(u.is_consistent(&GradualType::Str));
        assert!(!u.is_consistent(&GradualType::Bool));
    }

    #[test]
    fn test_named_type() {
        assert!(GradualType::Named("Foo".into()).is_consistent(&GradualType::Named("Foo".into())));
        assert!(!GradualType::Named("Foo".into()).is_consistent(&GradualType::Named("Bar".into())));
    }
}
