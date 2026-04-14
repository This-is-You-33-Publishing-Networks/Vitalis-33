//! Vitalis Generics — Type parameters, monomorphization, and generic resolution.
//!
//! Provides:
//! - Generic function definitions: `fn identity<T>(x: T) -> T { x }`
//! - Generic struct definitions: `struct Pair<A, B> { first: A, second: B }`
//! - Generic trait bounds: `fn add<T: Numeric>(a: T, b: T) -> T`
//! - Monomorphization: generics are expanded into concrete types at compile time
//! - Type inference for generic parameters from call-site arguments
//!
//! The monomorphizer walks the AST, finds generic usages, and generates
//! concrete specialized versions (e.g., `identity_i64`, `identity_f64`).

use std::collections::HashMap;
use std::fmt;

// ─── Generic Errors ─────────────────────────────────────────────────────

/// Errors produced during generic instantiation or inference.
#[derive(Debug, Clone, PartialEq)]
pub enum GenericError {
    /// Unknown generic function or struct name.
    UnknownGeneric(String),
    /// Wrong number of type arguments.
    ArityMismatch { name: String, expected: usize, got: usize },
    /// A concrete type does not satisfy a bound on a type parameter.
    BoundNotSatisfied {
        param: String,
        bound: String,
        concrete_type: String,
    },
    /// Could not infer a type parameter from call-site arguments.
    CannotInfer(String),
    /// Conflicting type inference for the same parameter.
    ConflictingInference {
        param: String,
        first: String,
        second: String,
    },
}

impl fmt::Display for GenericError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GenericError::UnknownGeneric(name) => {
                write!(f, "unknown generic `{}`", name)
            }
            GenericError::ArityMismatch { name, expected, got } => {
                write!(f, "`{}` expects {} type argument(s), got {}", name, expected, got)
            }
            GenericError::BoundNotSatisfied { param, bound, concrete_type } => {
                write!(f, "type `{}` does not satisfy bound `{}` on parameter `{}`", concrete_type, bound, param)
            }
            GenericError::CannotInfer(param) => {
                write!(f, "cannot infer type parameter `{}`", param)
            }
            GenericError::ConflictingInference { param, first, second } => {
                write!(f, "conflicting types for `{}`: `{}` vs `{}`", param, first, second)
            }
        }
    }
}

// ─── Type Parameters ────────────────────────────────────────────────────

/// A type parameter declaration: `T`, `T: Bound`, `T: Bound1 + Bound2`
#[derive(Debug, Clone, PartialEq)]
pub struct TypeParam {
    pub name: String,
    pub bounds: Vec<String>,
    pub default: Option<String>,
}

impl TypeParam {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            bounds: Vec::new(),
            default: None,
        }
    }

    pub fn with_bound(mut self, bound: &str) -> Self {
        self.bounds.push(bound.to_string());
        self
    }

    pub fn with_default(mut self, default: &str) -> Self {
        self.default = Some(default.to_string());
        self
    }

    pub fn has_bound(&self, bound: &str) -> bool {
        self.bounds.iter().any(|b| b == bound)
    }
}

impl fmt::Display for TypeParam {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)?;
        if !self.bounds.is_empty() {
            write!(f, ": {}", self.bounds.join(" + "))?;
        }
        if let Some(ref d) = self.default {
            write!(f, " = {}", d)?;
        }
        Ok(())
    }
}

// ─── Generic Signature ──────────────────────────────────────────────────

/// A generic function or struct signature with type parameters.
#[derive(Debug, Clone)]
pub struct GenericSig {
    pub name: String,
    pub type_params: Vec<TypeParam>,
    pub param_types: Vec<String>,
    pub return_type: Option<String>,
}

impl GenericSig {
    pub fn new(name: &str, type_params: Vec<TypeParam>) -> Self {
        Self {
            name: name.to_string(),
            type_params,
            param_types: Vec::new(),
            return_type: None,
        }
    }

