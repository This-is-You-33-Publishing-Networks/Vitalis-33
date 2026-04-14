//! Vitalis Standard Library — Phase 1 built-in functions.
//!
//! Covers: typed I/O, math (f64 + i64), type conversions, string operations.
//! All functions are registered with the JIT compiler as extern symbols
//! and callable directly from Vitalis (.sl) source code.

use std::collections::HashMap;
use crate::ir::IrType;

/// Describes a built-in function for the compiler.
#[derive(Debug, Clone)]
pub struct BuiltinFn {
    pub name: String,
    pub params: Vec<(&'static str, IrType)>,
    pub ret: IrType,
    pub runtime_name: String,
}

/// Returns the full set of Phase 1 built-in functions.
pub fn builtins() -> Vec<BuiltinFn> {
    vec![
        // ── I/O ──────────────────────────────────────────────────────
        BuiltinFn { name: "print".into(),        params: vec![("value", IrType::I64)],  ret: IrType::Void, runtime_name: "slang_print_i64".into() },
        BuiltinFn { name: "println".into(),      params: vec![("value", IrType::I64)],  ret: IrType::Void, runtime_name: "slang_println_i64".into() },
        BuiltinFn { name: "print_f64".into(),    params: vec![("value", IrType::F64)],  ret: IrType::Void, runtime_name: "slang_print_f64".into() },
        BuiltinFn { name: "println_f64".into(),  params: vec![("value", IrType::F64)],  ret: IrType::Void, runtime_name: "slang_println_f64".into() },
        BuiltinFn { name: "print_bool".into(),   params: vec![("value", IrType::Bool)], ret: IrType::Void, runtime_name: "slang_print_bool".into() },
        BuiltinFn { name: "println_bool".into(), params: vec![("value", IrType::Bool)], ret: IrType::Void, runtime_name: "slang_println_bool".into() },
        BuiltinFn { name: "print_str".into(),    params: vec![("s", IrType::Ptr)],      ret: IrType::Void, runtime_name: "slang_print_cstr".into() },
        BuiltinFn { name: "println_str".into(),  params: vec![("s", IrType::Ptr)],      ret: IrType::Void, runtime_name: "slang_println_cstr".into() },

        // ── Math (f64) ────────────────────────────────────────────────
        BuiltinFn { name: "sqrt".into(),   params: vec![("x", IrType::F64)], ret: IrType::F64, runtime_name: "slang_sqrt_f64".into() },
        BuiltinFn { name: "ln".into(),     params: vec![("x", IrType::F64)], ret: IrType::F64, runtime_name: "slang_ln_f64".into() },
        BuiltinFn { name: "log2".into(),   params: vec![("x", IrType::F64)], ret: IrType::F64, runtime_name: "slang_log2_f64".into() },
        BuiltinFn { name: "log10".into(),  params: vec![("x", IrType::F64)], ret: IrType::F64, runtime_name: "slang_log10_f64".into() },
        BuiltinFn { name: "sin".into(),    params: vec![("x", IrType::F64)], ret: IrType::F64, runtime_name: "slang_sin_f64".into() },
        BuiltinFn { name: "cos".into(),    params: vec![("x", IrType::F64)], ret: IrType::F64, runtime_name: "slang_cos_f64".into() },
        BuiltinFn { name: "exp".into(),    params: vec![("x", IrType::F64)], ret: IrType::F64, runtime_name: "slang_exp_f64".into() },
        BuiltinFn { name: "floor".into(),  params: vec![("x", IrType::F64)], ret: IrType::F64, runtime_name: "slang_floor_f64".into() },
        BuiltinFn { name: "ceil".into(),   params: vec![("x", IrType::F64)], ret: IrType::F64, runtime_name: "slang_ceil_f64".into() },
        BuiltinFn { name: "round".into(),  params: vec![("x", IrType::F64)], ret: IrType::F64, runtime_name: "slang_round_f64".into() },
        BuiltinFn { name: "abs_f64".into(), params: vec![("x", IrType::F64)], ret: IrType::F64, runtime_name: "slang_abs_f64".into() },
        BuiltinFn { name: "pow".into(),    params: vec![("base", IrType::F64), ("exp", IrType::F64)], ret: IrType::F64, runtime_name: "slang_pow_f64".into() },
        BuiltinFn { name: "min_f64".into(), params: vec![("a", IrType::F64), ("b", IrType::F64)], ret: IrType::F64, runtime_name: "slang_min_f64".into() },
        BuiltinFn { name: "max_f64".into(), params: vec![("a", IrType::F64), ("b", IrType::F64)], ret: IrType::F64, runtime_name: "slang_max_f64".into() },

        // ── Math (i64) ────────────────────────────────────────────────
        BuiltinFn { name: "abs".into(),    params: vec![("x", IrType::I64)], ret: IrType::I64, runtime_name: "slang_abs_i64".into() },
        BuiltinFn { name: "min".into(),    params: vec![("a", IrType::I64), ("b", IrType::I64)], ret: IrType::I64, runtime_name: "slang_min_i64".into() },
        BuiltinFn { name: "max".into(),    params: vec![("a", IrType::I64), ("b", IrType::I64)], ret: IrType::I64, runtime_name: "slang_max_i64".into() },

        // ── Type conversions ──────────────────────────────────────────
        BuiltinFn { name: "to_f64".into(),    params: vec![("x", IrType::I64)], ret: IrType::F64, runtime_name: "slang_i64_to_f64".into() },
        BuiltinFn { name: "to_i64".into(),    params: vec![("x", IrType::F64)], ret: IrType::I64, runtime_name: "slang_f64_to_i64".into() },
        BuiltinFn { name: "i64_to_f64".into(), params: vec![("x", IrType::I64)], ret: IrType::F64, runtime_name: "slang_i64_to_f64".into() },
        BuiltinFn { name: "f64_to_i64".into(), params: vec![("x", IrType::F64)], ret: IrType::I64, runtime_name: "slang_f64_to_i64".into() },

        // ── String operations ─────────────────────────────────────────
        BuiltinFn { name: "str_len".into(), params: vec![("s", IrType::Ptr)], ret: IrType::I64, runtime_name: "slang_str_len".into() },
        BuiltinFn { name: "str_eq".into(),  params: vec![("a", IrType::Ptr), ("b", IrType::Ptr)], ret: IrType::Bool, runtime_name: "slang_str_eq".into() },
        BuiltinFn { name: "str_cat".into(), params: vec![("a", IrType::Ptr), ("b", IrType::Ptr)], ret: IrType::Ptr,  runtime_name: "slang_str_cat".into() },

        // ── Extended math ─────────────────────────────────────────────
        BuiltinFn { name: "atan2".into(),     params: vec![("y", IrType::F64), ("x", IrType::F64)], ret: IrType::F64, runtime_name: "slang_atan2_f64".into() },
        BuiltinFn { name: "hypot".into(),     params: vec![("a", IrType::F64), ("b", IrType::F64)], ret: IrType::F64, runtime_name: "slang_hypot_f64".into() },
        BuiltinFn { name: "clamp_f64".into(), params: vec![("x", IrType::F64), ("lo", IrType::F64), ("hi", IrType::F64)], ret: IrType::F64, runtime_name: "slang_clamp_f64".into() },
        BuiltinFn { name: "clamp_i64".into(), params: vec![("x", IrType::I64), ("lo", IrType::I64), ("hi", IrType::I64)], ret: IrType::I64, runtime_name: "slang_clamp_i64".into() },
        BuiltinFn { name: "clamp".into(),     params: vec![("x", IrType::F64), ("lo", IrType::F64), ("hi", IrType::F64)], ret: IrType::F64, runtime_name: "slang_clamp_f64".into() },

        // ── Randomness (Xorshift64, no deps) ─────────────────────────
        BuiltinFn { name: "rand_f64".into(), params: vec![], ret: IrType::F64, runtime_name: "slang_rand_f64".into() },
        BuiltinFn { name: "rand_i64".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_rand_i64".into() },

        // ── Time ──────────────────────────────────────────────────────
        BuiltinFn { name: "clock_ns".into(),  params: vec![], ret: IrType::I64, runtime_name: "slang_clock_ns".into() },
        BuiltinFn { name: "clock_ms".into(),  params: vec![], ret: IrType::I64, runtime_name: "slang_clock_ms".into() },

        // ── Assertions (for test / debugging) ─────────────────────────
        BuiltinFn { name: "assert_eq".into(), params: vec![("a", IrType::I64), ("b", IrType::I64)], ret: IrType::Void, runtime_name: "slang_assert_eq_i64".into() },
        BuiltinFn { name: "assert_true".into(), params: vec![("cond", IrType::Bool)], ret: IrType::Void, runtime_name: "slang_assert_true".into() },

        // ── Bitwise operations ────────────────────────────────────────
        BuiltinFn { name: "popcount".into(), params: vec![("x", IrType::I64)], ret: IrType::I64, runtime_name: "slang_popcount".into() },
        BuiltinFn { name: "leading_zeros".into(), params: vec![("x", IrType::I64)], ret: IrType::I64, runtime_name: "slang_leading_zeros".into() },
        BuiltinFn { name: "trailing_zeros".into(), params: vec![("x", IrType::I64)], ret: IrType::I64, runtime_name: "slang_trailing_zeros".into() },

        // ── Extended math ─────────────────────────────────────────────
        BuiltinFn { name: "sign".into(), params: vec![("x", IrType::I64)], ret: IrType::I64, runtime_name: "slang_sign_i64".into() },
        BuiltinFn { name: "gcd".into(), params: vec![("a", IrType::I64), ("b", IrType::I64)], ret: IrType::I64, runtime_name: "slang_gcd".into() },
        BuiltinFn { name: "lcm".into(), params: vec![("a", IrType::I64), ("b", IrType::I64)], ret: IrType::I64, runtime_name: "slang_lcm".into() },
        BuiltinFn { name: "factorial".into(), params: vec![("n", IrType::I64)], ret: IrType::I64, runtime_name: "slang_factorial".into() },
        BuiltinFn { name: "fibonacci".into(), params: vec![("n", IrType::I64)], ret: IrType::I64, runtime_name: "slang_fibonacci".into() },
        BuiltinFn { name: "is_prime".into(), params: vec![("n", IrType::I64)], ret: IrType::Bool, runtime_name: "slang_is_prime".into() },
        BuiltinFn { name: "tan".into(), params: vec![("x", IrType::F64)], ret: IrType::F64, runtime_name: "slang_tan_f64".into() },
        BuiltinFn { name: "asin".into(), params: vec![("x", IrType::F64)], ret: IrType::F64, runtime_name: "slang_asin_f64".into() },
        BuiltinFn { name: "acos".into(), params: vec![("x", IrType::F64)], ret: IrType::F64, runtime_name: "slang_acos_f64".into() },
        BuiltinFn { name: "atan".into(), params: vec![("x", IrType::F64)], ret: IrType::F64, runtime_name: "slang_atan_f64".into() },

        // ── Hash & identity ───────────────────────────────────────────
        BuiltinFn { name: "hash".into(), params: vec![("x", IrType::I64)], ret: IrType::I64, runtime_name: "slang_hash_i64".into() },

        // ── Numeric utils ─────────────────────────────────────────────
        BuiltinFn { name: "lerp".into(), params: vec![("a", IrType::F64), ("b", IrType::F64), ("t", IrType::F64)], ret: IrType::F64, runtime_name: "slang_lerp_f64".into() },
        BuiltinFn { name: "smoothstep".into(), params: vec![("edge0", IrType::F64), ("edge1", IrType::F64), ("x", IrType::F64)], ret: IrType::F64, runtime_name: "slang_smoothstep_f64".into() },
        BuiltinFn { name: "wrap".into(), params: vec![("x", IrType::I64), ("lo", IrType::I64), ("hi", IrType::I64)], ret: IrType::I64, runtime_name: "slang_wrap_i64".into() },
        BuiltinFn { name: "map_range".into(), params: vec![("x", IrType::F64), ("in_lo", IrType::F64), ("in_hi", IrType::F64), ("out_lo", IrType::F64), ("out_hi", IrType::F64)], ret: IrType::F64, runtime_name: "slang_map_range_f64".into() },

        // ── Epoch timestamp ───────────────────────────────────────────
        BuiltinFn { name: "epoch_secs".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_epoch_secs".into() },

        // ── Phase 22: AI & Numeric functions ──────────────────────────
        BuiltinFn { name: "fma".into(), params: vec![("a", IrType::F64), ("b", IrType::F64), ("c", IrType::F64)], ret: IrType::F64, runtime_name: "slang_fma_f64".into() },
        BuiltinFn { name: "cbrt".into(), params: vec![("x", IrType::F64)], ret: IrType::F64, runtime_name: "slang_cbrt_f64".into() },
        BuiltinFn { name: "deg_to_rad".into(), params: vec![("x", IrType::F64)], ret: IrType::F64, runtime_name: "slang_deg_to_rad".into() },
        BuiltinFn { name: "rad_to_deg".into(), params: vec![("x", IrType::F64)], ret: IrType::F64, runtime_name: "slang_rad_to_deg".into() },
        BuiltinFn { name: "sigmoid".into(), params: vec![("x", IrType::F64)], ret: IrType::F64, runtime_name: "slang_sigmoid_f64".into() },
        BuiltinFn { name: "relu".into(), params: vec![("x", IrType::F64)], ret: IrType::F64, runtime_name: "slang_relu_f64".into() },
        BuiltinFn { name: "tanh".into(), params: vec![("x", IrType::F64)], ret: IrType::F64, runtime_name: "slang_tanh_f64".into() },
        BuiltinFn { name: "ipow".into(), params: vec![("base", IrType::I64), ("exp", IrType::I64)], ret: IrType::I64, runtime_name: "slang_ipow".into() },

        // ── Phase 23: Extended math & AI activations ───────────────────
        BuiltinFn { name: "sinh".into(), params: vec![("x", IrType::F64)], ret: IrType::F64, runtime_name: "slang_sinh_f64".into() },
        BuiltinFn { name: "cosh".into(), params: vec![("x", IrType::F64)], ret: IrType::F64, runtime_name: "slang_cosh_f64".into() },
        BuiltinFn { name: "log".into(), params: vec![("x", IrType::F64)], ret: IrType::F64, runtime_name: "slang_log_f64".into() },
        BuiltinFn { name: "exp2".into(), params: vec![("x", IrType::F64)], ret: IrType::F64, runtime_name: "slang_exp2_f64".into() },
        BuiltinFn { name: "copysign".into(), params: vec![("x", IrType::F64), ("y", IrType::F64)], ret: IrType::F64, runtime_name: "slang_copysign_f64".into() },
        BuiltinFn { name: "fract".into(), params: vec![("x", IrType::F64)], ret: IrType::F64, runtime_name: "slang_fract_f64".into() },
        BuiltinFn { name: "trunc".into(), params: vec![("x", IrType::F64)], ret: IrType::F64, runtime_name: "slang_trunc_f64".into() },
        BuiltinFn { name: "step".into(), params: vec![("edge", IrType::F64), ("x", IrType::F64)], ret: IrType::F64, runtime_name: "slang_step_f64".into() },
        BuiltinFn { name: "leaky_relu".into(), params: vec![("x", IrType::F64), ("alpha", IrType::F64)], ret: IrType::F64, runtime_name: "slang_leaky_relu_f64".into() },
        BuiltinFn { name: "elu".into(), params: vec![("x", IrType::F64), ("alpha", IrType::F64)], ret: IrType::F64, runtime_name: "slang_elu_f64".into() },

        // ── Phase 24: Advanced AI activations & math ──────────────────
        BuiltinFn { name: "swish".into(), params: vec![("x", IrType::F64)], ret: IrType::F64, runtime_name: "slang_swish_f64".into() },
        BuiltinFn { name: "gelu".into(), params: vec![("x", IrType::F64)], ret: IrType::F64, runtime_name: "slang_gelu_f64".into() },
        BuiltinFn { name: "softplus".into(), params: vec![("x", IrType::F64)], ret: IrType::F64, runtime_name: "slang_softplus_f64".into() },
        BuiltinFn { name: "mish".into(), params: vec![("x", IrType::F64)], ret: IrType::F64, runtime_name: "slang_mish_f64".into() },
        BuiltinFn { name: "log1p".into(), params: vec![("x", IrType::F64)], ret: IrType::F64, runtime_name: "slang_log1p_f64".into() },
        BuiltinFn { name: "expm1".into(), params: vec![("x", IrType::F64)], ret: IrType::F64, runtime_name: "slang_expm1_f64".into() },
        BuiltinFn { name: "recip".into(), params: vec![("x", IrType::F64)], ret: IrType::F64, runtime_name: "slang_recip_f64".into() },
        BuiltinFn { name: "rsqrt".into(), params: vec![("x", IrType::F64)], ret: IrType::F64, runtime_name: "slang_rsqrt_f64".into() },

        // ── Phase 25: Numerical & advanced activations ────────────────
        BuiltinFn { name: "selu".into(), params: vec![("x", IrType::F64)], ret: IrType::F64, runtime_name: "slang_selu_f64".into() },
        BuiltinFn { name: "hard_sigmoid".into(), params: vec![("x", IrType::F64)], ret: IrType::F64, runtime_name: "slang_hard_sigmoid_f64".into() },
        BuiltinFn { name: "hard_swish".into(), params: vec![("x", IrType::F64)], ret: IrType::F64, runtime_name: "slang_hard_swish_f64".into() },
        BuiltinFn { name: "log_sigmoid".into(), params: vec![("x", IrType::F64)], ret: IrType::F64, runtime_name: "slang_log_sigmoid_f64".into() },
        BuiltinFn { name: "celu".into(), params: vec![("x", IrType::F64)], ret: IrType::F64, runtime_name: "slang_celu_f64".into() },
        BuiltinFn { name: "softsign".into(), params: vec![("x", IrType::F64)], ret: IrType::F64, runtime_name: "slang_softsign_f64".into() },
        BuiltinFn { name: "gaussian".into(), params: vec![("x", IrType::F64)], ret: IrType::F64, runtime_name: "slang_gaussian_f64".into() },
        BuiltinFn { name: "sinc".into(), params: vec![("x", IrType::F64)], ret: IrType::F64, runtime_name: "slang_sinc_f64".into() },
        BuiltinFn { name: "inv_sqrt_approx".into(), params: vec![("x", IrType::F64)], ret: IrType::F64, runtime_name: "slang_inv_sqrt_approx_f64".into() },
        BuiltinFn { name: "logit".into(), params: vec![("x", IrType::F64)], ret: IrType::F64, runtime_name: "slang_logit_f64".into() },

        // ── v15: String operations ────────────────────────────────────
        BuiltinFn { name: "str_upper".into(),       params: vec![("s", IrType::Ptr)],                                              ret: IrType::Ptr,  runtime_name: "slang_str_upper".into() },
        BuiltinFn { name: "str_lower".into(),       params: vec![("s", IrType::Ptr)],                                              ret: IrType::Ptr,  runtime_name: "slang_str_lower".into() },
        BuiltinFn { name: "str_trim".into(),        params: vec![("s", IrType::Ptr)],                                              ret: IrType::Ptr,  runtime_name: "slang_str_trim".into() },
        BuiltinFn { name: "str_contains".into(),    params: vec![("s", IrType::Ptr), ("sub", IrType::Ptr)],                        ret: IrType::Bool, runtime_name: "slang_str_contains".into() },
        BuiltinFn { name: "str_starts_with".into(), params: vec![("s", IrType::Ptr), ("pfx", IrType::Ptr)],                        ret: IrType::Bool, runtime_name: "slang_str_starts_with".into() },
        BuiltinFn { name: "str_ends_with".into(),   params: vec![("s", IrType::Ptr), ("sfx", IrType::Ptr)],                        ret: IrType::Bool, runtime_name: "slang_str_ends_with".into() },
        BuiltinFn { name: "str_char_at".into(),     params: vec![("s", IrType::Ptr), ("i", IrType::I64)],                          ret: IrType::Ptr,  runtime_name: "slang_str_char_at".into() },
        BuiltinFn { name: "str_substr".into(),      params: vec![("s", IrType::Ptr), ("start", IrType::I64), ("len", IrType::I64)], ret: IrType::Ptr,  runtime_name: "slang_str_substr".into() },
        BuiltinFn { name: "str_index_of".into(),    params: vec![("s", IrType::Ptr), ("sub", IrType::Ptr)],                        ret: IrType::I64,  runtime_name: "slang_str_index_of".into() },
        BuiltinFn { name: "str_replace".into(),     params: vec![("s", IrType::Ptr), ("old", IrType::Ptr), ("new", IrType::Ptr)],  ret: IrType::Ptr,  runtime_name: "slang_str_replace".into() },
        BuiltinFn { name: "str_repeat".into(),      params: vec![("s", IrType::Ptr), ("n", IrType::I64)],                          ret: IrType::Ptr,  runtime_name: "slang_str_repeat".into() },
        BuiltinFn { name: "str_reverse".into(),     params: vec![("s", IrType::Ptr)],                                              ret: IrType::Ptr,  runtime_name: "slang_str_reverse".into() },
        BuiltinFn { name: "str_split_count".into(), params: vec![("s", IrType::Ptr), ("delim", IrType::Ptr)],                      ret: IrType::I64,  runtime_name: "slang_str_split_count".into() },
        BuiltinFn { name: "str_split_get".into(),   params: vec![("s", IrType::Ptr), ("delim", IrType::Ptr), ("i", IrType::I64)],  ret: IrType::Ptr,  runtime_name: "slang_str_split_get".into() },
        BuiltinFn { name: "to_string_i64".into(),   params: vec![("x", IrType::I64)],                                              ret: IrType::Ptr,  runtime_name: "slang_to_string_i64".into() },
        BuiltinFn { name: "to_string_f64".into(),   params: vec![("x", IrType::F64)],                                              ret: IrType::Ptr,  runtime_name: "slang_to_string_f64".into() },
        BuiltinFn { name: "to_string_bool".into(),  params: vec![("x", IrType::Bool)],                                             ret: IrType::Ptr,  runtime_name: "slang_to_string_bool".into() },
        BuiltinFn { name: "str_format_i64".into(),  params: vec![("fmt", IrType::Ptr), ("val", IrType::I64)],                       ret: IrType::Ptr,  runtime_name: "slang_str_format_i64".into() },
        BuiltinFn { name: "str_format_f64".into(),  params: vec![("fmt", IrType::Ptr), ("val", IrType::F64)],                       ret: IrType::Ptr,  runtime_name: "slang_str_format_f64".into() },
        BuiltinFn { name: "str_format_str".into(),  params: vec![("fmt", IrType::Ptr), ("val", IrType::Ptr)],                       ret: IrType::Ptr,  runtime_name: "slang_str_format_str".into() },
        BuiltinFn { name: "parse_int".into(),       params: vec![("s", IrType::Ptr)],                                              ret: IrType::I64,  runtime_name: "slang_parse_int".into() },
        BuiltinFn { name: "parse_float".into(),     params: vec![("s", IrType::Ptr)],                                              ret: IrType::F64,  runtime_name: "slang_parse_float".into() },

        // ── v15: File I/O ─────────────────────────────────────────────
        BuiltinFn { name: "file_read".into(),       params: vec![("path", IrType::Ptr)],                              ret: IrType::Ptr,  runtime_name: "slang_file_read".into() },
        BuiltinFn { name: "file_write".into(),      params: vec![("path", IrType::Ptr), ("content", IrType::Ptr)],    ret: IrType::Bool, runtime_name: "slang_file_write".into() },
        BuiltinFn { name: "file_append".into(),     params: vec![("path", IrType::Ptr), ("content", IrType::Ptr)],    ret: IrType::Bool, runtime_name: "slang_file_append".into() },
        BuiltinFn { name: "file_exists".into(),     params: vec![("path", IrType::Ptr)],                              ret: IrType::Bool, runtime_name: "slang_file_exists".into() },
        BuiltinFn { name: "file_delete".into(),     params: vec![("path", IrType::Ptr)],                              ret: IrType::Bool, runtime_name: "slang_file_delete".into() },
        BuiltinFn { name: "file_size".into(),       params: vec![("path", IrType::Ptr)],                              ret: IrType::I64,  runtime_name: "slang_file_size".into() },

        // ── v15: Map operations ───────────────────────────────────────
        BuiltinFn { name: "map_new".into(),         params: vec![],                                                   ret: IrType::I64,  runtime_name: "slang_map_new".into() },
        BuiltinFn { name: "map_set".into(),         params: vec![("m", IrType::I64), ("k", IrType::Ptr), ("v", IrType::I64)], ret: IrType::Void, runtime_name: "slang_map_set".into() },
        BuiltinFn { name: "map_get".into(),         params: vec![("m", IrType::I64), ("k", IrType::Ptr)],             ret: IrType::I64,  runtime_name: "slang_map_get".into() },
        BuiltinFn { name: "map_has".into(),         params: vec![("m", IrType::I64), ("k", IrType::Ptr)],             ret: IrType::Bool, runtime_name: "slang_map_has".into() },
        BuiltinFn { name: "map_remove".into(),      params: vec![("m", IrType::I64), ("k", IrType::Ptr)],             ret: IrType::Void, runtime_name: "slang_map_remove".into() },
        BuiltinFn { name: "map_len".into(),         params: vec![("m", IrType::I64)],                                 ret: IrType::I64,  runtime_name: "slang_map_len".into() },
        BuiltinFn { name: "map_keys".into(),        params: vec![("m", IrType::I64)],                                 ret: IrType::Ptr,  runtime_name: "slang_map_keys".into() },

        // ── v16: Set operations ───────────────────────────────────────
        BuiltinFn { name: "set_new".into(),         params: vec![],                                                   ret: IrType::I64,  runtime_name: "slang_set_new".into() },
        BuiltinFn { name: "set_add".into(),         params: vec![("s", IrType::I64), ("v", IrType::I64)],             ret: IrType::Void, runtime_name: "slang_set_add".into() },
        BuiltinFn { name: "set_has".into(),         params: vec![("s", IrType::I64), ("v", IrType::I64)],             ret: IrType::Bool, runtime_name: "slang_set_has".into() },
        BuiltinFn { name: "set_remove".into(),      params: vec![("s", IrType::I64), ("v", IrType::I64)],             ret: IrType::Void, runtime_name: "slang_set_remove".into() },
        BuiltinFn { name: "set_len".into(),         params: vec![("s", IrType::I64)],                                 ret: IrType::I64,  runtime_name: "slang_set_len".into() },
        BuiltinFn { name: "set_union".into(),       params: vec![("a", IrType::I64), ("b", IrType::I64)],             ret: IrType::I64,  runtime_name: "slang_set_union".into() },
        BuiltinFn { name: "set_intersect".into(),   params: vec![("a", IrType::I64), ("b", IrType::I64)],             ret: IrType::I64,  runtime_name: "slang_set_intersect".into() },
        BuiltinFn { name: "set_diff".into(),        params: vec![("a", IrType::I64), ("b", IrType::I64)],             ret: IrType::I64,  runtime_name: "slang_set_diff".into() },
        BuiltinFn { name: "set_to_array".into(),    params: vec![("s", IrType::I64)],                                 ret: IrType::Ptr,  runtime_name: "slang_set_to_array".into() },

        // ── v18: Tuple operations ─────────────────────────────────────
        BuiltinFn { name: "tuple_new2".into(),     params: vec![("a", IrType::I64), ("b", IrType::I64)],             ret: IrType::I64,  runtime_name: "slang_tuple_new2".into() },
        BuiltinFn { name: "tuple_new3".into(),     params: vec![("a", IrType::I64), ("b", IrType::I64), ("c", IrType::I64)], ret: IrType::I64, runtime_name: "slang_tuple_new3".into() },
        BuiltinFn { name: "tuple_new4".into(),     params: vec![("a", IrType::I64), ("b", IrType::I64), ("c", IrType::I64), ("d", IrType::I64)], ret: IrType::I64, runtime_name: "slang_tuple_new4".into() },
        BuiltinFn { name: "tuple_get".into(),      params: vec![("t", IrType::I64), ("idx", IrType::I64)],           ret: IrType::I64,  runtime_name: "slang_tuple_get".into() },
        BuiltinFn { name: "tuple_len".into(),      params: vec![("t", IrType::I64)],                                 ret: IrType::I64,  runtime_name: "slang_tuple_len".into() },

        // ── v15: Error handling ───────────────────────────────────────
        BuiltinFn { name: "error_set".into(),       params: vec![("code", IrType::I64), ("msg", IrType::Ptr)],        ret: IrType::Void, runtime_name: "slang_error_set".into() },
        BuiltinFn { name: "error_check".into(),     params: vec![],                                                   ret: IrType::I64,  runtime_name: "slang_error_check".into() },
        BuiltinFn { name: "error_msg".into(),       params: vec![],                                                   ret: IrType::Ptr,  runtime_name: "slang_error_msg".into() },
        BuiltinFn { name: "error_clear".into(),     params: vec![],                                                   ret: IrType::Void, runtime_name: "slang_error_clear".into() },

        // ── v15: Environment & System ─────────────────────────────────
        BuiltinFn { name: "env_get".into(),         params: vec![("key", IrType::Ptr)],                               ret: IrType::Ptr,  runtime_name: "slang_env_get".into() },
        BuiltinFn { name: "sleep_ms".into(),        params: vec![("ms", IrType::I64)],                                ret: IrType::Void, runtime_name: "slang_sleep_ms".into() },
        BuiltinFn { name: "eprint".into(),          params: vec![("s", IrType::Ptr)],                                 ret: IrType::Void, runtime_name: "slang_eprint".into() },
        BuiltinFn { name: "eprintln".into(),        params: vec![("s", IrType::Ptr)],                                 ret: IrType::Void, runtime_name: "slang_eprintln".into() },
        BuiltinFn { name: "pid".into(),             params: vec![],                                                   ret: IrType::I64,  runtime_name: "slang_pid".into() },

        // ── v142: Runtime Logging ─────────────────────────────────────
        BuiltinFn { name: "log_trace".into(),       params: vec![("msg", IrType::Ptr)],                               ret: IrType::Void, runtime_name: "slang_log_trace".into() },
        BuiltinFn { name: "log_debug".into(),       params: vec![("msg", IrType::Ptr)],                               ret: IrType::Void, runtime_name: "slang_log_debug".into() },
        BuiltinFn { name: "log_info".into(),        params: vec![("msg", IrType::Ptr)],                               ret: IrType::Void, runtime_name: "slang_log_info".into() },
        BuiltinFn { name: "log_warn".into(),        params: vec![("msg", IrType::Ptr)],                               ret: IrType::Void, runtime_name: "slang_log_warn".into() },
        BuiltinFn { name: "log_error".into(),       params: vec![("msg", IrType::Ptr)],                               ret: IrType::Void, runtime_name: "slang_log_error".into() },
        BuiltinFn { name: "log_level_set".into(),   params: vec![("level", IrType::I64)],                             ret: IrType::Void, runtime_name: "slang_log_level_set".into() },
        BuiltinFn { name: "log_level_get".into(),   params: vec![],                                                   ret: IrType::I64,  runtime_name: "slang_log_level_get".into() },

        // ── v143: Structured Audit Trail ─────────────────────────────
        BuiltinFn { name: "audit_event".into(),     params: vec![("category", IrType::Ptr), ("action", IrType::Ptr), ("detail", IrType::Ptr)], ret: IrType::Void, runtime_name: "slang_audit_event".into() },
        BuiltinFn { name: "audit_count".into(),     params: vec![],                                                   ret: IrType::I64,  runtime_name: "slang_audit_count".into() },
        BuiltinFn { name: "audit_dump".into(),      params: vec![],                                                   ret: IrType::Void, runtime_name: "slang_audit_dump".into() },
        BuiltinFn { name: "audit_clear".into(),     params: vec![],                                                   ret: IrType::Void, runtime_name: "slang_audit_clear".into() },
        BuiltinFn { name: "audit_last".into(),      params: vec![("buf", IrType::Ptr), ("buf_len", IrType::I64)],     ret: IrType::I64,  runtime_name: "slang_audit_last".into() },

        // ── v144: Metrics & Telemetry ───────────────────────────────
        BuiltinFn { name: "metric_counter".into(),     params: vec![("name", IrType::Ptr), ("delta", IrType::I64)],      ret: IrType::Void, runtime_name: "slang_metric_counter".into() },
        BuiltinFn { name: "metric_gauge".into(),       params: vec![("name", IrType::Ptr), ("value", IrType::F64)],      ret: IrType::Void, runtime_name: "slang_metric_gauge".into() },
        BuiltinFn { name: "metric_histogram".into(),   params: vec![("name", IrType::Ptr), ("value", IrType::F64)],      ret: IrType::Void, runtime_name: "slang_metric_histogram".into() },
        BuiltinFn { name: "metric_get_counter".into(), params: vec![("name", IrType::Ptr)],                               ret: IrType::F64,  runtime_name: "slang_metric_get_counter".into() },
        BuiltinFn { name: "metric_get_gauge".into(),   params: vec![("name", IrType::Ptr)],                               ret: IrType::F64,  runtime_name: "slang_metric_get_gauge".into() },
        BuiltinFn { name: "metric_dump".into(),        params: vec![],                                                   ret: IrType::Void, runtime_name: "slang_metric_dump".into() },
        BuiltinFn { name: "metric_clear".into(),       params: vec![],                                                   ret: IrType::Void, runtime_name: "slang_metric_clear".into() },

        // ── v145: Distributed Tracing Builtins ───────────────────────
        BuiltinFn { name: "span_start".into(),        params: vec![("name", IrType::Ptr)],                               ret: IrType::I64,  runtime_name: "slang_span_start".into() },
        BuiltinFn { name: "span_end".into(),          params: vec![],                                                   ret: IrType::I64,  runtime_name: "slang_span_end".into() },
        BuiltinFn { name: "span_set_tag".into(),      params: vec![("key", IrType::Ptr), ("value", IrType::Ptr)],        ret: IrType::Void, runtime_name: "slang_span_set_tag".into() },
        BuiltinFn { name: "trace_id".into(),          params: vec![],                                                   ret: IrType::Ptr,  runtime_name: "slang_trace_id".into() },
        BuiltinFn { name: "span_depth".into(),        params: vec![],                                                   ret: IrType::I64,  runtime_name: "slang_span_depth".into() },

        // ── v146: Health Check & Runtime Diagnostics ───────────────────
        BuiltinFn { name: "runtime_uptime_ms".into(),   params: vec![],                           ret: IrType::I64,  runtime_name: "slang_runtime_uptime_ms".into() },
        BuiltinFn { name: "runtime_memory_used".into(), params: vec![],                           ret: IrType::I64,  runtime_name: "slang_runtime_memory_used".into() },
        BuiltinFn { name: "runtime_version".into(),     params: vec![],                           ret: IrType::Ptr,  runtime_name: "slang_runtime_version".into() },
        BuiltinFn { name: "runtime_alloc_count".into(), params: vec![("n", IrType::I64)],          ret: IrType::I64,  runtime_name: "slang_runtime_alloc_count".into() },
        BuiltinFn { name: "runtime_alloc_total".into(), params: vec![],                           ret: IrType::I64,  runtime_name: "slang_runtime_alloc_total".into() },
        BuiltinFn { name: "runtime_cpu_count".into(),   params: vec![],                           ret: IrType::I64,  runtime_name: "slang_runtime_cpu_count".into() },

        // ── v147: Observable Pipeline Integration ────────────────────
        BuiltinFn { name: "pipeline_timer_start".into(),   params: vec![("name", IrType::Ptr)],      ret: IrType::I64,  runtime_name: "slang_pipeline_timer_start".into() },
        BuiltinFn { name: "pipeline_timer_end".into(),     params: vec![("name", IrType::Ptr)],      ret: IrType::I64,  runtime_name: "slang_pipeline_timer_end".into() },
        BuiltinFn { name: "pipeline_stage_count".into(),   params: vec![],                           ret: IrType::I64,  runtime_name: "slang_pipeline_stage_count".into() },
        BuiltinFn { name: "pipeline_dump_timings".into(),  params: vec![],                           ret: IrType::Void, runtime_name: "slang_pipeline_dump_timings".into() },
        BuiltinFn { name: "pipeline_clear_timings".into(), params: vec![],                           ret: IrType::Void, runtime_name: "slang_pipeline_clear_timings".into() },

        // ── v148: RBAC & Capability Permissions ──────────────────────
        BuiltinFn { name: "permission_check".into(),  params: vec![("capability", IrType::Ptr)],   ret: IrType::I64,  runtime_name: "slang_permission_check".into() },
        BuiltinFn { name: "permission_grant".into(),  params: vec![("capability", IrType::Ptr)],   ret: IrType::I64,  runtime_name: "slang_permission_grant".into() },
        BuiltinFn { name: "permission_revoke".into(), params: vec![("capability", IrType::Ptr)],   ret: IrType::I64,  runtime_name: "slang_permission_revoke".into() },
        BuiltinFn { name: "permission_list".into(),   params: vec![],                              ret: IrType::Ptr,  runtime_name: "slang_permission_list".into() },
        BuiltinFn { name: "permission_clear".into(),  params: vec![],                              ret: IrType::I64,  runtime_name: "slang_permission_clear".into() },

        // ── v149: Cryptographic Signing ────────────────────────────
        BuiltinFn { name: "crypto_sha256".into(),       params: vec![("msg", IrType::Ptr)],                                      ret: IrType::Ptr,  runtime_name: "slang_crypto_sha256".into() },
        BuiltinFn { name: "crypto_hmac_sign".into(),    params: vec![("key", IrType::Ptr), ("msg", IrType::Ptr)],                  ret: IrType::Ptr,  runtime_name: "slang_crypto_hmac_sign".into() },
        BuiltinFn { name: "crypto_hmac_verify".into(),  params: vec![("key", IrType::Ptr), ("msg", IrType::Ptr), ("sig", IrType::Ptr)], ret: IrType::I64, runtime_name: "slang_crypto_hmac_verify".into() },
        BuiltinFn { name: "crypto_base64_encode".into(),params: vec![("data", IrType::Ptr)],                                     ret: IrType::Ptr,  runtime_name: "slang_crypto_base64_encode".into() },
        BuiltinFn { name: "crypto_base64_decode".into(),params: vec![("data", IrType::Ptr)],                                     ret: IrType::Ptr,  runtime_name: "slang_crypto_base64_decode".into() },

        // ── v150: Sandbox Enforcement ──────────────────────────────
        BuiltinFn { name: "sandbox_create".into(),    params: vec![],                              ret: IrType::I64,  runtime_name: "slang_sandbox_create".into() },
        BuiltinFn { name: "sandbox_allow".into(),     params: vec![("capability", IrType::Ptr)],    ret: IrType::I64,  runtime_name: "slang_sandbox_allow".into() },
        BuiltinFn { name: "sandbox_check".into(),     params: vec![("capability", IrType::Ptr)],    ret: IrType::I64,  runtime_name: "slang_sandbox_check".into() },
        BuiltinFn { name: "sandbox_violations".into(),params: vec![],                              ret: IrType::I64,  runtime_name: "slang_sandbox_violations".into() },
        BuiltinFn { name: "sandbox_destroy".into(),   params: vec![],                              ret: IrType::I64,  runtime_name: "slang_sandbox_destroy".into() },

        // ── v151: Security Audit Logger ───────────────────────────
        BuiltinFn { name: "security_log".into(),       params: vec![("event", IrType::Ptr), ("severity", IrType::Ptr)], ret: IrType::I64,  runtime_name: "slang_security_log".into() },
        BuiltinFn { name: "security_log_count".into(), params: vec![],                              ret: IrType::I64,  runtime_name: "slang_security_log_count".into() },
        BuiltinFn { name: "security_log_verify".into(),params: vec![],                              ret: IrType::I64,  runtime_name: "slang_security_log_verify".into() },
        BuiltinFn { name: "security_log_dump".into(),  params: vec![],                              ret: IrType::Void, runtime_name: "slang_security_log_dump".into() },
        BuiltinFn { name: "security_log_clear".into(), params: vec![],                              ret: IrType::I64,  runtime_name: "slang_security_log_clear".into() },

        // ── v152: Input Validation Framework ──────────────────────
        BuiltinFn { name: "validate_email".into(),    params: vec![("s", IrType::Ptr)],              ret: IrType::I64,  runtime_name: "slang_validate_email".into() },
        BuiltinFn { name: "validate_url".into(),      params: vec![("s", IrType::Ptr)],              ret: IrType::I64,  runtime_name: "slang_validate_url".into() },
        BuiltinFn { name: "validate_ip".into(),       params: vec![("s", IrType::Ptr)],              ret: IrType::I64,  runtime_name: "slang_validate_ip".into() },
        BuiltinFn { name: "sanitize_html".into(),     params: vec![("s", IrType::Ptr)],              ret: IrType::Ptr,  runtime_name: "slang_sanitize_html".into() },
        BuiltinFn { name: "sanitize_sql".into(),      params: vec![("s", IrType::Ptr)],              ret: IrType::Ptr,  runtime_name: "slang_sanitize_sql".into() },

        // ── v153: Secure Communication Primitives ─────────────────
        BuiltinFn { name: "secure_channel_create".into(), params: vec![],                            ret: IrType::I64,  runtime_name: "slang_secure_channel_create".into() },
        BuiltinFn { name: "secure_channel_send".into(),   params: vec![("ch", IrType::I64), ("msg", IrType::Ptr)], ret: IrType::I64, runtime_name: "slang_secure_channel_send".into() },
        BuiltinFn { name: "secure_channel_recv".into(),   params: vec![("ch", IrType::I64)],         ret: IrType::Ptr,  runtime_name: "slang_secure_channel_recv".into() },
        BuiltinFn { name: "secure_channel_close".into(),  params: vec![("ch", IrType::I64)],         ret: IrType::I64,  runtime_name: "slang_secure_channel_close".into() },
        BuiltinFn { name: "constant_time_eq".into(),      params: vec![("a", IrType::Ptr), ("b", IrType::Ptr)], ret: IrType::I64, runtime_name: "slang_constant_time_eq".into() },

        // ── v154: Autonomous Improvement Lab v2 ───────────────────
        BuiltinFn { name: "improvement_run_trial".into(),  params: vec![("name", IrType::Ptr), ("score", IrType::F64)], ret: IrType::I64, runtime_name: "slang_improvement_run_trial".into() },
        BuiltinFn { name: "improvement_best_score".into(), params: vec![],                              ret: IrType::F64, runtime_name: "slang_improvement_best_score".into() },
        BuiltinFn { name: "improvement_history_count".into(), params: vec![],                           ret: IrType::I64, runtime_name: "slang_improvement_history_count".into() },
        BuiltinFn { name: "improvement_reset".into(),      params: vec![],                              ret: IrType::I64, runtime_name: "slang_improvement_reset".into() },
        // ── v155: LLM-Guided Mutation Templates ───────────────────
        BuiltinFn { name: "mutation_apply".into(),         params: vec![("name", IrType::Ptr)],          ret: IrType::I64, runtime_name: "slang_mutation_apply".into() },
        BuiltinFn { name: "mutation_list_count".into(),    params: vec![],                              ret: IrType::I64, runtime_name: "slang_mutation_list_count".into() },
        BuiltinFn { name: "mutation_score".into(),         params: vec![("idx", IrType::I64), ("score", IrType::F64)], ret: IrType::I64, runtime_name: "slang_mutation_score".into() },
        BuiltinFn { name: "mutation_undo".into(),          params: vec![("idx", IrType::I64)],           ret: IrType::I64, runtime_name: "slang_mutation_undo".into() },
        // ── v156: Evolution Fitness Profiles ───────────────────────
        BuiltinFn { name: "fitness_register".into(),       params: vec![("name", IrType::Ptr)],          ret: IrType::I64, runtime_name: "slang_fitness_register".into() },
        BuiltinFn { name: "fitness_evaluate".into(),       params: vec![("idx", IrType::I64), ("score", IrType::F64)], ret: IrType::I64, runtime_name: "slang_fitness_evaluate".into() },
        BuiltinFn { name: "fitness_pareto_count".into(),   params: vec![],                              ret: IrType::I64, runtime_name: "slang_fitness_pareto_count".into() },
        BuiltinFn { name: "fitness_clear".into(),          params: vec![],                              ret: IrType::I64, runtime_name: "slang_fitness_clear".into() },
        // ── v157: Cross-Module Evolution ───────────────────────────
        BuiltinFn { name: "evo_cross_module".into(),       params: vec![("name", IrType::Ptr)],          ret: IrType::I64, runtime_name: "slang_evo_cross_module".into() },
        BuiltinFn { name: "evo_dep_add".into(),            params: vec![("mod_idx", IrType::I64), ("dep", IrType::Ptr)], ret: IrType::I64, runtime_name: "slang_evo_dep_add".into() },
        BuiltinFn { name: "evo_dep_check".into(),          params: vec![("mod_idx", IrType::I64)],       ret: IrType::I64, runtime_name: "slang_evo_dep_check".into() },
        BuiltinFn { name: "evo_safe_mutate".into(),        params: vec![("mod_idx", IrType::I64)],       ret: IrType::I64, runtime_name: "slang_evo_safe_mutate".into() },
        // ── v158: Evolution Checkpointing ─────────────────────────
        BuiltinFn { name: "evo_checkpoint_save".into(),    params: vec![("name", IrType::Ptr)],          ret: IrType::I64, runtime_name: "slang_evo_checkpoint_save".into() },
        BuiltinFn { name: "evo_checkpoint_load".into(),    params: vec![("name", IrType::Ptr)],          ret: IrType::I64, runtime_name: "slang_evo_checkpoint_load".into() },
        BuiltinFn { name: "evo_checkpoint_list_count".into(), params: vec![],                           ret: IrType::I64, runtime_name: "slang_evo_checkpoint_list_count".into() },
        BuiltinFn { name: "evo_checkpoint_clear".into(),   params: vec![],                              ret: IrType::I64, runtime_name: "slang_evo_checkpoint_clear".into() },
        // ── v159: Meta-Evolution v2 ───────────────────────────────
        BuiltinFn { name: "meta_evo_register".into(),      params: vec![("name", IrType::Ptr), ("score", IrType::F64)], ret: IrType::I64, runtime_name: "slang_meta_evo_register".into() },
        BuiltinFn { name: "meta_evo_select".into(),        params: vec![],                              ret: IrType::I64, runtime_name: "slang_meta_evo_select".into() },
        BuiltinFn { name: "meta_evo_converged".into(),     params: vec![],                              ret: IrType::I64, runtime_name: "slang_meta_evo_converged".into() },
        BuiltinFn { name: "meta_evo_stats_count".into(),   params: vec![],                              ret: IrType::I64, runtime_name: "slang_meta_evo_stats_count".into() },
        // ── v160: Tensor-First Types ──────────────────────────────
        BuiltinFn { name: "tensor_create".into(),          params: vec![("rank", IrType::I64)],         ret: IrType::I64, runtime_name: "slang_tensor_create".into() },
        BuiltinFn { name: "tensor_rank".into(),            params: vec![("id", IrType::I64)],           ret: IrType::I64, runtime_name: "slang_tensor_rank".into() },
        BuiltinFn { name: "tensor_size".into(),            params: vec![("id", IrType::I64)],           ret: IrType::I64, runtime_name: "slang_tensor_size".into() },
        BuiltinFn { name: "tensor_set_v160".into(),             params: vec![("id", IrType::I64), ("idx", IrType::I64), ("val", IrType::F64)], ret: IrType::I64, runtime_name: "slang_tensor_set".into() },
        BuiltinFn { name: "tensor_get_v160".into(),             params: vec![("id", IrType::I64), ("idx", IrType::I64)], ret: IrType::F64, runtime_name: "slang_tensor_get".into() },
        BuiltinFn { name: "tensor_add_v160".into(),             params: vec![("a", IrType::I64), ("b", IrType::I64)], ret: IrType::I64, runtime_name: "slang_tensor_add".into() },
        BuiltinFn { name: "tensor_mul_v160".into(),             params: vec![("a", IrType::I64), ("b", IrType::I64)], ret: IrType::I64, runtime_name: "slang_tensor_mul".into() },
        // ── v161: Auto-Differentiation ────────────────────────────
        BuiltinFn { name: "grad_compute".into(),           params: vec![("tid", IrType::I64), ("x", IrType::F64)], ret: IrType::F64, runtime_name: "slang_grad_compute".into() },
        BuiltinFn { name: "grad_forward".into(),           params: vec![("tid", IrType::I64), ("x", IrType::F64)], ret: IrType::F64, runtime_name: "slang_grad_forward".into() },
        BuiltinFn { name: "grad_reverse".into(),           params: vec![("tid", IrType::I64), ("x", IrType::F64)], ret: IrType::F64, runtime_name: "slang_grad_reverse".into() },
        BuiltinFn { name: "grad_jacobian_dim".into(),      params: vec![("tid", IrType::I64)],          ret: IrType::I64, runtime_name: "slang_grad_jacobian_dim".into() },
        // ── v162: ML Pipeline ─────────────────────────────────────
        BuiltinFn { name: "ml_linear_fit".into(),          params: vec![("x1", IrType::F64), ("y1", IrType::F64), ("x2", IrType::F64), ("y2", IrType::F64)], ret: IrType::I64, runtime_name: "slang_ml_linear_fit".into() },
        BuiltinFn { name: "ml_predict".into(),             params: vec![("mid", IrType::I64), ("x", IrType::F64)], ret: IrType::F64, runtime_name: "slang_ml_predict".into() },
        BuiltinFn { name: "ml_accuracy".into(),            params: vec![("pred", IrType::F64), ("actual", IrType::F64)], ret: IrType::F64, runtime_name: "slang_ml_accuracy".into() },
        BuiltinFn { name: "ml_loss".into(),                params: vec![("pred", IrType::F64), ("actual", IrType::F64)], ret: IrType::F64, runtime_name: "slang_ml_loss".into() },
        // ── v163: NAS ─────────────────────────────────────────────
        BuiltinFn { name: "nas_search".into(),             params: vec![("name", IrType::Ptr)],         ret: IrType::I64, runtime_name: "slang_nas_search".into() },
        BuiltinFn { name: "nas_evaluate".into(),           params: vec![("idx", IrType::I64), ("score", IrType::F64)], ret: IrType::I64, runtime_name: "slang_nas_evaluate".into() },
        BuiltinFn { name: "nas_best".into(),               params: vec![],                              ret: IrType::I64, runtime_name: "slang_nas_best".into() },
        BuiltinFn { name: "nas_count".into(),              params: vec![],                              ret: IrType::I64, runtime_name: "slang_nas_count".into() },
        // ── v164: Feature Engineering ──────────────────────────────
        BuiltinFn { name: "feature_normalize".into(),      params: vec![("val", IrType::F64), ("min", IrType::F64), ("max", IrType::F64)], ret: IrType::F64, runtime_name: "slang_feature_normalize".into() },
        BuiltinFn { name: "feature_one_hot".into(),        params: vec![("val", IrType::I64), ("target", IrType::I64)], ret: IrType::F64, runtime_name: "slang_feature_one_hot".into() },
        BuiltinFn { name: "feature_variance".into(),       params: vec![("tid", IrType::I64)],          ret: IrType::F64, runtime_name: "slang_feature_variance".into() },
        BuiltinFn { name: "feature_correlate".into(),      params: vec![("a", IrType::I64), ("b", IrType::I64)], ret: IrType::F64, runtime_name: "slang_feature_correlate".into() },
        // ── v165: Model Serialization ──────────────────────────────
        BuiltinFn { name: "model_save".into(),             params: vec![("name", IrType::Ptr)],         ret: IrType::I64, runtime_name: "slang_model_save".into() },
        BuiltinFn { name: "model_load".into(),             params: vec![("name", IrType::Ptr)],         ret: IrType::I64, runtime_name: "slang_model_load".into() },
        BuiltinFn { name: "model_version".into(),          params: vec![],                              ret: IrType::I64, runtime_name: "slang_model_version".into() },
        BuiltinFn { name: "model_compatible".into(),       params: vec![("v1", IrType::I64), ("v2", IrType::I64)], ret: IrType::I64, runtime_name: "slang_model_compatible".into() },
        // ── v166: KV Store ──────────────────────────────────────────────
        BuiltinFn { name: "kv_put".into(),                 params: vec![("key", IrType::Ptr), ("val", IrType::Ptr)], ret: IrType::I64, runtime_name: "slang_kv_put".into() },
        BuiltinFn { name: "kv_get_len".into(),             params: vec![("key", IrType::Ptr)],          ret: IrType::I64, runtime_name: "slang_kv_get_len".into() },
        BuiltinFn { name: "kv_delete".into(),              params: vec![("key", IrType::Ptr)],          ret: IrType::I64, runtime_name: "slang_kv_delete".into() },
        BuiltinFn { name: "kv_wal_count".into(),           params: vec![],                              ret: IrType::I64, runtime_name: "slang_kv_wal_count".into() },
        // ── v167: B-Tree Index ──────────────────────────────────────────
        BuiltinFn { name: "btree_insert_v167".into(),           params: vec![("key", IrType::I64), ("val", IrType::I64)], ret: IrType::I64, runtime_name: "slang_btree_insert".into() },
        BuiltinFn { name: "btree_lookup".into(),           params: vec![("key", IrType::I64)],          ret: IrType::I64, runtime_name: "slang_btree_lookup".into() },
        BuiltinFn { name: "btree_range_count".into(),      params: vec![("lo", IrType::I64), ("hi", IrType::I64)], ret: IrType::I64, runtime_name: "slang_btree_range_count".into() },
        BuiltinFn { name: "btree_count".into(),            params: vec![],                              ret: IrType::I64, runtime_name: "slang_btree_count".into() },
        // ── v168: SQL Engine ────────────────────────────────────────────
        BuiltinFn { name: "sql_create_table".into(),       params: vec![("name", IrType::Ptr)],         ret: IrType::I64, runtime_name: "slang_sql_create_table".into() },
        BuiltinFn { name: "sql_insert".into(),             params: vec![("name", IrType::Ptr), ("c1", IrType::I64), ("c2", IrType::I64)], ret: IrType::I64, runtime_name: "slang_sql_insert".into() },
        BuiltinFn { name: "sql_count".into(),              params: vec![("name", IrType::Ptr)],         ret: IrType::I64, runtime_name: "slang_sql_count".into() },
        BuiltinFn { name: "sql_sum_col0".into(),           params: vec![("name", IrType::Ptr)],         ret: IrType::I64, runtime_name: "slang_sql_sum_col0".into() },
        // ── v169: Schema Migration ──────────────────────────────────────
        BuiltinFn { name: "schema_create".into(),          params: vec![("name", IrType::Ptr)],         ret: IrType::I64, runtime_name: "slang_schema_create".into() },
        BuiltinFn { name: "schema_migrate".into(),         params: vec![("name", IrType::Ptr)],         ret: IrType::I64, runtime_name: "slang_schema_migrate".into() },
        BuiltinFn { name: "schema_version".into(),         params: vec![("name", IrType::Ptr)],         ret: IrType::I64, runtime_name: "slang_schema_version".into() },
        BuiltinFn { name: "schema_compatible".into(),      params: vec![("v1", IrType::I64), ("v2", IrType::I64)], ret: IrType::I64, runtime_name: "slang_schema_compatible".into() },
        // ── v170: Transaction Log ───────────────────────────────────────
        BuiltinFn { name: "txn_begin".into(),              params: vec![],                              ret: IrType::I64, runtime_name: "slang_txn_begin".into() },
        BuiltinFn { name: "txn_commit".into(),             params: vec![("id", IrType::I64)],           ret: IrType::I64, runtime_name: "slang_txn_commit".into() },
        BuiltinFn { name: "txn_rollback".into(),           params: vec![("id", IrType::I64)],           ret: IrType::I64, runtime_name: "slang_txn_rollback".into() },
        BuiltinFn { name: "txn_log_count".into(),          params: vec![],                              ret: IrType::I64, runtime_name: "slang_txn_log_count".into() },
        // ── v171: Data Import/Export ─────────────────────────────────────
        BuiltinFn { name: "data_buf_create".into(),        params: vec![],                              ret: IrType::I64, runtime_name: "slang_data_buf_create".into() },
        BuiltinFn { name: "data_buf_push".into(),          params: vec![("id", IrType::I64), ("val", IrType::I64)], ret: IrType::I64, runtime_name: "slang_data_buf_push".into() },
        BuiltinFn { name: "data_buf_len".into(),           params: vec![("id", IrType::I64)],           ret: IrType::I64, runtime_name: "slang_data_buf_len".into() },
        BuiltinFn { name: "data_buf_get".into(),           params: vec![("id", IrType::I64), ("idx", IrType::I64)], ret: IrType::I64, runtime_name: "slang_data_buf_get".into() },
        // ── v172: TCP Sockets ──────────────────────────────────────
        BuiltinFn { name: "tcp_create".into(),              params: vec![],                              ret: IrType::I64, runtime_name: "slang_tcp_create".into() },
        BuiltinFn { name: "tcp_sim_connect".into(),      params: vec![("sock", IrType::I64), ("port", IrType::I64)], ret: IrType::I64, runtime_name: "slang_tcp_sim_connect".into() },
        BuiltinFn { name: "tcp_connected".into(),           params: vec![("sock", IrType::I64)],         ret: IrType::I64, runtime_name: "slang_tcp_connected".into() },
        BuiltinFn { name: "tcp_sim_close".into(),           params: vec![("sock", IrType::I64)],         ret: IrType::I64, runtime_name: "slang_tcp_sim_close".into() },
        // ── v173: HTTP Client ──────────────────────────────────────
        BuiltinFn { name: "http_sim_get".into(),           params: vec![("url", IrType::Ptr)],          ret: IrType::I64, runtime_name: "slang_http_sim_get".into() },
        BuiltinFn { name: "http_sim_post".into(),          params: vec![("url", IrType::Ptr)],          ret: IrType::I64, runtime_name: "slang_http_sim_post".into() },
        BuiltinFn { name: "http_request_count".into(),      params: vec![],                              ret: IrType::I64, runtime_name: "slang_http_request_count".into() },
        BuiltinFn { name: "http_url_valid".into(),          params: vec![("url", IrType::Ptr)],          ret: IrType::I64, runtime_name: "slang_http_url_valid".into() },
        // ── v174: WebSocket ────────────────────────────────────────
        BuiltinFn { name: "ws_create".into(),               params: vec![],                              ret: IrType::I64, runtime_name: "slang_ws_create".into() },
        BuiltinFn { name: "ws_send".into(),                 params: vec![("ch", IrType::I64), ("msg", IrType::Ptr)], ret: IrType::I64, runtime_name: "slang_ws_send".into() },
        BuiltinFn { name: "ws_msg_count".into(),            params: vec![("ch", IrType::I64)],           ret: IrType::I64, runtime_name: "slang_ws_msg_count".into() },
        BuiltinFn { name: "ws_close".into(),                params: vec![("ch", IrType::I64)],           ret: IrType::I64, runtime_name: "slang_ws_close".into() },
        // ── v175: RPC Framework ─────────────────────────────────────
        BuiltinFn { name: "rpc_register".into(),            params: vec![("name", IrType::Ptr)],         ret: IrType::I64, runtime_name: "slang_rpc_register".into() },
        BuiltinFn { name: "rpc_call".into(),                params: vec![("name", IrType::Ptr)],         ret: IrType::I64, runtime_name: "slang_rpc_call".into() },
        BuiltinFn { name: "rpc_total_calls".into(),         params: vec![],                              ret: IrType::I64, runtime_name: "slang_rpc_total_calls".into() },
        BuiltinFn { name: "rpc_service_count".into(),       params: vec![],                              ret: IrType::I64, runtime_name: "slang_rpc_service_count".into() },
        // ── v176: DNS Resolution ────────────────────────────────────
        BuiltinFn { name: "dns_resolve".into(),             params: vec![("domain", IrType::Ptr)],       ret: IrType::I64, runtime_name: "slang_dns_resolve".into() },
        BuiltinFn { name: "dns_cached".into(),              params: vec![("domain", IrType::Ptr)],       ret: IrType::I64, runtime_name: "slang_dns_cached".into() },
        BuiltinFn { name: "dns_cache_size".into(),          params: vec![],                              ret: IrType::I64, runtime_name: "slang_dns_cache_size".into() },
        BuiltinFn { name: "dns_cache_flush".into(),         params: vec![],                              ret: IrType::I64, runtime_name: "slang_dns_cache_flush".into() },
        // ── v177: TLS/SSL ───────────────────────────────────────────
        BuiltinFn { name: "tls_create".into(),              params: vec![("host", IrType::Ptr)],         ret: IrType::I64, runtime_name: "slang_tls_create".into() },
        BuiltinFn { name: "tls_active".into(),              params: vec![("id", IrType::I64)],           ret: IrType::I64, runtime_name: "slang_tls_active".into() },
        BuiltinFn { name: "tls_close".into(),               params: vec![("id", IrType::I64)],           ret: IrType::I64, runtime_name: "slang_tls_close".into() },
        BuiltinFn { name: "tls_cert_valid".into(),          params: vec![("id", IrType::I64)],           ret: IrType::I64, runtime_name: "slang_tls_cert_valid".into() },
        // ── v178: REPL Enhancements ───────────────────────────────
        BuiltinFn { name: "repl_history_add".into(),       params: vec![("cmd", IrType::Ptr)],          ret: IrType::I64, runtime_name: "slang_repl_history_add".into() },
        BuiltinFn { name: "repl_history_count".into(),     params: vec![],                              ret: IrType::I64, runtime_name: "slang_repl_history_count".into() },
        BuiltinFn { name: "repl_history_clear".into(),     params: vec![],                              ret: IrType::I64, runtime_name: "slang_repl_history_clear".into() },
        BuiltinFn { name: "repl_complete_count".into(),    params: vec![("prefix", IrType::Ptr)],       ret: IrType::I64, runtime_name: "slang_repl_complete_count".into() },
        // ── v179: Package Registry ────────────────────────────────
        BuiltinFn { name: "pkg_publish".into(),            params: vec![("name", IrType::Ptr), ("major", IrType::I64), ("minor", IrType::I64), ("patch", IrType::I64)], ret: IrType::I64, runtime_name: "slang_pkg_publish".into() },
        BuiltinFn { name: "pkg_installed".into(),          params: vec![("name", IrType::Ptr)],         ret: IrType::I64, runtime_name: "slang_pkg_installed".into() },
        BuiltinFn { name: "pkg_count".into(),              params: vec![],                              ret: IrType::I64, runtime_name: "slang_pkg_count".into() },
        BuiltinFn { name: "pkg_remove".into(),             params: vec![("name", IrType::Ptr)],         ret: IrType::I64, runtime_name: "slang_pkg_remove".into() },
        // ── v180: Documentation Generator ─────────────────────────
        BuiltinFn { name: "doc_add".into(),                params: vec![("name", IrType::Ptr), ("doc", IrType::Ptr)], ret: IrType::I64, runtime_name: "slang_doc_add".into() },
        BuiltinFn { name: "doc_count".into(),              params: vec![],                              ret: IrType::I64, runtime_name: "slang_doc_count".into() },
        BuiltinFn { name: "doc_has".into(),                params: vec![("name", IrType::Ptr)],         ret: IrType::I64, runtime_name: "slang_doc_has".into() },
        BuiltinFn { name: "doc_clear".into(),              params: vec![],                              ret: IrType::I64, runtime_name: "slang_doc_clear".into() },
        // ── v181: Benchmark Suite ─────────────────────────────────
        BuiltinFn { name: "bench_record".into(),           params: vec![("name", IrType::Ptr), ("ns", IrType::F64)], ret: IrType::I64, runtime_name: "slang_bench_record".into() },
        BuiltinFn { name: "bench_count".into(),            params: vec![],                              ret: IrType::I64, runtime_name: "slang_bench_count".into() },
        BuiltinFn { name: "bench_best".into(),             params: vec![("name", IrType::Ptr)],         ret: IrType::F64, runtime_name: "slang_bench_best".into() },
        BuiltinFn { name: "bench_clear".into(),            params: vec![],                              ret: IrType::I64, runtime_name: "slang_bench_clear".into() },
        // ── v182: Profiler ────────────────────────────────────────
        BuiltinFn { name: "profile_start".into(),          params: vec![],                              ret: IrType::I64, runtime_name: "slang_profile_start".into() },
        BuiltinFn { name: "profile_stop".into(),           params: vec![],                              ret: IrType::I64, runtime_name: "slang_profile_stop".into() },
        BuiltinFn { name: "profile_sample".into(),         params: vec![],                              ret: IrType::I64, runtime_name: "slang_profile_sample".into() },
        BuiltinFn { name: "profile_samples".into(),        params: vec![],                              ret: IrType::I64, runtime_name: "slang_profile_samples".into() },
        // ── v183: Interactive Playground ───────────────────────────
        BuiltinFn { name: "playground_eval".into(),        params: vec![("code", IrType::Ptr)],         ret: IrType::I64, runtime_name: "slang_playground_eval".into() },
        BuiltinFn { name: "playground_count".into(),       params: vec![],                              ret: IrType::I64, runtime_name: "slang_playground_count".into() },
        BuiltinFn { name: "playground_clear".into(),       params: vec![],                              ret: IrType::I64, runtime_name: "slang_playground_clear".into() },
        BuiltinFn { name: "playground_last_result".into(), params: vec![],                              ret: IrType::I64, runtime_name: "slang_playground_last_result".into() },
        // ── v184: Work-Stealing Scheduler ─────────────────────────
        BuiltinFn { name: "task_submit".into(),            params: vec![("priority", IrType::I64)],     ret: IrType::I64, runtime_name: "slang_task_submit".into() },
        BuiltinFn { name: "task_queue_len".into(),         params: vec![],                              ret: IrType::I64, runtime_name: "slang_task_queue_len".into() },
        BuiltinFn { name: "task_steal".into(),             params: vec![],                              ret: IrType::I64, runtime_name: "slang_task_steal".into() },
        BuiltinFn { name: "task_queue_clear".into(),       params: vec![],                              ret: IrType::I64, runtime_name: "slang_task_queue_clear".into() },
        // ── v185: Actor Model ─────────────────────────────────────
        BuiltinFn { name: "actor_spawn".into(),            params: vec![],                              ret: IrType::I64, runtime_name: "slang_actor_spawn".into() },
        BuiltinFn { name: "actor_send".into(),             params: vec![("actor", IrType::I64), ("msg", IrType::I64)], ret: IrType::I64, runtime_name: "slang_actor_send".into() },
        BuiltinFn { name: "actor_recv".into(),             params: vec![("actor", IrType::I64)],        ret: IrType::I64, runtime_name: "slang_actor_recv".into() },
        BuiltinFn { name: "actor_mailbox_len".into(),      params: vec![("actor", IrType::I64)],        ret: IrType::I64, runtime_name: "slang_actor_mailbox_len".into() },
        // ── v186: STM ─────────────────────────────────────────────
        BuiltinFn { name: "stm_new".into(),                params: vec![("init", IrType::I64)],         ret: IrType::I64, runtime_name: "slang_stm_new".into() },
        BuiltinFn { name: "stm_read".into(),               params: vec![("id", IrType::I64)],           ret: IrType::I64, runtime_name: "slang_stm_read".into() },
        BuiltinFn { name: "stm_write".into(),              params: vec![("id", IrType::I64), ("val", IrType::I64)], ret: IrType::I64, runtime_name: "slang_stm_write".into() },
        BuiltinFn { name: "stm_cas".into(),                params: vec![("id", IrType::I64), ("expected", IrType::I64), ("new_val", IrType::I64)], ret: IrType::I64, runtime_name: "slang_stm_cas".into() },
        // ── v187: Parallel Collections ────────────────────────────
        BuiltinFn { name: "par_sum".into(),                params: vec![("tensor_id", IrType::I64)],    ret: IrType::F64, runtime_name: "slang_par_sum".into() },
        BuiltinFn { name: "par_min".into(),                params: vec![("tensor_id", IrType::I64)],    ret: IrType::F64, runtime_name: "slang_par_min".into() },
        BuiltinFn { name: "par_max".into(),                params: vec![("tensor_id", IrType::I64)],    ret: IrType::F64, runtime_name: "slang_par_max".into() },
        BuiltinFn { name: "par_count".into(),              params: vec![("tensor_id", IrType::I64)],    ret: IrType::I64, runtime_name: "slang_par_count".into() },
        // ── v188: GPU Task Scheduling ─────────────────────────────
        BuiltinFn { name: "gpu_submit".into(),             params: vec![("kernel_id", IrType::I64)],    ret: IrType::I64, runtime_name: "slang_gpu_submit".into() },
        BuiltinFn { name: "gpu_queue_len".into(),          params: vec![],                              ret: IrType::I64, runtime_name: "slang_gpu_queue_len".into() },
        BuiltinFn { name: "gpu_flush".into(),              params: vec![],                              ret: IrType::I64, runtime_name: "slang_gpu_flush".into() },
        BuiltinFn { name: "gpu_available".into(),          params: vec![],                              ret: IrType::I64, runtime_name: "slang_gpu_available".into() },
        // ── v189: Distributed Computing ───────────────────────────
        BuiltinFn { name: "dist_node_add".into(),          params: vec![("name", IrType::Ptr)],         ret: IrType::I64, runtime_name: "slang_dist_node_add".into() },
        BuiltinFn { name: "dist_node_count".into(),        params: vec![],                              ret: IrType::I64, runtime_name: "slang_dist_node_count".into() },
        BuiltinFn { name: "dist_broadcast".into(),         params: vec![("msg", IrType::I64)],          ret: IrType::I64, runtime_name: "slang_dist_broadcast".into() },
        BuiltinFn { name: "dist_reduce".into(),            params: vec![("val", IrType::I64), ("op", IrType::I64)], ret: IrType::I64, runtime_name: "slang_dist_reduce".into() },
        // ── v190: ADTs v2 ─────────────────────────────────────────
        BuiltinFn { name: "type_register".into(),          params: vec![("name", IrType::Ptr), ("variant_count", IrType::I64)], ret: IrType::I64, runtime_name: "slang_type_register".into() },
        BuiltinFn { name: "type_variant_count".into(),     params: vec![("name", IrType::Ptr)],         ret: IrType::I64, runtime_name: "slang_type_variant_count".into() },
        BuiltinFn { name: "type_count".into(),             params: vec![],                              ret: IrType::I64, runtime_name: "slang_type_count".into() },
        BuiltinFn { name: "type_exists".into(),            params: vec![("name", IrType::Ptr)],         ret: IrType::I64, runtime_name: "slang_type_exists".into() },
        // ── v191: Higher-Kinded Types ─────────────────────────────
        BuiltinFn { name: "hkt_register".into(),           params: vec![("name", IrType::Ptr), ("kind_arity", IrType::I64)], ret: IrType::I64, runtime_name: "slang_hkt_register".into() },
        BuiltinFn { name: "hkt_arity".into(),              params: vec![("name", IrType::Ptr)],         ret: IrType::I64, runtime_name: "slang_hkt_arity".into() },
        BuiltinFn { name: "hkt_count".into(),              params: vec![],                              ret: IrType::I64, runtime_name: "slang_hkt_count".into() },
        BuiltinFn { name: "hkt_exists".into(),             params: vec![("name", IrType::Ptr)],         ret: IrType::I64, runtime_name: "slang_hkt_exists".into() },
        // ── v192: Dependent Types v2 ──────────────────────────────
        BuiltinFn { name: "dep_type_check_range".into(),   params: vec![("val", IrType::I64), ("lo", IrType::I64), ("hi", IrType::I64)], ret: IrType::I64, runtime_name: "slang_dep_type_check_range".into() },
        BuiltinFn { name: "dep_type_nat".into(),           params: vec![("val", IrType::I64)],          ret: IrType::I64, runtime_name: "slang_dep_type_nat".into() },
        BuiltinFn { name: "dep_type_positive".into(),      params: vec![("val", IrType::I64)],          ret: IrType::I64, runtime_name: "slang_dep_type_positive".into() },
        BuiltinFn { name: "dep_type_bounded_add".into(),   params: vec![("a", IrType::I64), ("b", IrType::I64), ("max", IrType::I64)], ret: IrType::I64, runtime_name: "slang_dep_type_bounded_add".into() },
        // ── v193: Effect Polymorphism ─────────────────────────────
        BuiltinFn { name: "effect_register".into(),        params: vec![("name", IrType::Ptr)],         ret: IrType::I64, runtime_name: "slang_effect_register".into() },
        BuiltinFn { name: "effect_add_handler".into(),     params: vec![("effect", IrType::Ptr), ("handler", IrType::Ptr)], ret: IrType::I64, runtime_name: "slang_effect_add_handler".into() },
        BuiltinFn { name: "effect_handler_count".into(),   params: vec![("effect", IrType::Ptr)],       ret: IrType::I64, runtime_name: "slang_effect_handler_count".into() },
        BuiltinFn { name: "effect_count".into(),           params: vec![],                              ret: IrType::I64, runtime_name: "slang_effect_count".into() },
        // ── v194: Type-Level Computation ──────────────────────────
        BuiltinFn { name: "type_level_add".into(),         params: vec![("a", IrType::I64), ("b", IrType::I64)], ret: IrType::I64, runtime_name: "slang_type_level_add".into() },
        BuiltinFn { name: "type_level_mul".into(),         params: vec![("a", IrType::I64), ("b", IrType::I64)], ret: IrType::I64, runtime_name: "slang_type_level_mul".into() },
        BuiltinFn { name: "type_level_eq".into(),          params: vec![("a", IrType::I64), ("b", IrType::I64)], ret: IrType::I64, runtime_name: "slang_type_level_eq".into() },
        BuiltinFn { name: "type_level_if".into(),          params: vec![("cond", IrType::I64), ("then_val", IrType::I64), ("else_val", IrType::I64)], ret: IrType::I64, runtime_name: "slang_type_level_if".into() },
        // ── v195: Gradual Typing ──────────────────────────────────
        BuiltinFn { name: "gradual_annotate".into(),       params: vec![("var", IrType::Ptr), ("typ", IrType::Ptr)], ret: IrType::I64, runtime_name: "slang_gradual_annotate".into() },
        BuiltinFn { name: "gradual_check".into(),          params: vec![("var", IrType::Ptr), ("typ", IrType::Ptr)], ret: IrType::I64, runtime_name: "slang_gradual_check".into() },
        BuiltinFn { name: "gradual_typed_count".into(),    params: vec![],                              ret: IrType::I64, runtime_name: "slang_gradual_typed_count".into() },
        BuiltinFn { name: "gradual_is_any".into(),         params: vec![("var", IrType::Ptr)],          ret: IrType::I64, runtime_name: "slang_gradual_is_any".into() },
        // ── v196: FFI v2 ──────────────────────────────────────────
        BuiltinFn { name: "ffi_bind".into(),               params: vec![("name", IrType::Ptr), ("lang", IrType::Ptr)], ret: IrType::I64, runtime_name: "slang_ffi_bind".into() },
        BuiltinFn { name: "ffi_bound".into(),              params: vec![("name", IrType::Ptr)],         ret: IrType::I64, runtime_name: "slang_ffi_bound".into() },
        BuiltinFn { name: "ffi_count".into(),              params: vec![],                              ret: IrType::I64, runtime_name: "slang_ffi_count".into() },
        BuiltinFn { name: "ffi_remove".into(),             params: vec![("name", IrType::Ptr)],         ret: IrType::I64, runtime_name: "slang_ffi_remove".into() },
        // ── v197: Cloud-Native Deployment ─────────────────────────
        BuiltinFn { name: "cloud_deploy".into(),           params: vec![("name", IrType::Ptr), ("target", IrType::Ptr)], ret: IrType::I64, runtime_name: "slang_cloud_deploy".into() },
        BuiltinFn { name: "cloud_deployment_count".into(), params: vec![],                              ret: IrType::I64, runtime_name: "slang_cloud_deployment_count".into() },
        BuiltinFn { name: "cloud_health_check".into(),     params: vec![],                              ret: IrType::I64, runtime_name: "slang_cloud_health_check".into() },
        BuiltinFn { name: "cloud_shutdown".into(),         params: vec![],                              ret: IrType::I64, runtime_name: "slang_cloud_shutdown".into() },
        // ── v198: Self-Hosting v3 ─────────────────────────────────
        BuiltinFn { name: "bootstrap_stage".into(),        params: vec![],                              ret: IrType::I64, runtime_name: "slang_bootstrap_stage".into() },
        BuiltinFn { name: "bootstrap_advance".into(),      params: vec![],                              ret: IrType::I64, runtime_name: "slang_bootstrap_advance".into() },
        BuiltinFn { name: "bootstrap_verify".into(),       params: vec![("s1", IrType::I64), ("s2", IrType::I64)], ret: IrType::I64, runtime_name: "slang_bootstrap_verify".into() },
        BuiltinFn { name: "bootstrap_reset".into(),        params: vec![],                              ret: IrType::I64, runtime_name: "slang_bootstrap_reset".into() },
        // ── v199: AI Language Server ──────────────────────────────
        BuiltinFn { name: "ai_suggest".into(),             params: vec![("context", IrType::Ptr)],      ret: IrType::I64, runtime_name: "slang_ai_suggest".into() },
        BuiltinFn { name: "ai_suggestion_count".into(),    params: vec![],                              ret: IrType::I64, runtime_name: "slang_ai_suggestion_count".into() },
        BuiltinFn { name: "ai_explain_error".into(),       params: vec![("code", IrType::I64)],         ret: IrType::I64, runtime_name: "slang_ai_explain_error".into() },
        BuiltinFn { name: "ai_clear".into(),               params: vec![],                              ret: IrType::I64, runtime_name: "slang_ai_clear".into() },
        // ── v200: Milestone ───────────────────────────────────────
        BuiltinFn { name: "vitalis_version".into(),        params: vec![],                              ret: IrType::I64, runtime_name: "slang_vitalis_version".into() },
        BuiltinFn { name: "vitalis_module_count".into(),   params: vec![],                              ret: IrType::I64, runtime_name: "slang_vitalis_module_count".into() },
        BuiltinFn { name: "vitalis_test_count".into(),     params: vec![],                              ret: IrType::I64, runtime_name: "slang_vitalis_test_count".into() },
        BuiltinFn { name: "vitalis_builtin_count".into(),  params: vec![],                              ret: IrType::I64, runtime_name: "slang_vitalis_builtin_count".into() },
            // ── v201-v206: Spike Engine ──
            BuiltinFn { name: "spike_emit".into(), params: vec![("p0", IrType::I64), ("p1", IrType::I64), ("p2", IrType::I64)], ret: IrType::I64, runtime_name: "slang_spike_emit".into() },
            BuiltinFn { name: "spike_queue_len".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_spike_queue_len".into() },
            BuiltinFn { name: "spike_next".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_spike_next".into() },
            BuiltinFn { name: "spike_clear".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_spike_clear".into() },
            BuiltinFn { name: "neuro_compartment_create".into(), params: vec![("p0", IrType::F64), ("p1", IrType::F64)], ret: IrType::I64, runtime_name: "slang_neuro_compartment_create".into() },
            BuiltinFn { name: "neuro_compartment_step".into(), params: vec![("p0", IrType::I64), ("p1", IrType::F64)], ret: IrType::I64, runtime_name: "slang_neuro_compartment_step".into() },
            BuiltinFn { name: "neuro_dendrite_propagate".into(), params: vec![("p0", IrType::F64), ("p1", IrType::I64)], ret: IrType::I64, runtime_name: "slang_neuro_dendrite_propagate".into() },
            BuiltinFn { name: "neuro_compartment_count".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_neuro_compartment_count".into() },
            BuiltinFn { name: "synapse_conductance".into(), params: vec![("p0", IrType::F64), ("p1", IrType::F64), ("p2", IrType::F64)], ret: IrType::I64, runtime_name: "slang_synapse_conductance".into() },
            BuiltinFn { name: "synapse_stp_facilitate".into(), params: vec![("p0", IrType::F64), ("p1", IrType::F64)], ret: IrType::I64, runtime_name: "slang_synapse_stp_facilitate".into() },
            BuiltinFn { name: "synapse_stp_depress".into(), params: vec![("p0", IrType::I64), ("p1", IrType::F64)], ret: IrType::I64, runtime_name: "slang_synapse_stp_depress".into() },
            BuiltinFn { name: "synapse_count".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_synapse_count".into() },
            BuiltinFn { name: "neuro_wilson_cowan".into(), params: vec![("p0", IrType::F64), ("p1", IrType::F64), ("p2", IrType::F64), ("p3", IrType::F64), ("p4", IrType::F64), ("p5", IrType::F64), ("p6", IrType::I64)], ret: IrType::I64, runtime_name: "slang_neuro_wilson_cowan".into() },
            BuiltinFn { name: "neuro_neural_mass".into(), params: vec![("p0", IrType::F64), ("p1", IrType::F64), ("p2", IrType::F64), ("p3", IrType::F64)], ret: IrType::I64, runtime_name: "slang_neuro_neural_mass".into() },
            BuiltinFn { name: "neuro_population_activity".into(), params: vec![("p0", IrType::I64), ("p1", IrType::F64), ("p2", IrType::F64)], ret: IrType::I64, runtime_name: "slang_neuro_population_activity".into() },
            BuiltinFn { name: "neuro_population_sync".into(), params: vec![("p0", IrType::F64), ("p1", IrType::F64)], ret: IrType::I64, runtime_name: "slang_neuro_population_sync".into() },
            BuiltinFn { name: "spike_encode_rate".into(), params: vec![("p0", IrType::F64), ("p1", IrType::F64), ("p2", IrType::I64)], ret: IrType::I64, runtime_name: "slang_spike_encode_rate".into() },
            BuiltinFn { name: "spike_encode_temporal".into(), params: vec![("p0", IrType::F64), ("p1", IrType::F64), ("p2", IrType::I64)], ret: IrType::I64, runtime_name: "slang_spike_encode_temporal".into() },
            BuiltinFn { name: "spike_decode_rate".into(), params: vec![("p0", IrType::I64), ("p1", IrType::I64), ("p2", IrType::F64)], ret: IrType::I64, runtime_name: "slang_spike_decode_rate".into() },
            BuiltinFn { name: "spike_encode_phase".into(), params: vec![("p0", IrType::F64), ("p1", IrType::F64)], ret: IrType::I64, runtime_name: "slang_spike_encode_phase".into() },
            BuiltinFn { name: "neuro_mem_alloc".into(), params: vec![("p0", IrType::I64)], ret: IrType::I64, runtime_name: "slang_neuro_mem_alloc".into() },
            BuiltinFn { name: "neuro_mem_read".into(), params: vec![("p0", IrType::I64)], ret: IrType::I64, runtime_name: "slang_neuro_mem_read".into() },
            BuiltinFn { name: "neuro_mem_write".into(), params: vec![("p0", IrType::I64), ("p1", IrType::I64)], ret: IrType::I64, runtime_name: "slang_neuro_mem_write".into() },
            BuiltinFn { name: "neuro_mem_near_compute".into(), params: vec![("p0", IrType::I64), ("p1", IrType::I64), ("p2", IrType::I64)], ret: IrType::I64, runtime_name: "slang_neuro_mem_near_compute".into() },
            // ── v207-v212: Loihi Simulator ──
            BuiltinFn { name: "loihi_core_create".into(), params: vec![("p0", IrType::F64), ("p1", IrType::F64), ("p2", IrType::I64)], ret: IrType::I64, runtime_name: "slang_loihi_core_create".into() },
            BuiltinFn { name: "loihi_core_config".into(), params: vec![("p0", IrType::I64), ("p1", IrType::I64), ("p2", IrType::F64)], ret: IrType::I64, runtime_name: "slang_loihi_core_config".into() },
            BuiltinFn { name: "loihi_core_neuron_count".into(), params: vec![("p0", IrType::I64)], ret: IrType::I64, runtime_name: "slang_loihi_core_neuron_count".into() },
            BuiltinFn { name: "loihi_core_count".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_loihi_core_count".into() },
            BuiltinFn { name: "loihi_route_spike".into(), params: vec![("p0", IrType::I64), ("p1", IrType::I64)], ret: IrType::I64, runtime_name: "slang_loihi_route_spike".into() },
            BuiltinFn { name: "loihi_route_multicast".into(), params: vec![("p0", IrType::I64)], ret: IrType::I64, runtime_name: "slang_loihi_route_multicast".into() },
            BuiltinFn { name: "loihi_noc_latency".into(), params: vec![("p0", IrType::I64)], ret: IrType::I64, runtime_name: "slang_loihi_noc_latency".into() },
            BuiltinFn { name: "loihi_noc_bandwidth".into(), params: vec![("p0", IrType::I64)], ret: IrType::I64, runtime_name: "slang_loihi_noc_bandwidth".into() },
            BuiltinFn { name: "loihi_learn_stdp".into(), params: vec![("p0", IrType::I64), ("p1", IrType::F64)], ret: IrType::I64, runtime_name: "slang_loihi_learn_stdp".into() },
            BuiltinFn { name: "loihi_learn_reward".into(), params: vec![("p0", IrType::F64), ("p1", IrType::F64), ("p2", IrType::F64)], ret: IrType::I64, runtime_name: "slang_loihi_learn_reward".into() },
            BuiltinFn { name: "loihi_learn_3factor".into(), params: vec![("p0", IrType::F64), ("p1", IrType::F64), ("p2", IrType::F64), ("p3", IrType::F64)], ret: IrType::I64, runtime_name: "slang_loihi_learn_3factor".into() },
            BuiltinFn { name: "loihi_learn_config".into(), params: vec![("p0", IrType::I64), ("p1", IrType::I64)], ret: IrType::I64, runtime_name: "slang_loihi_learn_config".into() },
            BuiltinFn { name: "loihi_timestep".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_loihi_timestep".into() },
            BuiltinFn { name: "loihi_barrier_sync".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_loihi_barrier_sync".into() },
            BuiltinFn { name: "loihi_async_tick".into(), params: vec![("p0", IrType::I64)], ret: IrType::I64, runtime_name: "slang_loihi_async_tick".into() },
            BuiltinFn { name: "loihi_time_now".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_loihi_time_now".into() },
            BuiltinFn { name: "loihi_energy_spike".into(), params: vec![("p0", IrType::I64)], ret: IrType::I64, runtime_name: "slang_loihi_energy_spike".into() },
            BuiltinFn { name: "loihi_energy_compute".into(), params: vec![("p0", IrType::I64)], ret: IrType::I64, runtime_name: "slang_loihi_energy_compute".into() },
            BuiltinFn { name: "loihi_power_total".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_loihi_power_total".into() },
            BuiltinFn { name: "loihi_energy_reset".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_loihi_energy_reset".into() },
            BuiltinFn { name: "loihi_inst_soma".into(), params: vec![("p0", IrType::I64), ("p1", IrType::F64)], ret: IrType::I64, runtime_name: "slang_loihi_inst_soma".into() },
            BuiltinFn { name: "loihi_inst_synapse".into(), params: vec![("p0", IrType::I64), ("p1", IrType::F64)], ret: IrType::I64, runtime_name: "slang_loihi_inst_synapse".into() },
            BuiltinFn { name: "loihi_inst_axon".into(), params: vec![("p0", IrType::I64)], ret: IrType::I64, runtime_name: "slang_loihi_inst_axon".into() },
            BuiltinFn { name: "loihi_inst_dendrite".into(), params: vec![("p0", IrType::I64), ("p1", IrType::F64)], ret: IrType::I64, runtime_name: "slang_loihi_inst_dendrite".into() },
            // ── v213-v218: SNN Learning ──
            BuiltinFn { name: "snn_surrogate_forward".into(), params: vec![("p0", IrType::F64), ("p1", IrType::F64), ("p2", IrType::F64), ("p3", IrType::I64)], ret: IrType::I64, runtime_name: "slang_snn_surrogate_forward".into() },
            BuiltinFn { name: "snn_surrogate_backward".into(), params: vec![("p0", IrType::F64), ("p1", IrType::F64), ("p2", IrType::F64)], ret: IrType::I64, runtime_name: "slang_snn_surrogate_backward".into() },
            BuiltinFn { name: "snn_surrogate_sigmoid".into(), params: vec![("p0", IrType::F64)], ret: IrType::I64, runtime_name: "slang_snn_surrogate_sigmoid".into() },
            BuiltinFn { name: "snn_surrogate_loss".into(), params: vec![("p0", IrType::F64), ("p1", IrType::F64), ("p2", IrType::I64)], ret: IrType::I64, runtime_name: "slang_snn_surrogate_loss".into() },
            BuiltinFn { name: "snn_bptt_forward".into(), params: vec![("p0", IrType::F64), ("p1", IrType::F64), ("p2", IrType::F64), ("p3", IrType::I64)], ret: IrType::I64, runtime_name: "slang_snn_bptt_forward".into() },
            BuiltinFn { name: "snn_bptt_backward".into(), params: vec![("p0", IrType::F64), ("p1", IrType::F64), ("p2", IrType::I64)], ret: IrType::I64, runtime_name: "slang_snn_bptt_backward".into() },
            BuiltinFn { name: "snn_bptt_truncate".into(), params: vec![("p0", IrType::F64), ("p1", IrType::F64), ("p2", IrType::I64)], ret: IrType::I64, runtime_name: "slang_snn_bptt_truncate".into() },
            BuiltinFn { name: "snn_bptt_gradient".into(), params: vec![("p0", IrType::F64), ("p1", IrType::I64)], ret: IrType::I64, runtime_name: "slang_snn_bptt_gradient".into() },
            BuiltinFn { name: "snn_nas_search".into(), params: vec![("p0", IrType::I64), ("p1", IrType::I64), ("p2", IrType::F64)], ret: IrType::I64, runtime_name: "slang_snn_nas_search".into() },
            BuiltinFn { name: "snn_nas_evaluate".into(), params: vec![("p0", IrType::I64), ("p1", IrType::I64)], ret: IrType::I64, runtime_name: "slang_snn_nas_evaluate".into() },
            BuiltinFn { name: "snn_nas_mutate".into(), params: vec![("p0", IrType::I64), ("p1", IrType::I64), ("p2", IrType::I64)], ret: IrType::I64, runtime_name: "slang_snn_nas_mutate".into() },
            BuiltinFn { name: "snn_nas_best".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_snn_nas_best".into() },
            BuiltinFn { name: "snn_fed_aggregate".into(), params: vec![("p0", IrType::F64), ("p1", IrType::I64)], ret: IrType::I64, runtime_name: "slang_snn_fed_aggregate".into() },
            BuiltinFn { name: "snn_fed_share".into(), params: vec![("p0", IrType::F64), ("p1", IrType::F64)], ret: IrType::I64, runtime_name: "slang_snn_fed_share".into() },
            BuiltinFn { name: "snn_fed_round".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_snn_fed_round".into() },
            BuiltinFn { name: "snn_fed_node_count".into(), params: vec![("p0", IrType::I64)], ret: IrType::I64, runtime_name: "slang_snn_fed_node_count".into() },
            BuiltinFn { name: "snn_transfer_freeze".into(), params: vec![("p0", IrType::I64)], ret: IrType::I64, runtime_name: "slang_snn_transfer_freeze".into() },
            BuiltinFn { name: "snn_transfer_finetune".into(), params: vec![("p0", IrType::F64), ("p1", IrType::I64)], ret: IrType::I64, runtime_name: "slang_snn_transfer_finetune".into() },
            BuiltinFn { name: "snn_transfer_adapt".into(), params: vec![("p0", IrType::F64), ("p1", IrType::F64)], ret: IrType::I64, runtime_name: "slang_snn_transfer_adapt".into() },
            BuiltinFn { name: "snn_transfer_similarity".into(), params: vec![("p0", IrType::F64), ("p1", IrType::F64)], ret: IrType::I64, runtime_name: "slang_snn_transfer_similarity".into() },
            BuiltinFn { name: "snn_continual_learn".into(), params: vec![("p0", IrType::F64), ("p1", IrType::F64), ("p2", IrType::F64), ("p3", IrType::F64), ("p4", IrType::F64), ("p5", IrType::F64)], ret: IrType::I64, runtime_name: "slang_snn_continual_learn".into() },
            BuiltinFn { name: "snn_continual_consolidate".into(), params: vec![("p0", IrType::F64), ("p1", IrType::I64)], ret: IrType::I64, runtime_name: "slang_snn_continual_consolidate".into() },
            BuiltinFn { name: "snn_continual_replay".into(), params: vec![("p0", IrType::I64), ("p1", IrType::I64)], ret: IrType::I64, runtime_name: "slang_snn_continual_replay".into() },
            BuiltinFn { name: "snn_continual_forget_score".into(), params: vec![("p0", IrType::F64), ("p1", IrType::F64)], ret: IrType::I64, runtime_name: "slang_snn_continual_forget_score".into() },
            // ── v219-v224: Processing-in-Memory ──
            BuiltinFn { name: "pim_alloc".into(), params: vec![("p0", IrType::I64)], ret: IrType::I64, runtime_name: "slang_pim_alloc".into() },
            BuiltinFn { name: "pim_compute_add".into(), params: vec![("p0", IrType::I64), ("p1", IrType::I64)], ret: IrType::I64, runtime_name: "slang_pim_compute_add".into() },
            BuiltinFn { name: "pim_compute_mul".into(), params: vec![("p0", IrType::I64), ("p1", IrType::I64)], ret: IrType::I64, runtime_name: "slang_pim_compute_mul".into() },
            BuiltinFn { name: "pim_transfer_cost".into(), params: vec![("p0", IrType::I64), ("p1", IrType::I64)], ret: IrType::I64, runtime_name: "slang_pim_transfer_cost".into() },
            BuiltinFn { name: "datacentric_map".into(), params: vec![("p0", IrType::I64), ("p1", IrType::F64), ("p2", IrType::F64)], ret: IrType::I64, runtime_name: "slang_datacentric_map".into() },
            BuiltinFn { name: "datacentric_reduce".into(), params: vec![("p0", IrType::I64), ("p1", IrType::I64), ("p2", IrType::I64)], ret: IrType::I64, runtime_name: "slang_datacentric_reduce".into() },
            BuiltinFn { name: "datacentric_scatter".into(), params: vec![("p0", IrType::I64), ("p1", IrType::I64)], ret: IrType::I64, runtime_name: "slang_datacentric_scatter".into() },
            BuiltinFn { name: "datacentric_gather".into(), params: vec![("p0", IrType::I64), ("p1", IrType::F64)], ret: IrType::I64, runtime_name: "slang_datacentric_gather".into() },
            BuiltinFn { name: "sparse_spike_propagate".into(), params: vec![("p0", IrType::I64), ("p1", IrType::I64)], ret: IrType::I64, runtime_name: "slang_sparse_spike_propagate".into() },
            BuiltinFn { name: "sparse_nonzero_count".into(), params: vec![("p0", IrType::I64), ("p1", IrType::I64)], ret: IrType::I64, runtime_name: "slang_sparse_nonzero_count".into() },
            BuiltinFn { name: "sparse_compress".into(), params: vec![("p0", IrType::I64), ("p1", IrType::I64), ("p2", IrType::I64)], ret: IrType::I64, runtime_name: "slang_sparse_compress".into() },
            BuiltinFn { name: "sparse_decompress".into(), params: vec![("p0", IrType::I64), ("p1", IrType::I64)], ret: IrType::I64, runtime_name: "slang_sparse_decompress".into() },
            BuiltinFn { name: "cache_oblivious_transpose".into(), params: vec![("p0", IrType::I64)], ret: IrType::I64, runtime_name: "slang_cache_oblivious_transpose".into() },
            BuiltinFn { name: "cache_oblivious_fft".into(), params: vec![("p0", IrType::I64)], ret: IrType::I64, runtime_name: "slang_cache_oblivious_fft".into() },
            BuiltinFn { name: "cache_oblivious_sort".into(), params: vec![("p0", IrType::I64)], ret: IrType::I64, runtime_name: "slang_cache_oblivious_sort".into() },
            BuiltinFn { name: "cache_oblivious_matmul".into(), params: vec![("p0", IrType::I64)], ret: IrType::I64, runtime_name: "slang_cache_oblivious_matmul".into() },
            BuiltinFn { name: "memcompute_fused_mac".into(), params: vec![("p0", IrType::F64), ("p1", IrType::F64), ("p2", IrType::F64)], ret: IrType::I64, runtime_name: "slang_memcompute_fused_mac".into() },
            BuiltinFn { name: "memcompute_fused_compare".into(), params: vec![("p0", IrType::I64), ("p1", IrType::I64)], ret: IrType::I64, runtime_name: "slang_memcompute_fused_compare".into() },
            BuiltinFn { name: "memcompute_fused_accumulate".into(), params: vec![("p0", IrType::I64), ("p1", IrType::I64), ("p2", IrType::I64)], ret: IrType::I64, runtime_name: "slang_memcompute_fused_accumulate".into() },
            BuiltinFn { name: "memcompute_pipeline_depth".into(), params: vec![("p0", IrType::I64)], ret: IrType::I64, runtime_name: "slang_memcompute_pipeline_depth".into() },
            BuiltinFn { name: "zerocopy_spike_buffer".into(), params: vec![("p0", IrType::I64)], ret: IrType::I64, runtime_name: "slang_zerocopy_spike_buffer".into() },
            BuiltinFn { name: "zerocopy_fanout".into(), params: vec![("p0", IrType::I64), ("p1", IrType::I64)], ret: IrType::I64, runtime_name: "slang_zerocopy_fanout".into() },
            BuiltinFn { name: "zerocopy_gather".into(), params: vec![("p0", IrType::I64)], ret: IrType::I64, runtime_name: "slang_zerocopy_gather".into() },
            BuiltinFn { name: "zerocopy_active_count".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_zerocopy_active_count".into() },
            // ── v225-v230: Brain Models ──
            BuiltinFn { name: "pred_coding_forward".into(), params: vec![("p0", IrType::F64), ("p1", IrType::F64), ("p2", IrType::F64)], ret: IrType::I64, runtime_name: "slang_pred_coding_forward".into() },
            BuiltinFn { name: "pred_coding_error".into(), params: vec![("p0", IrType::F64), ("p1", IrType::F64), ("p2", IrType::F64)], ret: IrType::I64, runtime_name: "slang_pred_coding_error".into() },
            BuiltinFn { name: "pred_coding_update".into(), params: vec![("p0", IrType::F64), ("p1", IrType::F64), ("p2", IrType::F64)], ret: IrType::I64, runtime_name: "slang_pred_coding_update".into() },
            BuiltinFn { name: "pred_coding_layers".into(), params: vec![("p0", IrType::I64), ("p1", IrType::I64)], ret: IrType::I64, runtime_name: "slang_pred_coding_layers".into() },
            BuiltinFn { name: "htm_spatial_pool".into(), params: vec![("p0", IrType::I64), ("p1", IrType::I64), ("p2", IrType::I64)], ret: IrType::I64, runtime_name: "slang_htm_spatial_pool".into() },
            BuiltinFn { name: "htm_temporal_memory".into(), params: vec![("p0", IrType::I64), ("p1", IrType::I64)], ret: IrType::I64, runtime_name: "slang_htm_temporal_memory".into() },
            BuiltinFn { name: "htm_anomaly_score".into(), params: vec![("p0", IrType::I64), ("p1", IrType::I64)], ret: IrType::I64, runtime_name: "slang_htm_anomaly_score".into() },
            BuiltinFn { name: "htm_column_count".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_htm_column_count".into() },
            BuiltinFn { name: "neuro_osc_gamma".into(), params: vec![("p0", IrType::F64), ("p1", IrType::F64), ("p2", IrType::F64)], ret: IrType::I64, runtime_name: "slang_neuro_osc_gamma".into() },
            BuiltinFn { name: "neuro_osc_theta".into(), params: vec![("p0", IrType::F64), ("p1", IrType::F64), ("p2", IrType::F64)], ret: IrType::I64, runtime_name: "slang_neuro_osc_theta".into() },
            BuiltinFn { name: "neuro_osc_couple".into(), params: vec![("p0", IrType::F64), ("p1", IrType::F64), ("p2", IrType::F64)], ret: IrType::I64, runtime_name: "slang_neuro_osc_couple".into() },
            BuiltinFn { name: "neuro_osc_phase_lock".into(), params: vec![("p0", IrType::F64), ("p1", IrType::F64)], ret: IrType::I64, runtime_name: "slang_neuro_osc_phase_lock".into() },
            BuiltinFn { name: "neuromod_dopamine".into(), params: vec![("p0", IrType::F64), ("p1", IrType::F64)], ret: IrType::I64, runtime_name: "slang_neuromod_dopamine".into() },
            BuiltinFn { name: "neuromod_serotonin".into(), params: vec![("p0", IrType::F64), ("p1", IrType::F64)], ret: IrType::I64, runtime_name: "slang_neuromod_serotonin".into() },
            BuiltinFn { name: "neuromod_acetylcholine".into(), params: vec![("p0", IrType::F64), ("p1", IrType::F64)], ret: IrType::I64, runtime_name: "slang_neuromod_acetylcholine".into() },
            BuiltinFn { name: "neuromod_apply".into(), params: vec![("p0", IrType::F64), ("p1", IrType::F64), ("p2", IrType::F64)], ret: IrType::I64, runtime_name: "slang_neuromod_apply".into() },
            BuiltinFn { name: "cortical_column_create".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_cortical_column_create".into() },
            BuiltinFn { name: "cortical_column_step".into(), params: vec![("p0", IrType::I64), ("p1", IrType::F64)], ret: IrType::I64, runtime_name: "slang_cortical_column_step".into() },
            BuiltinFn { name: "cortical_column_layer_activity".into(), params: vec![("p0", IrType::I64), ("p1", IrType::I64)], ret: IrType::I64, runtime_name: "slang_cortical_column_layer_activity".into() },
            BuiltinFn { name: "cortical_column_count".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_cortical_column_count".into() },
            BuiltinFn { name: "spike_attention_query".into(), params: vec![("p0", IrType::F64), ("p1", IrType::F64), ("p2", IrType::F64)], ret: IrType::I64, runtime_name: "slang_spike_attention_query".into() },
            BuiltinFn { name: "spike_attention_key".into(), params: vec![("p0", IrType::I64), ("p1", IrType::I64)], ret: IrType::I64, runtime_name: "slang_spike_attention_key".into() },
            BuiltinFn { name: "spike_attention_value".into(), params: vec![("p0", IrType::I64), ("p1", IrType::F64)], ret: IrType::I64, runtime_name: "slang_spike_attention_value".into() },
            BuiltinFn { name: "spike_attention_score".into(), params: vec![("p0", IrType::F64), ("p1", IrType::I64)], ret: IrType::I64, runtime_name: "slang_spike_attention_score".into() },
            // ── v231-v236: GPU Neuromorphic ──
            BuiltinFn { name: "gpu_spike_propagate".into(), params: vec![("p0", IrType::I64), ("p1", IrType::I64)], ret: IrType::I64, runtime_name: "slang_gpu_spike_propagate".into() },
            BuiltinFn { name: "gpu_spike_batch_size".into(), params: vec![("p0", IrType::I64)], ret: IrType::I64, runtime_name: "slang_gpu_spike_batch_size".into() },
            BuiltinFn { name: "gpu_spike_throughput".into(), params: vec![("p0", IrType::I64), ("p1", IrType::F64)], ret: IrType::I64, runtime_name: "slang_gpu_spike_throughput".into() },
            BuiltinFn { name: "gpu_spike_sync".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_gpu_spike_sync".into() },
            BuiltinFn { name: "gpu_neuron_update".into(), params: vec![("p0", IrType::I64), ("p1", IrType::F64), ("p2", IrType::F64)], ret: IrType::I64, runtime_name: "slang_gpu_neuron_update".into() },
            BuiltinFn { name: "gpu_neuron_batch".into(), params: vec![("p0", IrType::I64), ("p1", IrType::I64)], ret: IrType::I64, runtime_name: "slang_gpu_neuron_batch".into() },
            BuiltinFn { name: "gpu_neuron_occupancy".into(), params: vec![("p0", IrType::I64), ("p1", IrType::I64)], ret: IrType::I64, runtime_name: "slang_gpu_neuron_occupancy".into() },
            BuiltinFn { name: "gpu_neuron_count".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_gpu_neuron_count".into() },
            BuiltinFn { name: "gpu_synapse_spmv".into(), params: vec![("p0", IrType::I64), ("p1", IrType::F64), ("p2", IrType::F64)], ret: IrType::I64, runtime_name: "slang_gpu_synapse_spmv".into() },
            BuiltinFn { name: "gpu_synapse_csr".into(), params: vec![("p0", IrType::I64), ("p1", IrType::I64), ("p2", IrType::I64)], ret: IrType::I64, runtime_name: "slang_gpu_synapse_csr".into() },
            BuiltinFn { name: "gpu_synapse_nnz".into(), params: vec![("p0", IrType::I64), ("p1", IrType::F64)], ret: IrType::I64, runtime_name: "slang_gpu_synapse_nnz".into() },
            BuiltinFn { name: "gpu_synapse_density".into(), params: vec![("p0", IrType::I64), ("p1", IrType::I64), ("p2", IrType::I64)], ret: IrType::I64, runtime_name: "slang_gpu_synapse_density".into() },
            BuiltinFn { name: "gpu_event_push".into(), params: vec![("p0", IrType::I64), ("p1", IrType::I64)], ret: IrType::I64, runtime_name: "slang_gpu_event_push".into() },
            BuiltinFn { name: "gpu_event_pop".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_gpu_event_pop".into() },
            BuiltinFn { name: "gpu_event_merge".into(), params: vec![("p0", IrType::I64)], ret: IrType::I64, runtime_name: "slang_gpu_event_merge".into() },
            BuiltinFn { name: "gpu_event_size".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_gpu_event_size".into() },
            BuiltinFn { name: "mixed_prec_quantize".into(), params: vec![("p0", IrType::F64), ("p1", IrType::I64)], ret: IrType::I64, runtime_name: "slang_mixed_prec_quantize".into() },
            BuiltinFn { name: "mixed_prec_dequantize".into(), params: vec![("p0", IrType::I64), ("p1", IrType::I64)], ret: IrType::I64, runtime_name: "slang_mixed_prec_dequantize".into() },
            BuiltinFn { name: "mixed_prec_accumulate".into(), params: vec![("p0", IrType::F64), ("p1", IrType::F64)], ret: IrType::I64, runtime_name: "slang_mixed_prec_accumulate".into() },
            BuiltinFn { name: "mixed_prec_bits".into(), params: vec![("p0", IrType::F64)], ret: IrType::I64, runtime_name: "slang_mixed_prec_bits".into() },
            BuiltinFn { name: "multi_gpu_partition".into(), params: vec![("p0", IrType::I64), ("p1", IrType::I64)], ret: IrType::I64, runtime_name: "slang_multi_gpu_partition".into() },
            BuiltinFn { name: "multi_gpu_sync".into(), params: vec![("p0", IrType::I64), ("p1", IrType::I64)], ret: IrType::I64, runtime_name: "slang_multi_gpu_sync".into() },
            BuiltinFn { name: "multi_gpu_migrate".into(), params: vec![("p0", IrType::I64), ("p1", IrType::I64)], ret: IrType::I64, runtime_name: "slang_multi_gpu_migrate".into() },
            BuiltinFn { name: "multi_gpu_count".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_multi_gpu_count".into() },
            // ── v237-v242: Neuro Applications ──
            BuiltinFn { name: "spike_vision_encode".into(), params: vec![("p0", IrType::I64), ("p1", IrType::I64), ("p2", IrType::F64)], ret: IrType::I64, runtime_name: "slang_spike_vision_encode".into() },
            BuiltinFn { name: "spike_vision_edge".into(), params: vec![("p0", IrType::I64), ("p1", IrType::I64), ("p2", IrType::F64)], ret: IrType::I64, runtime_name: "slang_spike_vision_edge".into() },
            BuiltinFn { name: "spike_vision_motion".into(), params: vec![("p0", IrType::I64), ("p1", IrType::I64)], ret: IrType::I64, runtime_name: "slang_spike_vision_motion".into() },
            BuiltinFn { name: "spike_vision_frames".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_spike_vision_frames".into() },
            BuiltinFn { name: "spike_audio_encode".into(), params: vec![("p0", IrType::I64), ("p1", IrType::I64), ("p2", IrType::F64)], ret: IrType::I64, runtime_name: "slang_spike_audio_encode".into() },
            BuiltinFn { name: "spike_audio_frequency".into(), params: vec![("p0", IrType::I64)], ret: IrType::I64, runtime_name: "slang_spike_audio_frequency".into() },
            BuiltinFn { name: "spike_audio_onset".into(), params: vec![("p0", IrType::I64), ("p1", IrType::I64)], ret: IrType::I64, runtime_name: "slang_spike_audio_onset".into() },
            BuiltinFn { name: "spike_audio_classify".into(), params: vec![("p0", IrType::I64), ("p1", IrType::I64)], ret: IrType::I64, runtime_name: "slang_spike_audio_classify".into() },
            BuiltinFn { name: "spike_pid".into(), params: vec![("p0", IrType::F64), ("p1", IrType::F64), ("p2", IrType::F64)], ret: IrType::I64, runtime_name: "slang_spike_pid".into() },
            BuiltinFn { name: "spike_motor".into(), params: vec![("p0", IrType::F64), ("p1", IrType::F64)], ret: IrType::I64, runtime_name: "slang_spike_motor".into() },
            BuiltinFn { name: "spike_reflex".into(), params: vec![("p0", IrType::I64)], ret: IrType::I64, runtime_name: "slang_spike_reflex".into() },
            BuiltinFn { name: "spike_trajectory_cost".into(), params: vec![("p0", IrType::F64), ("p1", IrType::I64)], ret: IrType::I64, runtime_name: "slang_spike_trajectory_cost".into() },
            BuiltinFn { name: "spike_anomaly_score".into(), params: vec![("p0", IrType::F64), ("p1", IrType::F64)], ret: IrType::I64, runtime_name: "slang_spike_anomaly_score".into() },
            BuiltinFn { name: "spike_changepoint".into(), params: vec![("p0", IrType::F64), ("p1", IrType::F64), ("p2", IrType::F64)], ret: IrType::I64, runtime_name: "slang_spike_changepoint".into() },
            BuiltinFn { name: "spike_burst_detect".into(), params: vec![("p0", IrType::F64), ("p1", IrType::F64)], ret: IrType::I64, runtime_name: "slang_spike_burst_detect".into() },
            BuiltinFn { name: "spike_pattern_match".into(), params: vec![("p0", IrType::F64)], ret: IrType::I64, runtime_name: "slang_spike_pattern_match".into() },
            BuiltinFn { name: "spike_anneal".into(), params: vec![("p0", IrType::F64), ("p1", IrType::F64), ("p2", IrType::I64)], ret: IrType::I64, runtime_name: "slang_spike_anneal".into() },
            BuiltinFn { name: "spike_gradient".into(), params: vec![("p0", IrType::F64), ("p1", IrType::F64), ("p2", IrType::F64)], ret: IrType::I64, runtime_name: "slang_spike_gradient".into() },
            BuiltinFn { name: "spike_constraint".into(), params: vec![("p0", IrType::I64), ("p1", IrType::F64)], ret: IrType::I64, runtime_name: "slang_spike_constraint".into() },
            BuiltinFn { name: "spike_fitness".into(), params: vec![("p0", IrType::F64), ("p1", IrType::F64)], ret: IrType::I64, runtime_name: "slang_spike_fitness".into() },
            BuiltinFn { name: "spike_nlp_similarity".into(), params: vec![("p0", IrType::F64), ("p1", IrType::F64), ("p2", IrType::F64)], ret: IrType::I64, runtime_name: "slang_spike_nlp_similarity".into() },
            BuiltinFn { name: "spike_nlp_attention".into(), params: vec![("p0", IrType::F64), ("p1", IrType::F64)], ret: IrType::I64, runtime_name: "slang_spike_nlp_attention".into() },
            BuiltinFn { name: "spike_nlp_encode_len".into(), params: vec![("p0", IrType::I64), ("p1", IrType::I64)], ret: IrType::I64, runtime_name: "slang_spike_nlp_encode_len".into() },
            BuiltinFn { name: "spike_nlp_perplexity".into(), params: vec![("p0", IrType::F64)], ret: IrType::I64, runtime_name: "slang_spike_nlp_perplexity".into() },
            // ── v243-v248: Neuro Evolution ──
            BuiltinFn { name: "neat_crossover".into(), params: vec![("p0", IrType::F64), ("p1", IrType::F64)], ret: IrType::I64, runtime_name: "slang_neat_crossover".into() },
            BuiltinFn { name: "neat_mutate".into(), params: vec![("p0", IrType::I64), ("p1", IrType::I64)], ret: IrType::I64, runtime_name: "slang_neat_mutate".into() },
            BuiltinFn { name: "neat_speciate".into(), params: vec![("p0", IrType::I64), ("p1", IrType::I64), ("p2", IrType::F64)], ret: IrType::I64, runtime_name: "slang_neat_speciate".into() },
            BuiltinFn { name: "neat_generation".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_neat_generation".into() },
            BuiltinFn { name: "som_bmu".into(), params: vec![("p0", IrType::F64), ("p1", IrType::F64), ("p2", IrType::I64)], ret: IrType::I64, runtime_name: "slang_som_bmu".into() },
            BuiltinFn { name: "som_radius".into(), params: vec![("p0", IrType::F64), ("p1", IrType::I64), ("p2", IrType::I64)], ret: IrType::I64, runtime_name: "slang_som_radius".into() },
            BuiltinFn { name: "som_learning_rate".into(), params: vec![("p0", IrType::F64), ("p1", IrType::I64), ("p2", IrType::I64)], ret: IrType::I64, runtime_name: "slang_som_learning_rate".into() },
            BuiltinFn { name: "som_quant_error".into(), params: vec![("p0", IrType::F64)], ret: IrType::I64, runtime_name: "slang_som_quant_error".into() },
            BuiltinFn { name: "neuro_nas_evaluate".into(), params: vec![("p0", IrType::F64), ("p1", IrType::F64), ("p2", IrType::F64)], ret: IrType::I64, runtime_name: "slang_neuro_nas_evaluate".into() },
            BuiltinFn { name: "neuro_nas_sample".into(), params: vec![("p0", IrType::I64), ("p1", IrType::I64)], ret: IrType::I64, runtime_name: "slang_neuro_nas_sample".into() },
            BuiltinFn { name: "neuro_nas_prune".into(), params: vec![("p0", IrType::I64), ("p1", IrType::F64)], ret: IrType::I64, runtime_name: "slang_neuro_nas_prune".into() },
            BuiltinFn { name: "neuro_nas_best_score".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_neuro_nas_best_score".into() },
            BuiltinFn { name: "spike_rl_rstdp".into(), params: vec![("p0", IrType::F64), ("p1", IrType::F64)], ret: IrType::I64, runtime_name: "slang_spike_rl_rstdp".into() },
            BuiltinFn { name: "spike_rl_td".into(), params: vec![("p0", IrType::F64), ("p1", IrType::F64), ("p2", IrType::F64), ("p3", IrType::F64)], ret: IrType::I64, runtime_name: "slang_spike_rl_td".into() },
            BuiltinFn { name: "spike_rl_policy".into(), params: vec![("p0", IrType::F64), ("p1", IrType::F64)], ret: IrType::I64, runtime_name: "slang_spike_rl_policy".into() },
            BuiltinFn { name: "spike_rl_predict_reward".into(), params: vec![("p0", IrType::F64), ("p1", IrType::F64)], ret: IrType::I64, runtime_name: "slang_spike_rl_predict_reward".into() },
            BuiltinFn { name: "curiosity_reward".into(), params: vec![("p0", IrType::F64)], ret: IrType::I64, runtime_name: "slang_curiosity_reward".into() },
            BuiltinFn { name: "curiosity_info_gain".into(), params: vec![("p0", IrType::F64), ("p1", IrType::F64)], ret: IrType::I64, runtime_name: "slang_curiosity_info_gain".into() },
            BuiltinFn { name: "curiosity_novelty".into(), params: vec![("p0", IrType::F64)], ret: IrType::I64, runtime_name: "slang_curiosity_novelty".into() },
            BuiltinFn { name: "curiosity_decay".into(), params: vec![("p0", IrType::F64), ("p1", IrType::I64)], ret: IrType::I64, runtime_name: "slang_curiosity_decay".into() },
            BuiltinFn { name: "meta_maml_adapt".into(), params: vec![("p0", IrType::F64), ("p1", IrType::F64), ("p2", IrType::F64)], ret: IrType::I64, runtime_name: "slang_meta_maml_adapt".into() },
            BuiltinFn { name: "meta_reptile".into(), params: vec![("p0", IrType::F64), ("p1", IrType::F64), ("p2", IrType::F64)], ret: IrType::I64, runtime_name: "slang_meta_reptile".into() },
            BuiltinFn { name: "meta_task_similarity".into(), params: vec![("p0", IrType::F64), ("p1", IrType::F64), ("p2", IrType::F64)], ret: IrType::I64, runtime_name: "slang_meta_task_similarity".into() },
            BuiltinFn { name: "meta_convergence".into(), params: vec![("p0", IrType::F64), ("p1", IrType::F64), ("p2", IrType::I64)], ret: IrType::I64, runtime_name: "slang_meta_convergence".into() },
            // ── v249-v254: SNN-ANN Hybrid ──
            BuiltinFn { name: "snn_ann_to_rate".into(), params: vec![("p0", IrType::F64), ("p1", IrType::F64)], ret: IrType::I64, runtime_name: "slang_snn_ann_to_rate".into() },
            BuiltinFn { name: "snn_rate_to_ann".into(), params: vec![("p0", IrType::F64), ("p1", IrType::F64)], ret: IrType::I64, runtime_name: "slang_snn_rate_to_ann".into() },
            BuiltinFn { name: "snn_conversion_loss".into(), params: vec![("p0", IrType::F64), ("p1", IrType::F64)], ret: IrType::I64, runtime_name: "slang_snn_conversion_loss".into() },
            BuiltinFn { name: "snn_optimal_timesteps".into(), params: vec![("p0", IrType::F64)], ret: IrType::I64, runtime_name: "slang_snn_optimal_timesteps".into() },
            BuiltinFn { name: "hybrid_infer".into(), params: vec![("p0", IrType::F64), ("p1", IrType::F64), ("p2", IrType::F64)], ret: IrType::I64, runtime_name: "slang_hybrid_infer".into() },
            BuiltinFn { name: "hybrid_set_fraction".into(), params: vec![("p0", IrType::I64)], ret: IrType::I64, runtime_name: "slang_hybrid_set_fraction".into() },
            BuiltinFn { name: "hybrid_efficiency".into(), params: vec![("p0", IrType::F64), ("p1", IrType::F64)], ret: IrType::I64, runtime_name: "slang_hybrid_efficiency".into() },
            BuiltinFn { name: "hybrid_accuracy_gain".into(), params: vec![("p0", IrType::F64), ("p1", IrType::F64)], ret: IrType::I64, runtime_name: "slang_hybrid_accuracy_gain".into() },
            BuiltinFn { name: "spike_compile".into(), params: vec![("p0", IrType::I64), ("p1", IrType::I64), ("p2", IrType::I64)], ret: IrType::I64, runtime_name: "slang_spike_compile".into() },
            BuiltinFn { name: "spike_compile_optimize".into(), params: vec![("p0", IrType::I64), ("p1", IrType::F64)], ret: IrType::I64, runtime_name: "slang_spike_compile_optimize".into() },
            BuiltinFn { name: "spike_compile_count".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_spike_compile_count".into() },
            BuiltinFn { name: "spike_compile_memory".into(), params: vec![("p0", IrType::I64), ("p1", IrType::I64)], ret: IrType::I64, runtime_name: "slang_spike_compile_memory".into() },
            BuiltinFn { name: "diff_spike_ste".into(), params: vec![("p0", IrType::F64), ("p1", IrType::F64)], ret: IrType::I64, runtime_name: "slang_diff_spike_ste".into() },
            BuiltinFn { name: "diff_spike_sigmoid".into(), params: vec![("p0", IrType::F64), ("p1", IrType::F64), ("p2", IrType::F64)], ret: IrType::I64, runtime_name: "slang_diff_spike_sigmoid".into() },
            BuiltinFn { name: "diff_spike_fast_sigmoid".into(), params: vec![("p0", IrType::F64), ("p1", IrType::F64), ("p2", IrType::F64)], ret: IrType::I64, runtime_name: "slang_diff_spike_fast_sigmoid".into() },
            BuiltinFn { name: "diff_spike_accumulate".into(), params: vec![("p0", IrType::F64), ("p1", IrType::F64)], ret: IrType::I64, runtime_name: "slang_diff_spike_accumulate".into() },
            BuiltinFn { name: "neural_ode_euler".into(), params: vec![("p0", IrType::F64), ("p1", IrType::F64), ("p2", IrType::F64)], ret: IrType::I64, runtime_name: "slang_neural_ode_euler".into() },
            BuiltinFn { name: "neural_ode_rk4".into(), params: vec![("p0", IrType::F64), ("p1", IrType::F64), ("p2", IrType::F64), ("p3", IrType::F64), ("p4", IrType::F64), ("p5", IrType::F64)], ret: IrType::I64, runtime_name: "slang_neural_ode_rk4".into() },
            BuiltinFn { name: "neural_ode_adjoint".into(), params: vec![("p0", IrType::F64), ("p1", IrType::F64)], ret: IrType::I64, runtime_name: "slang_neural_ode_adjoint".into() },
            BuiltinFn { name: "neural_ode_adaptive_dt".into(), params: vec![("p0", IrType::F64), ("p1", IrType::F64), ("p2", IrType::F64)], ret: IrType::I64, runtime_name: "slang_neural_ode_adaptive_dt".into() },
            BuiltinFn { name: "hybrid_distill".into(), params: vec![("p0", IrType::F64), ("p1", IrType::F64), ("p2", IrType::F64)], ret: IrType::I64, runtime_name: "slang_hybrid_distill".into() },
            BuiltinFn { name: "hybrid_freeze".into(), params: vec![("p0", IrType::I64), ("p1", IrType::I64), ("p2", IrType::I64)], ret: IrType::I64, runtime_name: "slang_hybrid_freeze".into() },
            BuiltinFn { name: "hybrid_lr".into(), params: vec![("p0", IrType::F64), ("p1", IrType::I64), ("p2", IrType::I64), ("p3", IrType::I64)], ret: IrType::I64, runtime_name: "slang_hybrid_lr".into() },
            BuiltinFn { name: "hybrid_weighted_acc".into(), params: vec![("p0", IrType::F64), ("p1", IrType::F64), ("p2", IrType::F64)], ret: IrType::I64, runtime_name: "slang_hybrid_weighted_acc".into() },
            // ── v255-v260: Hippocampal Memory ──
            BuiltinFn { name: "hippo_encode".into(), params: vec![("p0", IrType::I64), ("p1", IrType::I64), ("p2", IrType::I64)], ret: IrType::I64, runtime_name: "slang_hippo_encode".into() },
            BuiltinFn { name: "hippo_recall".into(), params: vec![("p0", IrType::I64)], ret: IrType::I64, runtime_name: "slang_hippo_recall".into() },
            BuiltinFn { name: "hippo_replay".into(), params: vec![("p0", IrType::I64)], ret: IrType::I64, runtime_name: "slang_hippo_replay".into() },
            BuiltinFn { name: "hippo_count".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_hippo_count".into() },
            BuiltinFn { name: "wm_push".into(), params: vec![("p0", IrType::I64)], ret: IrType::I64, runtime_name: "slang_wm_push".into() },
            BuiltinFn { name: "wm_pop".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_wm_pop".into() },
            BuiltinFn { name: "wm_set_capacity".into(), params: vec![("p0", IrType::I64)], ret: IrType::I64, runtime_name: "slang_wm_set_capacity".into() },
            BuiltinFn { name: "wm_utilization".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_wm_utilization".into() },
            BuiltinFn { name: "sleep_consolidate".into(), params: vec![("p0", IrType::I64)], ret: IrType::I64, runtime_name: "slang_sleep_consolidate".into() },
            BuiltinFn { name: "sleep_rem".into(), params: vec![("p0", IrType::F64)], ret: IrType::I64, runtime_name: "slang_sleep_rem".into() },
            BuiltinFn { name: "sleep_nrem_ripple".into(), params: vec![("p0", IrType::I64)], ret: IrType::I64, runtime_name: "slang_sleep_nrem_ripple".into() },
            BuiltinFn { name: "sleep_duration".into(), params: vec![("p0", IrType::I64)], ret: IrType::I64, runtime_name: "slang_sleep_duration".into() },
            BuiltinFn { name: "hopfield_energy".into(), params: vec![("p0", IrType::I64), ("p1", IrType::F64), ("p2", IrType::F64)], ret: IrType::I64, runtime_name: "slang_hopfield_energy".into() },
            BuiltinFn { name: "hopfield_capacity".into(), params: vec![("p0", IrType::I64)], ret: IrType::I64, runtime_name: "slang_hopfield_capacity".into() },
            BuiltinFn { name: "hopfield_retrieve".into(), params: vec![("p0", IrType::I64), ("p1", IrType::I64)], ret: IrType::I64, runtime_name: "slang_hopfield_retrieve".into() },
            BuiltinFn { name: "hopfield_accuracy".into(), params: vec![("p0", IrType::I64), ("p1", IrType::F64)], ret: IrType::I64, runtime_name: "slang_hopfield_accuracy".into() },
            BuiltinFn { name: "synaptag_decay".into(), params: vec![("p0", IrType::F64), ("p1", IrType::F64), ("p2", IrType::F64)], ret: IrType::I64, runtime_name: "slang_synaptag_decay".into() },
            BuiltinFn { name: "synaptag_capture".into(), params: vec![("p0", IrType::F64), ("p1", IrType::F64)], ret: IrType::I64, runtime_name: "slang_synaptag_capture".into() },
            BuiltinFn { name: "synaptag_late_ltp".into(), params: vec![("p0", IrType::F64), ("p1", IrType::F64)], ret: IrType::I64, runtime_name: "slang_synaptag_late_ltp".into() },
            BuiltinFn { name: "synaptag_protein".into(), params: vec![("p0", IrType::F64), ("p1", IrType::F64)], ret: IrType::I64, runtime_name: "slang_synaptag_protein".into() },
            BuiltinFn { name: "memcompress_schema".into(), params: vec![("p0", IrType::I64), ("p1", IrType::F64)], ret: IrType::I64, runtime_name: "slang_memcompress_schema".into() },
            BuiltinFn { name: "memcompress_forget".into(), params: vec![("p0", IrType::I64), ("p1", IrType::F64)], ret: IrType::I64, runtime_name: "slang_memcompress_forget".into() },
            BuiltinFn { name: "memcompress_merge".into(), params: vec![("p0", IrType::I64), ("p1", IrType::F64)], ret: IrType::I64, runtime_name: "slang_memcompress_merge".into() },
            BuiltinFn { name: "memcompress_ratio".into(), params: vec![("p0", IrType::I64), ("p1", IrType::I64)], ret: IrType::I64, runtime_name: "slang_memcompress_ratio".into() },
            // ── v261-v266: Distributed Neuromorphic ──
            BuiltinFn { name: "neuro_cluster_init".into(), params: vec![("p0", IrType::I64)], ret: IrType::I64, runtime_name: "slang_neuro_cluster_init".into() },
            BuiltinFn { name: "neuro_cluster_distribute".into(), params: vec![("p0", IrType::I64)], ret: IrType::I64, runtime_name: "slang_neuro_cluster_distribute".into() },
            BuiltinFn { name: "neuro_cluster_load".into(), params: vec![("p0", IrType::I64)], ret: IrType::I64, runtime_name: "slang_neuro_cluster_load".into() },
            BuiltinFn { name: "neuro_cluster_nodes".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_neuro_cluster_nodes".into() },
            BuiltinFn { name: "spike_consensus_vote".into(), params: vec![("p0", IrType::I64), ("p1", IrType::I64)], ret: IrType::I64, runtime_name: "slang_spike_consensus_vote".into() },
            BuiltinFn { name: "spike_consensus_bft".into(), params: vec![("p0", IrType::I64)], ret: IrType::I64, runtime_name: "slang_spike_consensus_bft".into() },
            BuiltinFn { name: "spike_consensus_tick".into(), params: vec![("p0", IrType::I64), ("p1", IrType::I64)], ret: IrType::I64, runtime_name: "slang_spike_consensus_tick".into() },
            BuiltinFn { name: "spike_consensus_latency".into(), params: vec![("p0", IrType::I64), ("p1", IrType::I64)], ret: IrType::I64, runtime_name: "slang_spike_consensus_latency".into() },
            BuiltinFn { name: "fed_neuro_average".into(), params: vec![("p0", IrType::F64), ("p1", IrType::F64), ("p2", IrType::F64)], ret: IrType::I64, runtime_name: "slang_fed_neuro_average".into() },
            BuiltinFn { name: "fed_neuro_dp_noise".into(), params: vec![("p0", IrType::I64), ("p1", IrType::F64)], ret: IrType::I64, runtime_name: "slang_fed_neuro_dp_noise".into() },
            BuiltinFn { name: "fed_neuro_compress".into(), params: vec![("p0", IrType::I64), ("p1", IrType::F64)], ret: IrType::I64, runtime_name: "slang_fed_neuro_compress".into() },
            BuiltinFn { name: "fed_neuro_round".into(), params: vec![("p0", IrType::I64)], ret: IrType::I64, runtime_name: "slang_fed_neuro_round".into() },
            BuiltinFn { name: "edge_neuro_budget".into(), params: vec![("p0", IrType::I64)], ret: IrType::I64, runtime_name: "slang_edge_neuro_budget".into() },
            BuiltinFn { name: "edge_neuro_quantize".into(), params: vec![("p0", IrType::I64), ("p1", IrType::I64)], ret: IrType::I64, runtime_name: "slang_edge_neuro_quantize".into() },
            BuiltinFn { name: "edge_neuro_latency_ok".into(), params: vec![("p0", IrType::I64), ("p1", IrType::I64)], ret: IrType::I64, runtime_name: "slang_edge_neuro_latency_ok".into() },
            BuiltinFn { name: "edge_neuro_model_size".into(), params: vec![("p0", IrType::I64), ("p1", IrType::I64)], ret: IrType::I64, runtime_name: "slang_edge_neuro_model_size".into() },
            BuiltinFn { name: "stream_spike_process".into(), params: vec![("p0", IrType::I64), ("p1", IrType::I64)], ret: IrType::I64, runtime_name: "slang_stream_spike_process".into() },
            BuiltinFn { name: "stream_spike_rate".into(), params: vec![("p0", IrType::I64), ("p1", IrType::F64)], ret: IrType::I64, runtime_name: "slang_stream_spike_rate".into() },
            BuiltinFn { name: "stream_spike_backpressure".into(), params: vec![("p0", IrType::I64), ("p1", IrType::I64)], ret: IrType::I64, runtime_name: "slang_stream_spike_backpressure".into() },
            BuiltinFn { name: "stream_spike_total".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_stream_spike_total".into() },
            BuiltinFn { name: "neuro_platform_caps".into(), params: vec![("p0", IrType::I64)], ret: IrType::I64, runtime_name: "slang_neuro_platform_caps".into() },
            BuiltinFn { name: "neuro_platform_map".into(), params: vec![("p0", IrType::I64), ("p1", IrType::I64)], ret: IrType::I64, runtime_name: "slang_neuro_platform_map".into() },
            BuiltinFn { name: "neuro_platform_overhead".into(), params: vec![("p0", IrType::I64), ("p1", IrType::I64)], ret: IrType::I64, runtime_name: "slang_neuro_platform_overhead".into() },
            BuiltinFn { name: "neuro_platform_power".into(), params: vec![("p0", IrType::I64), ("p1", IrType::I64)], ret: IrType::I64, runtime_name: "slang_neuro_platform_power".into() },
            // ── v267-v272: Neuro Tooling ──
            BuiltinFn { name: "neuro_viz_spike_raster".into(), params: vec![("p0", IrType::I64), ("p1", IrType::I64), ("p2", IrType::F64)], ret: IrType::I64, runtime_name: "slang_neuro_viz_spike_raster".into() },
            BuiltinFn { name: "neuro_viz_membrane".into(), params: vec![("p0", IrType::I64), ("p1", IrType::I64)], ret: IrType::I64, runtime_name: "slang_neuro_viz_membrane".into() },
            BuiltinFn { name: "neuro_viz_connectivity".into(), params: vec![("p0", IrType::I64), ("p1", IrType::I64)], ret: IrType::I64, runtime_name: "slang_neuro_viz_connectivity".into() },
            BuiltinFn { name: "neuro_viz_frames".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_neuro_viz_frames".into() },
            BuiltinFn { name: "neuro_debug_break".into(), params: vec![("p0", IrType::I64)], ret: IrType::I64, runtime_name: "slang_neuro_debug_break".into() },
            BuiltinFn { name: "neuro_debug_inspect".into(), params: vec![("p0", IrType::I64), ("p1", IrType::I64)], ret: IrType::I64, runtime_name: "slang_neuro_debug_inspect".into() },
            BuiltinFn { name: "neuro_debug_step".into(), params: vec![("p0", IrType::I64), ("p1", IrType::F64)], ret: IrType::I64, runtime_name: "slang_neuro_debug_step".into() },
            BuiltinFn { name: "neuro_debug_breakpoints".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_neuro_debug_breakpoints".into() },
            BuiltinFn { name: "neuro_profile_throughput".into(), params: vec![("p0", IrType::I64), ("p1", IrType::F64)], ret: IrType::I64, runtime_name: "slang_neuro_profile_throughput".into() },
            BuiltinFn { name: "neuro_profile_memory".into(), params: vec![("p0", IrType::I64), ("p1", IrType::I64)], ret: IrType::I64, runtime_name: "slang_neuro_profile_memory".into() },
            BuiltinFn { name: "neuro_profile_energy".into(), params: vec![("p0", IrType::I64), ("p1", IrType::I64)], ret: IrType::I64, runtime_name: "slang_neuro_profile_energy".into() },
            BuiltinFn { name: "neuro_profile_samples".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_neuro_profile_samples".into() },
            BuiltinFn { name: "neuro_dsl_neuron".into(), params: vec![("p0", IrType::I64), ("p1", IrType::I64)], ret: IrType::I64, runtime_name: "slang_neuro_dsl_neuron".into() },
            BuiltinFn { name: "neuro_dsl_synapse".into(), params: vec![("p0", IrType::I64), ("p1", IrType::I64), ("p2", IrType::F64)], ret: IrType::I64, runtime_name: "slang_neuro_dsl_synapse".into() },
            BuiltinFn { name: "neuro_dsl_network".into(), params: vec![("p0", IrType::I64), ("p1", IrType::F64)], ret: IrType::I64, runtime_name: "slang_neuro_dsl_network".into() },
            BuiltinFn { name: "neuro_dsl_validate".into(), params: vec![("p0", IrType::I64), ("p1", IrType::I64)], ret: IrType::I64, runtime_name: "slang_neuro_dsl_validate".into() },
            BuiltinFn { name: "neuro_bench_spike_lat".into(), params: vec![("p0", IrType::I64)], ret: IrType::I64, runtime_name: "slang_neuro_bench_spike_lat".into() },
            BuiltinFn { name: "neuro_bench_neuron_tput".into(), params: vec![("p0", IrType::I64), ("p1", IrType::I64)], ret: IrType::I64, runtime_name: "slang_neuro_bench_neuron_tput".into() },
            BuiltinFn { name: "neuro_bench_synapse_rate".into(), params: vec![("p0", IrType::I64), ("p1", IrType::F64)], ret: IrType::I64, runtime_name: "slang_neuro_bench_synapse_rate".into() },
            BuiltinFn { name: "neuro_bench_efficiency".into(), params: vec![("p0", IrType::I64), ("p1", IrType::I64)], ret: IrType::I64, runtime_name: "slang_neuro_bench_efficiency".into() },
            BuiltinFn { name: "neuro_test_timing".into(), params: vec![("p0", IrType::I64), ("p1", IrType::I64)], ret: IrType::I64, runtime_name: "slang_neuro_test_timing".into() },
            BuiltinFn { name: "neuro_test_accuracy".into(), params: vec![("p0", IrType::F64), ("p1", IrType::F64)], ret: IrType::I64, runtime_name: "slang_neuro_test_accuracy".into() },
            BuiltinFn { name: "neuro_test_convergence".into(), params: vec![("p0", IrType::F64), ("p1", IrType::F64)], ret: IrType::I64, runtime_name: "slang_neuro_test_convergence".into() },
            BuiltinFn { name: "neuro_test_spike_gen".into(), params: vec![("p0", IrType::I64), ("p1", IrType::F64), ("p2", IrType::I64)], ret: IrType::I64, runtime_name: "slang_neuro_test_spike_gen".into() },
            // ── v273-v278: Quantum Neuromorphic ──
            BuiltinFn { name: "quantum_spike_encode".into(), params: vec![("p0", IrType::F64), ("p1", IrType::I64)], ret: IrType::I64, runtime_name: "slang_quantum_spike_encode".into() },
            BuiltinFn { name: "quantum_spike_decode".into(), params: vec![("p0", IrType::I64), ("p1", IrType::I64)], ret: IrType::I64, runtime_name: "slang_quantum_spike_decode".into() },
            BuiltinFn { name: "quantum_spike_superpose".into(), params: vec![("p0", IrType::I64), ("p1", IrType::I64)], ret: IrType::I64, runtime_name: "slang_quantum_spike_superpose".into() },
            BuiltinFn { name: "quantum_spike_fidelity".into(), params: vec![("p0", IrType::F64)], ret: IrType::I64, runtime_name: "slang_quantum_spike_fidelity".into() },
            BuiltinFn { name: "quantum_plasticity_stdp".into(), params: vec![("p0", IrType::F64), ("p1", IrType::F64)], ret: IrType::I64, runtime_name: "slang_quantum_plasticity_stdp".into() },
            BuiltinFn { name: "quantum_plasticity_anneal".into(), params: vec![("p0", IrType::F64), ("p1", IrType::F64)], ret: IrType::I64, runtime_name: "slang_quantum_plasticity_anneal".into() },
            BuiltinFn { name: "quantum_plasticity_tunnel".into(), params: vec![("p0", IrType::F64), ("p1", IrType::F64)], ret: IrType::I64, runtime_name: "slang_quantum_plasticity_tunnel".into() },
            BuiltinFn { name: "quantum_plasticity_t2".into(), params: vec![("p0", IrType::I64), ("p1", IrType::F64)], ret: IrType::I64, runtime_name: "slang_quantum_plasticity_t2".into() },
            BuiltinFn { name: "quantum_reservoir_init".into(), params: vec![("p0", IrType::I64)], ret: IrType::I64, runtime_name: "slang_quantum_reservoir_init".into() },
            BuiltinFn { name: "quantum_reservoir_project".into(), params: vec![("p0", IrType::F64), ("p1", IrType::I64)], ret: IrType::I64, runtime_name: "slang_quantum_reservoir_project".into() },
            BuiltinFn { name: "quantum_reservoir_kernel".into(), params: vec![("p0", IrType::F64), ("p1", IrType::F64), ("p2", IrType::F64)], ret: IrType::I64, runtime_name: "slang_quantum_reservoir_kernel".into() },
            BuiltinFn { name: "quantum_reservoir_dim".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_quantum_reservoir_dim".into() },
            BuiltinFn { name: "qsnn_var_update".into(), params: vec![("p0", IrType::F64), ("p1", IrType::F64), ("p2", IrType::F64)], ret: IrType::I64, runtime_name: "slang_qsnn_var_update".into() },
            BuiltinFn { name: "qsnn_var_cost".into(), params: vec![("p0", IrType::F64), ("p1", IrType::F64)], ret: IrType::I64, runtime_name: "slang_qsnn_var_cost".into() },
            BuiltinFn { name: "qsnn_var_depth".into(), params: vec![("p0", IrType::I64), ("p1", IrType::I64)], ret: IrType::I64, runtime_name: "slang_qsnn_var_depth".into() },
            BuiltinFn { name: "qsnn_var_expressibility".into(), params: vec![("p0", IrType::I64), ("p1", IrType::I64)], ret: IrType::I64, runtime_name: "slang_qsnn_var_expressibility".into() },
            BuiltinFn { name: "quantum_qec_shor".into(), params: vec![("p0", IrType::I64)], ret: IrType::I64, runtime_name: "slang_quantum_qec_shor".into() },
            BuiltinFn { name: "quantum_qec_surface".into(), params: vec![("p0", IrType::I64)], ret: IrType::I64, runtime_name: "slang_quantum_qec_surface".into() },
            BuiltinFn { name: "quantum_qec_syndrome".into(), params: vec![("p0", IrType::I64)], ret: IrType::I64, runtime_name: "slang_quantum_qec_syndrome".into() },
            BuiltinFn { name: "quantum_qec_overhead".into(), params: vec![("p0", IrType::I64), ("p1", IrType::I64)], ret: IrType::I64, runtime_name: "slang_quantum_qec_overhead".into() },
            BuiltinFn { name: "quantum_bridge_encode".into(), params: vec![("p0", IrType::F64)], ret: IrType::I64, runtime_name: "slang_quantum_bridge_encode".into() },
            BuiltinFn { name: "quantum_bridge_decode".into(), params: vec![("p0", IrType::F64)], ret: IrType::I64, runtime_name: "slang_quantum_bridge_decode".into() },
            BuiltinFn { name: "quantum_bridge_cost".into(), params: vec![("p0", IrType::I64), ("p1", IrType::I64)], ret: IrType::I64, runtime_name: "slang_quantum_bridge_cost".into() },
            BuiltinFn { name: "quantum_bridge_advantage".into(), params: vec![("p0", IrType::I64), ("p1", IrType::I64)], ret: IrType::I64, runtime_name: "slang_quantum_bridge_advantage".into() },
            // ── v279-v284: Neuro Safety ──
            BuiltinFn { name: "neuro_verify_timing".into(), params: vec![("p0", IrType::I64), ("p1", IrType::I64), ("p2", IrType::I64)], ret: IrType::I64, runtime_name: "slang_neuro_verify_timing".into() },
            BuiltinFn { name: "neuro_verify_membrane".into(), params: vec![("p0", IrType::F64), ("p1", IrType::F64), ("p2", IrType::F64)], ret: IrType::I64, runtime_name: "slang_neuro_verify_membrane".into() },
            BuiltinFn { name: "neuro_verify_symmetry".into(), params: vec![("p0", IrType::F64), ("p1", IrType::F64), ("p2", IrType::F64)], ret: IrType::I64, runtime_name: "slang_neuro_verify_symmetry".into() },
            BuiltinFn { name: "neuro_verify_liveness".into(), params: vec![("p0", IrType::I64), ("p1", IrType::I64), ("p2", IrType::I64)], ret: IrType::I64, runtime_name: "slang_neuro_verify_liveness".into() },
            BuiltinFn { name: "neuro_safe_rate_clamp".into(), params: vec![("p0", IrType::I64), ("p1", IrType::I64)], ret: IrType::I64, runtime_name: "slang_neuro_safe_rate_clamp".into() },
            BuiltinFn { name: "neuro_safe_runaway".into(), params: vec![("p0", IrType::I64), ("p1", IrType::I64)], ret: IrType::I64, runtime_name: "slang_neuro_safe_runaway".into() },
            BuiltinFn { name: "neuro_safe_dead_neuron".into(), params: vec![("p0", IrType::I64)], ret: IrType::I64, runtime_name: "slang_neuro_safe_dead_neuron".into() },
            BuiltinFn { name: "neuro_safe_violations".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_neuro_safe_violations".into() },
            BuiltinFn { name: "neuro_explain_contribution".into(), params: vec![("p0", IrType::F64), ("p1", IrType::I64)], ret: IrType::I64, runtime_name: "slang_neuro_explain_contribution".into() },
            BuiltinFn { name: "neuro_explain_ablation".into(), params: vec![("p0", IrType::F64), ("p1", IrType::F64)], ret: IrType::I64, runtime_name: "slang_neuro_explain_ablation".into() },
            BuiltinFn { name: "neuro_explain_saliency".into(), params: vec![("p0", IrType::F64), ("p1", IrType::F64)], ret: IrType::I64, runtime_name: "slang_neuro_explain_saliency".into() },
            BuiltinFn { name: "neuro_explain_lrp".into(), params: vec![("p0", IrType::F64), ("p1", IrType::F64), ("p2", IrType::F64)], ret: IrType::I64, runtime_name: "slang_neuro_explain_lrp".into() },
            BuiltinFn { name: "neuro_robust_eps_check".into(), params: vec![("p0", IrType::F64), ("p1", IrType::F64)], ret: IrType::I64, runtime_name: "slang_neuro_robust_eps_check".into() },
            BuiltinFn { name: "neuro_robust_margin".into(), params: vec![("p0", IrType::F64), ("p1", IrType::F64)], ret: IrType::I64, runtime_name: "slang_neuro_robust_margin".into() },
            BuiltinFn { name: "neuro_robust_certified_radius".into(), params: vec![("p0", IrType::F64), ("p1", IrType::F64)], ret: IrType::I64, runtime_name: "slang_neuro_robust_certified_radius".into() },
            BuiltinFn { name: "neuro_robust_inject_noise".into(), params: vec![("p0", IrType::F64), ("p1", IrType::F64)], ret: IrType::I64, runtime_name: "slang_neuro_robust_inject_noise".into() },
            BuiltinFn { name: "neuro_fair_dp_gap".into(), params: vec![("p0", IrType::F64), ("p1", IrType::F64)], ret: IrType::I64, runtime_name: "slang_neuro_fair_dp_gap".into() },
            BuiltinFn { name: "neuro_fair_eo_gap".into(), params: vec![("p0", IrType::F64), ("p1", IrType::F64)], ret: IrType::I64, runtime_name: "slang_neuro_fair_eo_gap".into() },
            BuiltinFn { name: "neuro_fair_calibration".into(), params: vec![("p0", IrType::F64), ("p1", IrType::F64)], ret: IrType::I64, runtime_name: "slang_neuro_fair_calibration".into() },
            BuiltinFn { name: "neuro_fair_lipschitz".into(), params: vec![("p0", IrType::F64), ("p1", IrType::F64)], ret: IrType::I64, runtime_name: "slang_neuro_fair_lipschitz".into() },
            BuiltinFn { name: "neuro_cert_bounds".into(), params: vec![("p0", IrType::F64), ("p1", IrType::F64), ("p2", IrType::F64)], ret: IrType::I64, runtime_name: "slang_neuro_cert_bounds".into() },
            BuiltinFn { name: "neuro_cert_ibp_width".into(), params: vec![("p0", IrType::F64), ("p1", IrType::F64)], ret: IrType::I64, runtime_name: "slang_neuro_cert_ibp_width".into() },
            BuiltinFn { name: "neuro_cert_crown".into(), params: vec![("p0", IrType::F64), ("p1", IrType::F64)], ret: IrType::I64, runtime_name: "slang_neuro_cert_crown".into() },
            BuiltinFn { name: "neuro_cert_accuracy".into(), params: vec![("p0", IrType::I64), ("p1", IrType::I64)], ret: IrType::I64, runtime_name: "slang_neuro_cert_accuracy".into() },
            // ── v285-v290: Neuro Performance ──
            BuiltinFn { name: "neuro_simd_accumulate".into(), params: vec![("p0", IrType::I64), ("p1", IrType::I64), ("p2", IrType::I64), ("p3", IrType::I64)], ret: IrType::I64, runtime_name: "slang_neuro_simd_accumulate".into() },
            BuiltinFn { name: "neuro_simd_threshold".into(), params: vec![("p0", IrType::I64), ("p1", IrType::I64), ("p2", IrType::I64), ("p3", IrType::I64)], ret: IrType::I64, runtime_name: "slang_neuro_simd_threshold".into() },
            BuiltinFn { name: "neuro_simd_decay".into(), params: vec![("p0", IrType::F64), ("p1", IrType::F64)], ret: IrType::I64, runtime_name: "slang_neuro_simd_decay".into() },
            BuiltinFn { name: "neuro_simd_throughput".into(), params: vec![("p0", IrType::I64), ("p1", IrType::I64)], ret: IrType::I64, runtime_name: "slang_neuro_simd_throughput".into() },
            BuiltinFn { name: "neuro_jit_compile".into(), params: vec![("p0", IrType::I64), ("p1", IrType::I64)], ret: IrType::I64, runtime_name: "slang_neuro_jit_compile".into() },
            BuiltinFn { name: "neuro_jit_speedup".into(), params: vec![("p0", IrType::I64), ("p1", IrType::I64)], ret: IrType::I64, runtime_name: "slang_neuro_jit_speedup".into() },
            BuiltinFn { name: "neuro_jit_cache_hit".into(), params: vec![("p0", IrType::I64), ("p1", IrType::I64)], ret: IrType::I64, runtime_name: "slang_neuro_jit_cache_hit".into() },
            BuiltinFn { name: "neuro_jit_compiled_count".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_neuro_jit_compiled_count".into() },
            BuiltinFn { name: "neuro_precision_auto".into(), params: vec![("p0", IrType::F64)], ret: IrType::I64, runtime_name: "slang_neuro_precision_auto".into() },
            BuiltinFn { name: "neuro_precision_quant_error".into(), params: vec![("p0", IrType::F64), ("p1", IrType::I64)], ret: IrType::I64, runtime_name: "slang_neuro_precision_quant_error".into() },
            BuiltinFn { name: "neuro_precision_savings".into(), params: vec![("p0", IrType::I64), ("p1", IrType::I64)], ret: IrType::I64, runtime_name: "slang_neuro_precision_savings".into() },
            BuiltinFn { name: "neuro_precision_scale".into(), params: vec![("p0", IrType::F64), ("p1", IrType::F64)], ret: IrType::I64, runtime_name: "slang_neuro_precision_scale".into() },
            BuiltinFn { name: "neuro_spec_predict".into(), params: vec![("p0", IrType::F64), ("p1", IrType::F64)], ret: IrType::I64, runtime_name: "slang_neuro_spec_predict".into() },
            BuiltinFn { name: "neuro_spec_gain".into(), params: vec![("p0", IrType::I64), ("p1", IrType::I64)], ret: IrType::I64, runtime_name: "slang_neuro_spec_gain".into() },
            BuiltinFn { name: "neuro_spec_rollback_cost".into(), params: vec![("p0", IrType::I64)], ret: IrType::I64, runtime_name: "slang_neuro_spec_rollback_cost".into() },
            BuiltinFn { name: "neuro_spec_confidence".into(), params: vec![("p0", IrType::I64), ("p1", IrType::I64)], ret: IrType::I64, runtime_name: "slang_neuro_spec_confidence".into() },
            BuiltinFn { name: "neuro_pgo_sample".into(), params: vec![("p0", IrType::I64)], ret: IrType::I64, runtime_name: "slang_neuro_pgo_sample".into() },
            BuiltinFn { name: "neuro_pgo_is_hot".into(), params: vec![("p0", IrType::I64), ("p1", IrType::I64)], ret: IrType::I64, runtime_name: "slang_neuro_pgo_is_hot".into() },
            BuiltinFn { name: "neuro_pgo_unroll".into(), params: vec![("p0", IrType::I64)], ret: IrType::I64, runtime_name: "slang_neuro_pgo_unroll".into() },
            BuiltinFn { name: "neuro_pgo_total_samples".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_neuro_pgo_total_samples".into() },
            BuiltinFn { name: "neuro_zero_send".into(), params: vec![("p0", IrType::I64), ("p1", IrType::I64)], ret: IrType::I64, runtime_name: "slang_neuro_zero_send".into() },
            BuiltinFn { name: "neuro_zero_update".into(), params: vec![("p0", IrType::F64), ("p1", IrType::F64), ("p2", IrType::F64)], ret: IrType::I64, runtime_name: "slang_neuro_zero_update".into() },
            BuiltinFn { name: "neuro_zero_conn_type".into(), params: vec![("p0", IrType::F64)], ret: IrType::I64, runtime_name: "slang_neuro_zero_conn_type".into() },
            BuiltinFn { name: "neuro_zero_overhead".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_neuro_zero_overhead".into() },
            // ── v291: Advanced Spike Analytics ──
            BuiltinFn { name: "spike_analytics_mean_rate".into(), params: vec![("p0", IrType::I64), ("p1", IrType::I64)], ret: IrType::I64, runtime_name: "slang_spike_analytics_mean_rate".into() },
            BuiltinFn { name: "spike_analytics_cv_isi".into(), params: vec![("p0", IrType::F64), ("p1", IrType::F64)], ret: IrType::I64, runtime_name: "slang_spike_analytics_cv_isi".into() },
            BuiltinFn { name: "spike_analytics_fano_factor".into(), params: vec![("p0", IrType::F64), ("p1", IrType::F64)], ret: IrType::I64, runtime_name: "slang_spike_analytics_fano_factor".into() },
            BuiltinFn { name: "spike_analytics_burst_index".into(), params: vec![("p0", IrType::I64), ("p1", IrType::I64)], ret: IrType::I64, runtime_name: "slang_spike_analytics_burst_index".into() },
            // ── v292: Neural Network Metrics ──
            BuiltinFn { name: "nn_metric_sparsity".into(), params: vec![("p0", IrType::I64), ("p1", IrType::I64)], ret: IrType::I64, runtime_name: "slang_nn_metric_sparsity".into() },
            BuiltinFn { name: "nn_metric_entropy".into(), params: vec![("p0", IrType::F64)], ret: IrType::I64, runtime_name: "slang_nn_metric_entropy".into() },
            BuiltinFn { name: "nn_metric_mutual_info".into(), params: vec![("p0", IrType::F64), ("p1", IrType::F64)], ret: IrType::I64, runtime_name: "slang_nn_metric_mutual_info".into() },
            BuiltinFn { name: "nn_metric_transfer_entropy".into(), params: vec![("p0", IrType::F64), ("p1", IrType::F64), ("p2", IrType::F64)], ret: IrType::I64, runtime_name: "slang_nn_metric_transfer_entropy".into() },
            // ── v293: Spike Train Distance ──
            BuiltinFn { name: "spike_dist_victor_purpura".into(), params: vec![("p0", IrType::F64), ("p1", IrType::F64), ("p2", IrType::F64)], ret: IrType::I64, runtime_name: "slang_spike_dist_victor_purpura".into() },
            BuiltinFn { name: "spike_dist_van_rossum".into(), params: vec![("p0", IrType::F64), ("p1", IrType::F64), ("p2", IrType::F64)], ret: IrType::I64, runtime_name: "slang_spike_dist_van_rossum".into() },
            BuiltinFn { name: "spike_dist_schreiber".into(), params: vec![("p0", IrType::F64), ("p1", IrType::F64)], ret: IrType::I64, runtime_name: "slang_spike_dist_schreiber".into() },
            BuiltinFn { name: "spike_dist_earth_mover".into(), params: vec![("p0", IrType::I64), ("p1", IrType::I64)], ret: IrType::I64, runtime_name: "slang_spike_dist_earth_mover".into() },
            // ── v294: Neural Coding ──
            BuiltinFn { name: "neural_code_rate".into(), params: vec![("p0", IrType::I64), ("p1", IrType::I64)], ret: IrType::I64, runtime_name: "slang_neural_code_rate".into() },
            BuiltinFn { name: "neural_code_temporal".into(), params: vec![("p0", IrType::F64), ("p1", IrType::F64)], ret: IrType::I64, runtime_name: "slang_neural_code_temporal".into() },
            BuiltinFn { name: "neural_code_population".into(), params: vec![("p0", IrType::I64), ("p1", IrType::I64), ("p2", IrType::I64)], ret: IrType::I64, runtime_name: "slang_neural_code_population".into() },
            BuiltinFn { name: "neural_code_sparse".into(), params: vec![("p0", IrType::I64), ("p1", IrType::F64)], ret: IrType::I64, runtime_name: "slang_neural_code_sparse".into() },
            // ── v295: Synaptic Plasticity Metrics ──
            BuiltinFn { name: "synap_metric_ltp_ratio".into(), params: vec![("p0", IrType::F64), ("p1", IrType::F64)], ret: IrType::I64, runtime_name: "slang_synap_metric_ltp_ratio".into() },
            BuiltinFn { name: "synap_metric_ltd_ratio".into(), params: vec![("p0", IrType::F64), ("p1", IrType::F64)], ret: IrType::I64, runtime_name: "slang_synap_metric_ltd_ratio".into() },
            BuiltinFn { name: "synap_metric_homeostatic".into(), params: vec![("p0", IrType::F64), ("p1", IrType::F64), ("p2", IrType::F64)], ret: IrType::I64, runtime_name: "slang_synap_metric_homeostatic".into() },
            BuiltinFn { name: "synap_metric_metaplasticity".into(), params: vec![("p0", IrType::F64), ("p1", IrType::I64)], ret: IrType::I64, runtime_name: "slang_synap_metric_metaplasticity".into() },
            // ── v296: Network Topology ──
            BuiltinFn { name: "topo_clustering_coeff".into(), params: vec![("p0", IrType::I64), ("p1", IrType::I64)], ret: IrType::I64, runtime_name: "slang_topo_clustering_coeff".into() },
            BuiltinFn { name: "topo_path_length".into(), params: vec![("p0", IrType::I64), ("p1", IrType::I64)], ret: IrType::I64, runtime_name: "slang_topo_path_length".into() },
            BuiltinFn { name: "topo_small_world".into(), params: vec![("p0", IrType::F64), ("p1", IrType::F64)], ret: IrType::I64, runtime_name: "slang_topo_small_world".into() },
            BuiltinFn { name: "topo_modularity".into(), params: vec![("p0", IrType::I64), ("p1", IrType::I64), ("p2", IrType::I64)], ret: IrType::I64, runtime_name: "slang_topo_modularity".into() },
            // ── v297: Neuromorphic IO ──
            BuiltinFn { name: "neuro_io_aer_encode".into(), params: vec![("p0", IrType::I64), ("p1", IrType::I64)], ret: IrType::I64, runtime_name: "slang_neuro_io_aer_encode".into() },
            BuiltinFn { name: "neuro_io_aer_decode".into(), params: vec![("p0", IrType::I64)], ret: IrType::I64, runtime_name: "slang_neuro_io_aer_decode".into() },
            BuiltinFn { name: "neuro_io_dvs_encode".into(), params: vec![("p0", IrType::I64), ("p1", IrType::I64), ("p2", IrType::F64)], ret: IrType::I64, runtime_name: "slang_neuro_io_dvs_encode".into() },
            BuiltinFn { name: "neuro_io_serial_pack".into(), params: vec![("p0", IrType::I64), ("p1", IrType::I64)], ret: IrType::I64, runtime_name: "slang_neuro_io_serial_pack".into() },
            // ── v298: Neural Dynamics ──
            BuiltinFn { name: "dyn_lyapunov_exp".into(), params: vec![("p0", IrType::F64), ("p1", IrType::F64), ("p2", IrType::F64)], ret: IrType::I64, runtime_name: "slang_dyn_lyapunov_exp".into() },
            BuiltinFn { name: "dyn_bifurcation".into(), params: vec![("p0", IrType::F64), ("p1", IrType::F64)], ret: IrType::I64, runtime_name: "slang_dyn_bifurcation".into() },
            BuiltinFn { name: "dyn_phase_portrait".into(), params: vec![("p0", IrType::F64), ("p1", IrType::F64)], ret: IrType::I64, runtime_name: "slang_dyn_phase_portrait".into() },
            BuiltinFn { name: "dyn_attractor_dim".into(), params: vec![("p0", IrType::F64), ("p1", IrType::I64)], ret: IrType::I64, runtime_name: "slang_dyn_attractor_dim".into() },
            // ── v299: Neuromorphic Scheduler ──
            BuiltinFn { name: "neuro_sched_priority".into(), params: vec![("p0", IrType::I64), ("p1", IrType::I64)], ret: IrType::I64, runtime_name: "slang_neuro_sched_priority".into() },
            BuiltinFn { name: "neuro_sched_deadline".into(), params: vec![("p0", IrType::I64), ("p1", IrType::I64)], ret: IrType::I64, runtime_name: "slang_neuro_sched_deadline".into() },
            BuiltinFn { name: "neuro_sched_edf".into(), params: vec![("p0", IrType::I64), ("p1", IrType::I64), ("p2", IrType::I64)], ret: IrType::I64, runtime_name: "slang_neuro_sched_edf".into() },
            BuiltinFn { name: "neuro_sched_utilization".into(), params: vec![("p0", IrType::I64), ("p1", IrType::I64)], ret: IrType::I64, runtime_name: "slang_neuro_sched_utilization".into() },
            // ── v300: Milestone ──
            BuiltinFn { name: "vitalis_v300_version".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_vitalis_v300_version".into() },
            BuiltinFn { name: "vitalis_v300_total_builtins".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_vitalis_v300_total_builtins".into() },
            BuiltinFn { name: "vitalis_v300_neuro_modules".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_vitalis_v300_neuro_modules".into() },
            BuiltinFn { name: "vitalis_v300_milestone".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_vitalis_v300_milestone".into() },

            // ── v301: Escape Analysis ──
            BuiltinFn { name: "escape_analyze".into(), params: vec![("f", IrType::I64)], ret: IrType::I64, runtime_name: "slang_escape_analyze".into() },
            BuiltinFn { name: "escape_stack_promoted".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_escape_stack_promoted".into() },
            BuiltinFn { name: "escape_summary".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_escape_summary".into() },
            BuiltinFn { name: "escape_clear".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_escape_clear".into() },

            // ── v302: Tail Call Optimization ──
            BuiltinFn { name: "tco_detect".into(), params: vec![("f", IrType::I64)], ret: IrType::I64, runtime_name: "slang_tco_detect".into() },
            BuiltinFn { name: "tco_optimized_count".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_tco_optimized_count".into() },
            BuiltinFn { name: "tco_depth_limit".into(), params: vec![("l", IrType::I64)], ret: IrType::I64, runtime_name: "slang_tco_depth_limit".into() },
            BuiltinFn { name: "tco_enabled".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_tco_enabled".into() },

            // ── v303: Algebraic Simplification ──
            BuiltinFn { name: "opt_algebraic_count".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_opt_algebraic_count".into() },
            BuiltinFn { name: "opt_algebraic_enable".into(), params: vec![("on", IrType::I64)], ret: IrType::I64, runtime_name: "slang_opt_algebraic_enable".into() },
            BuiltinFn { name: "opt_strength_reduced".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_opt_strength_reduced".into() },
            BuiltinFn { name: "opt_identity_removed".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_opt_identity_removed".into() },

            // ── v304: Interprocedural Analysis ──
            BuiltinFn { name: "ipa_add_edge".into(), params: vec![("caller", IrType::I64), ("callee", IrType::I64)], ret: IrType::I64, runtime_name: "slang_ipa_add_edge".into() },
            BuiltinFn { name: "ipa_call_graph_size".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_ipa_call_graph_size".into() },
            BuiltinFn { name: "ipa_mark_pure".into(), params: vec![("f", IrType::I64)], ret: IrType::I64, runtime_name: "slang_ipa_mark_pure".into() },
            BuiltinFn { name: "ipa_pure_functions".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_ipa_pure_functions".into() },
            BuiltinFn { name: "ipa_record_const_args".into(), params: vec![("f", IrType::I64), ("n", IrType::I64)], ret: IrType::I64, runtime_name: "slang_ipa_record_const_args".into() },
            BuiltinFn { name: "ipa_const_args".into(), params: vec![("f", IrType::I64)], ret: IrType::I64, runtime_name: "slang_ipa_const_args".into() },
            BuiltinFn { name: "ipa_summary".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_ipa_summary".into() },
            BuiltinFn { name: "ipa_clear".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_ipa_clear".into() },

            // ── v305: LTO ──
            BuiltinFn { name: "lto_inline_count".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_lto_inline_count".into() },
            BuiltinFn { name: "lto_dead_globals".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_lto_dead_globals".into() },
            BuiltinFn { name: "lto_devirtualized".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_lto_devirtualized".into() },
            BuiltinFn { name: "lto_enabled".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_lto_enabled".into() },

            // ── v306: Vectorization ──
            BuiltinFn { name: "vec_slp_opportunities".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_vec_slp_opportunities".into() },
            BuiltinFn { name: "vec_slp_applied".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_vec_slp_applied".into() },
            BuiltinFn { name: "vec_width".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_vec_width".into() },
            BuiltinFn { name: "vec_speedup_estimate".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_vec_speedup_estimate".into() },

            // ── v307: Compile-Time Execution ──
            BuiltinFn { name: "consteval_string".into(), params: vec![("h", IrType::I64)], ret: IrType::I64, runtime_name: "slang_consteval_string".into() },
            BuiltinFn { name: "consteval_array".into(), params: vec![("l", IrType::I64)], ret: IrType::I64, runtime_name: "slang_consteval_array".into() },
            BuiltinFn { name: "consteval_struct".into(), params: vec![("f", IrType::I64)], ret: IrType::I64, runtime_name: "slang_consteval_struct".into() },
            BuiltinFn { name: "consteval_count".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_consteval_count".into() },

            // ── v308: PGO ──
            BuiltinFn { name: "pgo_record".into(), params: vec![("f", IrType::I64), ("c", IrType::I64)], ret: IrType::I64, runtime_name: "slang_pgo_record".into() },
            BuiltinFn { name: "pgo_hotness".into(), params: vec![("f", IrType::I64)], ret: IrType::I64, runtime_name: "slang_pgo_hotness".into() },
            BuiltinFn { name: "pgo_branch_bias".into(), params: vec![("b", IrType::I64)], ret: IrType::I64, runtime_name: "slang_pgo_branch_bias".into() },
            BuiltinFn { name: "pgo_total_samples".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_pgo_total_samples".into() },

            // ── v309: Register Allocation ──
            BuiltinFn { name: "regalloc_spill_count".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_regalloc_spill_count".into() },
            BuiltinFn { name: "regalloc_move_count".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_regalloc_move_count".into() },
            BuiltinFn { name: "regalloc_pressure".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_regalloc_pressure".into() },
            BuiltinFn { name: "regalloc_coalesced".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_regalloc_coalesced".into() },

            // ── v310: Debug Info ──
            BuiltinFn { name: "debug_line_count".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_debug_line_count".into() },
            BuiltinFn { name: "debug_var_count".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_debug_var_count".into() },
            BuiltinFn { name: "debug_scope_depth".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_debug_scope_depth".into() },
            BuiltinFn { name: "debug_info_size".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_debug_info_size".into() },

            // ── v311: Existential Types ──
            BuiltinFn { name: "existential_create".into(), params: vec![("w", IrType::I64)], ret: IrType::I64, runtime_name: "slang_existential_create".into() },
            BuiltinFn { name: "existential_open".into(), params: vec![("id", IrType::I64)], ret: IrType::I64, runtime_name: "slang_existential_open".into() },
            BuiltinFn { name: "existential_pack".into(), params: vec![("id", IrType::I64), ("w", IrType::I64)], ret: IrType::I64, runtime_name: "slang_existential_pack".into() },
            BuiltinFn { name: "existential_count".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_existential_count".into() },

            // ── v312: Row Types ──
            BuiltinFn { name: "row_type_fields".into(), params: vec![("id", IrType::I64)], ret: IrType::I64, runtime_name: "slang_row_type_fields".into() },
            BuiltinFn { name: "row_type_create".into(), params: vec![("f", IrType::I64)], ret: IrType::I64, runtime_name: "slang_row_type_create".into() },
            BuiltinFn { name: "row_type_extend".into(), params: vec![("id", IrType::I64), ("f", IrType::I64)], ret: IrType::I64, runtime_name: "slang_row_type_extend".into() },
            BuiltinFn { name: "row_type_restrict".into(), params: vec![("id", IrType::I64), ("f", IrType::I64)], ret: IrType::I64, runtime_name: "slang_row_type_restrict".into() },
            BuiltinFn { name: "row_type_compatible".into(), params: vec![("a", IrType::I64), ("b", IrType::I64)], ret: IrType::I64, runtime_name: "slang_row_type_compatible".into() },

            // ── v313: Linear Types ──
            BuiltinFn { name: "linear_create".into(), params: vec![("r", IrType::I64)], ret: IrType::I64, runtime_name: "slang_linear_create".into() },
            BuiltinFn { name: "linear_check".into(), params: vec![("id", IrType::I64)], ret: IrType::I64, runtime_name: "slang_linear_check".into() },
            BuiltinFn { name: "linear_consume".into(), params: vec![("id", IrType::I64)], ret: IrType::I64, runtime_name: "slang_linear_consume".into() },
            BuiltinFn { name: "session_create".into(), params: vec![("states", IrType::I64)], ret: IrType::I64, runtime_name: "slang_session_create".into() },
            BuiltinFn { name: "session_state".into(), params: vec![("id", IrType::I64)], ret: IrType::I64, runtime_name: "slang_session_state".into() },
            BuiltinFn { name: "session_advance".into(), params: vec![("id", IrType::I64)], ret: IrType::I64, runtime_name: "slang_session_advance".into() },

            // ── v314: GADTs ──
            BuiltinFn { name: "gadt_create".into(), params: vec![("t", IrType::I64), ("i", IrType::I64)], ret: IrType::I64, runtime_name: "slang_gadt_create".into() },
            BuiltinFn { name: "gadt_refine".into(), params: vec![("id", IrType::I64), ("c", IrType::I64)], ret: IrType::I64, runtime_name: "slang_gadt_refine".into() },
            BuiltinFn { name: "gadt_witness".into(), params: vec![("id", IrType::I64)], ret: IrType::I64, runtime_name: "slang_gadt_witness".into() },
            BuiltinFn { name: "gadt_count".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_gadt_count".into() },

            // ── v315: Type Classes ──
            BuiltinFn { name: "typeclass_instances".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_typeclass_instances".into() },
            BuiltinFn { name: "typeclass_resolve".into(), params: vec![("c", IrType::I64)], ret: IrType::I64, runtime_name: "slang_typeclass_resolve".into() },
            BuiltinFn { name: "typeclass_coherence".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_typeclass_coherence".into() },
            BuiltinFn { name: "typeclass_register".into(), params: vec![("c", IrType::I64), ("i", IrType::I64)], ret: IrType::I64, runtime_name: "slang_typeclass_register".into() },

            // ── v316: Dependent Types ──
            BuiltinFn { name: "dependent_proof".into(), params: vec![("p", IrType::I64), ("w", IrType::I64)], ret: IrType::I64, runtime_name: "slang_dependent_proof".into() },
            BuiltinFn { name: "dependent_index".into(), params: vec![("a", IrType::I64), ("i", IrType::I64)], ret: IrType::I64, runtime_name: "slang_dependent_index".into() },
            BuiltinFn { name: "dependent_refine".into(), params: vec![("v", IrType::I64)], ret: IrType::I64, runtime_name: "slang_dependent_refine".into() },
            BuiltinFn { name: "dependent_check".into(), params: vec![("v", IrType::I64)], ret: IrType::I64, runtime_name: "slang_dependent_check".into() },

            // ── v317: Effect Inference ──
            BuiltinFn { name: "effect_infer".into(), params: vec![("f", IrType::I64)], ret: IrType::I64, runtime_name: "slang_effect_infer".into() },
            BuiltinFn { name: "effect_row".into(), params: vec![("f", IrType::I64)], ret: IrType::I64, runtime_name: "slang_effect_row".into() },
            BuiltinFn { name: "effect_mask".into(), params: vec![("e", IrType::I64), ("m", IrType::I64)], ret: IrType::I64, runtime_name: "slang_effect_mask".into() },
            BuiltinFn { name: "effect_polymorphic".into(), params: vec![("f", IrType::I64)], ret: IrType::I64, runtime_name: "slang_effect_polymorphic".into() },

            // ── v318: Mixture of Experts ──
            BuiltinFn { name: "moe_create".into(), params: vec![("n", IrType::I64), ("k", IrType::I64)], ret: IrType::I64, runtime_name: "slang_moe_create".into() },
            BuiltinFn { name: "moe_route".into(), params: vec![("id", IrType::I64), ("input", IrType::I64)], ret: IrType::I64, runtime_name: "slang_moe_route".into() },
            BuiltinFn { name: "moe_expert_load".into(), params: vec![("id", IrType::I64)], ret: IrType::I64, runtime_name: "slang_moe_expert_load".into() },
            BuiltinFn { name: "moe_aux_loss".into(), params: vec![("id", IrType::I64)], ret: IrType::I64, runtime_name: "slang_moe_aux_loss".into() },

            // ── v319: Quantization ──
            BuiltinFn { name: "quant_int8".into(), params: vec![("v", IrType::I64)], ret: IrType::I64, runtime_name: "slang_quant_int8".into() },
            BuiltinFn { name: "quant_int4".into(), params: vec![("v", IrType::I64)], ret: IrType::I64, runtime_name: "slang_quant_int4".into() },
            BuiltinFn { name: "quant_error".into(), params: vec![("o", IrType::I64), ("q", IrType::I64)], ret: IrType::I64, runtime_name: "slang_quant_error".into() },
            BuiltinFn { name: "quant_calibrate".into(), params: vec![("min", IrType::I64), ("max", IrType::I64)], ret: IrType::I64, runtime_name: "slang_quant_calibrate".into() },

            // ── v320: Attention Variants ──
            BuiltinFn { name: "attn_flash".into(), params: vec![("q", IrType::I64), ("k", IrType::I64), ("v", IrType::I64)], ret: IrType::I64, runtime_name: "slang_attn_flash".into() },
            BuiltinFn { name: "attn_linear".into(), params: vec![("q", IrType::I64), ("k", IrType::I64)], ret: IrType::I64, runtime_name: "slang_attn_linear".into() },
            BuiltinFn { name: "attn_sparse".into(), params: vec![("q", IrType::I64), ("k", IrType::I64), ("s", IrType::I64)], ret: IrType::I64, runtime_name: "slang_attn_sparse".into() },
            BuiltinFn { name: "attn_sliding_window".into(), params: vec![("q", IrType::I64), ("k", IrType::I64), ("w", IrType::I64)], ret: IrType::I64, runtime_name: "slang_attn_sliding_window".into() },

            // ── v321: Distillation ──
            BuiltinFn { name: "distill_kd_loss".into(), params: vec![("s", IrType::I64), ("t", IrType::I64), ("temp", IrType::I64)], ret: IrType::I64, runtime_name: "slang_distill_kd_loss".into() },
            BuiltinFn { name: "distill_feature_loss".into(), params: vec![("s", IrType::I64), ("t", IrType::I64)], ret: IrType::I64, runtime_name: "slang_distill_feature_loss".into() },
            BuiltinFn { name: "distill_attention_transfer".into(), params: vec![("s", IrType::I64), ("t", IrType::I64)], ret: IrType::I64, runtime_name: "slang_distill_attention_transfer".into() },
            BuiltinFn { name: "distill_temperature".into(), params: vec![("t", IrType::I64)], ret: IrType::I64, runtime_name: "slang_distill_temperature".into() },

            // ── v322: GNN ──
            BuiltinFn { name: "gnn_create".into(), params: vec![("n", IrType::I64)], ret: IrType::I64, runtime_name: "slang_gnn_create".into() },
            BuiltinFn { name: "gnn_add_edge".into(), params: vec![("id", IrType::I64), ("s", IrType::I64), ("d", IrType::I64)], ret: IrType::I64, runtime_name: "slang_gnn_add_edge".into() },
            BuiltinFn { name: "gnn_set_feature".into(), params: vec![("id", IrType::I64), ("n", IrType::I64), ("f", IrType::I64)], ret: IrType::I64, runtime_name: "slang_gnn_set_feature".into() },
            BuiltinFn { name: "gnn_message_pass".into(), params: vec![("id", IrType::I64)], ret: IrType::I64, runtime_name: "slang_gnn_message_pass".into() },
            BuiltinFn { name: "gnn_conv".into(), params: vec![("id", IrType::I64), ("node", IrType::I64)], ret: IrType::I64, runtime_name: "slang_gnn_conv".into() },
            BuiltinFn { name: "gnn_attention".into(), params: vec![("id", IrType::I64), ("n", IrType::I64)], ret: IrType::I64, runtime_name: "slang_gnn_attention".into() },
            BuiltinFn { name: "gnn_readout".into(), params: vec![("id", IrType::I64)], ret: IrType::I64, runtime_name: "slang_gnn_readout".into() },

            // ── v323: Diffusion ──
            BuiltinFn { name: "diffusion_forward".into(), params: vec![("x", IrType::I64), ("t", IrType::I64)], ret: IrType::I64, runtime_name: "slang_diffusion_forward".into() },
            BuiltinFn { name: "diffusion_reverse".into(), params: vec![("x", IrType::I64), ("t", IrType::I64)], ret: IrType::I64, runtime_name: "slang_diffusion_reverse".into() },
            BuiltinFn { name: "diffusion_schedule".into(), params: vec![("t", IrType::I64), ("steps", IrType::I64)], ret: IrType::I64, runtime_name: "slang_diffusion_schedule".into() },
            BuiltinFn { name: "diffusion_sample".into(), params: vec![("id", IrType::I64)], ret: IrType::I64, runtime_name: "slang_diffusion_sample".into() },

            // ── v324: RL ──
            BuiltinFn { name: "rl_q_update".into(), params: vec![("q", IrType::I64), ("r", IrType::I64), ("lr", IrType::I64)], ret: IrType::I64, runtime_name: "slang_rl_q_update".into() },
            BuiltinFn { name: "rl_policy_gradient".into(), params: vec![("r", IrType::I64), ("lp", IrType::I64)], ret: IrType::I64, runtime_name: "slang_rl_policy_gradient".into() },
            BuiltinFn { name: "rl_advantage".into(), params: vec![("v", IrType::I64), ("q", IrType::I64)], ret: IrType::I64, runtime_name: "slang_rl_advantage".into() },
            BuiltinFn { name: "rl_reward_discount".into(), params: vec![("r", IrType::I64), ("g", IrType::I64)], ret: IrType::I64, runtime_name: "slang_rl_reward_discount".into() },

            // ── v325: Embedding Search ──
            BuiltinFn { name: "hnsw_create".into(), params: vec![("dim", IrType::I64), ("m", IrType::I64)], ret: IrType::I64, runtime_name: "slang_hnsw_create".into() },
            BuiltinFn { name: "hnsw_insert".into(), params: vec![("id", IrType::I64), ("vec", IrType::I64)], ret: IrType::I64, runtime_name: "slang_hnsw_insert".into() },
            BuiltinFn { name: "hnsw_search".into(), params: vec![("id", IrType::I64), ("q", IrType::I64), ("k", IrType::I64)], ret: IrType::I64, runtime_name: "slang_hnsw_search".into() },
            BuiltinFn { name: "hnsw_recall".into(), params: vec![("id", IrType::I64)], ret: IrType::I64, runtime_name: "slang_hnsw_recall".into() },

            // ── v326: Tokenizer ──
            BuiltinFn { name: "tokenizer_bpe_train".into(), params: vec![("d", IrType::I64), ("v", IrType::I64)], ret: IrType::I64, runtime_name: "slang_tokenizer_bpe_train".into() },
            BuiltinFn { name: "tokenizer_encode".into(), params: vec![("h", IrType::I64)], ret: IrType::I64, runtime_name: "slang_tokenizer_encode".into() },
            BuiltinFn { name: "tokenizer_decode".into(), params: vec![("t", IrType::I64)], ret: IrType::I64, runtime_name: "slang_tokenizer_decode".into() },
            BuiltinFn { name: "tokenizer_vocab_size".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_tokenizer_vocab_size".into() },

            // ── v327: RLHF ──
            BuiltinFn { name: "rlhf_reward".into(), params: vec![("r", IrType::I64), ("q", IrType::I64)], ret: IrType::I64, runtime_name: "slang_rlhf_reward".into() },
            BuiltinFn { name: "rlhf_kl_penalty".into(), params: vec![("p", IrType::I64), ("q", IrType::I64)], ret: IrType::I64, runtime_name: "slang_rlhf_kl_penalty".into() },
            BuiltinFn { name: "rlhf_preference".into(), params: vec![("a", IrType::I64), ("b", IrType::I64)], ret: IrType::I64, runtime_name: "slang_rlhf_preference".into() },
            BuiltinFn { name: "rlhf_ppo_clip".into(), params: vec![("r", IrType::I64), ("e", IrType::I64)], ret: IrType::I64, runtime_name: "slang_rlhf_ppo_clip".into() },

            // ── v328: Async Runtime ──
            BuiltinFn { name: "async_spawn_task".into(), params: vec![("t", IrType::I64)], ret: IrType::I64, runtime_name: "slang_async_spawn_task".into() },
            BuiltinFn { name: "async_yield_now".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_async_yield_now".into() },
            BuiltinFn { name: "async_select".into(), params: vec![("a", IrType::I64), ("b", IrType::I64)], ret: IrType::I64, runtime_name: "slang_async_select".into() },
            BuiltinFn { name: "async_timeout".into(), params: vec![("ms", IrType::I64)], ret: IrType::I64, runtime_name: "slang_async_timeout".into() },

            // ── v329: Work Stealing ──
            BuiltinFn { name: "ws_create_pool".into(), params: vec![("w", IrType::I64)], ret: IrType::I64, runtime_name: "slang_ws_create_pool".into() },
            BuiltinFn { name: "ws_submit".into(), params: vec![("p", IrType::I64), ("t", IrType::I64)], ret: IrType::I64, runtime_name: "slang_ws_submit".into() },
            BuiltinFn { name: "ws_steal_count".into(), params: vec![("p", IrType::I64)], ret: IrType::I64, runtime_name: "slang_ws_steal_count".into() },
            BuiltinFn { name: "ws_active_workers".into(), params: vec![("p", IrType::I64)], ret: IrType::I64, runtime_name: "slang_ws_active_workers".into() },

            // ── v330: Connection Pool ──
            BuiltinFn { name: "conn_pool_create".into(), params: vec![("max", IrType::I64)], ret: IrType::I64, runtime_name: "slang_conn_pool_create".into() },
            BuiltinFn { name: "pool_acquire".into(), params: vec![("id", IrType::I64)], ret: IrType::I64, runtime_name: "slang_pool_acquire".into() },
            BuiltinFn { name: "pool_release".into(), params: vec![("id", IrType::I64), ("c", IrType::I64)], ret: IrType::I64, runtime_name: "slang_pool_release".into() },
            BuiltinFn { name: "pool_stats".into(), params: vec![("id", IrType::I64)], ret: IrType::I64, runtime_name: "slang_pool_stats".into() },

            // ── v331: Protobuf ──
            BuiltinFn { name: "protobuf_encode".into(), params: vec![("m", IrType::I64)], ret: IrType::I64, runtime_name: "slang_protobuf_encode".into() },
            BuiltinFn { name: "protobuf_decode".into(), params: vec![("d", IrType::I64)], ret: IrType::I64, runtime_name: "slang_protobuf_decode".into() },
            BuiltinFn { name: "protobuf_field".into(), params: vec![("m", IrType::I64), ("f", IrType::I64)], ret: IrType::I64, runtime_name: "slang_protobuf_field".into() },
            BuiltinFn { name: "protobuf_size".into(), params: vec![("m", IrType::I64)], ret: IrType::I64, runtime_name: "slang_protobuf_size".into() },

            // ── v332: Consensus (Raft) ──
            BuiltinFn { name: "raft_propose".into(), params: vec![("v", IrType::I64)], ret: IrType::I64, runtime_name: "slang_raft_propose".into() },
            BuiltinFn { name: "raft_commit_index".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_raft_commit_index".into() },
            BuiltinFn { name: "raft_leader".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_raft_leader".into() },
            BuiltinFn { name: "raft_term".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_raft_term".into() },

            // ── v333: Event Sourcing ──
            BuiltinFn { name: "event_store_create".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_event_store_create".into() },
            BuiltinFn { name: "event_append".into(), params: vec![("id", IrType::I64), ("ev", IrType::I64), ("p", IrType::I64)], ret: IrType::I64, runtime_name: "slang_event_append".into() },
            BuiltinFn { name: "event_replay".into(), params: vec![("id", IrType::I64)], ret: IrType::I64, runtime_name: "slang_event_replay".into() },
            BuiltinFn { name: "event_snapshot".into(), params: vec![("id", IrType::I64)], ret: IrType::I64, runtime_name: "slang_event_snapshot".into() },
            BuiltinFn { name: "event_project".into(), params: vec![("id", IrType::I64), ("p", IrType::I64)], ret: IrType::I64, runtime_name: "slang_event_project".into() },

            // ── v334: Stream Processing ──
            BuiltinFn { name: "stream_create".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_stream_create".into() },
            BuiltinFn { name: "stream_window_tumbling".into(), params: vec![("id", IrType::I64), ("ms", IrType::I64)], ret: IrType::I64, runtime_name: "slang_stream_window_tumbling".into() },
            BuiltinFn { name: "stream_window_sliding".into(), params: vec![("id", IrType::I64), ("ms", IrType::I64), ("slide", IrType::I64)], ret: IrType::I64, runtime_name: "slang_stream_window_sliding".into() },
            BuiltinFn { name: "stream_watermark".into(), params: vec![("id", IrType::I64), ("ts", IrType::I64)], ret: IrType::I64, runtime_name: "slang_stream_watermark".into() },
            BuiltinFn { name: "stream_late_count".into(), params: vec![("id", IrType::I64)], ret: IrType::I64, runtime_name: "slang_stream_late_count".into() },

            // ── v335: Message Queue ──
            BuiltinFn { name: "mq_create_topic".into(), params: vec![("partitions", IrType::I64)], ret: IrType::I64, runtime_name: "slang_mq_create_topic".into() },
            BuiltinFn { name: "mq_publish".into(), params: vec![("id", IrType::I64), ("msg", IrType::I64)], ret: IrType::I64, runtime_name: "slang_mq_publish".into() },
            BuiltinFn { name: "mq_subscribe".into(), params: vec![("id", IrType::I64), ("group", IrType::I64)], ret: IrType::I64, runtime_name: "slang_mq_subscribe".into() },
            BuiltinFn { name: "mq_consume".into(), params: vec![("id", IrType::I64)], ret: IrType::I64, runtime_name: "slang_mq_consume".into() },
            BuiltinFn { name: "mq_offset".into(), params: vec![("id", IrType::I64)], ret: IrType::I64, runtime_name: "slang_mq_offset".into() },

            // ── v336: CQRS ──
            BuiltinFn { name: "cqrs_command".into(), params: vec![("cmd", IrType::I64), ("payload", IrType::I64)], ret: IrType::I64, runtime_name: "slang_cqrs_command".into() },
            BuiltinFn { name: "cqrs_query".into(), params: vec![("q", IrType::I64)], ret: IrType::I64, runtime_name: "slang_cqrs_query".into() },
            BuiltinFn { name: "cqrs_command_count".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_cqrs_command_count".into() },
            BuiltinFn { name: "cqrs_query_count".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_cqrs_query_count".into() },

            // ── v337: GraphQL ──
            BuiltinFn { name: "graphql_schema_create".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_graphql_schema_create".into() },
            BuiltinFn { name: "graphql_add_type".into(), params: vec![("id", IrType::I64), ("t", IrType::I64)], ret: IrType::I64, runtime_name: "slang_graphql_add_type".into() },
            BuiltinFn { name: "graphql_add_field".into(), params: vec![("id", IrType::I64), ("t", IrType::I64), ("f", IrType::I64)], ret: IrType::I64, runtime_name: "slang_graphql_add_field".into() },
            BuiltinFn { name: "graphql_validate".into(), params: vec![("id", IrType::I64)], ret: IrType::I64, runtime_name: "slang_graphql_validate".into() },
            BuiltinFn { name: "graphql_type_count".into(), params: vec![("id", IrType::I64)], ret: IrType::I64, runtime_name: "slang_graphql_type_count".into() },

            // ── v338: JWT ──
            BuiltinFn { name: "jwt_create".into(), params: vec![("sub", IrType::I64), ("exp", IrType::I64)], ret: IrType::I64, runtime_name: "slang_jwt_create".into() },
            BuiltinFn { name: "jwt_verify".into(), params: vec![("t", IrType::I64), ("secret", IrType::I64)], ret: IrType::I64, runtime_name: "slang_jwt_verify".into() },
            BuiltinFn { name: "jwt_claims".into(), params: vec![("t", IrType::I64)], ret: IrType::I64, runtime_name: "slang_jwt_claims".into() },
            BuiltinFn { name: "jwt_expired".into(), params: vec![("t", IrType::I64), ("now", IrType::I64)], ret: IrType::I64, runtime_name: "slang_jwt_expired".into() },
            BuiltinFn { name: "jwt_set_claim".into(), params: vec![("t", IrType::I64), ("k", IrType::I64), ("v", IrType::I64)], ret: IrType::I64, runtime_name: "slang_jwt_set_claim".into() },
            BuiltinFn { name: "jwt_get_claim".into(), params: vec![("t", IrType::I64), ("k", IrType::I64)], ret: IrType::I64, runtime_name: "slang_jwt_get_claim".into() },

            // ── v339: OAuth2 ──
            BuiltinFn { name: "oauth2_auth_url".into(), params: vec![("client", IrType::I64), ("scope", IrType::I64)], ret: IrType::I64, runtime_name: "slang_oauth2_auth_url".into() },
            BuiltinFn { name: "oauth2_exchange".into(), params: vec![("code", IrType::I64), ("secret", IrType::I64)], ret: IrType::I64, runtime_name: "slang_oauth2_exchange".into() },
            BuiltinFn { name: "oauth2_refresh".into(), params: vec![("token", IrType::I64)], ret: IrType::I64, runtime_name: "slang_oauth2_refresh".into() },
            BuiltinFn { name: "oauth2_pkce_verify".into(), params: vec![("verifier", IrType::I64), ("challenge", IrType::I64)], ret: IrType::I64, runtime_name: "slang_oauth2_pkce_verify".into() },
            BuiltinFn { name: "oauth2_state".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_oauth2_state".into() },

            // ── v340: Rate Limiter ──
            BuiltinFn { name: "ratelimit_check".into(), params: vec![("t", IrType::I64)], ret: IrType::I64, runtime_name: "slang_ratelimit_check".into() },
            BuiltinFn { name: "ratelimit_remaining".into(), params: vec![("b", IrType::I64)], ret: IrType::I64, runtime_name: "slang_ratelimit_remaining".into() },
            BuiltinFn { name: "ratelimit_reset".into(), params: vec![("c", IrType::I64)], ret: IrType::I64, runtime_name: "slang_ratelimit_reset".into() },
            BuiltinFn { name: "ratelimit_window".into(), params: vec![("s", IrType::I64)], ret: IrType::I64, runtime_name: "slang_ratelimit_window".into() },

            // ── v341: Chaos Engineering ──
            BuiltinFn { name: "chaos_inject_fault".into(), params: vec![("svc", IrType::I64), ("t", IrType::I64)], ret: IrType::I64, runtime_name: "slang_chaos_inject_fault".into() },
            BuiltinFn { name: "chaos_inject_latency".into(), params: vec![("svc", IrType::I64), ("ms", IrType::I64)], ret: IrType::I64, runtime_name: "slang_chaos_inject_latency".into() },
            BuiltinFn { name: "chaos_error_rate".into(), params: vec![("svc", IrType::I64), ("pct", IrType::I64)], ret: IrType::I64, runtime_name: "slang_chaos_error_rate".into() },
            BuiltinFn { name: "chaos_partition".into(), params: vec![("a", IrType::I64), ("b", IrType::I64)], ret: IrType::I64, runtime_name: "slang_chaos_partition".into() },
            BuiltinFn { name: "chaos_fault_count".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_chaos_fault_count".into() },
            BuiltinFn { name: "chaos_total_latency".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_chaos_total_latency".into() },
            BuiltinFn { name: "chaos_is_partitioned".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_chaos_is_partitioned".into() },
            BuiltinFn { name: "chaos_current_error_rate".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_chaos_current_error_rate".into() },

            // ── v342: RBAC ──
            BuiltinFn { name: "rbac_assign_role".into(), params: vec![("u", IrType::I64), ("r", IrType::I64)], ret: IrType::I64, runtime_name: "slang_rbac_assign_role".into() },
            BuiltinFn { name: "rbac_check_perm".into(), params: vec![("u", IrType::I64), ("p", IrType::I64)], ret: IrType::I64, runtime_name: "slang_rbac_check_perm".into() },
            BuiltinFn { name: "rbac_grant".into(), params: vec![("r", IrType::I64), ("p", IrType::I64)], ret: IrType::I64, runtime_name: "slang_rbac_grant".into() },
            BuiltinFn { name: "rbac_revoke".into(), params: vec![("r", IrType::I64), ("p", IrType::I64)], ret: IrType::I64, runtime_name: "slang_rbac_revoke".into() },

            // ── v343: CSP ──
            BuiltinFn { name: "csp_create".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_csp_create".into() },
            BuiltinFn { name: "csp_add_directive".into(), params: vec![("id", IrType::I64), ("d", IrType::I64)], ret: IrType::I64, runtime_name: "slang_csp_add_directive".into() },
            BuiltinFn { name: "csp_nonce".into(), params: vec![("id", IrType::I64)], ret: IrType::I64, runtime_name: "slang_csp_nonce".into() },
            BuiltinFn { name: "csp_directive_count".into(), params: vec![("id", IrType::I64)], ret: IrType::I64, runtime_name: "slang_csp_directive_count".into() },
            BuiltinFn { name: "csp_report_only".into(), params: vec![("id", IrType::I64)], ret: IrType::I64, runtime_name: "slang_csp_report_only".into() },
            BuiltinFn { name: "csp_check".into(), params: vec![("id", IrType::I64), ("r", IrType::I64)], ret: IrType::I64, runtime_name: "slang_csp_check".into() },

            // ── v344: Input Validation ──
            BuiltinFn { name: "validate_range".into(), params: vec![("v", IrType::I64), ("min", IrType::I64), ("max", IrType::I64)], ret: IrType::I64, runtime_name: "slang_validate_range".into() },
            BuiltinFn { name: "validate_length".into(), params: vec![("v", IrType::I64), ("min", IrType::I64), ("max", IrType::I64)], ret: IrType::I64, runtime_name: "slang_validate_length".into() },
            BuiltinFn { name: "validate_pattern".into(), params: vec![("v", IrType::I64), ("p", IrType::I64)], ret: IrType::I64, runtime_name: "slang_validate_pattern".into() },
            BuiltinFn { name: "validate_sanitize".into(), params: vec![("v", IrType::I64)], ret: IrType::I64, runtime_name: "slang_validate_sanitize".into() },

            // ── v345: Audit Log ──
            BuiltinFn { name: "audit_log".into(), params: vec![("actor", IrType::I64), ("action", IrType::I64)], ret: IrType::I64, runtime_name: "slang_audit_log".into() },
            BuiltinFn { name: "audit_last_action".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_audit_last_action".into() },

            // ── v346: Encryption ──
            BuiltinFn { name: "encrypt_xor".into(), params: vec![("d", IrType::I64), ("k", IrType::I64)], ret: IrType::I64, runtime_name: "slang_encrypt_xor".into() },
            BuiltinFn { name: "encrypt_rotate".into(), params: vec![("d", IrType::I64), ("b", IrType::I64)], ret: IrType::I64, runtime_name: "slang_encrypt_rotate".into() },
            BuiltinFn { name: "encrypt_hash".into(), params: vec![("d", IrType::I64)], ret: IrType::I64, runtime_name: "slang_encrypt_hash".into() },
            BuiltinFn { name: "encrypt_verify".into(), params: vec![("d", IrType::I64), ("h", IrType::I64)], ret: IrType::I64, runtime_name: "slang_encrypt_verify".into() },

            // ── v347: Certificate ──
            BuiltinFn { name: "cert_create".into(), params: vec![("i", IrType::I64), ("e", IrType::I64)], ret: IrType::I64, runtime_name: "slang_cert_create".into() },
            BuiltinFn { name: "cert_verify".into(), params: vec![("c", IrType::I64), ("n", IrType::I64)], ret: IrType::I64, runtime_name: "slang_cert_verify".into() },
            BuiltinFn { name: "cert_expiry".into(), params: vec![("c", IrType::I64)], ret: IrType::I64, runtime_name: "slang_cert_expiry".into() },
            BuiltinFn { name: "cert_chain_length".into(), params: vec![("c", IrType::I64)], ret: IrType::I64, runtime_name: "slang_cert_chain_length".into() },

            // ── v348: Test Runner ──
            BuiltinFn { name: "test_discover".into(), params: vec![("module", IrType::I64)], ret: IrType::I64, runtime_name: "slang_test_discover".into() },
            BuiltinFn { name: "test_pass".into(), params: vec![("id", IrType::I64)], ret: IrType::I64, runtime_name: "slang_test_pass".into() },
            BuiltinFn { name: "test_fail".into(), params: vec![("id", IrType::I64), ("msg", IrType::I64)], ret: IrType::I64, runtime_name: "slang_test_fail".into() },
            BuiltinFn { name: "test_skip".into(), params: vec![("id", IrType::I64)], ret: IrType::I64, runtime_name: "slang_test_skip".into() },
            BuiltinFn { name: "test_coverage".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_test_coverage".into() },
            BuiltinFn { name: "test_pass_rate".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_test_pass_rate".into() },
            BuiltinFn { name: "test_total".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_test_total".into() },

            // ── v349: Snapshot Testing ──
            BuiltinFn { name: "snapshot_capture".into(), params: vec![("name", IrType::I64), ("val", IrType::I64)], ret: IrType::I64, runtime_name: "slang_snapshot_capture".into() },
            BuiltinFn { name: "snapshot_compare".into(), params: vec![("name", IrType::I64), ("val", IrType::I64)], ret: IrType::I64, runtime_name: "slang_snapshot_compare".into() },
            BuiltinFn { name: "snapshot_update".into(), params: vec![("name", IrType::I64), ("val", IrType::I64)], ret: IrType::I64, runtime_name: "slang_snapshot_update".into() },
            BuiltinFn { name: "snapshot_version".into(), params: vec![("name", IrType::I64)], ret: IrType::I64, runtime_name: "slang_snapshot_version".into() },
            BuiltinFn { name: "snapshot_match_count".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_snapshot_match_count".into() },
            BuiltinFn { name: "snapshot_mismatch_count".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_snapshot_mismatch_count".into() },

            // ── v350: Fuzzer ──
            BuiltinFn { name: "fuzz_add_corpus".into(), params: vec![("input", IrType::I64)], ret: IrType::I64, runtime_name: "slang_fuzz_add_corpus".into() },
            BuiltinFn { name: "fuzz_run".into(), params: vec![("iters", IrType::I64)], ret: IrType::I64, runtime_name: "slang_fuzz_run".into() },
            BuiltinFn { name: "fuzz_crash".into(), params: vec![("input", IrType::I64)], ret: IrType::I64, runtime_name: "slang_fuzz_crash".into() },
            BuiltinFn { name: "fuzz_corpus_size".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_fuzz_corpus_size".into() },
            BuiltinFn { name: "fuzz_coverage".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_fuzz_coverage".into() },
            BuiltinFn { name: "fuzz_crash_count".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_fuzz_crash_count".into() },
            BuiltinFn { name: "fuzz_unique_crashes".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_fuzz_unique_crashes".into() },
            BuiltinFn { name: "fuzz_total_runs".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_fuzz_total_runs".into() },

            // ── v351: Property Testing ──
            BuiltinFn { name: "prop_check".into(), params: vec![("v", IrType::I64), ("p", IrType::I64)], ret: IrType::I64, runtime_name: "slang_prop_check".into() },
            BuiltinFn { name: "prop_shrink".into(), params: vec![("v", IrType::I64)], ret: IrType::I64, runtime_name: "slang_prop_shrink".into() },
            BuiltinFn { name: "prop_counterexample".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_prop_counterexample".into() },
            BuiltinFn { name: "prop_total_checks".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_prop_total_checks".into() },

            // ── v352: Mutation Testing ──
            BuiltinFn { name: "mutation_inject".into(), params: vec![("l", IrType::I64)], ret: IrType::I64, runtime_name: "slang_mutation_inject".into() },
            BuiltinFn { name: "mutation_killed".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_mutation_killed".into() },
            BuiltinFn { name: "mutation_survived".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_mutation_survived".into() },
            BuiltinFn { name: "mut_test_score".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_mut_test_score".into() },

            // ── v353: API Compat ──
            BuiltinFn { name: "api_semver_diff".into(), params: vec![("old", IrType::I64), ("new", IrType::I64)], ret: IrType::I64, runtime_name: "slang_api_semver_diff".into() },
            BuiltinFn { name: "api_breaking_change".into(), params: vec![("old", IrType::I64), ("new", IrType::I64)], ret: IrType::I64, runtime_name: "slang_api_breaking_change".into() },
            BuiltinFn { name: "api_addition".into(), params: vec![("ver", IrType::I64), ("sym", IrType::I64)], ret: IrType::I64, runtime_name: "slang_api_addition".into() },
            BuiltinFn { name: "api_deprecation".into(), params: vec![("ver", IrType::I64), ("sym", IrType::I64)], ret: IrType::I64, runtime_name: "slang_api_deprecation".into() },
            BuiltinFn { name: "api_surface".into(), params: vec![("ver", IrType::I64)], ret: IrType::I64, runtime_name: "slang_api_surface".into() },
            BuiltinFn { name: "api_breaking_count".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_api_breaking_count".into() },
            BuiltinFn { name: "api_addition_count".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_api_addition_count".into() },
            BuiltinFn { name: "api_deprecation_count".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_api_deprecation_count".into() },

            // ── v354: Migration ──
            BuiltinFn { name: "migration_create".into(), params: vec![("ver", IrType::I64)], ret: IrType::I64, runtime_name: "slang_migration_create".into() },
            BuiltinFn { name: "migration_transform".into(), params: vec![("id", IrType::I64), ("data", IrType::I64)], ret: IrType::I64, runtime_name: "slang_migration_transform".into() },
            BuiltinFn { name: "migration_rollback".into(), params: vec![("id", IrType::I64)], ret: IrType::I64, runtime_name: "slang_migration_rollback".into() },
            BuiltinFn { name: "migration_progress".into(), params: vec![("id", IrType::I64)], ret: IrType::I64, runtime_name: "slang_migration_progress".into() },
            BuiltinFn { name: "migration_delta".into(), params: vec![("from", IrType::I64), ("to", IrType::I64)], ret: IrType::I64, runtime_name: "slang_migration_delta".into() },

            // ── v355: Code Actions ──
            BuiltinFn { name: "codeaction_extract".into(), params: vec![("s", IrType::I64)], ret: IrType::I64, runtime_name: "slang_codeaction_extract".into() },
            BuiltinFn { name: "codeaction_inline".into(), params: vec![("f", IrType::I64)], ret: IrType::I64, runtime_name: "slang_codeaction_inline".into() },
            BuiltinFn { name: "codeaction_rename".into(), params: vec![("o", IrType::I64), ("n", IrType::I64)], ret: IrType::I64, runtime_name: "slang_codeaction_rename".into() },
            BuiltinFn { name: "codeaction_count".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_codeaction_count".into() },

            // ── v356: Telemetry ──
            BuiltinFn { name: "telemetry_compile_time".into(), params: vec![("ms", IrType::I64)], ret: IrType::I64, runtime_name: "slang_telemetry_compile_time".into() },
            BuiltinFn { name: "telemetry_peak_memory".into(), params: vec![("b", IrType::I64)], ret: IrType::I64, runtime_name: "slang_telemetry_peak_memory".into() },
            BuiltinFn { name: "telemetry_cache_hits".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_telemetry_cache_hits".into() },
            BuiltinFn { name: "telemetry_error_count".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_telemetry_error_count".into() },

            // ── v357: Build System ──
            BuiltinFn { name: "build_target".into(), params: vec![("t", IrType::I64)], ret: IrType::I64, runtime_name: "slang_build_target".into() },
            BuiltinFn { name: "build_parallel".into(), params: vec![("j", IrType::I64)], ret: IrType::I64, runtime_name: "slang_build_parallel".into() },
            BuiltinFn { name: "build_cache_hit".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_build_cache_hit".into() },
            BuiltinFn { name: "build_artifact_count".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_build_artifact_count".into() },

            // ── v358: OpenAPI ──
            BuiltinFn { name: "openapi_create".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_openapi_create".into() },
            BuiltinFn { name: "openapi_add_route".into(), params: vec![("id", IrType::I64), ("method", IrType::I64), ("path", IrType::I64)], ret: IrType::I64, runtime_name: "slang_openapi_add_route".into() },
            BuiltinFn { name: "openapi_add_param".into(), params: vec![("id", IrType::I64), ("route", IrType::I64), ("p", IrType::I64)], ret: IrType::I64, runtime_name: "slang_openapi_add_param".into() },
            BuiltinFn { name: "openapi_validate".into(), params: vec![("id", IrType::I64)], ret: IrType::I64, runtime_name: "slang_openapi_validate".into() },
            BuiltinFn { name: "openapi_route_count".into(), params: vec![("id", IrType::I64)], ret: IrType::I64, runtime_name: "slang_openapi_route_count".into() },

            // ── v359: Benchmarking ──
            BuiltinFn { name: "bench_start".into(), params: vec![("id", IrType::I64)], ret: IrType::I64, runtime_name: "slang_bench_start".into() },
            BuiltinFn { name: "bench_stop".into(), params: vec![("id", IrType::I64)], ret: IrType::I64, runtime_name: "slang_bench_stop".into() },
            BuiltinFn { name: "bench_iterations".into(), params: vec![("id", IrType::I64), ("n", IrType::I64)], ret: IrType::I64, runtime_name: "slang_bench_iterations".into() },
            BuiltinFn { name: "bench_throughput".into(), params: vec![("id", IrType::I64)], ret: IrType::I64, runtime_name: "slang_bench_throughput".into() },

            // ── v360: Profiler ──
            BuiltinFn { name: "profile_begin".into(), params: vec![("f", IrType::I64)], ret: IrType::I64, runtime_name: "slang_profile_begin".into() },
            BuiltinFn { name: "profile_end".into(), params: vec![("f", IrType::I64)], ret: IrType::I64, runtime_name: "slang_profile_end".into() },
            BuiltinFn { name: "profile_flamegraph".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_profile_flamegraph".into() },
            BuiltinFn { name: "profile_hotspot".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_profile_hotspot".into() },

            // ── v361: Release ──
            BuiltinFn { name: "release_changelog".into(), params: vec![("ver", IrType::I64), ("entry", IrType::I64)], ret: IrType::I64, runtime_name: "slang_release_changelog".into() },
            BuiltinFn { name: "release_version_bump".into(), params: vec![("cur", IrType::I64), ("kind", IrType::I64)], ret: IrType::I64, runtime_name: "slang_release_version_bump".into() },
            BuiltinFn { name: "release_package".into(), params: vec![("ver", IrType::I64)], ret: IrType::I64, runtime_name: "slang_release_package".into() },
            BuiltinFn { name: "release_version".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_release_version".into() },
            BuiltinFn { name: "release_changelog_count".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_release_changelog_count".into() },
            BuiltinFn { name: "release_packages_built".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_release_packages_built".into() },

            // ── v362: Plugin System ──
            BuiltinFn { name: "plugin_load".into(), params: vec![("name", IrType::I64)], ret: IrType::I64, runtime_name: "slang_plugin_load".into() },
            BuiltinFn { name: "plugin_register_hook".into(), params: vec![("id", IrType::I64), ("hook", IrType::I64)], ret: IrType::I64, runtime_name: "slang_plugin_register_hook".into() },
            BuiltinFn { name: "plugin_activate".into(), params: vec![("id", IrType::I64)], ret: IrType::I64, runtime_name: "slang_plugin_activate".into() },
            BuiltinFn { name: "plugin_unload".into(), params: vec![("id", IrType::I64)], ret: IrType::I64, runtime_name: "slang_plugin_unload".into() },
            BuiltinFn { name: "plugin_state".into(), params: vec![("id", IrType::I64)], ret: IrType::I64, runtime_name: "slang_plugin_state".into() },
            BuiltinFn { name: "plugin_hook_count".into(), params: vec![("id", IrType::I64)], ret: IrType::I64, runtime_name: "slang_plugin_hook_count".into() },
            BuiltinFn { name: "plugin_count".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_plugin_count".into() },

            // ── v363: Wasm Component Model ──
            BuiltinFn { name: "wasm_component_create".into(), params: vec![("m", IrType::I64)], ret: IrType::I64, runtime_name: "slang_wasm_component_create".into() },
            BuiltinFn { name: "wasm_component_link".into(), params: vec![("a", IrType::I64), ("b", IrType::I64)], ret: IrType::I64, runtime_name: "slang_wasm_component_link".into() },
            BuiltinFn { name: "wasm_component_instantiate".into(), params: vec![("c", IrType::I64)], ret: IrType::I64, runtime_name: "slang_wasm_component_instantiate".into() },
            BuiltinFn { name: "wasm_component_count".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_wasm_component_count".into() },

            // ── v364: WASI Preview2 ──
            BuiltinFn { name: "wasi_fs_read".into(), params: vec![("fd", IrType::I64)], ret: IrType::I64, runtime_name: "slang_wasi_fs_read".into() },
            BuiltinFn { name: "wasi_fs_write".into(), params: vec![("fd", IrType::I64), ("d", IrType::I64)], ret: IrType::I64, runtime_name: "slang_wasi_fs_write".into() },
            BuiltinFn { name: "wasi_clock".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_wasi_clock".into() },
            BuiltinFn { name: "wasi_random".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_wasi_random".into() },

            // ── v365: Package Registry ──
            BuiltinFn { name: "registry_publish".into(), params: vec![("n", IrType::I64), ("v", IrType::I64)], ret: IrType::I64, runtime_name: "slang_registry_publish".into() },
            BuiltinFn { name: "registry_resolve".into(), params: vec![("n", IrType::I64)], ret: IrType::I64, runtime_name: "slang_registry_resolve".into() },
            BuiltinFn { name: "registry_download".into(), params: vec![("n", IrType::I64)], ret: IrType::I64, runtime_name: "slang_registry_download".into() },
            BuiltinFn { name: "registry_version_count".into(), params: vec![("n", IrType::I64)], ret: IrType::I64, runtime_name: "slang_registry_version_count".into() },

            // ── v366: Milestone ──
            BuiltinFn { name: "vitalis_v366_version".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_vitalis_v366_version".into() },
            BuiltinFn { name: "vitalis_v366_total_builtins".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_vitalis_v366_total_builtins".into() },
            BuiltinFn { name: "vitalis_v366_modules".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_vitalis_v366_modules".into() },
            BuiltinFn { name: "vitalis_v366_milestone".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_vitalis_v366_milestone".into() },

        BuiltinFn { name: "format_int".into(),      params: vec![("fmt", IrType::Ptr), ("val", IrType::I64)],         ret: IrType::Ptr,  runtime_name: "slang_format_int".into() },
        BuiltinFn { name: "format_float".into(),    params: vec![("fmt", IrType::Ptr), ("val", IrType::F64)],         ret: IrType::Ptr,  runtime_name: "slang_format_float".into() },

        // ── v15: JSON ─────────────────────────────────────────────────
        BuiltinFn { name: "json_encode".into(),     params: vec![("m", IrType::I64)],                                 ret: IrType::Ptr,  runtime_name: "slang_json_encode".into() },
        BuiltinFn { name: "json_decode".into(),     params: vec![("s", IrType::Ptr)],                                 ret: IrType::I64,  runtime_name: "slang_json_decode".into() },

        // ── v18: Collection methods ───────────────────────────────────
        BuiltinFn { name: "array_push".into(),      params: vec![("arr", IrType::Ptr), ("val", IrType::I64)],         ret: IrType::Ptr,  runtime_name: "slang_array_push".into() },
        BuiltinFn { name: "array_pop".into(),       params: vec![("arr", IrType::Ptr)],                               ret: IrType::I64,  runtime_name: "slang_array_pop".into() },
        BuiltinFn { name: "array_contains".into(),  params: vec![("arr", IrType::Ptr), ("val", IrType::I64)],         ret: IrType::Bool, runtime_name: "slang_array_contains".into() },
        BuiltinFn { name: "array_reverse".into(),   params: vec![("arr", IrType::Ptr)],                               ret: IrType::Ptr,  runtime_name: "slang_array_reverse".into() },
        BuiltinFn { name: "array_sort".into(),      params: vec![("arr", IrType::Ptr)],                               ret: IrType::Ptr,  runtime_name: "slang_array_sort".into() },
        BuiltinFn { name: "array_join".into(),      params: vec![("arr", IrType::Ptr), ("delim", IrType::Ptr)],       ret: IrType::Ptr,  runtime_name: "slang_array_join".into() },
        BuiltinFn { name: "array_slice".into(),     params: vec![("arr", IrType::Ptr), ("start", IrType::I64), ("end", IrType::I64)], ret: IrType::Ptr, runtime_name: "slang_array_slice".into() },
        BuiltinFn { name: "array_find".into(),      params: vec![("arr", IrType::Ptr), ("val", IrType::I64)],         ret: IrType::I64,  runtime_name: "slang_array_find".into() },
        // ── Iterator / functional array ops ────────────────────────
        BuiltinFn { name: "array_range".into(),        params: vec![("start", IrType::I64), ("end", IrType::I64)],     ret: IrType::Ptr,  runtime_name: "slang_array_range".into() },
        BuiltinFn { name: "array_sum".into(),          params: vec![("arr", IrType::Ptr)],                             ret: IrType::I64,  runtime_name: "slang_array_sum".into() },
        BuiltinFn { name: "array_min".into(),          params: vec![("arr", IrType::Ptr)],                             ret: IrType::I64,  runtime_name: "slang_array_min".into() },
        BuiltinFn { name: "array_max".into(),          params: vec![("arr", IrType::Ptr)],                             ret: IrType::I64,  runtime_name: "slang_array_max".into() },
        BuiltinFn { name: "array_any".into(),          params: vec![("arr", IrType::Ptr), ("val", IrType::I64)],       ret: IrType::Bool, runtime_name: "slang_array_any".into() },
        BuiltinFn { name: "array_all_positive".into(), params: vec![("arr", IrType::Ptr)],                             ret: IrType::Bool, runtime_name: "slang_array_all_positive".into() },
        BuiltinFn { name: "array_count".into(),        params: vec![("arr", IrType::Ptr), ("val", IrType::I64)],       ret: IrType::I64,  runtime_name: "slang_array_count".into() },
        BuiltinFn { name: "array_flatten".into(),      params: vec![("arr", IrType::Ptr)],                             ret: IrType::Ptr,  runtime_name: "slang_array_flatten".into() },
        BuiltinFn { name: "array_zip".into(),          params: vec![("a", IrType::Ptr), ("b", IrType::Ptr)],           ret: IrType::Ptr,  runtime_name: "slang_array_zip".into() },
        BuiltinFn { name: "array_enumerate".into(),    params: vec![("arr", IrType::Ptr)],                             ret: IrType::Ptr,  runtime_name: "slang_array_enumerate".into() },
        BuiltinFn { name: "array_take".into(),         params: vec![("arr", IrType::Ptr), ("n", IrType::I64)],         ret: IrType::Ptr,  runtime_name: "slang_array_take".into() },
        BuiltinFn { name: "array_drop".into(),         params: vec![("arr", IrType::Ptr), ("n", IrType::I64)],         ret: IrType::Ptr,  runtime_name: "slang_array_drop".into() },
        BuiltinFn { name: "array_unique".into(),       params: vec![("arr", IrType::Ptr)],                             ret: IrType::Ptr,  runtime_name: "slang_array_unique".into() },
        BuiltinFn { name: "error_message".into(),   params: vec![],                                                   ret: IrType::Ptr,  runtime_name: "slang_error_message".into() },

        // ── v18: Regex ────────────────────────────────────────────────
        BuiltinFn { name: "regex_is_match".into(),       params: vec![("pattern", IrType::Ptr), ("text", IrType::Ptr)],                                ret: IrType::Bool, runtime_name: "slang_regex_is_match".into() },
        BuiltinFn { name: "regex_match_v18".into(),      params: vec![("pattern", IrType::Ptr), ("text", IrType::Ptr)],                                ret: IrType::Bool, runtime_name: "slang_regex_match".into() },
        BuiltinFn { name: "regex_find_v18".into(),       params: vec![("pattern", IrType::Ptr), ("text", IrType::Ptr)],                                ret: IrType::Ptr,  runtime_name: "slang_regex_find".into() },
        BuiltinFn { name: "regex_replace_v18".into(),    params: vec![("pattern", IrType::Ptr), ("text", IrType::Ptr), ("replacement", IrType::Ptr)],  ret: IrType::Ptr,  runtime_name: "slang_regex_replace".into() },
        BuiltinFn { name: "regex_split_count".into(),    params: vec![("pattern", IrType::Ptr), ("text", IrType::Ptr)],                                ret: IrType::I64,  runtime_name: "slang_regex_split_count".into() },
        BuiltinFn { name: "regex_split_get".into(),      params: vec![("pattern", IrType::Ptr), ("text", IrType::Ptr), ("idx", IrType::I64)],          ret: IrType::Ptr,  runtime_name: "slang_regex_split_get".into() },
        BuiltinFn { name: "regex_find_all_count".into(), params: vec![("pattern", IrType::Ptr), ("text", IrType::Ptr)],                                ret: IrType::I64,  runtime_name: "slang_regex_find_all_count".into() },
        BuiltinFn { name: "regex_find_all_get".into(),   params: vec![("pattern", IrType::Ptr), ("text", IrType::Ptr), ("idx", IrType::I64)],          ret: IrType::Ptr,  runtime_name: "slang_regex_find_all_get".into() },

        // ── v370: Async runtime ─────────────────────────────────────────
        BuiltinFn { name: "spawn".into(),         params: vec![("task_id", IrType::I64)],                                              ret: IrType::I64,  runtime_name: "slang_spawn".into() },
        BuiltinFn { name: "task_result".into(),   params: vec![("task_id", IrType::I64)],                                              ret: IrType::I64,  runtime_name: "slang_task_result".into() },
        BuiltinFn { name: "task_await".into(),    params: vec![("task_id", IrType::I64)],                                              ret: IrType::I64,  runtime_name: "slang_task_await".into() },
        BuiltinFn { name: "async_run_all".into(), params: vec![],                                                                       ret: IrType::I64,  runtime_name: "slang_async_run_all".into() },

        // ── v28: Graphics & Visualization ──────────────────────────────
        BuiltinFn { name: "gfx_create_canvas".into(),  params: vec![("width", IrType::I64), ("height", IrType::I64)],                ret: IrType::I64,  runtime_name: "slang_gfx_create_canvas".into() },
        BuiltinFn { name: "gfx_draw_rect".into(),      params: vec![("x", IrType::I64), ("y", IrType::I64), ("w", IrType::I64), ("h", IrType::I64)], ret: IrType::Void, runtime_name: "slang_gfx_draw_rect".into() },
        BuiltinFn { name: "gfx_draw_circle".into(),    params: vec![("cx", IrType::I64), ("cy", IrType::I64), ("r", IrType::I64)], ret: IrType::Void, runtime_name: "slang_gfx_draw_circle".into() },
        BuiltinFn { name: "gfx_draw_line".into(),      params: vec![("x1", IrType::I64), ("y1", IrType::I64), ("x2", IrType::I64), ("y2", IrType::I64)], ret: IrType::Void, runtime_name: "slang_gfx_draw_line".into() },
        BuiltinFn { name: "gfx_set_color".into(),      params: vec![("r", IrType::I64), ("g", IrType::I64), ("b", IrType::I64), ("a", IrType::I64)], ret: IrType::Void, runtime_name: "slang_gfx_set_color".into() },
        BuiltinFn { name: "gfx_fill".into(),            params: vec![("r", IrType::I64), ("g", IrType::I64), ("b", IrType::I64)],   ret: IrType::Void, runtime_name: "slang_gfx_fill".into() },
        BuiltinFn { name: "gfx_stroke".into(),          params: vec![("r", IrType::I64), ("g", IrType::I64), ("b", IrType::I64)],   ret: IrType::Void, runtime_name: "slang_gfx_stroke".into() },
        BuiltinFn { name: "gfx_stroke_weight".into(),   params: vec![("weight", IrType::I64)],                                        ret: IrType::Void, runtime_name: "slang_gfx_stroke_weight".into() },
        BuiltinFn { name: "gfx_translate".into(),       params: vec![("x", IrType::I64), ("y", IrType::I64)],                         ret: IrType::Void, runtime_name: "slang_gfx_translate".into() },
        BuiltinFn { name: "gfx_rotate".into(),          params: vec![("angle", IrType::I64)],                                          ret: IrType::Void, runtime_name: "slang_gfx_rotate".into() },
        BuiltinFn { name: "gfx_scale".into(),           params: vec![("sx", IrType::I64), ("sy", IrType::I64)],                       ret: IrType::Void, runtime_name: "slang_gfx_scale".into() },
        BuiltinFn { name: "gfx_to_svg".into(),          params: vec![("canvas", IrType::I64)],                                         ret: IrType::Ptr,  runtime_name: "slang_gfx_to_svg".into() },
        BuiltinFn { name: "chart_pie".into(),            params: vec![("data", IrType::Ptr), ("title", IrType::Ptr)],                  ret: IrType::Ptr,  runtime_name: "slang_chart_pie".into() },
        BuiltinFn { name: "chart_bar".into(),            params: vec![("data", IrType::Ptr), ("title", IrType::Ptr)],                  ret: IrType::Ptr,  runtime_name: "slang_chart_bar".into() },
        BuiltinFn { name: "chart_line".into(),           params: vec![("data", IrType::Ptr), ("title", IrType::Ptr)],                  ret: IrType::Ptr,  runtime_name: "slang_chart_line".into() },
        BuiltinFn { name: "chart_scatter".into(),        params: vec![("data", IrType::Ptr), ("title", IrType::Ptr)],                  ret: IrType::Ptr,  runtime_name: "slang_chart_scatter".into() },
        BuiltinFn { name: "chart_histogram".into(),      params: vec![("data", IrType::Ptr), ("bins", IrType::I64)],                   ret: IrType::Ptr,  runtime_name: "slang_chart_histogram".into() },
        BuiltinFn { name: "shader_compile".into(),       params: vec![("source", IrType::Ptr), ("backend", IrType::Ptr)],              ret: IrType::Ptr,  runtime_name: "slang_shader_compile".into() },
        BuiltinFn { name: "gui_create_window".into(),    params: vec![("title", IrType::Ptr), ("w", IrType::I64), ("h", IrType::I64)], ret: IrType::I64, runtime_name: "slang_gui_create_window".into() },
        BuiltinFn { name: "gui_add_button".into(),       params: vec![("label", IrType::Ptr)],                                          ret: IrType::I64,  runtime_name: "slang_gui_add_button".into() },
        BuiltinFn { name: "gui_add_text".into(),         params: vec![("content", IrType::Ptr)],                                        ret: IrType::I64,  runtime_name: "slang_gui_add_text".into() },
        BuiltinFn { name: "gui_add_slider".into(),       params: vec![("min", IrType::I64), ("max", IrType::I64)],                     ret: IrType::I64,  runtime_name: "slang_gui_add_slider".into() },
        BuiltinFn { name: "gui_set_theme".into(),        params: vec![("theme", IrType::Ptr)],                                          ret: IrType::Void, runtime_name: "slang_gui_set_theme".into() },
        BuiltinFn { name: "noise_perlin".into(),         params: vec![("x", IrType::I64)],                                              ret: IrType::I64,  runtime_name: "slang_noise_perlin".into() },
        BuiltinFn { name: "noise_perlin2d".into(),       params: vec![("x", IrType::I64), ("y", IrType::I64)],                         ret: IrType::I64,  runtime_name: "slang_noise_perlin2d".into() },
        BuiltinFn { name: "particle_create".into(),      params: vec![("x", IrType::I64), ("y", IrType::I64)],                         ret: IrType::I64,  runtime_name: "slang_particle_create".into() },
        BuiltinFn { name: "node_graph_create".into(),    params: vec![("name", IrType::Ptr)],                                           ret: IrType::I64,  runtime_name: "slang_node_graph_create".into() },
        BuiltinFn { name: "node_graph_add_node".into(),  params: vec![("graph", IrType::I64), ("name", IrType::Ptr), ("op", IrType::Ptr)], ret: IrType::I64, runtime_name: "slang_node_graph_add_node".into() },
        BuiltinFn { name: "node_graph_connect".into(),   params: vec![("graph", IrType::I64), ("from", IrType::I64), ("to", IrType::I64)], ret: IrType::I64, runtime_name: "slang_node_graph_connect".into() },
        BuiltinFn { name: "node_graph_evaluate".into(),  params: vec![("graph", IrType::I64)],                                          ret: IrType::Ptr,  runtime_name: "slang_node_graph_evaluate".into() },

        // ── v18: Networking ───────────────────────────────────────────
        BuiltinFn { name: "http_get".into(),      params: vec![("url", IrType::Ptr)],                                                  ret: IrType::Ptr,  runtime_name: "slang_http_get".into() },
        BuiltinFn { name: "http_post".into(),     params: vec![("url", IrType::Ptr), ("body", IrType::Ptr)],                           ret: IrType::Ptr,  runtime_name: "slang_http_post".into() },
        BuiltinFn { name: "http_status".into(),   params: vec![("url", IrType::Ptr)],                                                  ret: IrType::I64,  runtime_name: "slang_http_status".into() },
        BuiltinFn { name: "tcp_connect".into(),   params: vec![("host", IrType::Ptr), ("port", IrType::I64)],                          ret: IrType::I64,  runtime_name: "slang_tcp_connect".into() },
        BuiltinFn { name: "tcp_send".into(),      params: vec![("handle", IrType::I64), ("data", IrType::Ptr)],                        ret: IrType::I64,  runtime_name: "slang_tcp_send".into() },
        BuiltinFn { name: "tcp_close".into(),     params: vec![("handle", IrType::I64)],                                               ret: IrType::Void, runtime_name: "slang_tcp_close".into() },

        // ── v29: Profiler & PGO ──────────────────────────────────────────────
        BuiltinFn { name: "profiler_start".into(),        params: vec![("name", IrType::Ptr)],                                           ret: IrType::Void, runtime_name: "slang_profiler_start".into() },
        BuiltinFn { name: "profiler_stop".into(),         params: vec![("name", IrType::Ptr)],                                           ret: IrType::I64,  runtime_name: "slang_profiler_stop".into() },
        BuiltinFn { name: "profiler_report".into(),       params: vec![],                                                                 ret: IrType::Ptr,  runtime_name: "slang_profiler_report".into() },
        BuiltinFn { name: "profiler_flamegraph".into(),   params: vec![],                                                                 ret: IrType::Ptr,  runtime_name: "slang_profiler_flamegraph".into() },
        BuiltinFn { name: "profiler_hotpath".into(),      params: vec![("threshold", IrType::F64)],                                      ret: IrType::Ptr,  runtime_name: "slang_profiler_hotpath".into() },

        // ── v29: Memory Pools ────────────────────────────────────────────────
        BuiltinFn { name: "arena_create".into(),          params: vec![("capacity", IrType::I64)],                                       ret: IrType::I64,  runtime_name: "slang_arena_create".into() },
        BuiltinFn { name: "arena_alloc".into(),           params: vec![("arena", IrType::I64), ("size", IrType::I64)],                   ret: IrType::I64,  runtime_name: "slang_arena_alloc".into() },
        BuiltinFn { name: "arena_reset".into(),           params: vec![("arena", IrType::I64)],                                          ret: IrType::Void, runtime_name: "slang_arena_reset".into() },
        BuiltinFn { name: "pool_create".into(),           params: vec![("block_size", IrType::I64), ("count", IrType::I64)],             ret: IrType::I64,  runtime_name: "slang_pool_create".into() },
        BuiltinFn { name: "pool_alloc".into(),            params: vec![("pool", IrType::I64)],                                           ret: IrType::I64,  runtime_name: "slang_pool_alloc".into() },
        BuiltinFn { name: "pool_free".into(),             params: vec![("pool", IrType::I64), ("ptr", IrType::I64)],                     ret: IrType::Void, runtime_name: "slang_pool_free".into() },

        // ── v29: FFI Bindgen ─────────────────────────────────────────────────
        BuiltinFn { name: "ffi_type_size".into(),         params: vec![("type_name", IrType::Ptr)],                                      ret: IrType::I64,  runtime_name: "slang_ffi_type_size".into() },
        BuiltinFn { name: "ffi_type_align".into(),        params: vec![("type_name", IrType::Ptr)],                                      ret: IrType::I64,  runtime_name: "slang_ffi_type_align".into() },
        BuiltinFn { name: "ffi_gen_header".into(),        params: vec![("module", IrType::Ptr)],                                         ret: IrType::Ptr,  runtime_name: "slang_ffi_gen_header".into() },
        BuiltinFn { name: "ffi_gen_typescript".into(),    params: vec![("module", IrType::Ptr)],                                         ret: IrType::Ptr,  runtime_name: "slang_ffi_gen_typescript".into() },

        // ── v29: Type Classes ────────────────────────────────────────────────
        BuiltinFn { name: "kind_check".into(),            params: vec![("type_expr", IrType::Ptr)],                                      ret: IrType::Ptr,  runtime_name: "slang_kind_check".into() },
        BuiltinFn { name: "resolve_instance".into(),      params: vec![("class", IrType::Ptr), ("type_arg", IrType::Ptr)],               ret: IrType::Ptr,  runtime_name: "slang_resolve_instance".into() },

        // ── v29: Build System ────────────────────────────────────────────────
        BuiltinFn { name: "build_graph_create".into(),    params: vec![],                                                                 ret: IrType::I64,  runtime_name: "slang_build_graph_create".into() },
        BuiltinFn { name: "build_add_unit".into(),        params: vec![("graph", IrType::I64), ("name", IrType::Ptr)],                   ret: IrType::I64,  runtime_name: "slang_build_add_unit".into() },
        BuiltinFn { name: "build_add_dep".into(),         params: vec![("graph", IrType::I64), ("from", IrType::I64), ("to", IrType::I64)], ret: IrType::I64, runtime_name: "slang_build_add_dep".into() },
        BuiltinFn { name: "build_topo_sort".into(),       params: vec![("graph", IrType::I64)],                                          ret: IrType::Ptr,  runtime_name: "slang_build_topo_sort".into() },
        BuiltinFn { name: "content_hash".into(),          params: vec![("data", IrType::Ptr)],                                           ret: IrType::Ptr,  runtime_name: "slang_content_hash".into() },

        // ── v29: Benchmarks ──────────────────────────────────────────────────
        BuiltinFn { name: "bench_mean".into(),            params: vec![("data", IrType::Ptr)],                                           ret: IrType::F64,  runtime_name: "slang_bench_mean".into() },
        BuiltinFn { name: "bench_median".into(),          params: vec![("data", IrType::Ptr)],                                           ret: IrType::F64,  runtime_name: "slang_bench_median".into() },
        BuiltinFn { name: "bench_stddev".into(),          params: vec![("data", IrType::Ptr)],                                           ret: IrType::F64,  runtime_name: "slang_bench_stddev".into() },
        BuiltinFn { name: "bench_percentile".into(),      params: vec![("data", IrType::Ptr), ("p", IrType::F64)],                       ret: IrType::F64,  runtime_name: "slang_bench_percentile".into() },
        BuiltinFn { name: "bench_ci95".into(),            params: vec![("data", IrType::Ptr)],                                           ret: IrType::Ptr,  runtime_name: "slang_bench_ci95".into() },
        BuiltinFn { name: "bench_regression".into(),      params: vec![("old", IrType::Ptr), ("new", IrType::Ptr)],                      ret: IrType::Ptr,  runtime_name: "slang_bench_regression".into() },

        // ── v30: Regex Engine ────────────────────────────────────────────────
        BuiltinFn { name: "regex_match".into(),           params: vec![("pattern", IrType::Ptr), ("text", IrType::Ptr)],                 ret: IrType::I64,  runtime_name: "vitalis_regex_is_match".into() },
        BuiltinFn { name: "regex_find".into(),            params: vec![("pattern", IrType::Ptr), ("text", IrType::Ptr)],                 ret: IrType::Ptr,  runtime_name: "vitalis_regex_find_first".into() },
        BuiltinFn { name: "regex_find_all".into(),        params: vec![("pattern", IrType::Ptr), ("text", IrType::Ptr)],                 ret: IrType::Ptr,  runtime_name: "vitalis_regex_find_all_matches".into() },
        BuiltinFn { name: "regex_captures".into(),        params: vec![("pattern", IrType::Ptr), ("text", IrType::Ptr)],                 ret: IrType::Ptr,  runtime_name: "vitalis_regex_captures_first".into() },
        BuiltinFn { name: "regex_replace".into(),         params: vec![("pattern", IrType::Ptr), ("text", IrType::Ptr), ("rep", IrType::Ptr)], ret: IrType::Ptr, runtime_name: "vitalis_regex_replace_first".into() },
        BuiltinFn { name: "regex_replace_all".into(),     params: vec![("pattern", IrType::Ptr), ("text", IrType::Ptr), ("rep", IrType::Ptr)], ret: IrType::Ptr, runtime_name: "vitalis_regex_replace_all_matches".into() },
        BuiltinFn { name: "regex_split".into(),           params: vec![("pattern", IrType::Ptr), ("text", IrType::Ptr)],                 ret: IrType::Ptr,  runtime_name: "vitalis_regex_split_by".into() },

        // ── v30: Serialization ───────────────────────────────────────────────
        BuiltinFn { name: "json_parse".into(),            params: vec![("json", IrType::Ptr)],                                           ret: IrType::Ptr,  runtime_name: "vitalis_json_parse".into() },
        BuiltinFn { name: "json_stringify".into(),        params: vec![("json", IrType::Ptr)],                                           ret: IrType::Ptr,  runtime_name: "vitalis_json_stringify".into() },
        BuiltinFn { name: "json_get".into(),              params: vec![("json", IrType::Ptr), ("path", IrType::Ptr)],                    ret: IrType::Ptr,  runtime_name: "vitalis_json_get".into() },
        BuiltinFn { name: "base64_encode".into(),         params: vec![("data", IrType::Ptr)],                                           ret: IrType::Ptr,  runtime_name: "vitalis_ser_base64_encode".into() },
        BuiltinFn { name: "base64_decode".into(),         params: vec![("data", IrType::Ptr)],                                           ret: IrType::Ptr,  runtime_name: "vitalis_ser_base64_decode".into() },
        BuiltinFn { name: "hex_encode".into(),            params: vec![("data", IrType::Ptr)],                                           ret: IrType::Ptr,  runtime_name: "vitalis_ser_hex_encode".into() },
        BuiltinFn { name: "hex_decode".into(),            params: vec![("data", IrType::Ptr)],                                           ret: IrType::Ptr,  runtime_name: "vitalis_ser_hex_decode".into() },
        BuiltinFn { name: "url_encode".into(),            params: vec![("data", IrType::Ptr)],                                           ret: IrType::Ptr,  runtime_name: "vitalis_ser_url_encode".into() },
        BuiltinFn { name: "url_decode".into(),            params: vec![("data", IrType::Ptr)],                                           ret: IrType::Ptr,  runtime_name: "vitalis_ser_url_decode".into() },

        // ── v30: Property Testing ────────────────────────────────────────────
        BuiltinFn { name: "qc_gen_i64".into(),            params: vec![("seed", IrType::I64)],                                           ret: IrType::I64,  runtime_name: "vitalis_qc_gen_i64".into() },
        BuiltinFn { name: "qc_gen_f64".into(),            params: vec![("seed", IrType::I64)],                                           ret: IrType::F64,  runtime_name: "vitalis_qc_gen_f64".into() },
        BuiltinFn { name: "qc_gen_bool".into(),           params: vec![("seed", IrType::I64)],                                           ret: IrType::I64,  runtime_name: "vitalis_qc_gen_bool".into() },
        BuiltinFn { name: "qc_shrink_i64".into(),         params: vec![("val", IrType::I64)],                                            ret: IrType::I64,  runtime_name: "vitalis_qc_shrink_i64".into() },
        BuiltinFn { name: "qc_test_commutative".into(),   params: vec![("seed", IrType::I64), ("count", IrType::I64)],                  ret: IrType::I64,  runtime_name: "vitalis_qc_test_commutative_add".into() },
        BuiltinFn { name: "qc_test_sort_idempotent".into(), params: vec![("seed", IrType::I64), ("count", IrType::I64)],                ret: IrType::I64,  runtime_name: "vitalis_qc_test_sort_idempotent".into() },
        BuiltinFn { name: "qc_chi_squared".into(),        params: vec![("seed", IrType::I64), ("count", IrType::I64)],                  ret: IrType::F64,  runtime_name: "vitalis_qc_chi_squared".into() },

        // ── v30: Data Structures ─────────────────────────────────────────────
        BuiltinFn { name: "btree_create".into(),          params: vec![("min_degree", IrType::I64)],                                     ret: IrType::I64,  runtime_name: "vitalis_btree_create".into() },
        BuiltinFn { name: "btree_insert".into(),          params: vec![("tree", IrType::I64), ("key", IrType::I64)],                     ret: IrType::I64,  runtime_name: "vitalis_btree_insert".into() },
        BuiltinFn { name: "btree_search".into(),          params: vec![("tree", IrType::I64), ("key", IrType::I64)],                     ret: IrType::I64,  runtime_name: "vitalis_btree_search".into() },
        BuiltinFn { name: "btree_len".into(),             params: vec![("tree", IrType::I64)],                                           ret: IrType::I64,  runtime_name: "vitalis_btree_len".into() },
        BuiltinFn { name: "ringbuf_create".into(),        params: vec![("capacity", IrType::I64)],                                       ret: IrType::I64,  runtime_name: "vitalis_ringbuf_create".into() },
        BuiltinFn { name: "ringbuf_push".into(),          params: vec![("buf", IrType::I64), ("val", IrType::I64)],                      ret: IrType::I64,  runtime_name: "vitalis_ringbuf_push_back".into() },
        BuiltinFn { name: "ringbuf_pop".into(),           params: vec![("buf", IrType::I64)],                                            ret: IrType::I64,  runtime_name: "vitalis_ringbuf_pop_front".into() },
        BuiltinFn { name: "uf_create".into(),             params: vec![("n", IrType::I64)],                                              ret: IrType::I64,  runtime_name: "vitalis_uf_create".into() },
        BuiltinFn { name: "uf_union".into(),              params: vec![("uf", IrType::I64), ("a", IrType::I64), ("b", IrType::I64)],     ret: IrType::I64,  runtime_name: "vitalis_uf_union".into() },
        BuiltinFn { name: "uf_find".into(),               params: vec![("uf", IrType::I64), ("x", IrType::I64)],                         ret: IrType::I64,  runtime_name: "vitalis_uf_find".into() },
        BuiltinFn { name: "uf_connected".into(),          params: vec![("uf", IrType::I64), ("a", IrType::I64), ("b", IrType::I64)],     ret: IrType::I64,  runtime_name: "vitalis_uf_connected".into() },
        BuiltinFn { name: "lru_create".into(),            params: vec![("capacity", IrType::I64)],                                       ret: IrType::I64,  runtime_name: "vitalis_lru_create".into() },
        BuiltinFn { name: "lru_put".into(),               params: vec![("cache", IrType::I64), ("key", IrType::I64), ("val", IrType::I64)], ret: IrType::I64, runtime_name: "vitalis_lru_put".into() },
        BuiltinFn { name: "lru_get".into(),               params: vec![("cache", IrType::I64), ("key", IrType::I64)],                    ret: IrType::I64,  runtime_name: "vitalis_lru_get".into() },

        // ── v30: Networking ──────────────────────────────────────────────────
        BuiltinFn { name: "url_parse".into(),             params: vec![("url", IrType::Ptr)],                                            ret: IrType::Ptr,  runtime_name: "vitalis_url_parse".into() },
        BuiltinFn { name: "http_build_request".into(),    params: vec![("method", IrType::Ptr), ("path", IrType::Ptr), ("host", IrType::Ptr)], ret: IrType::Ptr, runtime_name: "vitalis_http_build_request".into() },
        BuiltinFn { name: "http_parse_request".into(),    params: vec![("raw", IrType::Ptr)],                                            ret: IrType::Ptr,  runtime_name: "vitalis_http_parse_request".into() },
        BuiltinFn { name: "is_valid_ipv4".into(),         params: vec![("addr", IrType::Ptr)],                                           ret: IrType::I64,  runtime_name: "vitalis_is_valid_ipv4".into() },
        BuiltinFn { name: "is_valid_ipv6".into(),         params: vec![("addr", IrType::Ptr)],                                           ret: IrType::I64,  runtime_name: "vitalis_is_valid_ipv6".into() },
        BuiltinFn { name: "parse_query_string".into(),    params: vec![("query", IrType::Ptr)],                                          ret: IrType::Ptr,  runtime_name: "vitalis_parse_query_string".into() },
        BuiltinFn { name: "dns_build_query".into(),       params: vec![("name", IrType::Ptr), ("type", IrType::I64)],                    ret: IrType::Ptr,  runtime_name: "vitalis_dns_build_query".into() },

        // ── v30: ECS ─────────────────────────────────────────────────────────
        BuiltinFn { name: "ecs_world_create".into(),      params: vec![],                                                                 ret: IrType::I64,  runtime_name: "vitalis_ecs_world_create".into() },
        BuiltinFn { name: "ecs_spawn".into(),             params: vec![("world", IrType::I64)],                                          ret: IrType::I64,  runtime_name: "vitalis_ecs_spawn".into() },
        BuiltinFn { name: "ecs_despawn".into(),           params: vec![("world", IrType::I64), ("entity", IrType::I64)],                 ret: IrType::I64,  runtime_name: "vitalis_ecs_despawn".into() },
        BuiltinFn { name: "ecs_add_component".into(),     params: vec![("world", IrType::I64), ("entity", IrType::I64), ("type", IrType::I64), ("val", IrType::I64)], ret: IrType::I64, runtime_name: "vitalis_ecs_add_component".into() },
        BuiltinFn { name: "ecs_get_component".into(),     params: vec![("world", IrType::I64), ("entity", IrType::I64), ("type", IrType::I64)], ret: IrType::I64, runtime_name: "vitalis_ecs_get_component".into() },
        BuiltinFn { name: "ecs_has_component".into(),     params: vec![("world", IrType::I64), ("entity", IrType::I64), ("type", IrType::I64)], ret: IrType::I64, runtime_name: "vitalis_ecs_has_component".into() },
        BuiltinFn { name: "ecs_entity_count".into(),      params: vec![("world", IrType::I64)],                                          ret: IrType::I64,  runtime_name: "vitalis_ecs_entity_count".into() },

        // ── v31: Tensor Engine ───────────────────────────────────────────────
        BuiltinFn { name: "tensor_zeros".into(),          params: vec![("rows", IrType::I64), ("cols", IrType::I64)],                    ret: IrType::I64,  runtime_name: "vitalis_tensor_zeros".into() },
        BuiltinFn { name: "tensor_ones".into(),           params: vec![("rows", IrType::I64), ("cols", IrType::I64)],                    ret: IrType::I64,  runtime_name: "vitalis_tensor_ones".into() },
        BuiltinFn { name: "tensor_rand".into(),           params: vec![("rows", IrType::I64), ("cols", IrType::I64), ("seed", IrType::I64)], ret: IrType::I64, runtime_name: "vitalis_tensor_rand".into() },
        BuiltinFn { name: "tensor_matmul".into(),         params: vec![("a", IrType::I64), ("b", IrType::I64)],                          ret: IrType::I64,  runtime_name: "vitalis_tensor_matmul".into() },
        BuiltinFn { name: "tensor_add".into(),            params: vec![("a", IrType::I64), ("b", IrType::I64)],                          ret: IrType::I64,  runtime_name: "vitalis_tensor_add".into() },
        BuiltinFn { name: "tensor_shape".into(),          params: vec![("t", IrType::I64)],                                              ret: IrType::Ptr,  runtime_name: "vitalis_tensor_shape".into() },
        BuiltinFn { name: "tensor_relu".into(),           params: vec![("t", IrType::I64)],                                              ret: IrType::I64,  runtime_name: "vitalis_tensor_relu".into() },
        BuiltinFn { name: "tensor_softmax".into(),        params: vec![("t", IrType::I64)],                                              ret: IrType::I64,  runtime_name: "vitalis_tensor_softmax".into() },
        BuiltinFn { name: "tensor_free".into(),           params: vec![("t", IrType::I64)],                                              ret: IrType::I64,  runtime_name: "vitalis_tensor_free".into() },

        // ── v115: Simplified tensor creation + ops (JIT-callable) ────────────
        BuiltinFn { name: "tensor_zeros_2d".into(),       params: vec![("rows", IrType::I64), ("cols", IrType::I64)],                    ret: IrType::I64,  runtime_name: "vitalis_tensor_zeros_2d".into() },
        BuiltinFn { name: "tensor_ones_2d".into(),        params: vec![("rows", IrType::I64), ("cols", IrType::I64)],                    ret: IrType::I64,  runtime_name: "vitalis_tensor_ones_2d".into() },
        BuiltinFn { name: "tensor_zeros_1d".into(),       params: vec![("len", IrType::I64)],                                            ret: IrType::I64,  runtime_name: "vitalis_tensor_zeros_1d".into() },
        BuiltinFn { name: "tensor_scalar".into(),         params: vec![("val", IrType::I64)],                                            ret: IrType::I64,  runtime_name: "vitalis_tensor_scalar".into() },
        BuiltinFn { name: "tensor_set".into(),            params: vec![("t", IrType::I64), ("idx", IrType::I64), ("val", IrType::I64)],  ret: IrType::Void, runtime_name: "vitalis_tensor_set".into() },
        BuiltinFn { name: "tensor_mul".into(),            params: vec![("a", IrType::I64), ("b", IrType::I64)],                          ret: IrType::I64,  runtime_name: "vitalis_tensor_mul".into() },
        BuiltinFn { name: "tensor_transpose".into(),      params: vec![("t", IrType::I64)],                                              ret: IrType::I64,  runtime_name: "vitalis_tensor_transpose".into() },
        BuiltinFn { name: "tensor_numel".into(),          params: vec![("t", IrType::I64)],                                              ret: IrType::I64,  runtime_name: "vitalis_tensor_numel".into() },
        BuiltinFn { name: "tensor_ndim".into(),           params: vec![("t", IrType::I64)],                                              ret: IrType::I64,  runtime_name: "vitalis_tensor_ndim".into() },
        BuiltinFn { name: "tensor_get".into(),            params: vec![("t", IrType::I64), ("idx", IrType::I64)],                        ret: IrType::F64,  runtime_name: "vitalis_tensor_get".into() },
        BuiltinFn { name: "tensor_sum".into(),            params: vec![("t", IrType::I64)],                                              ret: IrType::F64,  runtime_name: "vitalis_tensor_sum".into() },
        BuiltinFn { name: "tensor_mean".into(),           params: vec![("t", IrType::I64)],                                              ret: IrType::F64,  runtime_name: "vitalis_tensor_mean".into() },

        // ── v31: Autograd ────────────────────────────────────────────────────
        BuiltinFn { name: "autograd_tape_new".into(),     params: vec![],                                                                 ret: IrType::I64,  runtime_name: "vitalis_tape_new".into() },
        BuiltinFn { name: "autograd_var".into(),          params: vec![("val", IrType::F64)],                                            ret: IrType::I64,  runtime_name: "vitalis_tape_variable".into() },
        BuiltinFn { name: "autograd_backward".into(),     params: vec![("tape", IrType::I64), ("loss", IrType::I64)],                    ret: IrType::I64,  runtime_name: "vitalis_tape_backward".into() },

        // ── v116: Simplified autograd (JIT-callable) ─────────────────────────
        BuiltinFn { name: "autograd_scalar".into(),       params: vec![("val", IrType::I64)],                                            ret: IrType::I64,  runtime_name: "vitalis_autograd_scalar".into() },
        BuiltinFn { name: "autograd_value".into(),        params: vec![("id", IrType::I64)],                                             ret: IrType::F64,  runtime_name: "vitalis_autograd_value".into() },
        BuiltinFn { name: "autograd_add".into(),          params: vec![("a", IrType::I64), ("b", IrType::I64)],                          ret: IrType::I64,  runtime_name: "vitalis_autograd_add".into() },
        BuiltinFn { name: "autograd_mul".into(),          params: vec![("a", IrType::I64), ("b", IrType::I64)],                          ret: IrType::I64,  runtime_name: "vitalis_autograd_mul".into() },
        BuiltinFn { name: "autograd_sum".into(),          params: vec![("id", IrType::I64)],                                             ret: IrType::I64,  runtime_name: "vitalis_autograd_sum".into() },
        BuiltinFn { name: "autograd_grad_scalar".into(),  params: vec![("id", IrType::I64)],                                             ret: IrType::F64,  runtime_name: "vitalis_autograd_grad_scalar".into() },
        BuiltinFn { name: "autograd_numel".into(),        params: vec![("id", IrType::I64)],                                             ret: IrType::I64,  runtime_name: "vitalis_autograd_numel".into() },
        BuiltinFn { name: "autograd_clear".into(),        params: vec![],                                                                 ret: IrType::Void, runtime_name: "vitalis_autograd_clear".into() },
        BuiltinFn { name: "autograd_no_grad".into(),      params: vec![("enabled", IrType::I64)],                                        ret: IrType::Void, runtime_name: "vitalis_autograd_no_grad".into() },

        // ── v114: GPU Compute ────────────────────────────────────────────────
        BuiltinFn { name: "gpu_pipeline_new".into(),      params: vec![],                                                                 ret: IrType::I64,  runtime_name: "vitalis_gpu_pipeline_new".into() },
        BuiltinFn { name: "gpu_add_kernel".into(),        params: vec![("pipe", IrType::I64), ("kind", IrType::I64), ("size", IrType::I64)], ret: IrType::I64, runtime_name: "vitalis_gpu_add_kernel".into() },
        BuiltinFn { name: "gpu_create_buffer".into(),     params: vec![("pipe", IrType::I64), ("count", IrType::I64)],                    ret: IrType::I64,  runtime_name: "vitalis_gpu_create_buffer".into() },
        BuiltinFn { name: "gpu_dispatch".into(),          params: vec![("pipe", IrType::I64)],                                            ret: IrType::I64,  runtime_name: "vitalis_gpu_dispatch".into() },
        BuiltinFn { name: "gpu_buffer_count".into(),      params: vec![("pipe", IrType::I64)],                                            ret: IrType::I64,  runtime_name: "vitalis_gpu_buffer_count".into() },
        BuiltinFn { name: "gpu_kernel_count".into(),      params: vec![("pipe", IrType::I64)],                                            ret: IrType::I64,  runtime_name: "vitalis_gpu_kernel_count".into() },
        BuiltinFn { name: "gpu_pipeline_free".into(),     params: vec![("pipe", IrType::I64)],                                            ret: IrType::Void, runtime_name: "vitalis_gpu_pipeline_free".into() },

        // ── v32: Training Engine ─────────────────────────────────────────────
        BuiltinFn { name: "train_adamw_step".into(),      params: vec![("opt", IrType::I64), ("grad", IrType::Ptr), ("n", IrType::I64)], ret: IrType::I64,  runtime_name: "vitalis_train_adamw_step".into() },
        BuiltinFn { name: "train_cross_entropy".into(),   params: vec![("logits", IrType::Ptr), ("target", IrType::I64), ("n", IrType::I64)], ret: IrType::F64, runtime_name: "vitalis_train_cross_entropy".into() },
        BuiltinFn { name: "train_mse".into(),             params: vec![("pred", IrType::Ptr), ("targ", IrType::Ptr), ("n", IrType::I64)], ret: IrType::F64, runtime_name: "vitalis_train_mse".into() },
        BuiltinFn { name: "train_grad_norm".into(),       params: vec![("grads", IrType::Ptr), ("n", IrType::I64)],                      ret: IrType::F64,  runtime_name: "vitalis_train_grad_norm".into() },

        // ── v33: Transformer ─────────────────────────────────────────────────
        BuiltinFn { name: "attention_sdpa".into(),        params: vec![("q", IrType::Ptr), ("k", IrType::Ptr), ("v", IrType::Ptr), ("n", IrType::I64), ("dk", IrType::I64)], ret: IrType::Ptr, runtime_name: "vitalis_attention_sdpa".into() },
        BuiltinFn { name: "flash_attention".into(),       params: vec![("q", IrType::Ptr), ("k", IrType::Ptr), ("v", IrType::Ptr), ("n", IrType::I64), ("dk", IrType::I64)], ret: IrType::Ptr, runtime_name: "vitalis_flash_attention".into() },

        // ── v34: Inference & Quantization ────────────────────────────────────
        BuiltinFn { name: "inference_argmax".into(),      params: vec![("logits", IrType::Ptr), ("n", IrType::I64)],                     ret: IrType::I64,  runtime_name: "vitalis_inference_argmax".into() },
        BuiltinFn { name: "inference_top_k".into(),       params: vec![("logits", IrType::Ptr), ("n", IrType::I64), ("k", IrType::I64)], ret: IrType::I64, runtime_name: "vitalis_inference_apply_top_k".into() },
        BuiltinFn { name: "inference_top_p".into(),       params: vec![("logits", IrType::Ptr), ("n", IrType::I64), ("p", IrType::F64)], ret: IrType::I64, runtime_name: "vitalis_inference_apply_top_p".into() },
        BuiltinFn { name: "quantize_int8".into(),         params: vec![("data", IrType::Ptr), ("n", IrType::I64)],                       ret: IrType::I64,  runtime_name: "vitalis_quantize_int8".into() },
        BuiltinFn { name: "quantize_int4".into(),         params: vec![("data", IrType::Ptr), ("n", IrType::I64)],                       ret: IrType::I64,  runtime_name: "vitalis_quantize_int4".into() },
        BuiltinFn { name: "quantize_error".into(),        params: vec![("data", IrType::Ptr), ("n", IrType::I64)],                       ret: IrType::F64,  runtime_name: "vitalis_quantize_error".into() },
        BuiltinFn { name: "lora_create".into(),           params: vec![("in_dim", IrType::I64), ("out_dim", IrType::I64), ("rank", IrType::I64)], ret: IrType::I64, runtime_name: "vitalis_lora_create".into() },
        BuiltinFn { name: "lora_params".into(),           params: vec![("id", IrType::I64)],                                             ret: IrType::I64,  runtime_name: "vitalis_lora_params".into() },

        // ── v35: Code Intelligence & Program Synthesis ───────────────────────
        BuiltinFn { name: "code_cyclomatic".into(),       params: vec![("code", IrType::Ptr)],                                           ret: IrType::I64,  runtime_name: "vitalis_code_cyclomatic".into() },
        BuiltinFn { name: "code_cognitive".into(),        params: vec![("code", IrType::Ptr)],                                           ret: IrType::I64,  runtime_name: "vitalis_code_cognitive".into() },
        BuiltinFn { name: "code_maintainability".into(),  params: vec![("code", IrType::Ptr)],                                           ret: IrType::F64,  runtime_name: "vitalis_code_maintainability".into() },
        BuiltinFn { name: "synth_complexity".into(),      params: vec![("expr", IrType::Ptr)],                                           ret: IrType::I64,  runtime_name: "vitalis_synth_complexity".into() },

        // ── v35: Self-Optimizer ──────────────────────────────────────────────
        BuiltinFn { name: "selfopt_num_passes".into(),    params: vec![],                                                                 ret: IrType::I64,  runtime_name: "vitalis_selfopt_num_passes".into() },
        BuiltinFn { name: "selfopt_tier".into(),          params: vec![("call_count", IrType::I64)],                                     ret: IrType::I64,  runtime_name: "vitalis_selfopt_tier".into() },

        // ── v36: Autonomous Agents ───────────────────────────────────────────
        BuiltinFn { name: "agent_create".into(),          params: vec![("name", IrType::Ptr), ("len", IrType::I64)],                     ret: IrType::I64,  runtime_name: "vitalis_agent_create".into() },
        BuiltinFn { name: "agent_success_rate".into(),    params: vec![("id", IrType::I64)],                                             ret: IrType::F64,  runtime_name: "vitalis_agent_success_rate".into() },
        BuiltinFn { name: "agent_total_actions".into(),   params: vec![("id", IrType::I64)],                                             ret: IrType::I64,  runtime_name: "vitalis_agent_total_actions".into() },
        BuiltinFn { name: "agent_free".into(),            params: vec![("id", IrType::I64)],                                             ret: IrType::I64,  runtime_name: "vitalis_agent_free".into() },

        // ── v36: Reward Model (RLHF) ────────────────────────────────────────
        BuiltinFn { name: "reward_create".into(),         params: vec![("dim", IrType::I64), ("lr", IrType::F64)],                       ret: IrType::I64,  runtime_name: "vitalis_reward_create".into() },
        BuiltinFn { name: "reward_score".into(),          params: vec![("id", IrType::I64), ("features", IrType::Ptr), ("n", IrType::I64)], ret: IrType::F64, runtime_name: "vitalis_reward_score".into() },
        BuiltinFn { name: "reward_free".into(),           params: vec![("id", IrType::I64)],                                             ret: IrType::I64,  runtime_name: "vitalis_reward_free".into() },

        // ── v37: Differentiable Programming ──────────────────────────────────
        BuiltinFn { name: "dual_new".into(),               params: vec![("val", IrType::F64), ("dot", IrType::F64)],                    ret: IrType::I64,  runtime_name: "vitalis_dual_new".into() },
        BuiltinFn { name: "dual_mul".into(),               params: vec![("a", IrType::I64), ("b", IrType::I64)],                       ret: IrType::I64,  runtime_name: "vitalis_dual_mul".into() },
        BuiltinFn { name: "forward_deriv".into(),          params: vec![("x", IrType::F64)],                                           ret: IrType::F64,  runtime_name: "vitalis_forward_deriv".into() },
        BuiltinFn { name: "shape_broadcast_ok".into(),     params: vec![("a", IrType::I64), ("b", IrType::I64)],                       ret: IrType::I64,  runtime_name: "vitalis_shape_broadcast_ok".into() },

        // ── v37: Probabilistic Programming ───────────────────────────────────
        BuiltinFn { name: "prob_normal".into(),            params: vec![("mean", IrType::F64), ("std", IrType::F64)],                  ret: IrType::I64,  runtime_name: "vitalis_prob_normal".into() },
        BuiltinFn { name: "prob_log_prob".into(),          params: vec![("id", IrType::I64), ("x", IrType::F64)],                      ret: IrType::F64,  runtime_name: "vitalis_prob_log_prob".into() },
        BuiltinFn { name: "prob_sample".into(),            params: vec![("id", IrType::I64), ("seed", IrType::I64)],                   ret: IrType::F64,  runtime_name: "vitalis_prob_sample".into() },
        BuiltinFn { name: "prob_free".into(),              params: vec![("id", IrType::I64)],                                          ret: IrType::I64,  runtime_name: "vitalis_prob_free".into() },
        BuiltinFn { name: "mcmc_normal_mean".into(),       params: vec![("observed", IrType::Ptr), ("n", IrType::I64)],                ret: IrType::F64,  runtime_name: "vitalis_mcmc_normal_mean".into() },

        // ── v38: Reinforcement Learning ──────────────────────────────────────
        BuiltinFn { name: "rl_create".into(),              params: vec![("n_states", IrType::I64), ("n_actions", IrType::I64)],        ret: IrType::I64,  runtime_name: "vitalis_rl_create".into() },
        BuiltinFn { name: "rl_get_q".into(),               params: vec![("id", IrType::I64), ("state", IrType::I64), ("action", IrType::I64)], ret: IrType::F64, runtime_name: "vitalis_rl_get_q".into() },
        BuiltinFn { name: "rl_update".into(),              params: vec![("id", IrType::I64), ("s", IrType::I64), ("a", IrType::I64), ("r", IrType::F64), ("sp", IrType::I64)], ret: IrType::I64, runtime_name: "vitalis_rl_update".into() },
        BuiltinFn { name: "rl_free".into(),                params: vec![("id", IrType::I64)],                                          ret: IrType::I64,  runtime_name: "vitalis_rl_free".into() },

        // ── v38: Simulation Environments ─────────────────────────────────────
        BuiltinFn { name: "sim_grid_new".into(),           params: vec![("w", IrType::I64), ("h", IrType::I64)],                       ret: IrType::I64,  runtime_name: "vitalis_sim_grid_new".into() },
        BuiltinFn { name: "sim_grid_step".into(),          params: vec![("id", IrType::I64), ("action", IrType::I64)],                 ret: IrType::F64,  runtime_name: "vitalis_sim_grid_step".into() },
        BuiltinFn { name: "sim_grid_reset".into(),         params: vec![("id", IrType::I64)],                                          ret: IrType::I64,  runtime_name: "vitalis_sim_grid_reset".into() },
        BuiltinFn { name: "sim_free".into(),               params: vec![("id", IrType::I64)],                                          ret: IrType::I64,  runtime_name: "vitalis_sim_free".into() },

        // ── v39: Data Pipeline ───────────────────────────────────────────────
        BuiltinFn { name: "data_create".into(),            params: vec![("n_samples", IrType::I64), ("n_features", IrType::I64)],      ret: IrType::I64,  runtime_name: "vitalis_data_create".into() },
        BuiltinFn { name: "data_len".into(),               params: vec![("id", IrType::I64)],                                          ret: IrType::I64,  runtime_name: "vitalis_data_len".into() },
        BuiltinFn { name: "data_parse_csv".into(),         params: vec![("csv_ptr", IrType::Ptr)],                                     ret: IrType::I64,  runtime_name: "vitalis_data_parse_csv".into() },
        BuiltinFn { name: "data_free".into(),              params: vec![("id", IrType::I64)],                                          ret: IrType::I64,  runtime_name: "vitalis_data_free".into() },

        // ── v39: Experiment Tracking ─────────────────────────────────────────
        BuiltinFn { name: "exp_create".into(),             params: vec![("name_ptr", IrType::Ptr)],                                    ret: IrType::I64,  runtime_name: "vitalis_exp_create".into() },
        BuiltinFn { name: "exp_log_metric".into(),         params: vec![("id", IrType::I64), ("name_ptr", IrType::Ptr), ("val", IrType::F64)], ret: IrType::I64, runtime_name: "vitalis_exp_log_metric".into() },
        BuiltinFn { name: "exp_get_metric".into(),         params: vec![("id", IrType::I64), ("name_ptr", IrType::Ptr)],               ret: IrType::F64,  runtime_name: "vitalis_exp_get_metric".into() },
        BuiltinFn { name: "exp_complete".into(),           params: vec![("id", IrType::I64)],                                          ret: IrType::I64,  runtime_name: "vitalis_exp_complete".into() },
        BuiltinFn { name: "exp_free".into(),               params: vec![("id", IrType::I64)],                                          ret: IrType::I64,  runtime_name: "vitalis_exp_free".into() },

        // ── v40: Model Serving ───────────────────────────────────────────────
        BuiltinFn { name: "serve_create".into(),           params: vec![("name_ptr", IrType::Ptr)],                                    ret: IrType::I64,  runtime_name: "vitalis_serve_create".into() },
        BuiltinFn { name: "serve_load_model".into(),       params: vec![("id", IrType::I64), ("name_ptr", IrType::Ptr)],               ret: IrType::I64,  runtime_name: "vitalis_serve_load_model".into() },
        BuiltinFn { name: "serve_predict".into(),          params: vec![("id", IrType::I64), ("features", IrType::Ptr), ("n", IrType::I64)], ret: IrType::F64, runtime_name: "vitalis_serve_predict".into() },
        BuiltinFn { name: "serve_free".into(),             params: vec![("id", IrType::I64)],                                          ret: IrType::I64,  runtime_name: "vitalis_serve_free".into() },

        // ── v40: AI Observability ────────────────────────────────────────────
        BuiltinFn { name: "obs_create_drift".into(),       params: vec![("n_features", IrType::I64), ("threshold", IrType::F64)],      ret: IrType::I64,  runtime_name: "vitalis_obs_create_drift".into() },
        BuiltinFn { name: "obs_check_drift".into(),        params: vec![("id", IrType::I64), ("data_ptr", IrType::Ptr), ("n_samples", IrType::I64), ("n_features", IrType::I64)], ret: IrType::F64, runtime_name: "vitalis_obs_check_drift".into() },
        BuiltinFn { name: "obs_free".into(),               params: vec![("id", IrType::I64)],                                          ret: IrType::I64,  runtime_name: "vitalis_obs_free".into() },

        // ── v41: WASM AOT ───────────────────────────────────────────────────
        BuiltinFn { name: "wasm_aot_create".into(),        params: vec![("name", IrType::Ptr)],                                        ret: IrType::I64,  runtime_name: "vitalis_wasm_aot_create".into() },
        BuiltinFn { name: "wasm_aot_size".into(),          params: vec![("id", IrType::I64)],                                          ret: IrType::I64,  runtime_name: "vitalis_wasm_aot_size".into() },
        BuiltinFn { name: "wasm_aot_exports".into(),       params: vec![("id", IrType::I64)],                                          ret: IrType::I64,  runtime_name: "vitalis_wasm_aot_exports".into() },
        BuiltinFn { name: "wasm_aot_free".into(),          params: vec![("id", IrType::I64)],                                          ret: IrType::I64,  runtime_name: "vitalis_wasm_aot_free".into() },

        // ── v42: Distributed Build ──────────────────────────────────────────
        BuiltinFn { name: "distbuild_create".into(),       params: vec![],                                                             ret: IrType::I64,  runtime_name: "vitalis_distbuild_create".into() },
        BuiltinFn { name: "distbuild_add_node".into(),     params: vec![("id", IrType::I64), ("name", IrType::Ptr), ("cores", IrType::I64)], ret: IrType::I64, runtime_name: "vitalis_distbuild_add_node".into() },
        BuiltinFn { name: "distbuild_utilization".into(),  params: vec![("id", IrType::I64)],                                          ret: IrType::F64,  runtime_name: "vitalis_distbuild_utilization".into() },
        BuiltinFn { name: "distbuild_free".into(),         params: vec![("id", IrType::I64)],                                          ret: IrType::I64,  runtime_name: "vitalis_distbuild_free".into() },

        // ── v43: Formal Verification ────────────────────────────────────────
        BuiltinFn { name: "verify_create".into(),          params: vec![],                                                             ret: IrType::I64,  runtime_name: "vitalis_verify_create".into() },
        BuiltinFn { name: "verify_paths".into(),           params: vec![("id", IrType::I64)],                                          ret: IrType::I64,  runtime_name: "vitalis_verify_paths".into() },
        BuiltinFn { name: "verify_errors".into(),          params: vec![("id", IrType::I64)],                                          ret: IrType::I64,  runtime_name: "vitalis_verify_errors".into() },
        BuiltinFn { name: "verify_free".into(),            params: vec![("id", IrType::I64)],                                          ret: IrType::I64,  runtime_name: "vitalis_verify_free".into() },

        // ── v43: IDE Features ───────────────────────────────────────────────
        BuiltinFn { name: "ide_create".into(),             params: vec![],                                                             ret: IrType::I64,  runtime_name: "vitalis_ide_create".into() },
        BuiltinFn { name: "ide_history_len".into(),        params: vec![("id", IrType::I64)],                                          ret: IrType::I64,  runtime_name: "vitalis_ide_history_len".into() },
        BuiltinFn { name: "ide_free".into(),               params: vec![("id", IrType::I64)],                                          ret: IrType::I64,  runtime_name: "vitalis_ide_free".into() },

        // ── v44: Neural Architecture Search ─────────────────────────────────
        BuiltinFn { name: "nas_create".into(),             params: vec![("pop_size", IrType::I64), ("seed", IrType::I64)],             ret: IrType::I64,  runtime_name: "vitalis_nas_create".into() },
        BuiltinFn { name: "nas_generation".into(),         params: vec![("id", IrType::I64)],                                          ret: IrType::I64,  runtime_name: "vitalis_nas_generation".into() },
        BuiltinFn { name: "nas_best_fitness".into(),       params: vec![("id", IrType::I64)],                                          ret: IrType::F64,  runtime_name: "vitalis_nas_best_fitness".into() },
        BuiltinFn { name: "nas_pop_size".into(),           params: vec![("id", IrType::I64)],                                          ret: IrType::I64,  runtime_name: "vitalis_nas_pop_size".into() },
        BuiltinFn { name: "nas_free".into(),               params: vec![("id", IrType::I64)],                                          ret: IrType::I64,  runtime_name: "vitalis_nas_free".into() },

        // ── v44: Continual Learning ─────────────────────────────────────────
        BuiltinFn { name: "cl_create_ewc".into(),          params: vec![("lambda", IrType::F64)],                                       ret: IrType::I64,  runtime_name: "vitalis_cl_create_ewc".into() },
        BuiltinFn { name: "cl_create_replay".into(),       params: vec![("buffer_size", IrType::I64)],                                  ret: IrType::I64,  runtime_name: "vitalis_cl_create_replay".into() },
        BuiltinFn { name: "cl_tasks_seen".into(),          params: vec![("id", IrType::I64)],                                          ret: IrType::I64,  runtime_name: "vitalis_cl_tasks_seen".into() },
        BuiltinFn { name: "cl_avg_accuracy".into(),        params: vec![("id", IrType::I64)],                                          ret: IrType::F64,  runtime_name: "vitalis_cl_avg_accuracy".into() },
        BuiltinFn { name: "cl_free".into(),                params: vec![("id", IrType::I64)],                                          ret: IrType::I64,  runtime_name: "vitalis_cl_free".into() },

        // ── v44: Federated Learning ─────────────────────────────────────────
        BuiltinFn { name: "fed_create".into(),             params: vec![("n_params", IrType::I64), ("seed", IrType::I64)],             ret: IrType::I64,  runtime_name: "vitalis_fed_create".into() },
        BuiltinFn { name: "fed_add_client".into(),         params: vec![("id", IrType::I64), ("data_size", IrType::I64)],              ret: IrType::I64,  runtime_name: "vitalis_fed_add_client".into() },
        BuiltinFn { name: "fed_train_round".into(),        params: vec![("id", IrType::I64)],                                          ret: IrType::I64,  runtime_name: "vitalis_fed_train_round".into() },
        BuiltinFn { name: "fed_round".into(),              params: vec![("id", IrType::I64)],                                          ret: IrType::I64,  runtime_name: "vitalis_fed_round".into() },
        BuiltinFn { name: "fed_num_clients".into(),        params: vec![("id", IrType::I64)],                                          ret: IrType::I64,  runtime_name: "vitalis_fed_num_clients".into() },
        BuiltinFn { name: "fed_free".into(),               params: vec![("id", IrType::I64)],                                          ret: IrType::I64,  runtime_name: "vitalis_fed_free".into() },

        // ── v60: Self-hosting bootstrap primitives ──────────────────────
        BuiltinFn { name: "char_to_int".into(),      params: vec![("s", IrType::Ptr)],                                             ret: IrType::I64,  runtime_name: "slang_char_to_int".into() },
        BuiltinFn { name: "int_to_char".into(),      params: vec![("n", IrType::I64)],                                             ret: IrType::Ptr,  runtime_name: "slang_int_to_char".into() },
        BuiltinFn { name: "array_new".into(),        params: vec![("size", IrType::I64)],                                          ret: IrType::Ptr,  runtime_name: "slang_array_new".into() },
        BuiltinFn { name: "array_len".into(),        params: vec![("arr", IrType::Ptr)],                                           ret: IrType::I64,  runtime_name: "slang_array_len".into() },
        BuiltinFn { name: "array_get".into(),        params: vec![("arr", IrType::Ptr), ("idx", IrType::I64)],                     ret: IrType::I64,  runtime_name: "slang_array_get_i64".into() },
        BuiltinFn { name: "array_set".into(),        params: vec![("arr", IrType::Ptr), ("idx", IrType::I64), ("val", IrType::I64)], ret: IrType::Void, runtime_name: "slang_array_set_i64".into() },
        BuiltinFn { name: "exit".into(),             params: vec![("code", IrType::I64)],                                          ret: IrType::Void, runtime_name: "slang_exit".into() },
        BuiltinFn { name: "file_write_bytes".into(), params: vec![("path", IrType::Ptr), ("arr", IrType::Ptr)],                    ret: IrType::Bool, runtime_name: "slang_file_write_bytes".into() },
        BuiltinFn { name: "file_read_bytes".into(),  params: vec![("path", IrType::Ptr)],                                          ret: IrType::Ptr,  runtime_name: "slang_file_read_bytes".into() },
        BuiltinFn { name: "args_count".into(),       params: vec![],                                                                ret: IrType::I64,  runtime_name: "slang_args_count".into() },
        BuiltinFn { name: "args_get".into(),         params: vec![("idx", IrType::I64)],                                           ret: IrType::Ptr,  runtime_name: "slang_args_get".into() },

        // ── Tensor/ML runtime (Void-Vitalis) ────────────────────────────
        BuiltinFn { name: "t_alloc".into(),      params: vec![("n", IrType::I64)],                                                                                     ret: IrType::Ptr,  runtime_name: "slang_array_new".into() },
        BuiltinFn { name: "t_get".into(),        params: vec![("ptr", IrType::Ptr), ("idx", IrType::I64)],                                                             ret: IrType::F64,  runtime_name: "slang_array_get_f64".into() },
        BuiltinFn { name: "t_set".into(),        params: vec![("ptr", IrType::Ptr), ("idx", IrType::I64), ("val", IrType::F64)],                                       ret: IrType::Void, runtime_name: "slang_array_set_f64".into() },
        BuiltinFn { name: "t_len".into(),        params: vec![("ptr", IrType::Ptr)],                                                                                   ret: IrType::I64,  runtime_name: "slang_array_len".into() },
        BuiltinFn { name: "t_fill".into(),       params: vec![("ptr", IrType::Ptr), ("n", IrType::I64), ("val", IrType::F64)],                                         ret: IrType::Void, runtime_name: "slang_t_fill".into() },
        BuiltinFn { name: "t_copy".into(),       params: vec![("src", IrType::Ptr), ("dst", IrType::Ptr), ("n", IrType::I64)],                                         ret: IrType::Void, runtime_name: "slang_t_copy".into() },
        BuiltinFn { name: "t_randn".into(),      params: vec![("ptr", IrType::Ptr), ("n", IrType::I64), ("seed", IrType::I64)],                                        ret: IrType::Void, runtime_name: "slang_t_randn".into() },
        BuiltinFn { name: "t_print_n".into(),    params: vec![("ptr", IrType::Ptr), ("n", IrType::I64)],                                                               ret: IrType::Void, runtime_name: "slang_t_print_n".into() },
        BuiltinFn { name: "t_matmul".into(),     params: vec![("a", IrType::Ptr), ("b", IrType::Ptr), ("out", IrType::Ptr), ("m", IrType::I64), ("n", IrType::I64), ("k", IrType::I64)], ret: IrType::Void, runtime_name: "slang_t_matmul".into() },
        BuiltinFn { name: "t_add_vv".into(),     params: vec![("a", IrType::Ptr), ("b", IrType::Ptr), ("out", IrType::Ptr), ("n", IrType::I64)],                       ret: IrType::Void, runtime_name: "slang_t_add_vv".into() },
        BuiltinFn { name: "t_sub_vv".into(),     params: vec![("a", IrType::Ptr), ("b", IrType::Ptr), ("out", IrType::Ptr), ("n", IrType::I64)],                       ret: IrType::Void, runtime_name: "slang_t_sub_vv".into() },
        BuiltinFn { name: "t_mul_vv".into(),     params: vec![("a", IrType::Ptr), ("b", IrType::Ptr), ("out", IrType::Ptr), ("n", IrType::I64)],                       ret: IrType::Void, runtime_name: "slang_t_mul_vv".into() },
        BuiltinFn { name: "t_scale".into(),      params: vec![("a", IrType::Ptr), ("s", IrType::F64), ("out", IrType::Ptr), ("n", IrType::I64)],                       ret: IrType::Void, runtime_name: "slang_t_scale".into() },
        BuiltinFn { name: "t_softmax".into(),    params: vec![("inp", IrType::Ptr), ("out", IrType::Ptr), ("rows", IrType::I64), ("cols", IrType::I64)],               ret: IrType::Void, runtime_name: "slang_t_softmax".into() },
        BuiltinFn { name: "t_sum".into(),        params: vec![("ptr", IrType::Ptr), ("n", IrType::I64)],                                                               ret: IrType::F64,  runtime_name: "slang_t_sum".into() },
        BuiltinFn { name: "t_add_bias".into(),   params: vec![("a", IrType::Ptr), ("bias", IrType::Ptr), ("out", IrType::Ptr), ("rows", IrType::I64), ("cols", IrType::I64)], ret: IrType::Void, runtime_name: "slang_t_add_bias".into() },
        BuiltinFn { name: "t_max_idx".into(),    params: vec![("ptr", IrType::Ptr), ("n", IrType::I64)],                                                               ret: IrType::I64,  runtime_name: "slang_t_max_idx".into() },
        BuiltinFn { name: "t_dot".into(),        params: vec![("a", IrType::Ptr), ("b", IrType::Ptr), ("n", IrType::I64)],                                             ret: IrType::F64,  runtime_name: "slang_t_dot".into() },
        BuiltinFn { name: "t_cross_entropy".into(), params: vec![("logits", IrType::Ptr), ("targets", IrType::Ptr), ("grad", IrType::Ptr), ("batch", IrType::I64), ("vocab", IrType::I64)], ret: IrType::F64, runtime_name: "slang_t_cross_entropy".into() },
        BuiltinFn { name: "t_transpose".into(),  params: vec![("a", IrType::Ptr), ("out", IrType::Ptr), ("rows", IrType::I64), ("cols", IrType::I64)],                 ret: IrType::Void, runtime_name: "slang_t_transpose".into() },
        BuiltinFn { name: "t_adamw".into(),      params: vec![("param", IrType::Ptr), ("grad", IrType::Ptr), ("m", IrType::Ptr), ("v", IrType::Ptr), ("n", IrType::I64), ("config", IrType::Ptr)], ret: IrType::Void, runtime_name: "slang_t_adamw".into() },
        BuiltinFn { name: "t_norm".into(),       params: vec![("ptr", IrType::Ptr), ("n", IrType::I64)],                                                               ret: IrType::F64,  runtime_name: "slang_t_norm".into() },

        // ── GUI builtins (minifb) ──
        BuiltinFn { name: "gui_open".into(),       params: vec![("w", IrType::I64), ("h", IrType::I64)],  ret: IrType::I64,  runtime_name: "slang_gui_open".into() },
        BuiltinFn { name: "gui_close".into(),      params: vec![],                                            ret: IrType::I64,  runtime_name: "slang_gui_close".into() },
        BuiltinFn { name: "gui_clear".into(),      params: vec![("color", IrType::I64)],                    ret: IrType::I64,  runtime_name: "slang_gui_clear".into() },
        BuiltinFn { name: "gui_rect".into(),       params: vec![("x", IrType::I64), ("y", IrType::I64), ("w", IrType::I64), ("h", IrType::I64), ("color", IrType::I64)], ret: IrType::I64, runtime_name: "slang_gui_rect".into() },
        BuiltinFn { name: "gui_line".into(),       params: vec![("x1", IrType::I64), ("y1", IrType::I64), ("x2", IrType::I64), ("y2", IrType::I64), ("color", IrType::I64)], ret: IrType::I64, runtime_name: "slang_gui_line".into() },
        BuiltinFn { name: "gui_text".into(),       params: vec![("x", IrType::I64), ("y", IrType::I64), ("s", IrType::Ptr), ("color", IrType::I64), ("scale", IrType::I64)], ret: IrType::I64, runtime_name: "slang_gui_text".into() },
        BuiltinFn { name: "gui_update".into(),     params: vec![],                                            ret: IrType::I64,  runtime_name: "slang_gui_update".into() },
        BuiltinFn { name: "gui_circle".into(),     params: vec![("cx", IrType::I64), ("cy", IrType::I64), ("r", IrType::I64), ("color", IrType::I64)], ret: IrType::I64, runtime_name: "slang_gui_circle".into() },

        // ── HD rendering builtins ──
        BuiltinFn { name: "gui_gradient_rect".into(),          params: vec![("x", IrType::I64), ("y", IrType::I64), ("w", IrType::I64), ("h", IrType::I64), ("c1", IrType::I64), ("c2", IrType::I64), ("dir", IrType::I64)], ret: IrType::I64, runtime_name: "slang_gui_gradient_rect".into() },
        BuiltinFn { name: "gui_pixel".into(),                  params: vec![("x", IrType::I64), ("y", IrType::I64), ("color", IrType::I64)], ret: IrType::I64, runtime_name: "slang_gui_pixel".into() },
        BuiltinFn { name: "gui_rounded_rect".into(),           params: vec![("x", IrType::I64), ("y", IrType::I64), ("w", IrType::I64), ("h", IrType::I64), ("radius", IrType::I64), ("color", IrType::I64)], ret: IrType::I64, runtime_name: "slang_gui_rounded_rect".into() },
        BuiltinFn { name: "gui_blend_rect".into(),             params: vec![("x", IrType::I64), ("y", IrType::I64), ("w", IrType::I64), ("h", IrType::I64), ("color", IrType::I64), ("alpha", IrType::I64)], ret: IrType::I64, runtime_name: "slang_gui_blend_rect".into() },
        BuiltinFn { name: "gui_thick_line".into(),             params: vec![("x1", IrType::I64), ("y1", IrType::I64), ("x2", IrType::I64), ("y2", IrType::I64), ("width", IrType::I64), ("color", IrType::I64)], ret: IrType::I64, runtime_name: "slang_gui_thick_line".into() },
        BuiltinFn { name: "gui_triangle".into(),               params: vec![("x1", IrType::I64), ("y1", IrType::I64), ("x2", IrType::I64), ("y2", IrType::I64), ("x3", IrType::I64), ("y3", IrType::I64), ("color", IrType::I64)], ret: IrType::I64, runtime_name: "slang_gui_triangle".into() },
        BuiltinFn { name: "gui_aa_circle".into(),              params: vec![("cx", IrType::I64), ("cy", IrType::I64), ("r", IrType::I64), ("color", IrType::I64)], ret: IrType::I64, runtime_name: "slang_gui_aa_circle".into() },
        BuiltinFn { name: "gui_gradient_rounded_rect".into(),  params: vec![("x", IrType::I64), ("y", IrType::I64), ("w", IrType::I64), ("h", IrType::I64), ("radius", IrType::I64), ("c1", IrType::I64), ("c2", IrType::I64), ("dir", IrType::I64)], ret: IrType::I64, runtime_name: "slang_gui_gradient_rounded_rect".into() },
        BuiltinFn { name: "gui_glow".into(),                   params: vec![("cx", IrType::I64), ("cy", IrType::I64), ("r", IrType::I64), ("color", IrType::I64), ("intensity", IrType::I64)], ret: IrType::I64, runtime_name: "slang_gui_glow".into() },

        // ── Phase 14: Systems Programming (v367-v376) ────────────────
        // Actor Model (v367) — spawn/send/recv already registered above
        BuiltinFn { name: "actor_stop".into(),           params: vec![("id", IrType::I64)], ret: IrType::I64, runtime_name: "slang_actor_stop".into() },
        BuiltinFn { name: "actor_count".into(),          params: vec![], ret: IrType::I64, runtime_name: "slang_actor_count".into() },
        BuiltinFn { name: "actor_mailbox_size".into(),   params: vec![("id", IrType::I64)], ret: IrType::I64, runtime_name: "slang_actor_mailbox_size".into() },
        BuiltinFn { name: "actor_is_alive".into(),       params: vec![("id", IrType::I64)], ret: IrType::I64, runtime_name: "slang_actor_is_alive".into() },
        BuiltinFn { name: "actor_supervise".into(),      params: vec![("supervisor", IrType::I64), ("child", IrType::I64)], ret: IrType::I64, runtime_name: "slang_actor_supervise".into() },

        // STM (v368) — new/read/write already registered above
        BuiltinFn { name: "stm_commit".into(),           params: vec![], ret: IrType::I64, runtime_name: "slang_stm_commit".into() },
        BuiltinFn { name: "stm_abort".into(),            params: vec![], ret: IrType::I64, runtime_name: "slang_stm_abort".into() },
        BuiltinFn { name: "stm_retry".into(),            params: vec![], ret: IrType::I64, runtime_name: "slang_stm_retry".into() },
        BuiltinFn { name: "stm_or_else".into(),          params: vec![("a", IrType::I64), ("b", IrType::I64)], ret: IrType::I64, runtime_name: "slang_stm_or_else".into() },
        BuiltinFn { name: "stm_atomically".into(),       params: vec![("id", IrType::I64)], ret: IrType::I64, runtime_name: "slang_stm_atomically".into() },

        // File System (v369)
        BuiltinFn { name: "vfs_create".into(),           params: vec![], ret: IrType::I64, runtime_name: "slang_vfs_create".into() },
        BuiltinFn { name: "vfs_write".into(),            params: vec![("id", IrType::I64), ("path_hash", IrType::I64), ("data_hash", IrType::I64)], ret: IrType::I64, runtime_name: "slang_vfs_write".into() },
        BuiltinFn { name: "vfs_read".into(),             params: vec![("id", IrType::I64), ("path_hash", IrType::I64)], ret: IrType::I64, runtime_name: "slang_vfs_read".into() },
        BuiltinFn { name: "vfs_exists".into(),           params: vec![("id", IrType::I64), ("path_hash", IrType::I64)], ret: IrType::I64, runtime_name: "slang_vfs_exists".into() },
        BuiltinFn { name: "vfs_delete".into(),           params: vec![("id", IrType::I64), ("path_hash", IrType::I64)], ret: IrType::I64, runtime_name: "slang_vfs_delete".into() },
        BuiltinFn { name: "vfs_list".into(),             params: vec![("id", IrType::I64), ("path_hash", IrType::I64)], ret: IrType::I64, runtime_name: "slang_vfs_list".into() },
        BuiltinFn { name: "vfs_mkdir".into(),            params: vec![("id", IrType::I64), ("path_hash", IrType::I64)], ret: IrType::I64, runtime_name: "slang_vfs_mkdir".into() },
        BuiltinFn { name: "vfs_size".into(),             params: vec![("id", IrType::I64), ("path_hash", IrType::I64)], ret: IrType::I64, runtime_name: "slang_vfs_size".into() },
        BuiltinFn { name: "vfs_is_dir".into(),           params: vec![("id", IrType::I64), ("path_hash", IrType::I64)], ret: IrType::I64, runtime_name: "slang_vfs_is_dir".into() },
        BuiltinFn { name: "vfs_rename".into(),           params: vec![("id", IrType::I64), ("from_hash", IrType::I64), ("to_hash", IrType::I64)], ret: IrType::I64, runtime_name: "slang_vfs_rename".into() },

        // Config Parser (v370)
        BuiltinFn { name: "config_parse".into(),         params: vec![("src_hash", IrType::I64)], ret: IrType::I64, runtime_name: "slang_config_parse".into() },
        BuiltinFn { name: "config_get".into(),           params: vec![("id", IrType::I64), ("key_hash", IrType::I64)], ret: IrType::I64, runtime_name: "slang_config_get".into() },
        BuiltinFn { name: "config_set".into(),           params: vec![("id", IrType::I64), ("key_hash", IrType::I64), ("val_hash", IrType::I64)], ret: IrType::I64, runtime_name: "slang_config_set".into() },
        BuiltinFn { name: "config_has".into(),           params: vec![("id", IrType::I64), ("key_hash", IrType::I64)], ret: IrType::I64, runtime_name: "slang_config_has".into() },
        BuiltinFn { name: "config_keys".into(),          params: vec![("id", IrType::I64)], ret: IrType::I64, runtime_name: "slang_config_keys".into() },
        BuiltinFn { name: "config_merge".into(),         params: vec![("a", IrType::I64), ("b", IrType::I64)], ret: IrType::I64, runtime_name: "slang_config_merge".into() },
        BuiltinFn { name: "config_validate".into(),      params: vec![("id", IrType::I64), ("schema_hash", IrType::I64)], ret: IrType::I64, runtime_name: "slang_config_validate".into() },
        BuiltinFn { name: "config_to_string".into(),     params: vec![("id", IrType::I64)], ret: IrType::I64, runtime_name: "slang_config_to_string".into() },

        // State Machine (v371)
        BuiltinFn { name: "fsm_create".into(),           params: vec![("initial_hash", IrType::I64)], ret: IrType::I64, runtime_name: "slang_fsm_create".into() },
        BuiltinFn { name: "fsm_add_state".into(),        params: vec![("id", IrType::I64), ("state_hash", IrType::I64)], ret: IrType::I64, runtime_name: "slang_fsm_add_state".into() },
        BuiltinFn { name: "fsm_add_transition".into(),   params: vec![("id", IrType::I64), ("from_hash", IrType::I64), ("event_hash", IrType::I64), ("to_hash", IrType::I64)], ret: IrType::I64, runtime_name: "slang_fsm_add_transition".into() },
        BuiltinFn { name: "fsm_trigger".into(),          params: vec![("id", IrType::I64), ("event_hash", IrType::I64)], ret: IrType::I64, runtime_name: "slang_fsm_trigger".into() },
        BuiltinFn { name: "fsm_current".into(),          params: vec![("id", IrType::I64)], ret: IrType::I64, runtime_name: "slang_fsm_current".into() },
        BuiltinFn { name: "fsm_can_trigger".into(),      params: vec![("id", IrType::I64), ("event_hash", IrType::I64)], ret: IrType::I64, runtime_name: "slang_fsm_can_trigger".into() },
        BuiltinFn { name: "fsm_reset".into(),            params: vec![("id", IrType::I64)], ret: IrType::I64, runtime_name: "slang_fsm_reset".into() },
        BuiltinFn { name: "fsm_state_count".into(),      params: vec![("id", IrType::I64)], ret: IrType::I64, runtime_name: "slang_fsm_state_count".into() },

        // Scheduler (v372)
        BuiltinFn { name: "sched_create".into(),         params: vec![], ret: IrType::I64, runtime_name: "slang_sched_create".into() },
        BuiltinFn { name: "sched_add_task".into(),       params: vec![("id", IrType::I64), ("name_hash", IrType::I64), ("time", IrType::I64)], ret: IrType::I64, runtime_name: "slang_sched_add_task".into() },
        BuiltinFn { name: "sched_run_pending".into(),    params: vec![("id", IrType::I64), ("now", IrType::I64)], ret: IrType::I64, runtime_name: "slang_sched_run_pending".into() },
        BuiltinFn { name: "sched_cancel".into(),         params: vec![("id", IrType::I64), ("task_id", IrType::I64)], ret: IrType::I64, runtime_name: "slang_sched_cancel".into() },
        BuiltinFn { name: "sched_pending_count".into(),  params: vec![("id", IrType::I64)], ret: IrType::I64, runtime_name: "slang_sched_pending_count".into() },
        BuiltinFn { name: "sched_cron_parse".into(),     params: vec![("expr_hash", IrType::I64)], ret: IrType::I64, runtime_name: "slang_sched_cron_parse".into() },
        BuiltinFn { name: "sched_cron_next".into(),      params: vec![("cron_id", IrType::I64), ("after", IrType::I64)], ret: IrType::I64, runtime_name: "slang_sched_cron_next".into() },
        BuiltinFn { name: "sched_clear".into(),          params: vec![("id", IrType::I64)], ret: IrType::I64, runtime_name: "slang_sched_clear".into() },

        // Cache (v373)
        BuiltinFn { name: "cache_create".into(),         params: vec![("capacity", IrType::I64), ("policy", IrType::I64)], ret: IrType::I64, runtime_name: "slang_cache_create".into() },
        BuiltinFn { name: "cache_put".into(),            params: vec![("id", IrType::I64), ("key", IrType::I64), ("val", IrType::I64)], ret: IrType::I64, runtime_name: "slang_cache_put".into() },
        BuiltinFn { name: "cache_get".into(),            params: vec![("id", IrType::I64), ("key", IrType::I64)], ret: IrType::I64, runtime_name: "slang_cache_get".into() },
        BuiltinFn { name: "cache_remove".into(),         params: vec![("id", IrType::I64), ("key", IrType::I64)], ret: IrType::I64, runtime_name: "slang_cache_remove".into() },
        BuiltinFn { name: "cache_contains".into(),       params: vec![("id", IrType::I64), ("key", IrType::I64)], ret: IrType::I64, runtime_name: "slang_cache_contains".into() },
        BuiltinFn { name: "cache_size".into(),           params: vec![("id", IrType::I64)], ret: IrType::I64, runtime_name: "slang_cache_size".into() },
        BuiltinFn { name: "cache_clear".into(),          params: vec![("id", IrType::I64)], ret: IrType::I64, runtime_name: "slang_cache_clear".into() },
        BuiltinFn { name: "cache_hit_rate".into(),       params: vec![("id", IrType::I64)], ret: IrType::F64, runtime_name: "slang_cache_hit_rate".into() },
        BuiltinFn { name: "cache_eviction_count".into(), params: vec![("id", IrType::I64)], ret: IrType::I64, runtime_name: "slang_cache_eviction_count".into() },
        BuiltinFn { name: "cache_capacity".into(),       params: vec![("id", IrType::I64)], ret: IrType::I64, runtime_name: "slang_cache_capacity".into() },

        // Search Index (v374)
        BuiltinFn { name: "idx_create".into(),           params: vec![], ret: IrType::I64, runtime_name: "slang_idx_create".into() },
        BuiltinFn { name: "idx_add_doc".into(),          params: vec![("id", IrType::I64), ("doc_hash", IrType::I64)], ret: IrType::I64, runtime_name: "slang_idx_add_doc".into() },
        BuiltinFn { name: "idx_search".into(),           params: vec![("id", IrType::I64), ("query_hash", IrType::I64)], ret: IrType::I64, runtime_name: "slang_idx_search".into() },
        BuiltinFn { name: "idx_doc_count".into(),        params: vec![("id", IrType::I64)], ret: IrType::I64, runtime_name: "slang_idx_doc_count".into() },
        BuiltinFn { name: "idx_term_count".into(),       params: vec![("id", IrType::I64)], ret: IrType::I64, runtime_name: "slang_idx_term_count".into() },
        BuiltinFn { name: "idx_bm25_score".into(),       params: vec![("id", IrType::I64), ("term_hash", IrType::I64), ("doc_id", IrType::I64)], ret: IrType::F64, runtime_name: "slang_idx_bm25_score".into() },
        BuiltinFn { name: "idx_tfidf_score".into(),      params: vec![("id", IrType::I64), ("term_hash", IrType::I64), ("doc_id", IrType::I64)], ret: IrType::F64, runtime_name: "slang_idx_tfidf_score".into() },
        BuiltinFn { name: "idx_remove_doc".into(),       params: vec![("id", IrType::I64), ("doc_id", IrType::I64)], ret: IrType::I64, runtime_name: "slang_idx_remove_doc".into() },

        // Template Engine (v375)
        BuiltinFn { name: "tpl_render".into(),           params: vec![("tmpl_hash", IrType::I64)], ret: IrType::I64, runtime_name: "slang_tpl_render".into() },
        BuiltinFn { name: "tpl_set_var".into(),          params: vec![("name_hash", IrType::I64), ("val_hash", IrType::I64)], ret: IrType::I64, runtime_name: "slang_tpl_set_var".into() },
        BuiltinFn { name: "tpl_get_var".into(),          params: vec![("name_hash", IrType::I64)], ret: IrType::I64, runtime_name: "slang_tpl_get_var".into() },
        BuiltinFn { name: "tpl_has_var".into(),          params: vec![("name_hash", IrType::I64)], ret: IrType::I64, runtime_name: "slang_tpl_has_var".into() },
        BuiltinFn { name: "tpl_var_count".into(),        params: vec![], ret: IrType::I64, runtime_name: "slang_tpl_var_count".into() },
        BuiltinFn { name: "tpl_clear_vars".into(),       params: vec![], ret: IrType::I64, runtime_name: "slang_tpl_clear_vars".into() },
        BuiltinFn { name: "tpl_validate".into(),         params: vec![("tmpl_hash", IrType::I64)], ret: IrType::I64, runtime_name: "slang_tpl_validate".into() },
        BuiltinFn { name: "tpl_escape_html".into(),      params: vec![("input_hash", IrType::I64)], ret: IrType::I64, runtime_name: "slang_tpl_escape_html".into() },

        // Logging (v376) — debug/info/warn/error already registered above
        BuiltinFn { name: "log_init".into(),             params: vec![("level", IrType::I64)], ret: IrType::I64, runtime_name: "slang_log_init".into() },
        BuiltinFn { name: "log_set_level".into(),        params: vec![("level", IrType::I64)], ret: IrType::I64, runtime_name: "slang_log_set_level".into() },
        BuiltinFn { name: "log_count".into(),            params: vec![], ret: IrType::I64, runtime_name: "slang_log_count".into() },
        BuiltinFn { name: "log_clear".into(),            params: vec![], ret: IrType::I64, runtime_name: "slang_log_clear".into() },

        // ── Phase 15: Compiler & Language (v377-v386) ────────────────
        // Alias Analysis (v377)
        BuiltinFn { name: "alias_analyze".into(),        params: vec![("a", IrType::I64), ("b", IrType::I64)], ret: IrType::I64, runtime_name: "slang_alias_analyze".into() },
        BuiltinFn { name: "alias_may_alias".into(),      params: vec![("a", IrType::I64), ("b", IrType::I64)], ret: IrType::I64, runtime_name: "slang_alias_may_alias".into() },
        BuiltinFn { name: "alias_must_alias".into(),     params: vec![("a", IrType::I64), ("b", IrType::I64)], ret: IrType::I64, runtime_name: "slang_alias_must_alias".into() },
        BuiltinFn { name: "alias_no_alias".into(),       params: vec![("a", IrType::I64), ("b", IrType::I64)], ret: IrType::I64, runtime_name: "slang_alias_no_alias".into() },
        BuiltinFn { name: "alias_points_to".into(),      params: vec![("ptr", IrType::I64)], ret: IrType::I64, runtime_name: "slang_alias_points_to".into() },
        BuiltinFn { name: "alias_set_count".into(),      params: vec![], ret: IrType::I64, runtime_name: "slang_alias_set_count".into() },
        BuiltinFn { name: "alias_tbaa_check".into(),     params: vec![("type_a", IrType::I64), ("type_b", IrType::I64)], ret: IrType::I64, runtime_name: "slang_alias_tbaa_check".into() },
        BuiltinFn { name: "alias_escape_check".into(),   params: vec![("ptr", IrType::I64)], ret: IrType::I64, runtime_name: "slang_alias_escape_check".into() },

        // Loop Vectorizer (v378) — vec_width already registered above
        BuiltinFn { name: "vec_analyze".into(),          params: vec![("loop_id", IrType::I64)], ret: IrType::I64, runtime_name: "slang_vec_analyze".into() },
        BuiltinFn { name: "vec_is_vectorizable".into(),  params: vec![("loop_id", IrType::I64)], ret: IrType::I64, runtime_name: "slang_vec_is_vectorizable".into() },
        BuiltinFn { name: "vec_cost_model".into(),       params: vec![("loop_id", IrType::I64)], ret: IrType::I64, runtime_name: "slang_vec_cost_model".into() },
        BuiltinFn { name: "vec_unroll_factor".into(),    params: vec![("loop_id", IrType::I64)], ret: IrType::I64, runtime_name: "slang_vec_unroll_factor".into() },
        BuiltinFn { name: "vec_dependence_check".into(), params: vec![("loop_id", IrType::I64)], ret: IrType::I64, runtime_name: "slang_vec_dependence_check".into() },
        BuiltinFn { name: "vec_slp_analyze".into(),      params: vec![("loop_id", IrType::I64)], ret: IrType::I64, runtime_name: "slang_vec_slp_analyze".into() },
        BuiltinFn { name: "vec_transform".into(),        params: vec![("loop_id", IrType::I64), ("width", IrType::I64)], ret: IrType::I64, runtime_name: "slang_vec_transform".into() },

        // Sanitizer (v379)
        BuiltinFn { name: "san_check_bounds".into(),     params: vec![("ptr", IrType::I64), ("idx", IrType::I64), ("len", IrType::I64)], ret: IrType::I64, runtime_name: "slang_san_check_bounds".into() },
        BuiltinFn { name: "san_check_null".into(),       params: vec![("ptr", IrType::I64)], ret: IrType::I64, runtime_name: "slang_san_check_null".into() },
        BuiltinFn { name: "san_check_overflow".into(),   params: vec![("a", IrType::I64), ("b", IrType::I64), ("op", IrType::I64)], ret: IrType::I64, runtime_name: "slang_san_check_overflow".into() },
        BuiltinFn { name: "san_check_use_after_free".into(), params: vec![("ptr", IrType::I64)], ret: IrType::I64, runtime_name: "slang_san_check_use_after_free".into() },
        BuiltinFn { name: "san_check_double_free".into(),    params: vec![("ptr", IrType::I64)], ret: IrType::I64, runtime_name: "slang_san_check_double_free".into() },
        BuiltinFn { name: "san_check_leak".into(),       params: vec![], ret: IrType::I64, runtime_name: "slang_san_check_leak".into() },
        BuiltinFn { name: "san_report_count".into(),     params: vec![], ret: IrType::I64, runtime_name: "slang_san_report_count".into() },
        BuiltinFn { name: "san_clear_reports".into(),    params: vec![], ret: IrType::I64, runtime_name: "slang_san_clear_reports".into() },

        // Refactoring (v380)
        BuiltinFn { name: "refactor_rename".into(),      params: vec![("old_hash", IrType::I64), ("new_hash", IrType::I64)], ret: IrType::I64, runtime_name: "slang_refactor_rename".into() },
        BuiltinFn { name: "refactor_extract_fn".into(),  params: vec![("start", IrType::I64), ("end", IrType::I64), ("name_hash", IrType::I64)], ret: IrType::I64, runtime_name: "slang_refactor_extract_fn".into() },
        BuiltinFn { name: "refactor_inline".into(),      params: vec![("fn_hash", IrType::I64)], ret: IrType::I64, runtime_name: "slang_refactor_inline".into() },
        BuiltinFn { name: "refactor_move".into(),        params: vec![("sym_hash", IrType::I64), ("target_hash", IrType::I64)], ret: IrType::I64, runtime_name: "slang_refactor_move".into() },
        BuiltinFn { name: "refactor_add_param".into(),   params: vec![("fn_hash", IrType::I64), ("param_hash", IrType::I64)], ret: IrType::I64, runtime_name: "slang_refactor_add_param".into() },
        BuiltinFn { name: "refactor_remove_param".into(),params: vec![("fn_hash", IrType::I64), ("idx", IrType::I64)], ret: IrType::I64, runtime_name: "slang_refactor_remove_param".into() },
        BuiltinFn { name: "refactor_preview".into(),     params: vec![("op", IrType::I64)], ret: IrType::I64, runtime_name: "slang_refactor_preview".into() },
        BuiltinFn { name: "refactor_undo".into(),        params: vec![], ret: IrType::I64, runtime_name: "slang_refactor_undo".into() },

        // Parser Combinator (v381)
        BuiltinFn { name: "prs_literal".into(),          params: vec![("input_hash", IrType::I64)], ret: IrType::I64, runtime_name: "slang_prs_literal".into() },
        BuiltinFn { name: "prs_regex".into(),            params: vec![], ret: IrType::I64, runtime_name: "slang_prs_regex".into() },
        BuiltinFn { name: "prs_sequence".into(),         params: vec![], ret: IrType::I64, runtime_name: "slang_prs_sequence".into() },
        BuiltinFn { name: "prs_choice".into(),           params: vec![], ret: IrType::I64, runtime_name: "slang_prs_choice".into() },
        BuiltinFn { name: "prs_many".into(),             params: vec![], ret: IrType::I64, runtime_name: "slang_prs_many".into() },
        BuiltinFn { name: "prs_map".into(),              params: vec![], ret: IrType::I64, runtime_name: "slang_prs_map".into() },
        BuiltinFn { name: "prs_optional".into(),         params: vec![], ret: IrType::I64, runtime_name: "slang_prs_optional".into() },
        BuiltinFn { name: "prs_run".into(),              params: vec![("input_hash", IrType::I64)], ret: IrType::I64, runtime_name: "slang_prs_run".into() },

        // Symbolic Math (v382)
        BuiltinFn { name: "sym_parse".into(),            params: vec![("kind", IrType::I64), ("val", IrType::F64)], ret: IrType::I64, runtime_name: "slang_sym_parse".into() },
        BuiltinFn { name: "sym_simplify".into(),         params: vec![("id", IrType::I64)], ret: IrType::I64, runtime_name: "slang_sym_simplify".into() },
        BuiltinFn { name: "sym_diff".into(),             params: vec![("id", IrType::I64)], ret: IrType::I64, runtime_name: "slang_sym_diff".into() },
        BuiltinFn { name: "sym_eval".into(),             params: vec![("id", IrType::I64), ("x_val", IrType::F64)], ret: IrType::F64, runtime_name: "slang_sym_eval".into() },
        BuiltinFn { name: "sym_add".into(),              params: vec![("a", IrType::I64), ("b", IrType::I64)], ret: IrType::I64, runtime_name: "slang_sym_add".into() },
        BuiltinFn { name: "sym_mul".into(),              params: vec![("a", IrType::I64), ("b", IrType::I64)], ret: IrType::I64, runtime_name: "slang_sym_mul".into() },
        BuiltinFn { name: "sym_degree".into(),           params: vec![("id", IrType::I64)], ret: IrType::I64, runtime_name: "slang_sym_degree".into() },
        BuiltinFn { name: "sym_substitute".into(),       params: vec![("id", IrType::I64), ("val", IrType::F64)], ret: IrType::I64, runtime_name: "slang_sym_substitute".into() },
        BuiltinFn { name: "sym_expand".into(),           params: vec![("id", IrType::I64)], ret: IrType::I64, runtime_name: "slang_sym_expand".into() },
        BuiltinFn { name: "sym_factor".into(),           params: vec![("id", IrType::I64)], ret: IrType::I64, runtime_name: "slang_sym_factor".into() },

        // Reactive (v383)
        BuiltinFn { name: "rx_create".into(),            params: vec![("initial_value", IrType::I64)], ret: IrType::I64, runtime_name: "slang_rx_create".into() },
        BuiltinFn { name: "rx_map".into(),               params: vec![("id", IrType::I64), ("offset", IrType::I64)], ret: IrType::I64, runtime_name: "slang_rx_map".into() },
        BuiltinFn { name: "rx_filter".into(),            params: vec![("id", IrType::I64), ("threshold", IrType::I64)], ret: IrType::I64, runtime_name: "slang_rx_filter".into() },
        BuiltinFn { name: "rx_reduce".into(),            params: vec![("id", IrType::I64)], ret: IrType::I64, runtime_name: "slang_rx_reduce".into() },
        BuiltinFn { name: "rx_merge".into(),             params: vec![("a", IrType::I64), ("b", IrType::I64)], ret: IrType::I64, runtime_name: "slang_rx_merge".into() },
        BuiltinFn { name: "rx_take".into(),              params: vec![("id", IrType::I64), ("n", IrType::I64)], ret: IrType::I64, runtime_name: "slang_rx_take".into() },
        BuiltinFn { name: "rx_subscribe".into(),         params: vec![("id", IrType::I64), ("subscriber", IrType::I64)], ret: IrType::I64, runtime_name: "slang_rx_subscribe".into() },
        BuiltinFn { name: "rx_count".into(),             params: vec![("id", IrType::I64)], ret: IrType::I64, runtime_name: "slang_rx_count".into() },

        // Session Types (v384) — session_create already registered above
        BuiltinFn { name: "session_send".into(),         params: vec![("id", IrType::I64), ("val", IrType::I64)], ret: IrType::I64, runtime_name: "slang_session_send".into() },
        BuiltinFn { name: "session_recv".into(),         params: vec![("id", IrType::I64)], ret: IrType::I64, runtime_name: "slang_session_recv".into() },
        BuiltinFn { name: "session_choose".into(),       params: vec![("id", IrType::I64), ("branch", IrType::I64)], ret: IrType::I64, runtime_name: "slang_session_choose".into() },
        BuiltinFn { name: "session_offer".into(),        params: vec![("id", IrType::I64)], ret: IrType::I64, runtime_name: "slang_session_offer".into() },
        BuiltinFn { name: "session_close".into(),        params: vec![("id", IrType::I64)], ret: IrType::I64, runtime_name: "slang_session_close".into() },
        BuiltinFn { name: "session_is_dual".into(),     params: vec![("a", IrType::I64), ("b", IrType::I64)], ret: IrType::I64, runtime_name: "slang_session_is_dual".into() },
        BuiltinFn { name: "session_validate".into(),     params: vec![("id", IrType::I64)], ret: IrType::I64, runtime_name: "slang_session_validate".into() },

        // Gradual Typing (v385)
        BuiltinFn { name: "grad_check".into(),           params: vec![("a", IrType::I64), ("b", IrType::I64)], ret: IrType::I64, runtime_name: "slang_grad_check".into() },
        BuiltinFn { name: "grad_cast".into(),            params: vec![("val", IrType::I64), ("type_id", IrType::I64)], ret: IrType::I64, runtime_name: "slang_grad_cast".into() },
        BuiltinFn { name: "grad_is_dynamic".into(),      params: vec![("type_id", IrType::I64)], ret: IrType::I64, runtime_name: "slang_grad_is_dynamic".into() },
        BuiltinFn { name: "grad_guard".into(),           params: vec![("val", IrType::I64), ("type_id", IrType::I64)], ret: IrType::I64, runtime_name: "slang_grad_guard".into() },
        BuiltinFn { name: "grad_narrow".into(),          params: vec![("type_id", IrType::I64)], ret: IrType::I64, runtime_name: "slang_grad_narrow".into() },
        BuiltinFn { name: "grad_widen".into(),           params: vec![("type_id", IrType::I64)], ret: IrType::I64, runtime_name: "slang_grad_widen".into() },
        BuiltinFn { name: "grad_boundary".into(),        params: vec![("type_id", IrType::I64)], ret: IrType::I64, runtime_name: "slang_grad_boundary".into() },
        BuiltinFn { name: "grad_consistency".into(),     params: vec![("a", IrType::I64), ("b", IrType::I64)], ret: IrType::I64, runtime_name: "slang_grad_consistency".into() },

        // Code Coverage (v386)
        BuiltinFn { name: "cov_init".into(),             params: vec![("file_hash", IrType::I64)], ret: IrType::I64, runtime_name: "slang_cov_init".into() },
        BuiltinFn { name: "cov_mark_line".into(),        params: vec![("file_hash", IrType::I64), ("line", IrType::I64)], ret: IrType::I64, runtime_name: "slang_cov_mark_line".into() },
        BuiltinFn { name: "cov_hit_line".into(),         params: vec![("file_hash", IrType::I64), ("line", IrType::I64)], ret: IrType::I64, runtime_name: "slang_cov_hit_line".into() },
        BuiltinFn { name: "cov_line_pct".into(),         params: vec![("file_hash", IrType::I64)], ret: IrType::F64, runtime_name: "slang_cov_line_pct".into() },
        BuiltinFn { name: "cov_branch_pct".into(),       params: vec![("file_hash", IrType::I64)], ret: IrType::F64, runtime_name: "slang_cov_branch_pct".into() },
        BuiltinFn { name: "cov_total_lines".into(),      params: vec![("file_hash", IrType::I64)], ret: IrType::I64, runtime_name: "slang_cov_total_lines".into() },
        BuiltinFn { name: "cov_hit_lines".into(),        params: vec![("file_hash", IrType::I64)], ret: IrType::I64, runtime_name: "slang_cov_hit_lines".into() },
        BuiltinFn { name: "cov_reset".into(),            params: vec![], ret: IrType::I64, runtime_name: "slang_cov_reset".into() },

        // ── Phase 16: Distributed & Observability (v387-v396) ────────
        // DHT (v387)
        BuiltinFn { name: "dht_create".into(),           params: vec![("vnodes", IrType::I64)], ret: IrType::I64, runtime_name: "slang_dht_create".into() },
        BuiltinFn { name: "dht_put".into(),              params: vec![("id", IrType::I64), ("key", IrType::I64), ("val", IrType::I64)], ret: IrType::I64, runtime_name: "slang_dht_put".into() },
        BuiltinFn { name: "dht_get".into(),              params: vec![("id", IrType::I64), ("key", IrType::I64)], ret: IrType::I64, runtime_name: "slang_dht_get".into() },
        BuiltinFn { name: "dht_remove".into(),           params: vec![("id", IrType::I64), ("key", IrType::I64)], ret: IrType::I64, runtime_name: "slang_dht_remove".into() },
        BuiltinFn { name: "dht_contains".into(),         params: vec![("id", IrType::I64), ("key", IrType::I64)], ret: IrType::I64, runtime_name: "slang_dht_contains".into() },
        BuiltinFn { name: "dht_size".into(),             params: vec![("id", IrType::I64)], ret: IrType::I64, runtime_name: "slang_dht_size".into() },
        BuiltinFn { name: "dht_hash".into(),             params: vec![("key", IrType::I64)], ret: IrType::I64, runtime_name: "slang_dht_hash".into() },
        BuiltinFn { name: "dht_rebalance".into(),        params: vec![("id", IrType::I64), ("new_nodes", IrType::I64)], ret: IrType::I64, runtime_name: "slang_dht_rebalance".into() },

        // MapReduce (v388)
        BuiltinFn { name: "mr_create".into(),            params: vec![("mapper_id", IrType::I64), ("reducer_id", IrType::I64)], ret: IrType::I64, runtime_name: "slang_mr_create".into() },
        BuiltinFn { name: "mr_add_input".into(),         params: vec![("id", IrType::I64), ("input_hash", IrType::I64)], ret: IrType::I64, runtime_name: "slang_mr_add_input".into() },
        BuiltinFn { name: "mr_map".into(),               params: vec![("id", IrType::I64)], ret: IrType::I64, runtime_name: "slang_mr_map".into() },
        BuiltinFn { name: "mr_shuffle".into(),           params: vec![("id", IrType::I64)], ret: IrType::I64, runtime_name: "slang_mr_shuffle".into() },
        BuiltinFn { name: "mr_reduce".into(),            params: vec![("id", IrType::I64)], ret: IrType::I64, runtime_name: "slang_mr_reduce".into() },
        BuiltinFn { name: "mr_result".into(),            params: vec![("id", IrType::I64)], ret: IrType::I64, runtime_name: "slang_mr_result".into() },
        BuiltinFn { name: "mr_partition_count".into(),   params: vec![("id", IrType::I64)], ret: IrType::I64, runtime_name: "slang_mr_partition_count".into() },
        BuiltinFn { name: "mr_reset".into(),             params: vec![("id", IrType::I64)], ret: IrType::I64, runtime_name: "slang_mr_reset".into() },

        // Blockchain (v389)
        BuiltinFn { name: "chain_create".into(),         params: vec![("difficulty", IrType::I64)], ret: IrType::I64, runtime_name: "slang_chain_create".into() },
        BuiltinFn { name: "chain_add_block".into(),      params: vec![("id", IrType::I64), ("data_hash", IrType::I64)], ret: IrType::I64, runtime_name: "slang_chain_add_block".into() },
        BuiltinFn { name: "chain_validate".into(),       params: vec![("id", IrType::I64)], ret: IrType::I64, runtime_name: "slang_chain_validate".into() },
        BuiltinFn { name: "chain_length".into(),         params: vec![("id", IrType::I64)], ret: IrType::I64, runtime_name: "slang_chain_length".into() },
        BuiltinFn { name: "chain_latest_hash".into(),    params: vec![("id", IrType::I64)], ret: IrType::I64, runtime_name: "slang_chain_latest_hash".into() },
        BuiltinFn { name: "chain_get_block".into(),      params: vec![("id", IrType::I64), ("idx", IrType::I64)], ret: IrType::I64, runtime_name: "slang_chain_get_block".into() },
        BuiltinFn { name: "chain_difficulty".into(),     params: vec![("id", IrType::I64)], ret: IrType::I64, runtime_name: "slang_chain_difficulty".into() },
        BuiltinFn { name: "chain_is_valid".into(),       params: vec![("id", IrType::I64)], ret: IrType::I64, runtime_name: "slang_chain_is_valid".into() },
        BuiltinFn { name: "merkle_root".into(),          params: vec![("count", IrType::I64)], ret: IrType::I64, runtime_name: "slang_merkle_root".into() },
        BuiltinFn { name: "merkle_verify".into(),        params: vec![("root", IrType::I64), ("leaf_hash", IrType::I64), ("proof_depth", IrType::I64)], ret: IrType::I64, runtime_name: "slang_merkle_verify".into() },

        // TLS Engine (v390)
        BuiltinFn { name: "tls_handshake".into(),        params: vec![("id", IrType::I64)], ret: IrType::I64, runtime_name: "slang_tls_handshake".into() },
        BuiltinFn { name: "tls_encrypt".into(),          params: vec![("id", IrType::I64), ("data_hash", IrType::I64)], ret: IrType::I64, runtime_name: "slang_tls_encrypt".into() },
        BuiltinFn { name: "tls_decrypt".into(),          params: vec![("id", IrType::I64), ("data_hash", IrType::I64)], ret: IrType::I64, runtime_name: "slang_tls_decrypt".into() },
        BuiltinFn { name: "tls_derive_key".into(),       params: vec![("secret_hash", IrType::I64), ("label_hash", IrType::I64)], ret: IrType::I64, runtime_name: "slang_tls_derive_key".into() },
        BuiltinFn { name: "tls_verify_cert".into(),      params: vec![("id", IrType::I64)], ret: IrType::I64, runtime_name: "slang_tls_verify_cert".into() },
        BuiltinFn { name: "tls_create_cert".into(),      params: vec![("issuer_hash", IrType::I64), ("subject_hash", IrType::I64)], ret: IrType::I64, runtime_name: "slang_tls_create_cert".into() },
        BuiltinFn { name: "tls_session_id".into(),       params: vec![("id", IrType::I64)], ret: IrType::I64, runtime_name: "slang_tls_session_id".into() },
        BuiltinFn { name: "tls_is_secure".into(),        params: vec![("id", IrType::I64)], ret: IrType::I64, runtime_name: "slang_tls_is_secure".into() },

        // Circuit Breaker (v391)
        BuiltinFn { name: "cb_create".into(),            params: vec![("threshold", IrType::I64), ("timeout", IrType::I64)], ret: IrType::I64, runtime_name: "slang_cb_create".into() },
        BuiltinFn { name: "cb_call".into(),              params: vec![("id", IrType::I64), ("success", IrType::I64)], ret: IrType::I64, runtime_name: "slang_cb_call".into() },
        BuiltinFn { name: "cb_state".into(),             params: vec![("id", IrType::I64)], ret: IrType::I64, runtime_name: "slang_cb_state".into() },
        BuiltinFn { name: "cb_record_success".into(),    params: vec![("id", IrType::I64)], ret: IrType::I64, runtime_name: "slang_cb_record_success".into() },
        BuiltinFn { name: "cb_record_failure".into(),    params: vec![("id", IrType::I64)], ret: IrType::I64, runtime_name: "slang_cb_record_failure".into() },
        BuiltinFn { name: "cb_reset".into(),             params: vec![("id", IrType::I64)], ret: IrType::I64, runtime_name: "slang_cb_reset".into() },
        BuiltinFn { name: "cb_failure_count".into(),     params: vec![("id", IrType::I64)], ret: IrType::I64, runtime_name: "slang_cb_failure_count".into() },
        BuiltinFn { name: "cb_is_open".into(),           params: vec![("id", IrType::I64)], ret: IrType::I64, runtime_name: "slang_cb_is_open".into() },

        // Load Balancer (v392)
        BuiltinFn { name: "lb_create".into(),            params: vec![("strategy", IrType::I64)], ret: IrType::I64, runtime_name: "slang_lb_create".into() },
        BuiltinFn { name: "lb_add_backend".into(),       params: vec![("id", IrType::I64), ("name_hash", IrType::I64), ("weight", IrType::I64)], ret: IrType::I64, runtime_name: "slang_lb_add_backend".into() },
        BuiltinFn { name: "lb_remove_backend".into(),    params: vec![("id", IrType::I64), ("name_hash", IrType::I64)], ret: IrType::I64, runtime_name: "slang_lb_remove_backend".into() },
        BuiltinFn { name: "lb_next".into(),              params: vec![("id", IrType::I64)], ret: IrType::I64, runtime_name: "slang_lb_next".into() },
        BuiltinFn { name: "lb_backend_count".into(),     params: vec![("id", IrType::I64)], ret: IrType::I64, runtime_name: "slang_lb_backend_count".into() },
        BuiltinFn { name: "lb_set_weight".into(),        params: vec![("id", IrType::I64), ("name_hash", IrType::I64), ("weight", IrType::I64)], ret: IrType::I64, runtime_name: "slang_lb_set_weight".into() },
        BuiltinFn { name: "lb_health_check".into(),      params: vec![("id", IrType::I64)], ret: IrType::I64, runtime_name: "slang_lb_health_check".into() },
        BuiltinFn { name: "lb_strategy".into(),          params: vec![("id", IrType::I64)], ret: IrType::I64, runtime_name: "slang_lb_strategy".into() },

        // Service Discovery (v393)
        BuiltinFn { name: "sd_register".into(),          params: vec![("name_hash", IrType::I64), ("addr_hash", IrType::I64), ("port", IrType::I64)], ret: IrType::I64, runtime_name: "slang_sd_register".into() },
        BuiltinFn { name: "sd_deregister".into(),        params: vec![("svc_id", IrType::I64)], ret: IrType::I64, runtime_name: "slang_sd_deregister".into() },
        BuiltinFn { name: "sd_discover".into(),          params: vec![("name_hash", IrType::I64)], ret: IrType::I64, runtime_name: "slang_sd_discover".into() },
        BuiltinFn { name: "sd_health_check".into(),      params: vec![], ret: IrType::I64, runtime_name: "slang_sd_health_check".into() },
        BuiltinFn { name: "sd_service_count".into(),     params: vec![], ret: IrType::I64, runtime_name: "slang_sd_service_count".into() },
        BuiltinFn { name: "sd_is_healthy".into(),        params: vec![("svc_id", IrType::I64)], ret: IrType::I64, runtime_name: "slang_sd_is_healthy".into() },
        BuiltinFn { name: "sd_list_services".into(),     params: vec![], ret: IrType::I64, runtime_name: "slang_sd_list_services".into() },
        BuiltinFn { name: "sd_heartbeat".into(),         params: vec![("svc_id", IrType::I64)], ret: IrType::I64, runtime_name: "slang_sd_heartbeat".into() },

        // Metrics Engine (v394) — counter/histogram already registered above
        BuiltinFn { name: "metric_inc".into(),           params: vec![("name_hash", IrType::I64), ("val", IrType::I64)], ret: IrType::I64, runtime_name: "slang_metric_inc".into() },
        BuiltinFn { name: "metric_gauge_set".into(),     params: vec![("name_hash", IrType::I64), ("val", IrType::I64)], ret: IrType::I64, runtime_name: "slang_metric_gauge_set".into() },
        BuiltinFn { name: "metric_gauge_get".into(),     params: vec![("name_hash", IrType::I64)], ret: IrType::F64, runtime_name: "slang_metric_gauge_get".into() },
        BuiltinFn { name: "metric_p50".into(),           params: vec![("name_hash", IrType::I64)], ret: IrType::F64, runtime_name: "slang_metric_p50".into() },
        BuiltinFn { name: "metric_p99".into(),           params: vec![("name_hash", IrType::I64)], ret: IrType::F64, runtime_name: "slang_metric_p99".into() },
        BuiltinFn { name: "metric_count".into(),         params: vec![("name_hash", IrType::I64)], ret: IrType::I64, runtime_name: "slang_metric_count".into() },
        BuiltinFn { name: "metric_sum".into(),           params: vec![("name_hash", IrType::I64)], ret: IrType::F64, runtime_name: "slang_metric_sum".into() },
        BuiltinFn { name: "metric_reset".into(),         params: vec![], ret: IrType::I64, runtime_name: "slang_metric_reset".into() },

        // Log Aggregator (v395)
        BuiltinFn { name: "agg_create".into(),           params: vec![("max_entries", IrType::I64)], ret: IrType::I64, runtime_name: "slang_agg_create".into() },
        BuiltinFn { name: "agg_ingest".into(),           params: vec![("id", IrType::I64), ("level", IrType::I64), ("msg_hash", IrType::I64)], ret: IrType::I64, runtime_name: "slang_agg_ingest".into() },
        BuiltinFn { name: "agg_query".into(),            params: vec![("id", IrType::I64), ("pattern_hash", IrType::I64)], ret: IrType::I64, runtime_name: "slang_agg_query".into() },
        BuiltinFn { name: "agg_count".into(),            params: vec![("id", IrType::I64)], ret: IrType::I64, runtime_name: "slang_agg_count".into() },
        BuiltinFn { name: "agg_filter_level".into(),     params: vec![("id", IrType::I64), ("min_level", IrType::I64)], ret: IrType::I64, runtime_name: "slang_agg_filter_level".into() },
        BuiltinFn { name: "agg_last_n".into(),           params: vec![("id", IrType::I64), ("n", IrType::I64)], ret: IrType::I64, runtime_name: "slang_agg_last_n".into() },
        BuiltinFn { name: "agg_clear".into(),            params: vec![("id", IrType::I64)], ret: IrType::I64, runtime_name: "slang_agg_clear".into() },
        BuiltinFn { name: "agg_pattern_match".into(),    params: vec![("id", IrType::I64), ("pattern_hash", IrType::I64)], ret: IrType::I64, runtime_name: "slang_agg_pattern_match".into() },

        // ML Pipeline (v396)
        BuiltinFn { name: "pipe_create".into(),          params: vec![], ret: IrType::I64, runtime_name: "slang_pipe_create".into() },
        BuiltinFn { name: "pipe_add_stage".into(),       params: vec![("id", IrType::I64), ("kind", IrType::I64)], ret: IrType::I64, runtime_name: "slang_pipe_add_stage".into() },
        BuiltinFn { name: "pipe_execute".into(),         params: vec![("id", IrType::I64), ("data_len", IrType::I64)], ret: IrType::I64, runtime_name: "slang_pipe_execute".into() },
        BuiltinFn { name: "pipe_stage_count".into(),     params: vec![("id", IrType::I64)], ret: IrType::I64, runtime_name: "slang_pipe_stage_count".into() },
        BuiltinFn { name: "pipe_remove_stage".into(),    params: vec![("id", IrType::I64), ("stage_id", IrType::I64)], ret: IrType::I64, runtime_name: "slang_pipe_remove_stage".into() },
        BuiltinFn { name: "pipe_validate".into(),        params: vec![("id", IrType::I64)], ret: IrType::I64, runtime_name: "slang_pipe_validate".into() },
        BuiltinFn { name: "pipe_status".into(),          params: vec![("id", IrType::I64)], ret: IrType::I64, runtime_name: "slang_pipe_status".into() },
        BuiltinFn { name: "pipe_reset".into(),           params: vec![("id", IrType::I64)], ret: IrType::I64, runtime_name: "slang_pipe_reset".into() },

        // ── ERA I: Cognitive Compiler (v601–v700) ──────────────────────

        // Semantic Graph (v601)
        BuiltinFn { name: "semgraph_reaching_defs".into(), params: vec![("node_count", IrType::I64), ("edge_count", IrType::I64)], ret: IrType::I64, runtime_name: "vitalis_semgraph_reaching_defs".into() },
        BuiltinFn { name: "semgraph_cyclomatic".into(),    params: vec![("edges", IrType::I64), ("nodes", IrType::I64), ("components", IrType::I64)], ret: IrType::I64, runtime_name: "vitalis_semgraph_cyclomatic".into() },

        // Concept Extraction (v602)
        BuiltinFn { name: "concept_feature_sim".into(),    params: vec![("dim", IrType::I64)], ret: IrType::F64, runtime_name: "vitalis_concept_feature_similarity".into() },
        BuiltinFn { name: "concept_count_unique".into(),   params: vec![("total", IrType::I64)], ret: IrType::I64, runtime_name: "vitalis_concept_count_unique".into() },

        // Code Reasoning (v603)
        BuiltinFn { name: "sign_from_value".into(),        params: vec![("value", IrType::I64)], ret: IrType::I64, runtime_name: "vitalis_sign_from_value".into() },

        // Intent Understanding (v605)
        BuiltinFn { name: "intent_is_pure".into(),         params: vec![("intent_id", IrType::I64)], ret: IrType::I64, runtime_name: "vitalis_intent_is_pure".into() },
        BuiltinFn { name: "intent_has_side_effects".into(), params: vec![("intent_id", IrType::I64)], ret: IrType::I64, runtime_name: "vitalis_intent_has_side_effects".into() },

        // Natural Language Spec (v606)
        BuiltinFn { name: "nlspec_completeness".into(),    params: vec![("precond", IrType::I64), ("postcond", IrType::I64), ("has_params", IrType::I64)], ret: IrType::F64, runtime_name: "vitalis_nlspec_completeness".into() },

        // Code Analogy (v607)
        BuiltinFn { name: "analogy_similarity".into(),     params: vec![("shared", IrType::I64), ("total", IrType::I64)], ret: IrType::F64, runtime_name: "vitalis_analogy_similarity".into() },

        // Temporal Reasoning (v608)
        BuiltinFn { name: "temporal_happens_before".into(), params: vec![("from", IrType::I64), ("to", IrType::I64)], ret: IrType::I64, runtime_name: "vitalis_temporal_happens_before".into() },

        // Causal Inference (v609)
        BuiltinFn { name: "causal_influence".into(),       params: vec![("source", IrType::I64), ("target", IrType::I64)], ret: IrType::F64, runtime_name: "vitalis_causal_influence".into() },

        // Compiler Introspection (v626)
        BuiltinFn { name: "introspect_throughput".into(),  params: vec![("instructions", IrType::I64), ("duration_us", IrType::I64)], ret: IrType::F64, runtime_name: "vitalis_introspect_throughput".into() },

        // Adaptive Pipeline (v627)
        BuiltinFn { name: "adaptive_pass_cost".into(),     params: vec![("pass_id", IrType::I64)], ret: IrType::F64, runtime_name: "vitalis_adaptive_pass_cost_effectiveness".into() },

        // Workload Prediction (v628)
        BuiltinFn { name: "workload_classify".into(),      params: vec![("instruction_count", IrType::I64)], ret: IrType::I64, runtime_name: "vitalis_workload_classify".into() },
        BuiltinFn { name: "workload_parallelizability".into(), params: vec![("loops", IrType::I64), ("branches", IrType::I64), ("calls", IrType::I64)], ret: IrType::F64, runtime_name: "vitalis_workload_parallelizability".into() },

        // Compilation Learning (v629)
        BuiltinFn { name: "learning_bandit_score".into(),  params: vec![("arm", IrType::I64), ("alpha", IrType::I64), ("beta", IrType::I64)], ret: IrType::F64, runtime_name: "vitalis_learning_bandit_score".into() },

        // Architecture Advisor (v630)
        BuiltinFn { name: "arch_instability".into(),       params: vec![("fan_in", IrType::I64), ("fan_out", IrType::I64)], ret: IrType::F64, runtime_name: "vitalis_arch_instability".into() },
        BuiltinFn { name: "arch_quality".into(),           params: vec![("avg_distance", IrType::F64), ("has_cycles", IrType::I64), ("max_fan_out", IrType::I64)], ret: IrType::F64, runtime_name: "vitalis_arch_quality".into() },

        // Performance Prophecy (v631)
        BuiltinFn { name: "perf_complexity_ops".into(),    params: vec![("severity", IrType::I64), ("n", IrType::I64)], ret: IrType::F64, runtime_name: "vitalis_perf_complexity_ops".into() },
        BuiltinFn { name: "perf_cache_miss".into(),        params: vec![("pattern_id", IrType::I64), ("cache_line", IrType::I64), ("elem_size", IrType::I64)], ret: IrType::F64, runtime_name: "vitalis_perf_cache_miss_rate".into() },
        BuiltinFn { name: "perf_prophecy_score".into(),    params: vec![("complexity", IrType::I64), ("miss_rate", IrType::F64), ("branch_bias", IrType::F64)], ret: IrType::F64, runtime_name: "vitalis_perf_prophecy_score".into() },

        // Optimization Invention (v632)
        BuiltinFn { name: "opt_priority_score".into(),     params: vec![("confidence", IrType::F64), ("speedup", IrType::F64)], ret: IrType::F64, runtime_name: "vitalis_opt_invent_priority_score".into() },
        BuiltinFn { name: "opt_identity_count".into(),     params: vec![], ret: IrType::I64, runtime_name: "vitalis_opt_invent_identity_count".into() },

        // Root Cause Analysis (v652)
        BuiltinFn { name: "rca_cause_count".into(),        params: vec![("category_id", IrType::I64)], ret: IrType::I64, runtime_name: "vitalis_rca_typical_cause_count".into() },
        BuiltinFn { name: "rca_fault_prob".into(),         params: vec![("p1", IrType::F64), ("p2", IrType::F64), ("is_and", IrType::I64)], ret: IrType::F64, runtime_name: "vitalis_rca_fault_tree_prob".into() },

        // Fault Localization (v653)
        BuiltinFn { name: "fault_tarantula".into(),        params: vec![("ef", IrType::I64), ("ep", IrType::I64), ("nf", IrType::I64), ("np", IrType::I64)], ret: IrType::F64, runtime_name: "vitalis_fault_tarantula".into() },
        BuiltinFn { name: "fault_ochiai".into(),           params: vec![("ef", IrType::I64), ("ep", IrType::I64), ("nf", IrType::I64), ("np", IrType::I64)], ret: IrType::F64, runtime_name: "vitalis_fault_ochiai".into() },

        // Auto Fix Engine (v654)
        BuiltinFn { name: "autofix_confidence".into(),     params: vec![("fix_kind_id", IrType::I64)], ret: IrType::F64, runtime_name: "vitalis_autofix_confidence".into() },
        BuiltinFn { name: "autofix_patch_score".into(),    params: vec![("confidence", IrType::F64), ("edit_count", IrType::I64), ("passes", IrType::I64)], ret: IrType::F64, runtime_name: "vitalis_autofix_patch_score".into() },

        // Regression Prevention (v655)
        BuiltinFn { name: "regression_z_score".into(),     params: vec![("mean", IrType::F64), ("std_dev", IrType::F64), ("value", IrType::F64)], ret: IrType::F64, runtime_name: "vitalis_regression_z_score".into() },
        BuiltinFn { name: "regression_impact".into(),      params: vec![("changed", IrType::I64), ("tests", IrType::I64), ("deps", IrType::I64)], ret: IrType::F64, runtime_name: "vitalis_regression_impact_score".into() },

        // Specification Mining (v656)
        BuiltinFn { name: "specmine_r_squared".into(),     params: vec![("ss_res", IrType::F64), ("ss_tot", IrType::F64)], ret: IrType::F64, runtime_name: "vitalis_specmine_linear_r_squared".into() },

        // Debug Narrative (v657)
        BuiltinFn { name: "narrative_is_blocking".into(),  params: vec![("severity_id", IrType::I64)], ret: IrType::I64, runtime_name: "vitalis_narrative_severity_is_blocking".into() },

        // Zero-Bug Certification (v658)
        BuiltinFn { name: "cert_difficulty".into(),        params: vec![("property_id", IrType::I64)], ret: IrType::I64, runtime_name: "vitalis_cert_difficulty".into() },
        BuiltinFn { name: "cert_level".into(),             params: vec![("verified_pct", IrType::F64), ("has_violations", IrType::I64)], ret: IrType::I64, runtime_name: "vitalis_cert_level".into() },
    ]
}

/// Returns a mapping of user-visible names → runtime symbol names.
pub fn builtin_aliases() -> HashMap<String, String> {
    let mut map = HashMap::new();
    for b in builtins() {
        map.insert(b.name, b.runtime_name);
    }
    map
}

/// Returns a mapping of user-visible names → IR-level signatures.
/// Used by `IrBuilder::new()` so IR signatures are auto-derived from stdlib.
pub fn builtin_ir_sigs() -> HashMap<String, (Vec<IrType>, IrType)> {
    let mut map = HashMap::new();
    for b in builtins() {
        let param_types: Vec<IrType> = b.params.into_iter().map(|(_, t)| t).collect();
        map.insert(b.name, (param_types, b.ret));
    }
    map
}

/// Convert an `IrType` to the high-level `Type` used by the type checker.
pub fn ir_type_to_type(ir: &IrType) -> crate::types::Type {
    use crate::types::Type;
    match ir {
        IrType::I32 => Type::I32,
        IrType::I64 => Type::I64,
        IrType::F32 => Type::F32,
        IrType::F64 => Type::F64,
        IrType::Bool => Type::Bool,
        IrType::Ptr => Type::Str, // Ptr maps to Str at the type-checker level
        IrType::Void => Type::Void,
    }
}

/// Register all stdlib builtins into a type checker.
/// Call this from `TypeChecker::new()` to avoid duplicating builtin definitions.
pub fn register_builtins_for_typechecker(
    register: &mut dyn FnMut(&str, Vec<crate::types::Type>, crate::types::Type),
) {
    for b in builtins() {
        let params: Vec<crate::types::Type> = b.params.iter().map(|(_, t)| ir_type_to_type(t)).collect();
        let ret = ir_type_to_type(&b.ret);
        register(&b.name, params, ret);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_duplicate_builtin_names() {
        let all = builtins();
        let mut seen = std::collections::HashSet::new();
        let mut dups = Vec::new();
        for b in &all {
            if !seen.insert(&b.name) {
                dups.push(b.name.clone());
            }
        }
        assert!(dups.is_empty(), "duplicate builtin names: {:?}", dups);
    }

    #[test]
    fn test_builtins_non_empty() {
        assert!(builtins().len() > 100, "expected 100+ builtins");
    }

    #[test]
    fn test_builtin_aliases_match_count() {
        let all = builtins();
        let aliases = builtin_aliases();
        // aliases is a HashMap so duplicates would collapse — count should match
        assert_eq!(all.len(), aliases.len(), "duplicate names detected: builtins() has {} entries but aliases has {}", all.len(), aliases.len());
    }

    #[test]
    fn test_core_builtins_present() {
        let aliases = builtin_aliases();
        for name in &["println", "print", "sqrt", "abs", "to_f64", "to_i64", "str_len", "array_len", "array_push"] {
            assert!(aliases.contains_key(*name), "missing core builtin: {}", name);
        }
    }

    // ── v129: Unified Registry Tests ────────────────────────────────

    #[test]
    fn test_v129_ir_sigs_match_builtins() {
        let all = builtins();
        let sigs = builtin_ir_sigs();
        assert_eq!(all.len(), sigs.len(), "ir_sigs count should match builtins count");
        for b in &all {
            assert!(sigs.contains_key(&b.name), "ir_sigs missing: {}", b.name);
        }
    }

    #[test]
    fn test_v129_ir_sig_param_count_matches() {
        let sigs = builtin_ir_sigs();
        for b in builtins() {
            let (params, _) = sigs.get(&b.name).unwrap();
            assert_eq!(params.len(), b.params.len(),
                "param count mismatch for {}: ir_sigs has {} but builtin has {}",
                b.name, params.len(), b.params.len());
        }
    }

    #[test]
    fn test_v129_type_checker_registration() {
        let mut count = 0usize;
        register_builtins_for_typechecker(&mut |_name, _params, _ret| {
            count += 1;
        });
        let all = builtins();
        assert_eq!(count, all.len(), "type checker should register all builtins");
    }

    #[test]
    fn test_v129_ir_type_to_type_mapping() {
        use crate::ir::IrType;
        use crate::types::Type;
        assert!(matches!(ir_type_to_type(&IrType::I64), Type::I64));
        assert!(matches!(ir_type_to_type(&IrType::F64), Type::F64));
        assert!(matches!(ir_type_to_type(&IrType::Bool), Type::Bool));
        assert!(matches!(ir_type_to_type(&IrType::Ptr), Type::Str));
        assert!(matches!(ir_type_to_type(&IrType::Void), Type::Void));
    }
}