    /// Check if a type name is one of this signature's type parameters.
    pub fn is_type_param(&self, name: &str) -> bool {
        self.type_params.iter().any(|tp| tp.name == name)
    }

    /// Get the index of a type parameter by name.
    pub fn type_param_index(&self, name: &str) -> Option<usize> {
        self.type_params.iter().position(|tp| tp.name == name)
    }

    /// Generate the mangled name for a specific instantiation.
    pub fn mangle(&self, concrete_types: &[String]) -> String {
        let suffix: Vec<&str> = concrete_types.iter().map(|s| s.as_str()).collect();
        format!("{}_{}", self.name, suffix.join("_"))
    }
}

// ─── Type Substitution ──────────────────────────────────────────────────

/// A mapping from type parameter names to concrete types.
#[derive(Debug, Clone, Default)]
pub struct TypeSubstitution {
    mappings: HashMap<String, String>,
}

impl TypeSubstitution {
    pub fn new() -> Self {
        Self { mappings: HashMap::new() }
    }

    pub fn bind(&mut self, param: &str, concrete: &str) {
        self.mappings.insert(param.to_string(), concrete.to_string());
    }

    pub fn resolve(&self, ty: &str) -> String {
        self.mappings.get(ty).cloned().unwrap_or_else(|| ty.to_string())
    }

    pub fn is_bound(&self, param: &str) -> bool {
        self.mappings.contains_key(param)
    }

    pub fn all_bound(&self, params: &[TypeParam]) -> bool {
        params.iter().all(|p| self.is_bound(&p.name))
    }
}

impl fmt::Display for TypeSubstitution {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let pairs: Vec<String> = self.mappings.iter()
            .map(|(k, v)| format!("{} → {}", k, v))
            .collect();
        write!(f, "{{{}}}", pairs.join(", "))
    }
}

// ─── Monomorphizer ──────────────────────────────────────────────────────

/// Tracks which generic instantiations have been requested and generates
/// concrete specialized versions.
#[derive(Debug, Default)]
pub struct Monomorphizer {
    /// All known generic function signatures.
    generic_fns: HashMap<String, GenericSig>,
    /// All known generic struct signatures.
    generic_structs: HashMap<String, GenericSig>,
    /// Generated concrete function names → (original_name, substitution).
    instantiated_fns: HashMap<String, (String, TypeSubstitution)>,
    /// Generated concrete struct names → (original_name, substitution).
    instantiated_structs: HashMap<String, (String, TypeSubstitution)>,
}

impl Monomorphizer {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a generic function signature.
    pub fn register_generic_fn(&mut self, sig: GenericSig) {
        self.generic_fns.insert(sig.name.clone(), sig);
    }

    /// Register a generic struct signature.
    pub fn register_generic_struct(&mut self, sig: GenericSig) {
        self.generic_structs.insert(sig.name.clone(), sig);
    }

    /// Check if a function is generic.
    pub fn is_generic_fn(&self, name: &str) -> bool {
        self.generic_fns.contains_key(name)
    }

    /// Check if a struct is generic.
    pub fn is_generic_struct(&self, name: &str) -> bool {
        self.generic_structs.contains_key(name)
    }

    /// Validate that concrete types satisfy all bounds on the generic signature.
    fn validate_bounds(sig: &GenericSig, concrete_types: &[String]) -> Result<(), GenericError> {
        for (param, concrete) in sig.type_params.iter().zip(concrete_types) {
            for bound_name in &param.bounds {
                if let Some(bound) = BuiltinBound::from_name(bound_name) {
                    if !bound.satisfied_by(concrete) {
                        return Err(GenericError::BoundNotSatisfied {
                            param: param.name.clone(),
                            bound: bound_name.clone(),
                            concrete_type: concrete.clone(),
                        });
                    }
                }
                // Unknown bounds are ignored (user-defined traits not enforced here)
            }
        }
        Ok(())
    }

    /// Request a concrete instantiation of a generic function.
    /// Returns the mangled name of the concrete version, or None on arity/unknown errors.
    pub fn instantiate_fn(&mut self, name: &str, concrete_types: &[String]) -> Option<String> {
        self.try_instantiate_fn(name, concrete_types).ok()
    }

    /// Request a concrete instantiation of a generic function with full error reporting.
    pub fn try_instantiate_fn(&mut self, name: &str, concrete_types: &[String]) -> Result<String, GenericError> {
        let sig = self.generic_fns.get(name)
            .ok_or_else(|| GenericError::UnknownGeneric(name.to_string()))?
            .clone();
        if concrete_types.len() != sig.type_params.len() {
            return Err(GenericError::ArityMismatch {
                name: name.to_string(),
                expected: sig.type_params.len(),
                got: concrete_types.len(),
            });
        }

        Self::validate_bounds(&sig, concrete_types)?;

        let mangled = sig.mangle(concrete_types);
        if !self.instantiated_fns.contains_key(&mangled) {
            let mut sub = TypeSubstitution::new();
            for (param, concrete) in sig.type_params.iter().zip(concrete_types) {
                sub.bind(&param.name, concrete);
            }
            self.instantiated_fns.insert(mangled.clone(), (name.to_string(), sub));
        }
        Ok(mangled)
    }

    /// Request a concrete instantiation of a generic struct.
    /// Returns the mangled name, or None on arity/unknown errors.
    pub fn instantiate_struct(&mut self, name: &str, concrete_types: &[String]) -> Option<String> {
        self.try_instantiate_struct(name, concrete_types).ok()
    }

    /// Request a concrete instantiation of a generic struct with full error reporting.
    pub fn try_instantiate_struct(&mut self, name: &str, concrete_types: &[String]) -> Result<String, GenericError> {
        let sig = self.generic_structs.get(name)
            .ok_or_else(|| GenericError::UnknownGeneric(name.to_string()))?
            .clone();
        if concrete_types.len() != sig.type_params.len() {
            return Err(GenericError::ArityMismatch {
                name: name.to_string(),
                expected: sig.type_params.len(),
                got: concrete_types.len(),
            });
        }

        Self::validate_bounds(&sig, concrete_types)?;

        let mangled = sig.mangle(concrete_types);
        if !self.instantiated_structs.contains_key(&mangled) {
            let mut sub = TypeSubstitution::new();
            for (param, concrete) in sig.type_params.iter().zip(concrete_types) {
                sub.bind(&param.name, concrete);
            }
            self.instantiated_structs.insert(mangled.clone(), (name.to_string(), sub));
        }
        Ok(mangled)
    }

    /// Get all instantiated function names with their substitutions.
    pub fn instantiated_functions(&self) -> &HashMap<String, (String, TypeSubstitution)> {
        &self.instantiated_fns
    }

    /// Get all instantiated struct names with their substitutions.
    pub fn instantiated_structs(&self) -> &HashMap<String, (String, TypeSubstitution)> {
        &self.instantiated_structs
    }

    /// Number of registered generic functions.
    pub fn generic_fn_count(&self) -> usize {
        self.generic_fns.len()
    }

    /// Number of generated concrete instantiations.
    pub fn instantiation_count(&self) -> usize {
        self.instantiated_fns.len() + self.instantiated_structs.len()
    }
}

// ─── Type Inference ─────────────────────────────────────────────────────

/// Infer type parameters from argument types at a call site.
pub fn infer_type_params(
    sig: &GenericSig,
    arg_types: &[String],
) -> Option<TypeSubstitution> {
    try_infer_type_params(sig, arg_types).ok()
}

/// Infer type parameters with full error reporting.
pub fn try_infer_type_params(
    sig: &GenericSig,
    arg_types: &[String],
) -> Result<TypeSubstitution, GenericError> {
    if arg_types.len() != sig.param_types.len() {
        return Err(GenericError::ArityMismatch {
            name: sig.name.clone(),
            expected: sig.param_types.len(),
            got: arg_types.len(),
        });
    }

    let mut sub = TypeSubstitution::new();
    for (param_ty, arg_ty) in sig.param_types.iter().zip(arg_types) {
        if sig.is_type_param(param_ty) {
            if sub.is_bound(param_ty) {
                // Already bound — check consistency
                if sub.resolve(param_ty) != *arg_ty {
                    return Err(GenericError::ConflictingInference {
                        param: param_ty.clone(),
                        first: sub.resolve(param_ty),
                        second: arg_ty.clone(),
                    });
                }
            } else {
                sub.bind(param_ty, arg_ty);
            }
        }
    }

    // Check all params are bound (or have defaults)
    for tp in &sig.type_params {
        if !sub.is_bound(&tp.name) {
            if let Some(ref default) = tp.default {
                sub.bind(&tp.name, default);
            } else {
                return Err(GenericError::CannotInfer(tp.name.clone()));
            }
        }
    }

    // Validate bounds on inferred types
    let concrete_types: Vec<String> = sig.type_params.iter()
        .map(|tp| sub.resolve(&tp.name))
        .collect();
    Monomorphizer::validate_bounds(sig, &concrete_types)?;

    Ok(sub)
}

// ─── Built-in Trait Bounds ──────────────────────────────────────────────

/// Well-known trait bounds that the compiler understands.
#[derive(Debug, Clone, PartialEq)]
pub enum BuiltinBound {
    /// Type supports numeric operations (+, -, *, /)
    Numeric,
    /// Type supports equality comparison (==, !=)
    Eq,
    /// Type supports ordering comparison (<, >, <=, >=)
    Ord,
    /// Type can be displayed as a string
    Display,
    /// Type can be copied (all primitives)
    Copy,
    /// Type can be default-constructed
    Default,
}

impl BuiltinBound {
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "Numeric" => Some(BuiltinBound::Numeric),
            "Eq" => Some(BuiltinBound::Eq),
            "Ord" => Some(BuiltinBound::Ord),
            "Display" => Some(BuiltinBound::Display),
            "Copy" => Some(BuiltinBound::Copy),
            "Default" => Some(BuiltinBound::Default),
            _ => None,
        }
    }

    /// Check if a concrete type satisfies this bound.
    pub fn satisfied_by(&self, ty: &str) -> bool {
        match self {
            BuiltinBound::Numeric => matches!(ty, "i32" | "i64" | "f32" | "f64"),
            BuiltinBound::Eq => matches!(ty, "i32" | "i64" | "f32" | "f64" | "bool" | "str"),
            BuiltinBound::Ord => matches!(ty, "i32" | "i64" | "f32" | "f64"),
            BuiltinBound::Display => true, // All types can be displayed
            BuiltinBound::Copy => matches!(ty, "i32" | "i64" | "f32" | "f64" | "bool"),
            BuiltinBound::Default => matches!(ty, "i32" | "i64" | "f32" | "f64" | "bool"),
        }
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_param_creation() {
        let tp = TypeParam::new("T").with_bound("Numeric");
        assert_eq!(tp.name, "T");
        assert!(tp.has_bound("Numeric"));
        assert!(!tp.has_bound("Eq"));
    }

    #[test]
    fn test_type_param_display() {
        let tp = TypeParam::new("T").with_bound("Numeric").with_bound("Eq");
        assert_eq!(format!("{}", tp), "T: Numeric + Eq");
    }

    #[test]
    fn test_type_param_default() {
        let tp = TypeParam::new("T").with_default("i64");
        assert_eq!(tp.default, Some("i64".to_string()));
        assert_eq!(format!("{}", tp), "T = i64");
    }

    #[test]
    fn test_generic_sig_mangle() {
        let sig = GenericSig::new("identity", vec![TypeParam::new("T")]);
        assert_eq!(sig.mangle(&["i64".into()]), "identity_i64");
        assert_eq!(sig.mangle(&["f64".into()]), "identity_f64");
    }

    #[test]
    fn test_generic_sig_is_type_param() {
        let sig = GenericSig::new("swap", vec![
            TypeParam::new("A"),
            TypeParam::new("B"),
        ]);
        assert!(sig.is_type_param("A"));
        assert!(sig.is_type_param("B"));
        assert!(!sig.is_type_param("C"));
    }

    #[test]
    fn test_type_substitution() {
        let mut sub = TypeSubstitution::new();
        sub.bind("T", "i64");
        assert_eq!(sub.resolve("T"), "i64");
        assert_eq!(sub.resolve("U"), "U"); // Not bound, returns as-is
        assert!(sub.is_bound("T"));
        assert!(!sub.is_bound("U"));
    }

    #[test]
    fn test_monomorphizer_register() {
        let mut mono = Monomorphizer::new();
        let sig = GenericSig::new("identity", vec![TypeParam::new("T")]);
        mono.register_generic_fn(sig);
        assert!(mono.is_generic_fn("identity"));
        assert!(!mono.is_generic_fn("unknown"));
        assert_eq!(mono.generic_fn_count(), 1);
    }

    #[test]
    fn test_monomorphizer_instantiate() {
        let mut mono = Monomorphizer::new();
        let sig = GenericSig::new("identity", vec![TypeParam::new("T")]);
        mono.register_generic_fn(sig);

        let name = mono.instantiate_fn("identity", &["i64".into()]);
        assert_eq!(name, Some("identity_i64".to_string()));

        let name2 = mono.instantiate_fn("identity", &["f64".into()]);
        assert_eq!(name2, Some("identity_f64".to_string()));

        assert_eq!(mono.instantiation_count(), 2);
    }

    #[test]
    fn test_monomorphizer_dedup() {
        let mut mono = Monomorphizer::new();
        let sig = GenericSig::new("id", vec![TypeParam::new("T")]);
        mono.register_generic_fn(sig);

        mono.instantiate_fn("id", &["i64".into()]);
        mono.instantiate_fn("id", &["i64".into()]); // Duplicate
        assert_eq!(mono.instantiation_count(), 1); // Should not duplicate
    }

    #[test]
    fn test_monomorphizer_struct() {
        let mut mono = Monomorphizer::new();
        let sig = GenericSig::new("Pair", vec![
            TypeParam::new("A"),
            TypeParam::new("B"),
        ]);
        mono.register_generic_struct(sig);

        let name = mono.instantiate_struct("Pair", &["i64".into(), "str".into()]);
        assert_eq!(name, Some("Pair_i64_str".to_string()));
        assert!(mono.is_generic_struct("Pair"));
    }

    #[test]
    fn test_monomorphizer_wrong_arity() {
        let mut mono = Monomorphizer::new();
        let sig = GenericSig::new("identity", vec![TypeParam::new("T")]);
        mono.register_generic_fn(sig);

        // Too many type args
        assert_eq!(mono.instantiate_fn("identity", &["i64".into(), "f64".into()]), None);
        // Unknown function
        assert_eq!(mono.instantiate_fn("unknown", &["i64".into()]), None);
    }

    #[test]
    fn test_type_inference() {
        let mut sig = GenericSig::new("add", vec![TypeParam::new("T")]);
        sig.param_types = vec!["T".into(), "T".into()];

        let result = infer_type_params(&sig, &["i64".into(), "i64".into()]);
        assert!(result.is_some());
        let sub = result.unwrap();
        assert_eq!(sub.resolve("T"), "i64");
    }

    #[test]
    fn test_type_inference_conflict() {
        let mut sig = GenericSig::new("add", vec![TypeParam::new("T")]);
        sig.param_types = vec!["T".into(), "T".into()];

        // i64 vs f64 → conflict
        let result = infer_type_params(&sig, &["i64".into(), "f64".into()]);
        assert!(result.is_none());
    }

    #[test]
    fn test_type_inference_default() {
        let mut sig = GenericSig::new("zero", vec![
            TypeParam::new("T").with_default("i64"),
        ]);
        sig.param_types = vec![];

        let result = infer_type_params(&sig, &[]);
        assert!(result.is_some());
        assert_eq!(result.unwrap().resolve("T"), "i64");
    }

    #[test]
    fn test_builtin_bound_numeric() {
        assert!(BuiltinBound::Numeric.satisfied_by("i64"));
        assert!(BuiltinBound::Numeric.satisfied_by("f64"));
        assert!(!BuiltinBound::Numeric.satisfied_by("bool"));
        assert!(!BuiltinBound::Numeric.satisfied_by("str"));
    }

    #[test]
    fn test_builtin_bound_eq() {
        assert!(BuiltinBound::Eq.satisfied_by("i64"));
        assert!(BuiltinBound::Eq.satisfied_by("str"));
        assert!(BuiltinBound::Eq.satisfied_by("bool"));
    }

    #[test]
    fn test_builtin_bound_from_name() {
        assert_eq!(BuiltinBound::from_name("Numeric"), Some(BuiltinBound::Numeric));
        assert_eq!(BuiltinBound::from_name("Copy"), Some(BuiltinBound::Copy));
        assert_eq!(BuiltinBound::from_name("Unknown"), None);
    }

    #[test]
    fn test_substitution_display() {
        let mut sub = TypeSubstitution::new();
        sub.bind("T", "i64");
        let s = format!("{}", sub);
        assert!(s.contains("T → i64"));
    }

    #[test]
    fn test_sig_param_index() {
        let sig = GenericSig::new("f", vec![
            TypeParam::new("A"),
            TypeParam::new("B"),
        ]);
        assert_eq!(sig.type_param_index("A"), Some(0));
        assert_eq!(sig.type_param_index("B"), Some(1));
        assert_eq!(sig.type_param_index("C"), None);
    }

    // ─── v133 Tests: Generic Bounds Checking ────────────────────────────

    #[test]
    fn test_v133_bounds_enforced_on_instantiate_fn() {
        let mut mono = Monomorphizer::new();
        let sig = GenericSig::new("add", vec![
            TypeParam::new("T").with_bound("Numeric"),
        ]);
        mono.register_generic_fn(sig);

        // i64 satisfies Numeric → Ok
        assert_eq!(mono.instantiate_fn("add", &["i64".into()]), Some("add_i64".into()));
        // bool does NOT satisfy Numeric → None
        assert_eq!(mono.instantiate_fn("add", &["bool".into()]), None);
    }

    #[test]
    fn test_v133_bounds_enforced_on_instantiate_struct() {
        let mut mono = Monomorphizer::new();
        let sig = GenericSig::new("NumBox", vec![
            TypeParam::new("T").with_bound("Numeric"),
        ]);
        mono.register_generic_struct(sig);

        assert_eq!(mono.instantiate_struct("NumBox", &["f64".into()]), Some("NumBox_f64".into()));
        assert_eq!(mono.instantiate_struct("NumBox", &["str".into()]), None);
    }

    #[test]
    fn test_v133_try_instantiate_fn_error_details() {
        let mut mono = Monomorphizer::new();
        let sig = GenericSig::new("add", vec![
            TypeParam::new("T").with_bound("Numeric"),
        ]);
        mono.register_generic_fn(sig);

        let err = mono.try_instantiate_fn("add", &["bool".into()]).unwrap_err();
        assert_eq!(err, GenericError::BoundNotSatisfied {
            param: "T".into(),
            bound: "Numeric".into(),
            concrete_type: "bool".into(),
        });
    }

    #[test]
    fn test_v133_try_instantiate_fn_unknown() {
        let mut mono = Monomorphizer::new();
        let err = mono.try_instantiate_fn("nonexistent", &["i64".into()]).unwrap_err();
        assert_eq!(err, GenericError::UnknownGeneric("nonexistent".into()));
    }

    #[test]
    fn test_v133_try_instantiate_arity_error() {
        let mut mono = Monomorphizer::new();
        let sig = GenericSig::new("pair", vec![
            TypeParam::new("A"),
            TypeParam::new("B"),
        ]);
        mono.register_generic_fn(sig);

        let err = mono.try_instantiate_fn("pair", &["i64".into()]).unwrap_err();
        assert_eq!(err, GenericError::ArityMismatch {
            name: "pair".into(),
            expected: 2,
            got: 1,
        });
    }

    #[test]
    fn test_v133_multiple_bounds() {
        let mut mono = Monomorphizer::new();
        let sig = GenericSig::new("sort", vec![
            TypeParam::new("T").with_bound("Ord").with_bound("Copy"),
        ]);
        mono.register_generic_fn(sig);

        // i64 satisfies both Ord and Copy
        assert!(mono.try_instantiate_fn("sort", &["i64".into()]).is_ok());
        // str does NOT satisfy Ord
        assert!(mono.try_instantiate_fn("sort", &["str".into()]).is_err());
    }

    #[test]
    fn test_v133_copy_bound() {
        let mut mono = Monomorphizer::new();
        let sig = GenericSig::new("dup", vec![
            TypeParam::new("T").with_bound("Copy"),
        ]);
        mono.register_generic_fn(sig);

        assert!(mono.instantiate_fn("dup", &["i32".into()]).is_some());
        assert!(mono.instantiate_fn("dup", &["bool".into()]).is_some());
        // str is NOT Copy
        assert!(mono.instantiate_fn("dup", &["str".into()]).is_none());
    }

    #[test]
    fn test_v133_infer_with_bounds_satisfied() {
        let mut sig = GenericSig::new("add", vec![
            TypeParam::new("T").with_bound("Numeric"),
        ]);
        sig.param_types = vec!["T".into(), "T".into()];

        // i64 satisfies Numeric
        let result = try_infer_type_params(&sig, &["i64".into(), "i64".into()]);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().resolve("T"), "i64");
    }

    #[test]
    fn test_v133_infer_with_bounds_violated() {
        let mut sig = GenericSig::new("add", vec![
            TypeParam::new("T").with_bound("Numeric"),
        ]);
        sig.param_types = vec!["T".into(), "T".into()];

        // bool does NOT satisfy Numeric
        let result = try_infer_type_params(&sig, &["bool".into(), "bool".into()]);
        assert!(result.is_err());
        match result.unwrap_err() {
            GenericError::BoundNotSatisfied { param, bound, concrete_type } => {
                assert_eq!(param, "T");
                assert_eq!(bound, "Numeric");
                assert_eq!(concrete_type, "bool");
            }
            other => panic!("expected BoundNotSatisfied, got {:?}", other),
        }
    }

    #[test]
    fn test_v133_infer_conflicting_detailed() {
        let mut sig = GenericSig::new("eq", vec![TypeParam::new("T")]);
        sig.param_types = vec!["T".into(), "T".into()];

        let err = try_infer_type_params(&sig, &["i64".into(), "f64".into()]).unwrap_err();
        assert_eq!(err, GenericError::ConflictingInference {
            param: "T".into(),
            first: "i64".into(),
            second: "f64".into(),
        });
    }

    #[test]
    fn test_v133_infer_cannot_infer() {
        let mut sig = GenericSig::new("zero", vec![TypeParam::new("T")]);
        sig.param_types = vec![]; // no params to infer from

        let err = try_infer_type_params(&sig, &[]).unwrap_err();
        assert_eq!(err, GenericError::CannotInfer("T".into()));
    }

    #[test]
    fn test_v133_error_display() {
        let err = GenericError::BoundNotSatisfied {
            param: "T".into(),
            bound: "Numeric".into(),
            concrete_type: "bool".into(),
        };
        assert_eq!(
            format!("{}", err),
            "type `bool` does not satisfy bound `Numeric` on parameter `T`"
        );

        let err2 = GenericError::ArityMismatch {
            name: "pair".into(),
            expected: 2,
            got: 1,
        };
        assert_eq!(format!("{}", err2), "`pair` expects 2 type argument(s), got 1");
    }

    #[test]
    fn test_v133_display_bound() {
        // Display bound is satisfied by all types
        let mut mono = Monomorphizer::new();
        let sig = GenericSig::new("show", vec![
            TypeParam::new("T").with_bound("Display"),
        ]);
        mono.register_generic_fn(sig);
        assert!(mono.instantiate_fn("show", &["i32".into()]).is_some());
        assert!(mono.instantiate_fn("show", &["bool".into()]).is_some());
        assert!(mono.instantiate_fn("show", &["str".into()]).is_some());
    }

    #[test]
    fn test_v133_default_bound() {
        let mut mono = Monomorphizer::new();
        let sig = GenericSig::new("make_default", vec![
            TypeParam::new("T").with_bound("Default"),
        ]);
        mono.register_generic_fn(sig);

        assert!(mono.instantiate_fn("make_default", &["i64".into()]).is_some());
        assert!(mono.instantiate_fn("make_default", &["bool".into()]).is_some());
        // str does NOT satisfy Default
        assert!(mono.instantiate_fn("make_default", &["str".into()]).is_none());
    }

    #[test]
    fn test_v133_no_bounds_still_works() {
        // Unbounded generics should still work as before
        let mut mono = Monomorphizer::new();
        let sig = GenericSig::new("identity", vec![TypeParam::new("T")]);
        mono.register_generic_fn(sig);

        assert!(mono.instantiate_fn("identity", &["i64".into()]).is_some());
        assert!(mono.instantiate_fn("identity", &["bool".into()]).is_some());
        assert!(mono.instantiate_fn("identity", &["str".into()]).is_some());
        assert!(mono.instantiate_fn("identity", &["MyStruct".into()]).is_some());
    }

    #[test]
    fn test_v133_unknown_bound_ignored() {
        // User-defined bounds (not in BuiltinBound) should be ignored, not rejected
        let mut mono = Monomorphizer::new();
        let sig = GenericSig::new("widget", vec![
            TypeParam::new("T").with_bound("MyCustomTrait"),
        ]);
        mono.register_generic_fn(sig);

        // Unknown bound is ignored → instantiation succeeds
        assert!(mono.instantiate_fn("widget", &["anything".into()]).is_some());
    }

    #[test]
    fn test_v133_substitution_all_bound() {
        let params = vec![
            TypeParam::new("A"),
            TypeParam::new("B"),
        ];
        let mut sub = TypeSubstitution::new();
        assert!(!sub.all_bound(&params));
        sub.bind("A", "i64");
        assert!(!sub.all_bound(&params));
        sub.bind("B", "str");
        assert!(sub.all_bound(&params));
    }

    #[test]
    fn test_v133_try_instantiate_struct_error() {
        let mut mono = Monomorphizer::new();
        let sig = GenericSig::new("NumPair", vec![
            TypeParam::new("A").with_bound("Numeric"),
            TypeParam::new("B").with_bound("Numeric"),
        ]);
        mono.register_generic_struct(sig);

        // Both numeric → ok
        assert!(mono.try_instantiate_struct("NumPair", &["i64".into(), "f64".into()]).is_ok());
        // First ok, second fails
        let err = mono.try_instantiate_struct("NumPair", &["i64".into(), "bool".into()]).unwrap_err();
        assert_eq!(err, GenericError::BoundNotSatisfied {
            param: "B".into(),
            bound: "Numeric".into(),
            concrete_type: "bool".into(),
        });
    }

    #[test]
    fn test_v133_infer_default_with_bounds() {
        let mut sig = GenericSig::new("zero", vec![
            TypeParam::new("T").with_bound("Numeric").with_default("i64"),
        ]);
        sig.param_types = vec![];

        // Default i64 satisfies Numeric → ok
        let result = try_infer_type_params(&sig, &[]);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().resolve("T"), "i64");
    }
}
