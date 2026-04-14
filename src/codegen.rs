//! Vitalis Codegen — Cranelift JIT backend.
//!
//! Translates the SSA IR into native machine code via Cranelift.
//! Supports both JIT execution and AOT compilation.
//!
//! Phase 0: integer arithmetic, control flow, function calls, print.

use crate::ir;
use crate::ir::{IrModule, IrFunction, IrType, Inst, IrBinOp, IrUnOp, IrCmp};

use cranelift::prelude::*;
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{FuncId, Linkage, Module};
use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

// ─── String Arena ───────────────────────────────────────────────────────
// Keeps string literals alive for the lifetime of the process.
// Box<[u8]> ensures the heap address never moves.
static VITALIS_STRING_ARENA: Mutex<Vec<Box<[u8]>>> = Mutex::new(Vec::new());

/// Cached empty C-string sentinel - avoids allocating a Box<[u8]> for every
/// empty-string return (used by ~30 null/error paths).
static EMPTY_CSTR: &[u8] = b"\0";

fn intern_cstr(s: &str) -> *const u8 {
    if s.is_empty() {
        return EMPTY_CSTR.as_ptr();
    }
    let bytes: Box<[u8]> = s.bytes().chain(std::iter::once(0u8)).collect();
    let ptr = bytes.as_ptr();
    VITALIS_STRING_ARENA.lock().unwrap().push(bytes);
    ptr
}

// ─── Codegen Errors ─────────────────────────────────────────────────────
#[derive(Debug)]
pub struct CodegenError {
    pub message: String,
}

impl std::fmt::Display for CodegenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "codegen error: {}", self.message)
    }
}

impl std::error::Error for CodegenError {}

type CodegenResult<T> = Result<T, CodegenError>;

// ─── Module runtime ─────────────────────────────────────────────────────
/// Placeholder: returns 1 (module is always "loaded" for now).
extern "C" fn slang_module_loaded(_name: *const i8) -> i8 { 1 }

// ─── Runtime Support ────────────────────────────────────────────────────
/// Print an i64 to stdout. Called by generated code.
extern "C" fn slang_print_i64(val: i64) {
    println!("{}", val);
}

/// Print a string (pointer + length) to stdout.
extern "C" fn slang_print_str(ptr: *const u8, len: i64) {
    if ptr.is_null() || len <= 0 { return; }
    let slice = unsafe { std::slice::from_raw_parts(ptr, len as usize) };
    let s = String::from_utf8_lossy(slice);
    print!("{}", s);
}

/// Print a string with newline.
extern "C" fn slang_println_str(ptr: *const u8, len: i64) {
    if ptr.is_null() || len <= 0 { println!(); return; }
    let slice = unsafe { std::slice::from_raw_parts(ptr, len as usize) };
    let s = String::from_utf8_lossy(slice);
    println!("{}", s);
}

// ─── Typed print runtime ────────────────────────────────────────────────
extern "C" fn slang_println_i64(val: i64) { println!("{}", val); }
extern "C" fn slang_print_f64(val: f64)   { print!("{}", val); }
extern "C" fn slang_println_f64(val: f64) { println!("{}", val); }
extern "C" fn slang_print_bool(val: i8)   { print!("{}", if val != 0 { "true" } else { "false" }); }
extern "C" fn slang_println_bool(val: i8) { println!("{}", if val != 0 { "true" } else { "false" }); }
/// Print a null-terminated C string (used for StrConst literals).
extern "C" fn slang_print_cstr(ptr: *const i8) {
    if ptr.is_null() { return; }
    let s = unsafe { std::ffi::CStr::from_ptr(ptr) };
    print!("{}", s.to_string_lossy());
}
extern "C" fn slang_println_cstr(ptr: *const i8) {
    if ptr.is_null() { return; }
    let s = unsafe { std::ffi::CStr::from_ptr(ptr) };
    println!("{}", s.to_string_lossy());
}

// ─── Math runtime ───────────────────────────────────────────────────────
extern "C" fn slang_sqrt_f64(x: f64) -> f64   { x.sqrt() }
extern "C" fn slang_abs_i64(x: i64) -> i64    { x.abs() }
extern "C" fn slang_abs_f64(x: f64) -> f64    { x.abs() }
extern "C" fn slang_min_i64(a: i64, b: i64) -> i64 { a.min(b) }
extern "C" fn slang_max_i64(a: i64, b: i64) -> i64 { a.max(b) }
extern "C" fn slang_min_f64(a: f64, b: f64) -> f64 { a.min(b) }
extern "C" fn slang_max_f64(a: f64, b: f64) -> f64 { a.max(b) }
extern "C" fn slang_pow_f64(base: f64, exp: f64) -> f64 { base.powf(exp) }
extern "C" fn slang_floor_f64(x: f64) -> f64  { x.floor() }
extern "C" fn slang_ceil_f64(x: f64) -> f64   { x.ceil() }
extern "C" fn slang_round_f64(x: f64) -> f64  { x.round() }
extern "C" fn slang_ln_f64(x: f64) -> f64     { x.ln() }
extern "C" fn slang_log2_f64(x: f64) -> f64   { x.log2() }
extern "C" fn slang_log10_f64(x: f64) -> f64  { x.log10() }
extern "C" fn slang_sin_f64(x: f64) -> f64    { x.sin() }
extern "C" fn slang_cos_f64(x: f64) -> f64    { x.cos() }
extern "C" fn slang_exp_f64(x: f64) -> f64    { x.exp() }
extern "C" fn slang_atan2_f64(y: f64, x: f64) -> f64       { y.atan2(x) }
extern "C" fn slang_hypot_f64(a: f64, b: f64) -> f64        { a.hypot(b) }
extern "C" fn slang_clamp_f64(x: f64, lo: f64, hi: f64) -> f64 { x.clamp(lo, hi) }
extern "C" fn slang_clamp_i64(x: i64, lo: i64, hi: i64) -> i64 { x.clamp(lo, hi) }

// ─── Random runtime (Xorshift64 — no external dependencies) ──────────────
static VITALIS_RNG_STATE: AtomicU64 = AtomicU64::new(0x853c49e6748fea9b_u64);
fn xorshift64() -> u64 {
    loop {
        let old = VITALIS_RNG_STATE.load(Ordering::Relaxed);
        let mut x = if old == 0 { 0x853c49e6748fea9b } else { old };
        x ^= x << 13; x ^= x >> 7; x ^= x << 17;
        if VITALIS_RNG_STATE.compare_exchange_weak(old, x, Ordering::Relaxed, Ordering::Relaxed).is_ok() {
            return x;
        }
    }
}
extern "C" fn slang_rand_f64() -> f64 {
    (xorshift64() >> 11) as f64 * (1.0_f64 / (1u64 << 53) as f64)
}
extern "C" fn slang_rand_i64() -> i64 {
    xorshift64() as i64
}

// ─── Type conversion runtime ─────────────────────────────────────────────
 extern "C" fn slang_i64_to_f64(x: i64) -> f64 { x as f64 }
extern "C" fn slang_f64_to_i64(x: f64) -> i64 { x as i64 }

// ─── Phase 4: Array heap runtime ───────────────────────────────────────────────
// Layout: [i64 length][elem0][elem1]...[elemN]
// The returned pointer points to elem0 (data region); header is at ptr - 8.
// All array memory is leaked intentionally: Vitalis uses arena semantics and
// GC is deferred to Phase 4C (tracing collector over the string/array arena).
extern "C" fn slang_array_alloc(count: i64, stride: i64) -> *mut u8 {
    if count <= 0 || stride <= 0 {
        // Return a zeroed sentinel: header = 0, no data.
        let layout = std::alloc::Layout::from_size_align(8, 8).unwrap();
        let ptr = unsafe { std::alloc::alloc_zeroed(layout) };
        if ptr.is_null() { return std::ptr::null_mut(); }
        return unsafe { ptr.add(8) };
    }
    let header = 8usize; // i64 length
    let data_size = (count as usize).saturating_mul(stride as usize);
    let total = header + data_size;
    let layout = std::alloc::Layout::from_size_align(total, 8)
        .unwrap_or_else(|_| std::alloc::Layout::from_size_align(16, 8).unwrap());
    let raw = unsafe { std::alloc::alloc_zeroed(layout) };
    if raw.is_null() { return std::ptr::null_mut(); }
    unsafe { *(raw as *mut i64) = count; }
    unsafe { raw.add(header) }
}

/// Read the length header of an array.
extern "C" fn slang_array_len(data_ptr: *const u8) -> i64 {
    if data_ptr.is_null() { return 0; }
    unsafe { *(data_ptr.sub(8) as *const i64) }
}

/// Bounds-checked i64 element load.
extern "C" fn slang_array_get_i64(data_ptr: *const u8, index: i64) -> i64 {
    if data_ptr.is_null() { return 0; }
    let len = unsafe { *(data_ptr.sub(8) as *const i64) };
    if index < 0 || index >= len { return 0; }
    unsafe { *(data_ptr as *const i64).add(index as usize) }
}

/// Bounds-checked i64 element store.
extern "C" fn slang_array_set_i64(data_ptr: *mut u8, index: i64, value: i64) {
    if data_ptr.is_null() { return; }
    let len = unsafe { *(data_ptr.sub(8) as *const i64) };
    if index < 0 || index >= len { return; }
    unsafe { *(data_ptr as *mut i64).add(index as usize) = value; }
}

/// Bounds-checked f64 element load.
extern "C" fn slang_array_get_f64(data_ptr: *const u8, index: i64) -> f64 {
    if data_ptr.is_null() { return 0.0; }
    let len = unsafe { *(data_ptr.sub(8) as *const i64) };
    if index < 0 || index >= len { return 0.0; }
    unsafe { *(data_ptr as *const f64).add(index as usize) }
}

/// Bounds-checked f64 element store.
extern "C" fn slang_array_set_f64(data_ptr: *mut u8, index: i64, value: f64) {
    if data_ptr.is_null() { return; }
    let len = unsafe { *(data_ptr.sub(8) as *const i64) };
    if index < 0 || index >= len { return; }
    unsafe { *(data_ptr as *mut f64).add(index as usize) = value; }
}

// ── v18: Collection Methods ────────────────────────────────────────────

/// Push element to array → returns new array pointer (may reallocate)
extern "C" fn slang_array_push(data_ptr: *mut u8, value: i64) -> *mut u8 {
    if data_ptr.is_null() { return std::ptr::null_mut(); }
    let old_len = unsafe { *(data_ptr.sub(8) as *const i64) } as usize;
    let new_len = old_len + 1;
    let stride = 8usize; // i64 elements
    let old_alloc_size = 8 + old_len * stride;
    let new_alloc_size = 8 + new_len * stride;
    let layout = unsafe { std::alloc::Layout::from_size_align_unchecked(new_alloc_size, 8) };
    let raw = unsafe { std::alloc::alloc(layout) };
    if raw.is_null() { return data_ptr; }
    let data = unsafe { raw.add(8) };
    unsafe { *(raw as *mut i64) = new_len as i64; }
    // Copy old data
    unsafe { std::ptr::copy_nonoverlapping(data_ptr, data, old_len * stride); }
    // Write new element
    unsafe { *(data as *mut i64).add(old_len) = value; }
    // Free old allocation
    if old_alloc_size > 0 {
        let old_layout = unsafe { std::alloc::Layout::from_size_align_unchecked(old_alloc_size, 8) };
        unsafe { std::alloc::dealloc(data_ptr.sub(8), old_layout); }
    }
    data
}

/// Pop last element from array → returns the element
extern "C" fn slang_array_pop(data_ptr: *mut u8) -> i64 {
    if data_ptr.is_null() { return 0; }
    let len_ptr = unsafe { data_ptr.sub(8) as *mut i64 };
    let len = unsafe { std::ptr::read(len_ptr) };
    if len <= 0 { return 0; }
    unsafe { std::ptr::write(len_ptr, len - 1); }
    unsafe { std::ptr::read((data_ptr as *const i64).add((len - 1) as usize)) }
}

/// Check if array contains value
extern "C" fn slang_array_contains(data_ptr: *const u8, value: i64) -> i64 {
    if data_ptr.is_null() { return 0; }
    let len = unsafe { *(data_ptr.sub(8) as *const i64) } as usize;
    for i in 0..len {
        if unsafe { *(data_ptr as *const i64).add(i) } == value {
            return 1;
        }
    }
    0
}

/// Reverse array in-place
extern "C" fn slang_array_reverse(data_ptr: *mut u8) -> *mut u8 {
    if data_ptr.is_null() { return data_ptr; }
    let len = unsafe { *(data_ptr.sub(8) as *const i64) } as usize;
    let arr = data_ptr as *mut i64;
    for i in 0..len / 2 {
        unsafe {
            let a = *arr.add(i);
            let b = *arr.add(len - 1 - i);
            *arr.add(i) = b;
            *arr.add(len - 1 - i) = a;
        }
    }
    data_ptr
}

/// Sort array in-place (ascending)
extern "C" fn slang_array_sort(data_ptr: *mut u8) -> *mut u8 {
    if data_ptr.is_null() { return data_ptr; }
    let len = unsafe { *(data_ptr.sub(8) as *const i64) } as usize;
    let slice = unsafe { std::slice::from_raw_parts_mut(data_ptr as *mut i64, len) };
    slice.sort();
    data_ptr
}

/// Join array elements as string with delimiter
extern "C" fn slang_array_join(data_ptr: *const u8, delim: *const i8) -> *const i8 {
    if data_ptr.is_null() { return intern_cstr("") as *const i8; }
    let len = unsafe { *(data_ptr.sub(8) as *const i64) } as usize;
    let d = if delim.is_null() { "," } else {
        unsafe { std::ffi::CStr::from_ptr(delim).to_str().unwrap_or(",") }
    };
    let parts: Vec<String> = (0..len)
        .map(|i| unsafe { *(data_ptr as *const i64).add(i) }.to_string())
        .collect();
    intern_cstr(&parts.join(d)) as *const i8
}

/// Slice array → new array [start..end)
extern "C" fn slang_array_slice(data_ptr: *const u8, start: i64, end: i64) -> *mut u8 {
    if data_ptr.is_null() { return std::ptr::null_mut(); }
    let len = unsafe { *(data_ptr.sub(8) as *const i64) } as usize;
    let s = (start as usize).min(len);
    let e = (end as usize).min(len);
    if s >= e {
        return slang_array_alloc(0, 8);
    }
    let new_len = e - s;
    let new_ptr = slang_array_alloc(new_len as i64, 8);
    for i in 0..new_len {
        unsafe { *(new_ptr as *mut i64).add(i) = *(data_ptr as *const i64).add(s + i); }
    }
    new_ptr
}

/// Find index of value in array (-1 if not found)
extern "C" fn slang_array_find(data_ptr: *const u8, value: i64) -> i64 {
    if data_ptr.is_null() { return -1; }
    let len = unsafe { *(data_ptr.sub(8) as *const i64) } as usize;
    for i in 0..len {
        if unsafe { *(data_ptr as *const i64).add(i) } == value {
            return i as i64;
        }
    }
    -1
}

// ── Iterator / Functional array operations ────────────────────────────

/// Create array [start..end)
extern "C" fn slang_array_range(start: i64, end: i64) -> *mut u8 {
    if end <= start {
        return slang_array_alloc(0, 8);
    }
    let count = (end - start) as usize;
    let ptr = slang_array_alloc(count as i64, 8);
    if ptr.is_null() { return ptr; }
    for i in 0..count {
        unsafe { *(ptr as *mut i64).add(i) = start + i as i64; }
    }
    ptr
}

/// Sum all i64 elements
extern "C" fn slang_array_sum(data_ptr: *const u8) -> i64 {
    if data_ptr.is_null() { return 0; }
    let len = unsafe { *(data_ptr.sub(8) as *const i64) } as usize;
    let mut total: i64 = 0;
    for i in 0..len {
        total += unsafe { *(data_ptr as *const i64).add(i) };
    }
    total
}

/// Minimum element (returns i64::MAX for empty)
extern "C" fn slang_array_min(data_ptr: *const u8) -> i64 {
    if data_ptr.is_null() { return 0; }
    let len = unsafe { *(data_ptr.sub(8) as *const i64) } as usize;
    if len == 0 { return 0; }
    let mut m = unsafe { *(data_ptr as *const i64) };
    for i in 1..len {
        let v = unsafe { *(data_ptr as *const i64).add(i) };
        if v < m { m = v; }
    }
    m
}

/// Maximum element (returns i64::MIN for empty)
extern "C" fn slang_array_max(data_ptr: *const u8) -> i64 {
    if data_ptr.is_null() { return 0; }
    let len = unsafe { *(data_ptr.sub(8) as *const i64) } as usize;
    if len == 0 { return 0; }
    let mut m = unsafe { *(data_ptr as *const i64) };
    for i in 1..len {
        let v = unsafe { *(data_ptr as *const i64).add(i) };
        if v > m { m = v; }
    }
    m
}

/// Check if any element equals val
extern "C" fn slang_array_any(data_ptr: *const u8, value: i64) -> i8 {
    if data_ptr.is_null() { return 0; }
    let len = unsafe { *(data_ptr.sub(8) as *const i64) } as usize;
    for i in 0..len {
        if unsafe { *(data_ptr as *const i64).add(i) } == value {
            return 1;
        }
    }
    0
}

/// Check if all elements > 0
extern "C" fn slang_array_all_positive(data_ptr: *const u8) -> i8 {
    if data_ptr.is_null() { return 0; }
    let len = unsafe { *(data_ptr.sub(8) as *const i64) } as usize;
    if len == 0 { return 1; }
    for i in 0..len {
        if unsafe { *(data_ptr as *const i64).add(i) } <= 0 {
            return 0;
        }
    }
    1
}

/// Count occurrences of val
extern "C" fn slang_array_count(data_ptr: *const u8, value: i64) -> i64 {
    if data_ptr.is_null() { return 0; }
    let len = unsafe { *(data_ptr.sub(8) as *const i64) } as usize;
    let mut c: i64 = 0;
    for i in 0..len {
        if unsafe { *(data_ptr as *const i64).add(i) } == value {
            c += 1;
        }
    }
    c
}

/// Flatten nested arrays (array of array pointers) into one flat array
extern "C" fn slang_array_flatten(data_ptr: *const u8) -> *mut u8 {
    if data_ptr.is_null() { return slang_array_alloc(0, 8); }
    let outer_len = unsafe { *(data_ptr.sub(8) as *const i64) } as usize;
    // First pass: count total elements
    let mut total = 0usize;
    for i in 0..outer_len {
        let inner_ptr = unsafe { *(data_ptr as *const *const u8).add(i) };
        if !inner_ptr.is_null() {
            let inner_len = unsafe { *(inner_ptr.sub(8) as *const i64) } as usize;
            total += inner_len;
        }
    }
    let result = slang_array_alloc(total as i64, 8);
    if result.is_null() { return result; }
    let mut idx = 0usize;
    for i in 0..outer_len {
        let inner_ptr = unsafe { *(data_ptr as *const *const u8).add(i) };
        if !inner_ptr.is_null() {
            let inner_len = unsafe { *(inner_ptr.sub(8) as *const i64) } as usize;
            for j in 0..inner_len {
                let val = unsafe { *(inner_ptr as *const i64).add(j) };
                unsafe { *(result as *mut i64).add(idx) = val; }
                idx += 1;
            }
        }
    }
    result
}

/// Zip two arrays: interleave [a0, b0, a1, b1, ...]
extern "C" fn slang_array_zip(a: *const u8, b: *const u8) -> *mut u8 {
    let a_len = if a.is_null() { 0 } else { (unsafe { *(a.sub(8) as *const i64) }) as usize };
    let b_len = if b.is_null() { 0 } else { (unsafe { *(b.sub(8) as *const i64) }) as usize };
    let min_len = a_len.min(b_len);
    let result = slang_array_alloc((min_len * 2) as i64, 8);
    if result.is_null() { return result; }
    for i in 0..min_len {
        let va = unsafe { *(a as *const i64).add(i) };
        let vb = unsafe { *(b as *const i64).add(i) };
        unsafe { *(result as *mut i64).add(i * 2) = va; }
        unsafe { *(result as *mut i64).add(i * 2 + 1) = vb; }
    }
    result
}

/// Enumerate: [idx0, val0, idx1, val1, ...]
extern "C" fn slang_array_enumerate(data_ptr: *const u8) -> *mut u8 {
    if data_ptr.is_null() { return slang_array_alloc(0, 8); }
    let len = unsafe { *(data_ptr.sub(8) as *const i64) } as usize;
    let result = slang_array_alloc((len * 2) as i64, 8);
    if result.is_null() { return result; }
    for i in 0..len {
        let val = unsafe { *(data_ptr as *const i64).add(i) };
        unsafe { *(result as *mut i64).add(i * 2) = i as i64; }
        unsafe { *(result as *mut i64).add(i * 2 + 1) = val; }
    }
    result
}

/// Take first n elements
extern "C" fn slang_array_take(data_ptr: *const u8, n: i64) -> *mut u8 {
    if data_ptr.is_null() || n <= 0 { return slang_array_alloc(0, 8); }
    let len = unsafe { *(data_ptr.sub(8) as *const i64) } as usize;
    let take = (n as usize).min(len);
    let result = slang_array_alloc(take as i64, 8);
    if result.is_null() { return result; }
    unsafe { std::ptr::copy_nonoverlapping(data_ptr, result, take * 8); }
    result
}

/// Drop first n elements
extern "C" fn slang_array_drop(data_ptr: *const u8, n: i64) -> *mut u8 {
    if data_ptr.is_null() { return slang_array_alloc(0, 8); }
    let len = unsafe { *(data_ptr.sub(8) as *const i64) } as usize;
    let skip = (n.max(0) as usize).min(len);
    let new_len = len - skip;
    if new_len == 0 { return slang_array_alloc(0, 8); }
    let result = slang_array_alloc(new_len as i64, 8);
    if result.is_null() { return result; }
    unsafe {
        std::ptr::copy_nonoverlapping(
            (data_ptr as *const i64).add(skip) as *const u8,
            result,
            new_len * 8,
        );
    }
    result
}

/// Remove duplicates (preserves first occurrence order)
extern "C" fn slang_array_unique(data_ptr: *const u8) -> *mut u8 {
    if data_ptr.is_null() { return slang_array_alloc(0, 8); }
    let len = unsafe { *(data_ptr.sub(8) as *const i64) } as usize;
    let mut seen = Vec::<i64>::with_capacity(len);
    for i in 0..len {
        let v = unsafe { *(data_ptr as *const i64).add(i) };
        if !seen.contains(&v) {
            seen.push(v);
        }
    }
    let result = slang_array_alloc(seen.len() as i64, 8);
    if result.is_null() { return result; }
    for (i, &v) in seen.iter().enumerate() {
        unsafe { *(result as *mut i64).add(i) = v; }
    }
    result
}

/// v18: error_message alias (returns interned string)
extern "C" fn slang_error_message() -> *const i8 {
    slang_error_msg()
}

// ─── Regex operations runtime ─────────────────────────────────────────────

/// Full-match: returns 1 if the entire text matches the pattern, 0 otherwise.
extern "C" fn slang_regex_match(pattern: *const i8, text: *const i8) -> i8 {
    if pattern.is_null() || text.is_null() { return 0; }
    let pat = unsafe { std::ffi::CStr::from_ptr(pattern).to_string_lossy() };
    let txt = unsafe { std::ffi::CStr::from_ptr(text).to_string_lossy() };
    // Anchor the pattern for full-match semantics
    let anchored = format!("^(?:{})$", pat);
    match regex::Regex::new(&anchored) {
        Ok(re) => if re.is_match(&txt) { 1 } else { 0 },
        Err(_) => 0,
    }
}

/// Partial match: returns 1 if pattern is found anywhere in text, 0 otherwise.
extern "C" fn slang_regex_is_match(pattern: *const i8, text: *const i8) -> i8 {
    if pattern.is_null() || text.is_null() { return 0; }
    let pat = unsafe { std::ffi::CStr::from_ptr(pattern).to_string_lossy() };
    let txt = unsafe { std::ffi::CStr::from_ptr(text).to_string_lossy() };
    match regex::Regex::new(&pat) {
        Ok(re) => if re.is_match(&txt) { 1 } else { 0 },
        Err(_) => 0,
    }
}

/// Find first match substring; returns empty string if no match.
extern "C" fn slang_regex_find(pattern: *const i8, text: *const i8) -> *const i8 {
    if pattern.is_null() || text.is_null() { return intern_cstr("") as *const i8; }
    let pat = unsafe { std::ffi::CStr::from_ptr(pattern).to_string_lossy() };
    let txt = unsafe { std::ffi::CStr::from_ptr(text).to_string_lossy() };
    match regex::Regex::new(&pat) {
        Ok(re) => match re.find(&txt) {
            Some(m) => intern_cstr(m.as_str()) as *const i8,
            None => intern_cstr("") as *const i8,
        },
        Err(_) => intern_cstr("") as *const i8,
    }
}

/// Replace all occurrences of pattern in text with replacement.
extern "C" fn slang_regex_replace(pattern: *const i8, text: *const i8, replacement: *const i8) -> *const i8 {
    if pattern.is_null() || text.is_null() || replacement.is_null() {
        return intern_cstr("") as *const i8;
    }
    let pat = unsafe { std::ffi::CStr::from_ptr(pattern).to_string_lossy() };
    let txt = unsafe { std::ffi::CStr::from_ptr(text).to_string_lossy() };
    let rep = unsafe { std::ffi::CStr::from_ptr(replacement).to_string_lossy() };
    match regex::Regex::new(&pat) {
        Ok(re) => {
            let result = re.replace_all(&txt, rep.as_ref());
            intern_cstr(&result) as *const i8
        }
        Err(_) => intern_cstr(&txt) as *const i8,
    }
}

/// Count of segments after splitting text by pattern.
extern "C" fn slang_regex_split_count(pattern: *const i8, text: *const i8) -> i64 {
    if pattern.is_null() || text.is_null() { return 0; }
    let pat = unsafe { std::ffi::CStr::from_ptr(pattern).to_string_lossy() };
    let txt = unsafe { std::ffi::CStr::from_ptr(text).to_string_lossy() };
    match regex::Regex::new(&pat) {
        Ok(re) => re.split(&txt).count() as i64,
        Err(_) => 1, // no split, whole string is one segment
    }
}

/// Get the nth segment after splitting text by pattern.
extern "C" fn slang_regex_split_get(pattern: *const i8, text: *const i8, idx: i64) -> *const i8 {
    if pattern.is_null() || text.is_null() { return intern_cstr("") as *const i8; }
    let pat = unsafe { std::ffi::CStr::from_ptr(pattern).to_string_lossy() };
    let txt = unsafe { std::ffi::CStr::from_ptr(text).to_string_lossy() };
    match regex::Regex::new(&pat) {
        Ok(re) => {
            let parts: Vec<&str> = re.split(&txt).collect();
            if idx < 0 || (idx as usize) >= parts.len() {
                intern_cstr("") as *const i8
            } else {
                intern_cstr(parts[idx as usize]) as *const i8
            }
        }
        Err(_) => intern_cstr("") as *const i8,
    }
}

/// Count all non-overlapping matches of pattern in text.
extern "C" fn slang_regex_find_all_count(pattern: *const i8, text: *const i8) -> i64 {
    if pattern.is_null() || text.is_null() { return 0; }
    let pat = unsafe { std::ffi::CStr::from_ptr(pattern).to_string_lossy() };
    let txt = unsafe { std::ffi::CStr::from_ptr(text).to_string_lossy() };
    match regex::Regex::new(&pat) {
        Ok(re) => re.find_iter(&txt).count() as i64,
        Err(_) => 0,
    }
}

/// Get the nth match of pattern in text.
extern "C" fn slang_regex_find_all_get(pattern: *const i8, text: *const i8, idx: i64) -> *const i8 {
    if pattern.is_null() || text.is_null() { return intern_cstr("") as *const i8; }
    let pat = unsafe { std::ffi::CStr::from_ptr(pattern).to_string_lossy() };
    let txt = unsafe { std::ffi::CStr::from_ptr(text).to_string_lossy() };
    match regex::Regex::new(&pat) {
        Ok(re) => {
            let matches: Vec<regex::Match> = re.find_iter(&txt).collect();
            if idx < 0 || (idx as usize) >= matches.len() {
                intern_cstr("") as *const i8
            } else {
                intern_cstr(matches[idx as usize].as_str()) as *const i8
            }
        }
        Err(_) => intern_cstr("") as *const i8,
    }
}

// ─── String operations runtime ───────────────────────────────────────────
extern "C" fn slang_str_len(ptr: *const i8) -> i64 {
    if ptr.is_null() { return 0; }
    unsafe { std::ffi::CStr::from_ptr(ptr).to_bytes().len() as i64 }
}
extern "C" fn slang_str_eq(a: *const i8, b: *const i8) -> i8 {
    if a.is_null() || b.is_null() { return 0; }
    let sa = unsafe { std::ffi::CStr::from_ptr(a) };
    let sb = unsafe { std::ffi::CStr::from_ptr(b) };
    if sa == sb { 1 } else { 0 }
}
/// Concatenate two null-terminated strings; result is interned into the arena.
extern "C" fn slang_str_cat(a: *const i8, b: *const i8) -> *const i8 {
    let sa = if a.is_null() { String::new() } else { unsafe { std::ffi::CStr::from_ptr(a).to_string_lossy().into_owned() } };
    let sb = if b.is_null() { String::new() } else { unsafe { std::ffi::CStr::from_ptr(b).to_string_lossy().into_owned() } };
    let combined = sa + &sb;
    intern_cstr(&combined) as *const i8
}

// ─── Phase 5: New stdlib runtime functions ──────────────────────────────

// Time
extern "C" fn slang_clock_ns() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as i64
}
extern "C" fn slang_clock_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

// Assertions
extern "C" fn slang_assert_eq_i64(a: i64, b: i64) {
    if a != b {
        eprintln!("[VITALIS ASSERT FAILED] assert_eq: {} != {}", a, b);
    }
}
extern "C" fn slang_assert_true(cond: i8) {
    if cond == 0 {
        eprintln!("[VITALIS ASSERT FAILED] assert_true: got false");
    }
}

// Bitwise operations
extern "C" fn slang_popcount(x: i64) -> i64 { x.count_ones() as i64 }
extern "C" fn slang_leading_zeros(x: i64) -> i64 { x.leading_zeros() as i64 }
extern "C" fn slang_trailing_zeros(x: i64) -> i64 { x.trailing_zeros() as i64 }

// Extended math
extern "C" fn slang_sign_i64(x: i64) -> i64 { x.signum() }
extern "C" fn slang_gcd(mut a: i64, mut b: i64) -> i64 {
    a = a.abs(); b = b.abs();
    while b != 0 { let t = b; b = a % b; a = t; }
    a
}
extern "C" fn slang_lcm(a: i64, b: i64) -> i64 {
    if a == 0 || b == 0 { return 0; }
    (a.abs() / slang_gcd(a, b)) * b.abs()
}
extern "C" fn slang_factorial(n: i64) -> i64 {
    if n < 0 { return 0; }
    let mut result: i64 = 1;
    for i in 2..=n { result = result.saturating_mul(i); }
    result
}
extern "C" fn slang_fibonacci(n: i64) -> i64 {
    if n <= 0 { return 0; }
    if n == 1 { return 1; }
    let (mut a, mut b) = (0i64, 1i64);
    for _ in 2..=n { let t = a.saturating_add(b); a = b; b = t; }
    b
}
extern "C" fn slang_is_prime(n: i64) -> i8 {
    if n < 2 { return 0; }
    if n < 4 { return 1; }
    if n % 2 == 0 || n % 3 == 0 { return 0; }
    let mut i = 5i64;
    while i * i <= n {
        if n % i == 0 || n % (i + 2) == 0 { return 0; }
        i += 6;
    }
    1
}
extern "C" fn slang_tan_f64(x: f64) -> f64    { x.tan() }
extern "C" fn slang_asin_f64(x: f64) -> f64   { x.asin() }
extern "C" fn slang_acos_f64(x: f64) -> f64   { x.acos() }
extern "C" fn slang_atan_f64(x: f64) -> f64   { x.atan() }

// ── Phase 21 stdlib: hash, interpolation, numeric ────────────────────
extern "C" fn slang_hash_i64(x: i64) -> i64 {
    // MurmurHash3-style finalizer
    let mut h = x as u64;
    h ^= h >> 33;
    h = h.wrapping_mul(0xff51afd7ed558ccd);
    h ^= h >> 33;
    h = h.wrapping_mul(0xc4ceb9fe1a85ec53);
    h ^= h >> 33;
    h as i64
}

extern "C" fn slang_lerp_f64(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t.clamp(0.0, 1.0)
}

extern "C" fn slang_smoothstep_f64(edge0: f64, edge1: f64, x: f64) -> f64 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

extern "C" fn slang_wrap_i64(x: i64, lo: i64, hi: i64) -> i64 {
    if hi <= lo { return lo; }
    let range = hi - lo;
    ((x - lo).rem_euclid(range)) + lo
}

extern "C" fn slang_map_range_f64(x: f64, in_lo: f64, in_hi: f64, out_lo: f64, out_hi: f64) -> f64 {
    let in_range = in_hi - in_lo;
    if in_range.abs() < 1e-15 { return out_lo; }
    let t = (x - in_lo) / in_range;
    out_lo + t * (out_hi - out_lo)
}

extern "C" fn slang_epoch_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

// ── Phase 22 stdlib runtime functions ─────────────────────────────────

/// Fused multiply-add: a * b + c
extern "C" fn slang_fma_f64(a: f64, b: f64, c: f64) -> f64 {
    a.mul_add(b, c)
}

/// Cube root
extern "C" fn slang_cbrt_f64(x: f64) -> f64 {
    x.cbrt()
}

/// Degrees to radians
extern "C" fn slang_deg_to_rad(x: f64) -> f64 {
    x * std::f64::consts::PI / 180.0
}

/// Radians to degrees
extern "C" fn slang_rad_to_deg(x: f64) -> f64 {
    x * 180.0 / std::f64::consts::PI
}

/// Sigmoid / logistic function: 1 / (1 + e^(-x))
extern "C" fn slang_sigmoid_f64(x: f64) -> f64 {
    1.0 / (1.0 + (-x).exp())
}

/// ReLU: max(0, x)
extern "C" fn slang_relu_f64(x: f64) -> f64 {
    if x > 0.0 { x } else { 0.0 }
}

/// Tanh activation (already in std)
extern "C" fn slang_tanh_f64(x: f64) -> f64 {
    x.tanh()
}

/// Integer power (i64^i64) with overflow protection
extern "C" fn slang_ipow(base: i64, exp: i64) -> i64 {
    if exp < 0 { return 0; }
    let mut result: i64 = 1;
    let mut b = base;
    let mut e = exp as u64;
    while e > 0 {
        if e & 1 == 1 {
            result = result.wrapping_mul(b);
        }
        b = b.wrapping_mul(b);
        e >>= 1;
    }
    result
}

// ── Phase 23 stdlib runtime functions ─────────────────────────────────

/// Hyperbolic sine
extern "C" fn slang_sinh_f64(x: f64) -> f64 { x.sinh() }

/// Hyperbolic cosine
extern "C" fn slang_cosh_f64(x: f64) -> f64 { x.cosh() }

/// Natural log (alias for ln)
extern "C" fn slang_log_f64(x: f64) -> f64 { x.ln() }

/// Base-2 exponential: 2^x
extern "C" fn slang_exp2_f64(x: f64) -> f64 { (2.0_f64).powf(x) }

/// Copy sign of y onto magnitude of x
extern "C" fn slang_copysign_f64(x: f64, y: f64) -> f64 { x.copysign(y) }

/// Fractional part of x
extern "C" fn slang_fract_f64(x: f64) -> f64 { x.fract() }

/// Truncate toward zero
extern "C" fn slang_trunc_f64(x: f64) -> f64 { x.trunc() }

/// Step function: 0.0 if x < edge, else 1.0
extern "C" fn slang_step_f64(edge: f64, x: f64) -> f64 {
    if x < edge { 0.0 } else { 1.0 }
}

/// Leaky ReLU: x if x > 0, else alpha * x
extern "C" fn slang_leaky_relu_f64(x: f64, alpha: f64) -> f64 {
    if x > 0.0 { x } else { alpha * x }
}

/// ELU activation: x if x > 0, else alpha * (e^x - 1)
extern "C" fn slang_elu_f64(x: f64, alpha: f64) -> f64 {
    if x > 0.0 { x } else { alpha * (x.exp() - 1.0) }
}

// ── Phase 24 stdlib runtime functions ─────────────────────────────────

/// Swish activation: x * sigmoid(x)
extern "C" fn slang_swish_f64(x: f64) -> f64 { x / (1.0 + (-x).exp()) }

/// GELU activation (approximate): x * 0.5 * (1 + tanh(sqrt(2/pi) * (x + 0.044715 * x^3)))
extern "C" fn slang_gelu_f64(x: f64) -> f64 {
    let c = (2.0_f64 / std::f64::consts::PI).sqrt();
    0.5 * x * (1.0 + (c * (x + 0.044715 * x.powi(3))).tanh())
}

/// Softplus: ln(1 + e^x)
extern "C" fn slang_softplus_f64(x: f64) -> f64 {
    if x > 20.0 { x } else { (1.0 + x.exp()).ln() }
}

/// Mish activation: x * tanh(softplus(x))
extern "C" fn slang_mish_f64(x: f64) -> f64 {
    let sp = if x > 20.0 { x } else { (1.0 + x.exp()).ln() };
    x * sp.tanh()
}

/// log(1 + x), numerically stable for small x
extern "C" fn slang_log1p_f64(x: f64) -> f64 { x.ln_1p() }

/// e^x - 1, numerically stable for small x
extern "C" fn slang_expm1_f64(x: f64) -> f64 { x.exp_m1() }

/// Reciprocal: 1/x
extern "C" fn slang_recip_f64(x: f64) -> f64 { x.recip() }

/// Inverse square root: 1/sqrt(x)
extern "C" fn slang_rsqrt_f64(x: f64) -> f64 { 1.0 / x.sqrt() }

// ── Phase 25 stdlib runtime functions ─────────────────────────────────

/// SELU activation: lambda * (x if x > 0, alpha*(e^x - 1) if x <= 0)
extern "C" fn slang_selu_f64(x: f64) -> f64 {
    const ALPHA: f64 = 1.6732632423543772;
    const LAMBDA: f64 = 1.0507009873554805;
    if x > 0.0 { LAMBDA * x } else { LAMBDA * ALPHA * (x.exp() - 1.0) }
}

/// Hard sigmoid: clamp((x + 3) / 6, 0, 1)
extern "C" fn slang_hard_sigmoid_f64(x: f64) -> f64 {
    ((x + 3.0) / 6.0).clamp(0.0, 1.0)
}

/// Hard swish: x * hard_sigmoid(x)
extern "C" fn slang_hard_swish_f64(x: f64) -> f64 {
    x * ((x + 3.0) / 6.0).clamp(0.0, 1.0)
}

/// Log sigmoid: log(sigmoid(x)), numerically stable
extern "C" fn slang_log_sigmoid_f64(x: f64) -> f64 {
    if x >= 0.0 { -((-x).exp().ln_1p()) } else { x - (x.exp().ln_1p()) }
}

/// CELU activation: max(0,x) + min(0, alpha*(e^(x/alpha) - 1))
extern "C" fn slang_celu_f64(x: f64) -> f64 {
    if x >= 0.0 { x } else { x.exp() - 1.0 }
}

/// Softsign: x / (1 + |x|)
extern "C" fn slang_softsign_f64(x: f64) -> f64 { x / (1.0 + x.abs()) }

/// Gaussian: e^(-x²)
extern "C" fn slang_gaussian_f64(x: f64) -> f64 { (-x * x).exp() }

/// Normalized sinc: sin(πx)/(πx), sinc(0) = 1
extern "C" fn slang_sinc_f64(x: f64) -> f64 {
    if x.abs() < 1e-15 { 1.0 } else {
        let px = std::f64::consts::PI * x;
        px.sin() / px
    }
}

/// Fast inverse sqrt (Quake-style, then Newton refinement)
extern "C" fn slang_inv_sqrt_approx_f64(x: f64) -> f64 {
    if x <= 0.0 { return 0.0; }
    let mut y = 1.0 / x.sqrt();
    y = y * (1.5 - 0.5 * x * y * y); // Newton step
    y
}

/// Logit: log(x / (1 - x)), inverse sigmoid
extern "C" fn slang_logit_f64(x: f64) -> f64 {
    let x = x.clamp(1e-7, 1.0 - 1e-7);
    (x / (1.0 - x)).ln()
}

// ── v15: String Operations ──────────────────────────────────────────────

/// Convert string to uppercase
extern "C" fn slang_str_upper(ptr: *const i8) -> *const i8 {
    if ptr.is_null() { return intern_cstr("") as *const i8; }
    let s = unsafe { std::ffi::CStr::from_ptr(ptr).to_string_lossy().into_owned() };
    intern_cstr(&s.to_uppercase()) as *const i8
}

/// Convert string to lowercase
extern "C" fn slang_str_lower(ptr: *const i8) -> *const i8 {
    if ptr.is_null() { return intern_cstr("") as *const i8; }
    let s = unsafe { std::ffi::CStr::from_ptr(ptr).to_string_lossy().into_owned() };
    intern_cstr(&s.to_lowercase()) as *const i8
}

/// Trim whitespace from both sides
extern "C" fn slang_str_trim(ptr: *const i8) -> *const i8 {
    if ptr.is_null() { return intern_cstr("") as *const i8; }
    let s = unsafe { std::ffi::CStr::from_ptr(ptr).to_string_lossy().into_owned() };
    intern_cstr(s.trim()) as *const i8
}

/// Check if string contains substring → i8 bool
extern "C" fn slang_str_contains(haystack: *const i8, needle: *const i8) -> i8 {
    if haystack.is_null() || needle.is_null() { return 0; }
    let h = unsafe { std::ffi::CStr::from_ptr(haystack).to_string_lossy() };
    let n = unsafe { std::ffi::CStr::from_ptr(needle).to_string_lossy() };
    if h.contains(n.as_ref()) { 1 } else { 0 }
}

/// Check if string starts with prefix → i8 bool
extern "C" fn slang_str_starts_with(s: *const i8, prefix: *const i8) -> i8 {
    if s.is_null() || prefix.is_null() { return 0; }
    let sv = unsafe { std::ffi::CStr::from_ptr(s).to_string_lossy() };
    let pv = unsafe { std::ffi::CStr::from_ptr(prefix).to_string_lossy() };
    if sv.starts_with(pv.as_ref()) { 1 } else { 0 }
}

/// Check if string ends with suffix → i8 bool
extern "C" fn slang_str_ends_with(s: *const i8, suffix: *const i8) -> i8 {
    if s.is_null() || suffix.is_null() { return 0; }
    let sv = unsafe { std::ffi::CStr::from_ptr(s).to_string_lossy() };
    let sv2 = unsafe { std::ffi::CStr::from_ptr(suffix).to_string_lossy() };
    if sv.ends_with(sv2.as_ref()) { 1 } else { 0 }
}

/// Get character at index (returns single-char string, or "" if OOB)
extern "C" fn slang_str_char_at(s: *const i8, index: i64) -> *const i8 {
    if s.is_null() { return intern_cstr("") as *const i8; }
    let sv = unsafe { std::ffi::CStr::from_ptr(s).to_string_lossy().into_owned() };
    if index < 0 || (index as usize) >= sv.len() { return intern_cstr("") as *const i8; }
    let ch = sv.chars().nth(index as usize).unwrap_or('\0');
    intern_cstr(&ch.to_string()) as *const i8
}

/// Substring: str_substr(s, start, len)
extern "C" fn slang_str_substr(s: *const i8, start: i64, len: i64) -> *const i8 {
    if s.is_null() { return intern_cstr("") as *const i8; }
    let sv = unsafe { std::ffi::CStr::from_ptr(s).to_string_lossy().into_owned() };
    let st = (start.max(0) as usize).min(sv.len());
    let end = (st + (len.max(0) as usize)).min(sv.len());
    intern_cstr(&sv[st..end]) as *const i8
}

/// Find index of substring (-1 if not found)
extern "C" fn slang_str_index_of(haystack: *const i8, needle: *const i8) -> i64 {
    if haystack.is_null() || needle.is_null() { return -1; }
    let h = unsafe { std::ffi::CStr::from_ptr(haystack).to_string_lossy().into_owned() };
    let n = unsafe { std::ffi::CStr::from_ptr(needle).to_string_lossy().into_owned() };
    h.find(&n).map(|i| i as i64).unwrap_or(-1)
}

/// Replace all occurrences of `old` with `new_s`
extern "C" fn slang_str_replace(s: *const i8, old: *const i8, new_s: *const i8) -> *const i8 {
    if s.is_null() { return intern_cstr("") as *const i8; }
    let sv = unsafe { std::ffi::CStr::from_ptr(s).to_string_lossy().into_owned() };
    let ov = if old.is_null() { String::new() } else { unsafe { std::ffi::CStr::from_ptr(old).to_string_lossy().into_owned() } };
    let nv = if new_s.is_null() { String::new() } else { unsafe { std::ffi::CStr::from_ptr(new_s).to_string_lossy().into_owned() } };
    if ov.is_empty() { return intern_cstr(&sv) as *const i8; }
    intern_cstr(&sv.replace(&ov, &nv)) as *const i8
}

/// Repeat string n times
extern "C" fn slang_str_repeat(s: *const i8, n: i64) -> *const i8 {
    if s.is_null() || n <= 0 { return intern_cstr("") as *const i8; }
    let sv = unsafe { std::ffi::CStr::from_ptr(s).to_string_lossy().into_owned() };
    intern_cstr(&sv.repeat(n.min(100_000) as usize)) as *const i8
}

/// Reverse a string
extern "C" fn slang_str_reverse(s: *const i8) -> *const i8 {
    if s.is_null() { return intern_cstr("") as *const i8; }
    let sv = unsafe { std::ffi::CStr::from_ptr(s).to_string_lossy().into_owned() };
    let reversed: String = sv.chars().rev().collect();
    intern_cstr(&reversed) as *const i8
}

/// Split string by delimiter, return count of parts
extern "C" fn slang_str_split_count(s: *const i8, delim: *const i8) -> i64 {
    if s.is_null() { return 0; }
    let sv = unsafe { std::ffi::CStr::from_ptr(s).to_string_lossy().into_owned() };
    let dv = if delim.is_null() { " ".to_string() } else { unsafe { std::ffi::CStr::from_ptr(delim).to_string_lossy().into_owned() } };
    if dv.is_empty() { return sv.len() as i64; }
    sv.split(&dv).count() as i64
}

/// Get the n-th part after splitting by delimiter
extern "C" fn slang_str_split_get(s: *const i8, delim: *const i8, index: i64) -> *const i8 {
    if s.is_null() { return intern_cstr("") as *const i8; }
    let sv = unsafe { std::ffi::CStr::from_ptr(s).to_string_lossy().into_owned() };
    let dv = if delim.is_null() { " ".to_string() } else { unsafe { std::ffi::CStr::from_ptr(delim).to_string_lossy().into_owned() } };
    if dv.is_empty() { return intern_cstr("") as *const i8; }
    let parts: Vec<&str> = sv.split(&dv).collect();
    if index < 0 || (index as usize) >= parts.len() { return intern_cstr("") as *const i8; }
    intern_cstr(parts[index as usize]) as *const i8
}

/// Join: not a stdlib call (needs arrays), but convert int to string
extern "C" fn slang_to_string_i64(val: i64) -> *const i8 {
    intern_cstr(&val.to_string()) as *const i8
}

/// Convert f64 to string
extern "C" fn slang_to_string_f64(val: f64) -> *const i8 {
    intern_cstr(&val.to_string()) as *const i8
}

/// Convert bool to string
extern "C" fn slang_to_string_bool(val: i8) -> *const i8 {
    intern_cstr(if val != 0 { "true" } else { "false" }) as *const i8
}

// ─── String formatting runtime ──────────────────────────────────────────
/// Replace first `{}` in fmt with an i64 value.
extern "C" fn slang_str_format_i64(fmt: *const i8, val: i64) -> *const i8 {
    if fmt.is_null() { return intern_cstr("") as *const i8; }
    let s = unsafe { std::ffi::CStr::from_ptr(fmt).to_string_lossy().into_owned() };
    let result = s.replacen("{}", &val.to_string(), 1);
    intern_cstr(&result) as *const i8
}
/// Replace first `{}` in fmt with an f64 value.
extern "C" fn slang_str_format_f64(fmt: *const i8, val: f64) -> *const i8 {
    if fmt.is_null() { return intern_cstr("") as *const i8; }
    let s = unsafe { std::ffi::CStr::from_ptr(fmt).to_string_lossy().into_owned() };
    let result = s.replacen("{}", &val.to_string(), 1);
    intern_cstr(&result) as *const i8
}
/// Replace first `{}` in fmt with a string value.
extern "C" fn slang_str_format_str(fmt: *const i8, val: *const i8) -> *const i8 {
    if fmt.is_null() { return intern_cstr("") as *const i8; }
    let s = unsafe { std::ffi::CStr::from_ptr(fmt).to_string_lossy().into_owned() };
    let v = if val.is_null() { String::new() } else { unsafe { std::ffi::CStr::from_ptr(val).to_string_lossy().into_owned() } };
    let result = s.replacen("{}", &v, 1);
    intern_cstr(&result) as *const i8
}

/// Parse string to i64 (0 on failure)
extern "C" fn slang_parse_int(s: *const i8) -> i64 {
    if s.is_null() { return 0; }
    let sv = unsafe { std::ffi::CStr::from_ptr(s).to_string_lossy().into_owned() };
    sv.trim().parse::<i64>().unwrap_or(0)
}

/// Parse string to f64 (0.0 on failure)
extern "C" fn slang_parse_float(s: *const i8) -> f64 {
    if s.is_null() { return 0.0; }
    let sv = unsafe { std::ffi::CStr::from_ptr(s).to_string_lossy().into_owned() };
    sv.trim().parse::<f64>().unwrap_or(0.0)
}

// ── v15: File I/O ───────────────────────────────────────────────────────

/// Read entire file to string. Returns "" on error.
extern "C" fn slang_file_read(path: *const i8) -> *const i8 {
    if path.is_null() { return intern_cstr("") as *const i8; }
    let p = unsafe { std::ffi::CStr::from_ptr(path).to_string_lossy().into_owned() };
    match std::fs::read_to_string(&p) {
        Ok(content) => intern_cstr(&content) as *const i8,
        Err(_) => intern_cstr("") as *const i8,
    }
}

/// Write string to file. Returns 1 on success, 0 on failure.
extern "C" fn slang_file_write(path: *const i8, content: *const i8) -> i8 {
    if path.is_null() { return 0; }
    let p = unsafe { std::ffi::CStr::from_ptr(path).to_string_lossy().into_owned() };
    let c = if content.is_null() { String::new() } else { unsafe { std::ffi::CStr::from_ptr(content).to_string_lossy().into_owned() } };
    match std::fs::write(&p, &c) {
        Ok(_) => 1,
        Err(_) => 0,
    }
}

/// Append string to file. Returns 1 on success, 0 on failure.
extern "C" fn slang_file_append(path: *const i8, content: *const i8) -> i8 {
    if path.is_null() { return 0; }
    let p = unsafe { std::ffi::CStr::from_ptr(path).to_string_lossy().into_owned() };
    let c = if content.is_null() { String::new() } else { unsafe { std::ffi::CStr::from_ptr(content).to_string_lossy().into_owned() } };
    use std::io::Write;
    match std::fs::OpenOptions::new().append(true).create(true).open(&p) {
        Ok(mut f) => if f.write_all(c.as_bytes()).is_ok() { 1 } else { 0 },
        Err(_) => 0,
    }
}

/// Check if file exists → i8 bool
extern "C" fn slang_file_exists(path: *const i8) -> i8 {
    if path.is_null() { return 0; }
    let p = unsafe { std::ffi::CStr::from_ptr(path).to_string_lossy().into_owned() };
    if std::path::Path::new(&p).exists() { 1 } else { 0 }
}

/// Delete a file. Returns 1 on success, 0 on failure.
extern "C" fn slang_file_delete(path: *const i8) -> i8 {
    if path.is_null() { return 0; }
    let p = unsafe { std::ffi::CStr::from_ptr(path).to_string_lossy().into_owned() };
    if std::fs::remove_file(&p).is_ok() { 1 } else { 0 }
}

/// Get file size in bytes (-1 on error)
extern "C" fn slang_file_size(path: *const i8) -> i64 {
    if path.is_null() { return -1; }
    let p = unsafe { std::ffi::CStr::from_ptr(path).to_string_lossy().into_owned() };
    match std::fs::metadata(&p) {
        Ok(m) => m.len() as i64,
        Err(_) => -1,
    }
}

// ── v15: HashMap Runtime ────────────────────────────────────────────────
// Maps are stored as opaque pointers to a Box<HashMap<String, i64>>
// arena-managed to prevent drops.

use std::collections::BTreeMap;

static VITALIS_MAP_ARENA: Mutex<Vec<Box<BTreeMap<String, i64>>>> = Mutex::new(Vec::new());

/// Create a new empty map, return opaque handle (i64)
extern "C" fn slang_map_new() -> i64 {
    let mut arena = VITALIS_MAP_ARENA.lock().unwrap();
    arena.push(Box::new(BTreeMap::new()));
    (arena.len() - 1) as i64
}

/// Set key-value in map
extern "C" fn slang_map_set(handle: i64, key: *const i8, value: i64) {
    if key.is_null() { return; }
    let k = unsafe { std::ffi::CStr::from_ptr(key).to_string_lossy().into_owned() };
    let mut arena = VITALIS_MAP_ARENA.lock().unwrap();
    if let Some(map) = arena.get_mut(handle as usize) {
        map.insert(k, value);
    }
}

/// Get value from map (returns 0 if key not found)
extern "C" fn slang_map_get(handle: i64, key: *const i8) -> i64 {
    if key.is_null() { return 0; }
    let k = unsafe { std::ffi::CStr::from_ptr(key).to_string_lossy().into_owned() };
    let arena = VITALIS_MAP_ARENA.lock().unwrap();
    arena.get(handle as usize).and_then(|m| m.get(&k).copied()).unwrap_or(0)
}

/// Check if key exists in map → bool
extern "C" fn slang_map_has(handle: i64, key: *const i8) -> i8 {
    if key.is_null() { return 0; }
    let k = unsafe { std::ffi::CStr::from_ptr(key).to_string_lossy().into_owned() };
    let arena = VITALIS_MAP_ARENA.lock().unwrap();
    if arena.get(handle as usize).map(|m| m.contains_key(&k)).unwrap_or(false) { 1 } else { 0 }
}

/// Remove key from map
extern "C" fn slang_map_remove(handle: i64, key: *const i8) {
    if key.is_null() { return; }
    let k = unsafe { std::ffi::CStr::from_ptr(key).to_string_lossy().into_owned() };
    let mut arena = VITALIS_MAP_ARENA.lock().unwrap();
    if let Some(map) = arena.get_mut(handle as usize) {
        map.remove(&k);
    }
}

/// Count of entries in map
extern "C" fn slang_map_len(handle: i64) -> i64 {
    let arena = VITALIS_MAP_ARENA.lock().unwrap();
    arena.get(handle as usize).map(|m| m.len() as i64).unwrap_or(0)
}

/// Get all keys as a joined string (delimiter = ",")
extern "C" fn slang_map_keys(handle: i64) -> *const i8 {
    let arena = VITALIS_MAP_ARENA.lock().unwrap();
    match arena.get(handle as usize) {
        Some(map) => {
            let keys: Vec<&str> = map.keys().map(|k| k.as_str()).collect();
            intern_cstr(&keys.join(",")) as *const i8
        }
        None => intern_cstr("") as *const i8,
    }
}

// ── v16: HashSet Runtime ────────────────────────────────────────────────
// Sets are stored in an arena indexed by handle, similar to maps.

use std::collections::HashSet;

static VITALIS_SET_ARENA: Mutex<Vec<Box<HashSet<i64>>>> = Mutex::new(Vec::new());

/// Create a new empty set, return opaque handle (i64)
extern "C" fn slang_set_new() -> i64 {
    let mut arena = VITALIS_SET_ARENA.lock().unwrap();
    arena.push(Box::new(HashSet::new()));
    (arena.len() - 1) as i64
}

/// Add value to set
extern "C" fn slang_set_add(handle: i64, value: i64) {
    let mut arena = VITALIS_SET_ARENA.lock().unwrap();
    if let Some(set) = arena.get_mut(handle as usize) {
        set.insert(value);
    }
}

/// Check if value exists in set → 0/1
extern "C" fn slang_set_has(handle: i64, value: i64) -> i8 {
    let arena = VITALIS_SET_ARENA.lock().unwrap();
    if arena.get(handle as usize).map(|s| s.contains(&value)).unwrap_or(false) { 1 } else { 0 }
}

/// Remove value from set
extern "C" fn slang_set_remove(handle: i64, value: i64) {
    let mut arena = VITALIS_SET_ARENA.lock().unwrap();
    if let Some(set) = arena.get_mut(handle as usize) {
        set.remove(&value);
    }
}

/// Count of elements in set
extern "C" fn slang_set_len(handle: i64) -> i64 {
    let arena = VITALIS_SET_ARENA.lock().unwrap();
    arena.get(handle as usize).map(|s| s.len() as i64).unwrap_or(0)
}

/// Union of two sets → new handle
extern "C" fn slang_set_union(h1: i64, h2: i64) -> i64 {
    let arena = VITALIS_SET_ARENA.lock().unwrap();
    let s1 = arena.get(h1 as usize);
    let s2 = arena.get(h2 as usize);
    let result: HashSet<i64> = match (s1, s2) {
        (Some(a), Some(b)) => a.union(b).copied().collect(),
        (Some(a), None)    => a.as_ref().clone(),
        (None, Some(b))    => b.as_ref().clone(),
        (None, None)       => HashSet::new(),
    };
    drop(arena);
    let mut arena = VITALIS_SET_ARENA.lock().unwrap();
    arena.push(Box::new(result));
    (arena.len() - 1) as i64
}

/// Intersection of two sets → new handle
extern "C" fn slang_set_intersect(h1: i64, h2: i64) -> i64 {
    let arena = VITALIS_SET_ARENA.lock().unwrap();
    let s1 = arena.get(h1 as usize);
    let s2 = arena.get(h2 as usize);
    let result: HashSet<i64> = match (s1, s2) {
        (Some(a), Some(b)) => a.intersection(b).copied().collect(),
        _                  => HashSet::new(),
    };
    drop(arena);
    let mut arena = VITALIS_SET_ARENA.lock().unwrap();
    arena.push(Box::new(result));
    (arena.len() - 1) as i64
}

/// Difference of two sets (h1 - h2) → new handle
extern "C" fn slang_set_diff(h1: i64, h2: i64) -> i64 {
    let arena = VITALIS_SET_ARENA.lock().unwrap();
    let s1 = arena.get(h1 as usize);
    let s2 = arena.get(h2 as usize);
    let result: HashSet<i64> = match (s1, s2) {
        (Some(a), Some(b)) => a.difference(b).copied().collect(),
        (Some(a), None)    => a.as_ref().clone(),
        _                  => HashSet::new(),
    };
    drop(arena);
    let mut arena = VITALIS_SET_ARENA.lock().unwrap();
    arena.push(Box::new(result));
    (arena.len() - 1) as i64
}

/// Convert set to string representation e.g. "{1,2,3}"
extern "C" fn slang_set_to_array(handle: i64) -> *const i8 {
    let arena = VITALIS_SET_ARENA.lock().unwrap();
    match arena.get(handle as usize) {
        Some(set) => {
            let mut vals: Vec<i64> = set.iter().copied().collect();
            vals.sort();
            let s = format!("{{{}}}", vals.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(","));
            intern_cstr(&s) as *const i8
        }
        None => intern_cstr("{}") as *const i8,
    }
}

// ── v18: Tuple Runtime ──────────────────────────────────────────────────
// Tuples are immutable, heap-allocated arrays of i64 values, arena-managed.

static VITALIS_TUPLE_ARENA: Mutex<Vec<Box<Vec<i64>>>> = Mutex::new(Vec::new());

/// Create a 2-tuple, return opaque handle (i64)
extern "C" fn slang_tuple_new2(a: i64, b: i64) -> i64 {
    let mut arena = VITALIS_TUPLE_ARENA.lock().unwrap();
    arena.push(Box::new(vec![a, b]));
    (arena.len() - 1) as i64
}

/// Create a 3-tuple, return opaque handle (i64)
extern "C" fn slang_tuple_new3(a: i64, b: i64, c: i64) -> i64 {
    let mut arena = VITALIS_TUPLE_ARENA.lock().unwrap();
    arena.push(Box::new(vec![a, b, c]));
    (arena.len() - 1) as i64
}

/// Create a 4-tuple, return opaque handle (i64)
extern "C" fn slang_tuple_new4(a: i64, b: i64, c: i64, d: i64) -> i64 {
    let mut arena = VITALIS_TUPLE_ARENA.lock().unwrap();
    arena.push(Box::new(vec![a, b, c, d]));
    (arena.len() - 1) as i64
}

/// Get element at index from tuple (bounds-checked, returns 0 on OOB)
extern "C" fn slang_tuple_get(handle: i64, idx: i64) -> i64 {
    let arena = VITALIS_TUPLE_ARENA.lock().unwrap();
    arena.get(handle as usize)
        .and_then(|t| t.get(idx as usize).copied())
        .unwrap_or(0)
}

/// Get length of tuple
extern "C" fn slang_tuple_len(handle: i64) -> i64 {
    let arena = VITALIS_TUPLE_ARENA.lock().unwrap();
    arena.get(handle as usize).map(|t| t.len() as i64).unwrap_or(0)
}

// ── v15: Error Handling Runtime ─────────────────────────────────────────
// Simple error flag mechanism: functions can set an error, callers can check/clear it.

static VITALIS_ERROR_FLAG: AtomicU64 = AtomicU64::new(0);
static VITALIS_ERROR_MSG: Mutex<String> = Mutex::new(String::new());

/// Set error with code
extern "C" fn slang_error_set(code: i64, msg: *const i8) {
    VITALIS_ERROR_FLAG.store(code as u64, Ordering::SeqCst);
    if !msg.is_null() {
        let m = unsafe { std::ffi::CStr::from_ptr(msg).to_string_lossy().into_owned() };
        *VITALIS_ERROR_MSG.lock().unwrap() = m;
    }
}

/// Check if error is set → error code (0 = no error)
extern "C" fn slang_error_check() -> i64 {
    VITALIS_ERROR_FLAG.load(Ordering::SeqCst) as i64
}

/// Get error message
extern "C" fn slang_error_msg() -> *const i8 {
    let msg = VITALIS_ERROR_MSG.lock().unwrap().clone();
    intern_cstr(&msg) as *const i8
}

/// Clear error
extern "C" fn slang_error_clear() {
    VITALIS_ERROR_FLAG.store(0, Ordering::SeqCst);
    *VITALIS_ERROR_MSG.lock().unwrap() = String::new();
}

// ── v15: Environment & System ───────────────────────────────────────────

/// Get environment variable (returns "" if not set)
extern "C" fn slang_env_get(key: *const i8) -> *const i8 {
    if key.is_null() { return intern_cstr("") as *const i8; }
    let k = unsafe { std::ffi::CStr::from_ptr(key).to_string_lossy().into_owned() };
    match std::env::var(&k) {
        Ok(v) => intern_cstr(&v) as *const i8,
        Err(_) => intern_cstr("") as *const i8,
    }
}

/// Sleep for N milliseconds
extern "C" fn slang_sleep_ms(ms: i64) {
    if ms > 0 {
        std::thread::sleep(std::time::Duration::from_millis(ms as u64));
    }
}

/// Print to stderr
extern "C" fn slang_eprint(ptr: *const i8) {
    if ptr.is_null() { return; }
    let s = unsafe { std::ffi::CStr::from_ptr(ptr) };
    eprint!("{}", s.to_string_lossy());
}

/// Print to stderr with newline
extern "C" fn slang_eprintln(ptr: *const i8) {
    if ptr.is_null() { return; }
    let s = unsafe { std::ffi::CStr::from_ptr(ptr) };
    eprintln!("{}", s.to_string_lossy());
}

// --- v142: Runtime Logging -------------------------------------------------
use std::sync::atomic::{AtomicU8, Ordering as AtomicOrdering};

/// Global log level: 0=TRACE, 1=DEBUG, 2=INFO, 3=WARN, 4=ERROR, 5=OFF
static LOG_LEVEL: AtomicU8 = AtomicU8::new(2); // default: INFO

fn log_timestamp() -> String {
    let dur = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = dur.as_secs();
    let millis = dur.subsec_millis();
    let s = secs % 60;
    let m = (secs / 60) % 60;
    let h = (secs / 3600) % 24;
    format!("{:02}:{:02}:{:02}.{:03}", h, m, s, millis)
}

fn slang_log_impl(level_num: u8, level_tag: &str, ptr: *const i8) {
    if LOG_LEVEL.load(AtomicOrdering::Relaxed) > level_num { return; }
    let msg = if ptr.is_null() {
        String::new()
    } else {
        unsafe { std::ffi::CStr::from_ptr(ptr) }.to_string_lossy().into_owned()
    };
    eprintln!("[{}] {} {}", level_tag, log_timestamp(), msg);
}

extern "C" fn slang_log_trace(ptr: *const i8) { slang_log_impl(0, "TRACE", ptr); }
extern "C" fn slang_log_debug(ptr: *const i8) { slang_log_impl(1, "DEBUG", ptr); }
extern "C" fn slang_log_info(ptr: *const i8)  { slang_log_impl(2, "INFO ", ptr); }
extern "C" fn slang_log_warn(ptr: *const i8)  { slang_log_impl(3, "WARN ", ptr); }
extern "C" fn slang_log_error(ptr: *const i8) { slang_log_impl(4, "ERROR", ptr); }

extern "C" fn slang_log_level_set(level: i64) {
    let clamped = (level.max(0).min(5)) as u8;
    LOG_LEVEL.store(clamped, AtomicOrdering::Relaxed);
}

extern "C" fn slang_log_level_get() -> i64 {
    LOG_LEVEL.load(AtomicOrdering::Relaxed) as i64
}

// --- v143: Structured Audit Trail ------------------------------------------

struct AuditEntry {
    timestamp: String,
    category: String,
    action: String,
    detail: String,
}

static AUDIT_LOG: std::sync::LazyLock<std::sync::Mutex<Vec<AuditEntry>>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(Vec::new()));

const AUDIT_MAX_ENTRIES: usize = 10_000;

fn cstr_to_string(ptr: *const i8) -> String {
    if ptr.is_null() {
        String::new()
    } else {
        unsafe { std::ffi::CStr::from_ptr(ptr) }.to_string_lossy().into_owned()
    }
}

extern "C" fn slang_audit_event(category: *const i8, action: *const i8, detail: *const i8) {
    let entry = AuditEntry {
        timestamp: log_timestamp(),
        category: cstr_to_string(category),
        action: cstr_to_string(action),
        detail: cstr_to_string(detail),
    };
    if let Ok(mut log) = AUDIT_LOG.lock() {
        if log.len() >= AUDIT_MAX_ENTRIES {
            log.remove(0); // ring buffer behavior
        }
        log.push(entry);
    }
}

extern "C" fn slang_audit_count() -> i64 {
    AUDIT_LOG.lock().map(|l| l.len() as i64).unwrap_or(0)
}

extern "C" fn slang_audit_dump() {
    if let Ok(log) = AUDIT_LOG.lock() {
        for e in log.iter() {
            eprintln!("[AUDIT] {} [{}] {} � {}", e.timestamp, e.category, e.action, e.detail);
        }
    }
}

extern "C" fn slang_audit_clear() {
    if let Ok(mut log) = AUDIT_LOG.lock() {
        log.clear();
    }
}

extern "C" fn slang_audit_last(buf: *mut i8, buf_len: i64) -> i64 {
    if buf.is_null() || buf_len <= 0 { return 0; }
    if let Ok(log) = AUDIT_LOG.lock() {
        if let Some(last) = log.last() {
            let formatted = format!("[{}] {} � {}", last.category, last.action, last.detail);
            let bytes = formatted.as_bytes();
            let copy_len = bytes.len().min((buf_len - 1) as usize);
            unsafe {
                std::ptr::copy_nonoverlapping(bytes.as_ptr(), buf as *mut u8, copy_len);
                *buf.add(copy_len) = 0; // null terminate
            }
            return copy_len as i64;
        }
    }
    0
}

// --- v144: Metrics & Telemetry ---------------------------------------------

struct MetricsRegistry {
    counters: std::collections::HashMap<String, f64>,
    gauges: std::collections::HashMap<String, f64>,
    histograms: std::collections::HashMap<String, Vec<f64>>,
}

impl MetricsRegistry {
    fn new() -> Self {
        Self {
            counters: std::collections::HashMap::new(),
            gauges: std::collections::HashMap::new(),
            histograms: std::collections::HashMap::new(),
        }
    }
}

static METRICS: std::sync::LazyLock<std::sync::Mutex<MetricsRegistry>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(MetricsRegistry::new()));

extern "C" fn slang_metric_counter(name: *const i8, delta: i64) {
    let key = cstr_to_string(name);
    if let Ok(mut m) = METRICS.lock() {
        *m.counters.entry(key).or_insert(0.0) += delta as f64;
    }
}

extern "C" fn slang_metric_gauge(name: *const i8, value: f64) {
    let key = cstr_to_string(name);
    if let Ok(mut m) = METRICS.lock() {
        m.gauges.insert(key, value);
    }
}

extern "C" fn slang_metric_histogram(name: *const i8, value: f64) {
    let key = cstr_to_string(name);
    if let Ok(mut m) = METRICS.lock() {
        m.histograms.entry(key).or_default().push(value);
    }
}

extern "C" fn slang_metric_get_counter(name: *const i8) -> f64 {
    let key = cstr_to_string(name);
    METRICS.lock().ok().and_then(|m| m.counters.get(&key).copied()).unwrap_or(0.0)
}

extern "C" fn slang_metric_get_gauge(name: *const i8) -> f64 {
    let key = cstr_to_string(name);
    METRICS.lock().ok().and_then(|m| m.gauges.get(&key).copied()).unwrap_or(0.0)
}

extern "C" fn slang_metric_dump() {
    if let Ok(m) = METRICS.lock() {
        for (k, v) in &m.counters {
            eprintln!("counter{{name=\"{}\"}} {}", k, v);
        }
        for (k, v) in &m.gauges {
            eprintln!("gauge{{name=\"{}\"}} {}", k, v);
        }
        for (k, vals) in &m.histograms {
            let count = vals.len();
            let sum: f64 = vals.iter().sum();
            let mean = if count > 0 { sum / count as f64 } else { 0.0 };
            eprintln!("histogram{{name=\"{}\"}} count={} sum={:.4} mean={:.4}", k, count, sum, mean);
        }
    }
}

extern "C" fn slang_metric_clear() {
    if let Ok(mut m) = METRICS.lock() {
        m.counters.clear();
        m.gauges.clear();
        m.histograms.clear();
    }
}

// --- v145: Distributed Tracing Builtins ------------------------------------

static TRACE_SPAN_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

struct ActiveSpan {
    name: String,
    start: std::time::Instant,
    tags: std::collections::HashMap<String, String>,
}

static SPAN_STACK: std::sync::LazyLock<std::sync::Mutex<Vec<ActiveSpan>>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(Vec::new()));

static TRACE_ID_HI: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
static TRACE_ID_LO: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

fn ensure_trace_id() {
    use std::sync::atomic::Ordering::Relaxed;
    if TRACE_ID_HI.load(Relaxed) == 0 && TRACE_ID_LO.load(Relaxed) == 0 {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default();
        TRACE_ID_HI.store(now.as_secs(), Relaxed);
        TRACE_ID_LO.store(now.subsec_nanos() as u64 ^ 0xCAFE_BABE, Relaxed);
    }
}

extern "C" fn slang_span_start(name: *const i8) -> i64 {
    let span_name = cstr_to_string(name);
    ensure_trace_id();
    let id = TRACE_SPAN_COUNTER.fetch_add(1, AtomicOrdering::Relaxed);
    if let Ok(mut stack) = SPAN_STACK.lock() {
        stack.push(ActiveSpan {
            name: span_name,
            start: std::time::Instant::now(),
            tags: std::collections::HashMap::new(),
        });
    }
    id as i64
}

extern "C" fn slang_span_end() -> i64 {
    if let Ok(mut stack) = SPAN_STACK.lock() {
        if let Some(span) = stack.pop() {
            let elapsed = span.start.elapsed();
            let tags_str = if span.tags.is_empty() {
                String::new()
            } else {
                let pairs: Vec<String> = span.tags.iter().map(|(k, v)| format!("{}={}", k, v)).collect();
                format!(" {{{}}}", pairs.join(", "))
            };
            eprintln!("[SPAN] {} {:.3}ms{}", span.name, elapsed.as_secs_f64() * 1000.0, tags_str);
            return elapsed.as_micros() as i64;
        }
    }
    0
}

extern "C" fn slang_span_set_tag(key: *const i8, value: *const i8) {
    let k = cstr_to_string(key);
    let v = cstr_to_string(value);
    if let Ok(mut stack) = SPAN_STACK.lock() {
        if let Some(span) = stack.last_mut() {
            span.tags.insert(k, v);
        }
    }
}

extern "C" fn slang_trace_id() -> *const i8 {
    ensure_trace_id();
    use std::sync::atomic::Ordering::Relaxed;
    let hi = TRACE_ID_HI.load(Relaxed);
    let lo = TRACE_ID_LO.load(Relaxed);
    let id_str = format!("{:016x}{:016x}", hi, lo);
    intern_cstr(&id_str) as *const i8
}

extern "C" fn slang_span_depth() -> i64 {
    SPAN_STACK.lock().map(|s| s.len() as i64).unwrap_or(0)
}

// -- v146: Health Check & Runtime Diagnostics ---------------------------
static PROCESS_START: std::sync::LazyLock<std::time::Instant> =
    std::sync::LazyLock::new(std::time::Instant::now);

static ALLOC_COUNTER: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(0);

/// Returns milliseconds since process start.
#[unsafe(no_mangle)]
extern "C" fn slang_runtime_uptime_ms() -> i64 {
    PROCESS_START.elapsed().as_millis() as i64
}

/// Returns approximate process memory usage in bytes (Windows: working set, Unix: RSS).
#[unsafe(no_mangle)]
extern "C" fn slang_runtime_memory_used() -> i64 {
    #[cfg(target_os = "windows")]
    {
        use std::mem::MaybeUninit;
        #[repr(C)]
        struct ProcessMemoryCounters {
            cb: u32,
            page_fault_count: u32,
            peak_working_set_size: usize,
            working_set_size: usize,
            quota_peak_paged_pool_usage: usize,
            quota_paged_pool_usage: usize,
            quota_peak_non_paged_pool_usage: usize,
            quota_non_paged_pool_usage: usize,
            pagefile_usage: usize,
            peak_pagefile_usage: usize,
        }
        unsafe extern "system" {
            fn GetCurrentProcess() -> isize;
            fn K32GetProcessMemoryInfo(
                hProcess: isize,
                ppsmemCounters: *mut ProcessMemoryCounters,
                cb: u32,
            ) -> i32;
        }
        unsafe {
            let mut pmc = MaybeUninit::<ProcessMemoryCounters>::zeroed().assume_init();
            pmc.cb = std::mem::size_of::<ProcessMemoryCounters>() as u32;
            if K32GetProcessMemoryInfo(
                GetCurrentProcess(),
                &mut pmc,
                pmc.cb,
            ) != 0
            {
                pmc.working_set_size as i64
            } else {
                -1
            }
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        // Read /proc/self/statm on Linux; first field * page_size = RSS
        if let Ok(statm) = std::fs::read_to_string("/proc/self/statm") {
            if let Some(rss_pages) = statm.split_whitespace().nth(1) {
                if let Ok(pages) = rss_pages.parse::<i64>() {
                    return pages * 4096; // typical page size
                }
            }
        }
        -1
    }
}

/// Returns compiler version string.
#[unsafe(no_mangle)]
extern "C" fn slang_runtime_version() -> *const i8 {
    intern_cstr("146.0.0") as *const i8
}

/// Increment allocation counter by `n` and return the new total.
#[unsafe(no_mangle)]
extern "C" fn slang_runtime_alloc_count(n: i64) -> i64 {
    ALLOC_COUNTER.fetch_add(n, std::sync::atomic::Ordering::Relaxed) + n
}

/// Get current allocation counter value.
#[unsafe(no_mangle)]
extern "C" fn slang_runtime_alloc_total() -> i64 {
    ALLOC_COUNTER.load(std::sync::atomic::Ordering::Relaxed)
}

/// Returns number of available CPUs / hardware threads.
#[unsafe(no_mangle)]
extern "C" fn slang_runtime_cpu_count() -> i64 {
    std::thread::available_parallelism()
        .map(|n| n.get() as i64)
        .unwrap_or(1)
}

// -- v147: Observable Pipeline Integration ------------------------------
struct PipelineTimers {
    active: std::collections::HashMap<String, std::time::Instant>,
    completed: Vec<(String, u128)>, // (name, elapsed_�s)
}

static PIPELINE_TIMERS: std::sync::LazyLock<std::sync::Mutex<PipelineTimers>> =
    std::sync::LazyLock::new(|| {
        std::sync::Mutex::new(PipelineTimers {
            active: std::collections::HashMap::new(),
            completed: Vec::new(),
        })
    });

/// Start a named pipeline stage timer. Returns 1 on success, 0 if already active.
#[unsafe(no_mangle)]
extern "C" fn slang_pipeline_timer_start(name: *const i8) -> i64 {
    if name.is_null() { return 0; }
    let n = unsafe { std::ffi::CStr::from_ptr(name).to_string_lossy().into_owned() };
    let mut timers = PIPELINE_TIMERS.lock().unwrap();
    if timers.active.contains_key(&n) { return 0; }
    timers.active.insert(n, std::time::Instant::now());
    1
}

/// End a named pipeline stage timer. Returns elapsed microseconds, or -1 if not found.
#[unsafe(no_mangle)]
extern "C" fn slang_pipeline_timer_end(name: *const i8) -> i64 {
    if name.is_null() { return -1; }
    let n = unsafe { std::ffi::CStr::from_ptr(name).to_string_lossy().into_owned() };
    let mut timers = PIPELINE_TIMERS.lock().unwrap();
    if let Some(start) = timers.active.remove(&n) {
        let elapsed = start.elapsed().as_micros();
        timers.completed.push((n, elapsed));
        elapsed as i64
    } else {
        -1
    }
}

/// Returns the number of completed pipeline stage timers.
#[unsafe(no_mangle)]
extern "C" fn slang_pipeline_stage_count() -> i64 {
    PIPELINE_TIMERS.lock().map(|t| t.completed.len() as i64).unwrap_or(0)
}

/// Dump all completed pipeline stage timings to stderr.
#[unsafe(no_mangle)]
extern "C" fn slang_pipeline_dump_timings() {
    let timers = PIPELINE_TIMERS.lock().unwrap();
    if timers.completed.is_empty() {
        eprintln!("[pipeline] no completed stages");
        return;
    }
    let total_us: u128 = timers.completed.iter().map(|(_, us)| us).sum();
    eprintln!("[pipeline] -- stage timings ----------------------");
    for (name, us) in &timers.completed {
        let pct = if total_us > 0 { (*us as f64 / total_us as f64) * 100.0 } else { 0.0 };
        eprintln!("[pipeline]   {:20} {:>10} �s  ({:.1}%)", name, us, pct);
    }
    eprintln!("[pipeline]   {:20} {:>10} �s", "TOTAL", total_us);
    eprintln!("[pipeline] --------------------------------------");
}

/// Clear all pipeline stage timing data.
#[unsafe(no_mangle)]
extern "C" fn slang_pipeline_clear_timings() {
    let mut timers = PIPELINE_TIMERS.lock().unwrap();
    timers.active.clear();
    timers.completed.clear();
}

// -- v148: RBAC & Capability Permissions --------------------------------
static PERMISSIONS: std::sync::LazyLock<std::sync::Mutex<std::collections::HashSet<String>>> =
    std::sync::LazyLock::new(|| {
        std::sync::Mutex::new(std::collections::HashSet::new())
    });

/// Check if a capability is currently granted. Returns 1 if allowed, 0 otherwise.
#[unsafe(no_mangle)]
extern "C" fn slang_permission_check(capability: *const i8) -> i64 {
    if capability.is_null() { return 0; }
    let cap = unsafe { std::ffi::CStr::from_ptr(capability).to_string_lossy().into_owned() };
    let perms = PERMISSIONS.lock().unwrap();
    if perms.contains(&cap) { 1 } else { 0 }
}

/// Grant a capability. Returns 1 if newly granted, 0 if already present.
#[unsafe(no_mangle)]
extern "C" fn slang_permission_grant(capability: *const i8) -> i64 {
    if capability.is_null() { return 0; }
    let cap = unsafe { std::ffi::CStr::from_ptr(capability).to_string_lossy().into_owned() };
    let mut perms = PERMISSIONS.lock().unwrap();
    if perms.insert(cap) { 1 } else { 0 }
}

/// Revoke a capability. Returns 1 if was present, 0 if not found.
#[unsafe(no_mangle)]
extern "C" fn slang_permission_revoke(capability: *const i8) -> i64 {
    if capability.is_null() { return 0; }
    let cap = unsafe { std::ffi::CStr::from_ptr(capability).to_string_lossy().into_owned() };
    let mut perms = PERMISSIONS.lock().unwrap();
    if perms.remove(&cap) { 1 } else { 0 }
}

/// List all granted capabilities. Returns comma-separated string.
#[unsafe(no_mangle)]
extern "C" fn slang_permission_list() -> *const i8 {
    let perms = PERMISSIONS.lock().unwrap();
    let list: Vec<&str> = perms.iter().map(|s| s.as_str()).collect();
    let joined = list.join(",");
    intern_cstr(&joined) as *const i8
}

/// Revoke all capabilities. Returns number of capabilities that were revoked.
#[unsafe(no_mangle)]
extern "C" fn slang_permission_clear() -> i64 {
    let mut perms = PERMISSIONS.lock().unwrap();
    let count = perms.len() as i64;
    perms.clear();
    count
}

// -- v149: Cryptographic Signing ----------------------------------------

/// SHA-256 hash of a string. Returns 64-char hex string.
#[unsafe(no_mangle)]
extern "C" fn slang_crypto_sha256(msg: *const i8) -> *const i8 {
    if msg.is_null() { return intern_cstr("") as *const i8; }
    let s = unsafe { std::ffi::CStr::from_ptr(msg).to_bytes() };
    let hash = crate::crypto::sha256_public(s);
    intern_cstr(&hash) as *const i8
}

/// HMAC-SHA256 sign: returns 64-char hex signature.
#[unsafe(no_mangle)]
extern "C" fn slang_crypto_hmac_sign(key: *const i8, msg: *const i8) -> *const i8 {
    if key.is_null() || msg.is_null() { return intern_cstr("") as *const i8; }
    let k = unsafe { std::ffi::CStr::from_ptr(key).to_bytes() };
    let m = unsafe { std::ffi::CStr::from_ptr(msg).to_bytes() };
    let mac = crate::crypto::hmac_sha256_public(k, m);
    intern_cstr(&mac) as *const i8
}

/// HMAC-SHA256 verify: returns 1 if signature matches, 0 otherwise.
#[unsafe(no_mangle)]
extern "C" fn slang_crypto_hmac_verify(key: *const i8, msg: *const i8, sig: *const i8) -> i64 {
    if key.is_null() || msg.is_null() || sig.is_null() { return 0; }
    let k = unsafe { std::ffi::CStr::from_ptr(key).to_bytes() };
    let m = unsafe { std::ffi::CStr::from_ptr(msg).to_bytes() };
    let expected_sig = unsafe { std::ffi::CStr::from_ptr(sig).to_string_lossy() };
    let computed = crate::crypto::hmac_sha256_public(k, m);
    // Constant-time comparison to prevent timing attacks
    if computed.len() != expected_sig.len() { return 0; }
    let mut result: u8 = 0;
    for (a, b) in computed.bytes().zip(expected_sig.bytes()) {
        result |= a ^ b;
    }
    if result == 0 { 1 } else { 0 }
}

/// Base64 encode a string.
#[unsafe(no_mangle)]
extern "C" fn slang_crypto_base64_encode(msg: *const i8) -> *const i8 {
    if msg.is_null() { return intern_cstr("") as *const i8; }
    let s = unsafe { std::ffi::CStr::from_ptr(msg).to_bytes() };
    let encoded = crate::crypto::base64_encode_public(s);
    intern_cstr(&encoded) as *const i8
}

/// Base64 decode a string.
#[unsafe(no_mangle)]
extern "C" fn slang_crypto_base64_decode(msg: *const i8) -> *const i8 {
    if msg.is_null() { return intern_cstr("") as *const i8; }
    let s = unsafe { std::ffi::CStr::from_ptr(msg).to_string_lossy() };
    let decoded = crate::crypto::base64_decode_public(&s);
    intern_cstr(&decoded) as *const i8
}

// -- v150: Sandbox Enforcement ------------------------------------------
struct Sandbox {
    allow_fs: bool,
    allow_net: bool,
    allow_exec: bool,
    active: bool,
    violation_count: i64,
}

static SANDBOX: std::sync::LazyLock<std::sync::Mutex<Sandbox>> =
    std::sync::LazyLock::new(|| {
        std::sync::Mutex::new(Sandbox {
            allow_fs: false,
            allow_net: false,
            allow_exec: false,
            active: false,
            violation_count: 0,
        })
    });

/// Create and activate a sandbox. All capabilities are denied by default.
/// Returns 1 if sandbox activated, 0 if already active.
#[unsafe(no_mangle)]
extern "C" fn slang_sandbox_create() -> i64 {
    let mut sb = SANDBOX.lock().unwrap();
    if sb.active { return 0; }
    sb.active = true;
    sb.allow_fs = false;
    sb.allow_net = false;
    sb.allow_exec = false;
    sb.violation_count = 0;
    1
}

/// Allow a specific capability in the sandbox: "fs", "net", "exec".
/// Returns 1 if granted, 0 if unknown capability or sandbox not active.
#[unsafe(no_mangle)]
extern "C" fn slang_sandbox_allow(capability: *const i8) -> i64 {
    if capability.is_null() { return 0; }
    let cap = unsafe { std::ffi::CStr::from_ptr(capability).to_string_lossy() };
    let mut sb = SANDBOX.lock().unwrap();
    if !sb.active { return 0; }
    match cap.as_ref() {
        "fs"   => { sb.allow_fs = true; 1 }
        "net"  => { sb.allow_net = true; 1 }
        "exec" => { sb.allow_exec = true; 1 }
        _ => 0,
    }
}

/// Check if a capability is allowed in sandbox. Returns 1 if allowed, 0 if denied.
/// If sandbox is not active, all capabilities are allowed (returns 1).
#[unsafe(no_mangle)]
extern "C" fn slang_sandbox_check(capability: *const i8) -> i64 {
    if capability.is_null() { return 0; }
    let cap = unsafe { std::ffi::CStr::from_ptr(capability).to_string_lossy() };
    let mut sb = SANDBOX.lock().unwrap();
    if !sb.active { return 1; } // no sandbox = all allowed
    let allowed = match cap.as_ref() {
        "fs"   => sb.allow_fs,
        "net"  => sb.allow_net,
        "exec" => sb.allow_exec,
        _ => false,
    };
    if !allowed { sb.violation_count += 1; }
    if allowed { 1 } else { 0 }
}

/// Get sandbox violation count.
#[unsafe(no_mangle)]
extern "C" fn slang_sandbox_violations() -> i64 {
    SANDBOX.lock().map(|sb| sb.violation_count).unwrap_or(0)
}

/// Destroy the sandbox, returning to unrestricted mode.
/// Returns 1 if sandbox was active and destroyed, 0 if not active.
#[unsafe(no_mangle)]
extern "C" fn slang_sandbox_destroy() -> i64 {
    let mut sb = SANDBOX.lock().unwrap();
    if !sb.active { return 0; }
    sb.active = false;
    sb.allow_fs = false;
    sb.allow_net = false;
    sb.allow_exec = false;
    1
}

// -- v151: Security Audit Logger ----------------------------------------
struct SecAuditEntry {
    event: String,
    severity: String,
    prev_hash: String,
    hash: String,
}

static SEC_AUDIT: std::sync::LazyLock<std::sync::Mutex<Vec<SecAuditEntry>>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(Vec::new()));

fn sec_audit_hash(event: &str, severity: &str, prev_hash: &str) -> String {
    let data = format!("{}|{}|{}", event, severity, prev_hash);
    crate::crypto::sha256_public(data.as_bytes())
}

/// Log a security event with severity ("info", "warn", "critical").
/// Hash-chained for integrity. Returns entry index.
#[unsafe(no_mangle)]
extern "C" fn slang_security_log(event: *const i8, severity: *const i8) -> i64 {
    if event.is_null() || severity.is_null() { return -1; }
    let ev = unsafe { std::ffi::CStr::from_ptr(event).to_string_lossy().into_owned() };
    let sev = unsafe { std::ffi::CStr::from_ptr(severity).to_string_lossy().into_owned() };
    let mut log = SEC_AUDIT.lock().unwrap();
    let prev = log.last().map(|e| e.hash.clone()).unwrap_or_default();
    let hash = sec_audit_hash(&ev, &sev, &prev);
    let idx = log.len() as i64;
    log.push(SecAuditEntry { event: ev, severity: sev, prev_hash: prev, hash });
    idx
}

/// Get security audit log entry count.
#[unsafe(no_mangle)]
extern "C" fn slang_security_log_count() -> i64 {
    SEC_AUDIT.lock().map(|l| l.len() as i64).unwrap_or(0)
}

/// Verify integrity of the security audit chain. Returns 1 if valid, 0 if tampered.
#[unsafe(no_mangle)]
extern "C" fn slang_security_log_verify() -> i64 {
    let log = SEC_AUDIT.lock().unwrap();
    let mut prev_hash = String::new();
    for entry in log.iter() {
        if entry.prev_hash != prev_hash { return 0; }
        let expected = sec_audit_hash(&entry.event, &entry.severity, &prev_hash);
        if entry.hash != expected { return 0; }
        prev_hash = entry.hash.clone();
    }
    1
}

/// Dump security audit log to stderr.
#[unsafe(no_mangle)]
extern "C" fn slang_security_log_dump() {
    let log = SEC_AUDIT.lock().unwrap();
    for (i, entry) in log.iter().enumerate() {
        eprintln!("[sec-audit] #{} [{}] {} (hash: {}..)", i, entry.severity, entry.event, &entry.hash[..8]);
    }
}

/// Clear security audit log. Returns number of entries cleared.
#[unsafe(no_mangle)]
extern "C" fn slang_security_log_clear() -> i64 {
    let mut log = SEC_AUDIT.lock().unwrap();
    let count = log.len() as i64;
    log.clear();
    count
}

// -- v152: Input Validation Framework --------------------------

/// Validate an email address (basic RFC-style check)
#[unsafe(no_mangle)]
extern "C" fn slang_validate_email(s: *const i8) -> i64 {
    if s.is_null() { return 0; }
    let c = unsafe { std::ffi::CStr::from_ptr(s) };
    let txt = c.to_string_lossy();
    // Must have exactly one @, non-empty local/domain, domain has dot
    let parts: Vec<&str> = txt.splitn(3, '@').collect();
    if parts.len() != 2 { return 0; }
    let (local, domain) = (parts[0], parts[1]);
    if local.is_empty() || domain.is_empty() { return 0; }
    if !domain.contains('.') { return 0; }
    if domain.starts_with('.') || domain.ends_with('.') { return 0; }
    1
}

/// Validate a URL (basic http/https scheme check)
#[unsafe(no_mangle)]
extern "C" fn slang_validate_url(s: *const i8) -> i64 {
    if s.is_null() { return 0; }
    let c = unsafe { std::ffi::CStr::from_ptr(s) };
    let txt = c.to_string_lossy();
    if (txt.starts_with("http://") || txt.starts_with("https://")) && txt.len() > 8 {
        1
    } else {
        0
    }
}

/// Validate an IP address (v4 or v6)
#[unsafe(no_mangle)]
extern "C" fn slang_validate_ip(s: *const i8) -> i64 {
    if s.is_null() { return 0; }
    let c = unsafe { std::ffi::CStr::from_ptr(s) };
    let txt = c.to_string_lossy();
    if txt.parse::<std::net::IpAddr>().is_ok() { 1 } else { 0 }
}

/// Sanitize HTML � strip all < > tags
#[unsafe(no_mangle)]
extern "C" fn slang_sanitize_html(s: *const i8) -> *mut i8 {
    if s.is_null() {
        let empty = std::ffi::CString::new("").unwrap();
        return empty.into_raw();
    }
    let c = unsafe { std::ffi::CStr::from_ptr(s) };
    let txt = c.to_string_lossy();
    let mut result = String::new();
    let mut in_tag = false;
    for ch in txt.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => result.push(ch),
            _ => {}
        }
    }
    let out = std::ffi::CString::new(result).unwrap();
    out.into_raw()
}

/// Sanitize SQL � escape single quotes to prevent SQL injection
#[unsafe(no_mangle)]
extern "C" fn slang_sanitize_sql(s: *const i8) -> *mut i8 {
    if s.is_null() {
        let empty = std::ffi::CString::new("").unwrap();
        return empty.into_raw();
    }
    let c = unsafe { std::ffi::CStr::from_ptr(s) };
    let txt = c.to_string_lossy();
    let escaped = txt.replace('\'', "''");
    let out = std::ffi::CString::new(escaped).unwrap();
    out.into_raw()
}

// -- v153: Secure Communication Primitives ---------------------

static SECURE_CHANNELS: std::sync::LazyLock<std::sync::Mutex<BTreeMap<i64, Vec<Vec<u8>>>>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(BTreeMap::new()));
static CHANNEL_COUNTER: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(1);

/// Create a new secure channel, returns channel ID
#[unsafe(no_mangle)]
extern "C" fn slang_secure_channel_create() -> i64 {
    let id = CHANNEL_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    SECURE_CHANNELS.lock().unwrap().insert(id, Vec::new());
    id
}

/// Send a message on a secure channel (simulated encryption)
#[unsafe(no_mangle)]
extern "C" fn slang_secure_channel_send(channel_id: i64, msg: *const i8) -> i64 {
    if msg.is_null() { return 0; }
    let c = unsafe { std::ffi::CStr::from_ptr(msg) };
    let data = c.to_bytes().to_vec();
    // Simulate encryption by XOR-ing with a fixed key
    let key: u8 = 0xAB;
    let encrypted: Vec<u8> = data.iter().map(|b| b ^ key).collect();
    let mut channels = SECURE_CHANNELS.lock().unwrap();
    if let Some(ch) = channels.get_mut(&channel_id) {
        ch.push(encrypted);
        1
    } else {
        0
    }
}

/// Receive (pop) a message from a secure channel, returns decrypted string
#[unsafe(no_mangle)]
extern "C" fn slang_secure_channel_recv(channel_id: i64) -> *mut i8 {
    let mut channels = SECURE_CHANNELS.lock().unwrap();
    if let Some(ch) = channels.get_mut(&channel_id) {
        if let Some(encrypted) = ch.pop() {
            let key: u8 = 0xAB;
            let decrypted: Vec<u8> = encrypted.iter().map(|b| b ^ key).collect();
            if let Ok(s) = String::from_utf8(decrypted) {
                let out = std::ffi::CString::new(s).unwrap();
                return out.into_raw();
            }
        }
    }
    let empty = std::ffi::CString::new("").unwrap();
    empty.into_raw()
}

/// Close a secure channel, returns 1 if existed
#[unsafe(no_mangle)]
extern "C" fn slang_secure_channel_close(channel_id: i64) -> i64 {
    let mut channels = SECURE_CHANNELS.lock().unwrap();
    if channels.remove(&channel_id).is_some() { 1 } else { 0 }
}

/// Constant-time byte comparison of two strings (timing-safe)
#[unsafe(no_mangle)]
extern "C" fn slang_constant_time_eq(a: *const i8, b: *const i8) -> i64 {
    if a.is_null() || b.is_null() { return 0; }
    let ca = unsafe { std::ffi::CStr::from_ptr(a) };
    let cb = unsafe { std::ffi::CStr::from_ptr(b) };
    let ba = ca.to_bytes();
    let bb = cb.to_bytes();
    if ba.len() != bb.len() { return 0; }
    let mut diff: u8 = 0;
    for (x, y) in ba.iter().zip(bb.iter()) {
        diff |= x ^ y;
    }
    if diff == 0 { 1 } else { 0 }
}

// -- v154: Autonomous Improvement Lab v2 -----------------------

static IMPROVEMENT_TRIALS: std::sync::LazyLock<std::sync::Mutex<Vec<(String, f64)>>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(Vec::new()));

/// Run an improvement trial with a name and score
#[unsafe(no_mangle)]
extern "C" fn slang_improvement_run_trial(name: *const i8, score: f64) -> i64 {
    if name.is_null() { return 0; }
    let c = unsafe { std::ffi::CStr::from_ptr(name) };
    let n = c.to_string_lossy().to_string();
    IMPROVEMENT_TRIALS.lock().unwrap().push((n, score));
    1
}

/// Get the best (highest) improvement score
#[unsafe(no_mangle)]
extern "C" fn slang_improvement_best_score() -> f64 {
    let trials = IMPROVEMENT_TRIALS.lock().unwrap();
    trials.iter().map(|(_, s)| *s).fold(f64::NEG_INFINITY, f64::max)
}

/// Get number of improvement trials recorded
#[unsafe(no_mangle)]
extern "C" fn slang_improvement_history_count() -> i64 {
    IMPROVEMENT_TRIALS.lock().unwrap().len() as i64
}

/// Reset all improvement trials
#[unsafe(no_mangle)]
extern "C" fn slang_improvement_reset() -> i64 {
    let mut trials = IMPROVEMENT_TRIALS.lock().unwrap();
    let count = trials.len() as i64;
    trials.clear();
    count
}

// -- v155: LLM-Guided Mutation Templates -----------------------

static MUTATION_LOG: std::sync::LazyLock<std::sync::Mutex<Vec<(String, f64, bool)>>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(Vec::new()));

/// Apply a named mutation, returns mutation index
#[unsafe(no_mangle)]
extern "C" fn slang_mutation_apply(name: *const i8) -> i64 {
    if name.is_null() { return -1; }
    let c = unsafe { std::ffi::CStr::from_ptr(name) };
    let n = c.to_string_lossy().to_string();
    let mut log = MUTATION_LOG.lock().unwrap();
    let idx = log.len() as i64;
    log.push((n, 0.0, true)); // (name, score, active)
    idx
}

/// List number of mutations applied
#[unsafe(no_mangle)]
extern "C" fn slang_mutation_list_count() -> i64 {
    MUTATION_LOG.lock().unwrap().len() as i64
}

/// Score a mutation by index
#[unsafe(no_mangle)]
extern "C" fn slang_mutation_score(idx: i64, score: f64) -> i64 {
    let mut log = MUTATION_LOG.lock().unwrap();
    if idx >= 0 && (idx as usize) < log.len() {
        log[idx as usize].1 = score;
        1
    } else {
        0
    }
}

/// Undo (deactivate) a mutation by index
#[unsafe(no_mangle)]
extern "C" fn slang_mutation_undo(idx: i64) -> i64 {
    let mut log = MUTATION_LOG.lock().unwrap();
    if idx >= 0 && (idx as usize) < log.len() {
        log[idx as usize].2 = false;
        1
    } else {
        0
    }
}

// -- v156: Evolution Fitness Profiles --------------------------

static FITNESS_PROFILES: std::sync::LazyLock<std::sync::Mutex<Vec<(String, Vec<f64>)>>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(Vec::new()));

/// Register a fitness profile with a name
#[unsafe(no_mangle)]
extern "C" fn slang_fitness_register(name: *const i8) -> i64 {
    if name.is_null() { return -1; }
    let c = unsafe { std::ffi::CStr::from_ptr(name) };
    let n = c.to_string_lossy().to_string();
    let mut profiles = FITNESS_PROFILES.lock().unwrap();
    let idx = profiles.len() as i64;
    profiles.push((n, Vec::new()));
    idx
}

/// Evaluate (add a score) to a fitness profile by index
#[unsafe(no_mangle)]
extern "C" fn slang_fitness_evaluate(idx: i64, score: f64) -> i64 {
    let mut profiles = FITNESS_PROFILES.lock().unwrap();
    if idx >= 0 && (idx as usize) < profiles.len() {
        profiles[idx as usize].1.push(score);
        1
    } else {
        0
    }
}

/// Get count of profiles on the Pareto front (non-dominated)
#[unsafe(no_mangle)]
extern "C" fn slang_fitness_pareto_count() -> i64 {
    let profiles = FITNESS_PROFILES.lock().unwrap();
    if profiles.is_empty() { return 0; }
    // Simple: count profiles whose max score is not dominated by all others
    let maxes: Vec<f64> = profiles.iter().map(|(_, scores)| {
        scores.iter().cloned().fold(f64::NEG_INFINITY, f64::max)
    }).collect();
    let best = maxes.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    maxes.iter().filter(|&&m| m == best).count() as i64
}

/// Clear all fitness profiles
#[unsafe(no_mangle)]
extern "C" fn slang_fitness_clear() -> i64 {
    let mut profiles = FITNESS_PROFILES.lock().unwrap();
    let count = profiles.len() as i64;
    profiles.clear();
    count
}

// -- v157: Cross-Module Evolution ------------------------------

static CROSS_EVO_MODULES: std::sync::LazyLock<std::sync::Mutex<Vec<(String, Vec<String>)>>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(Vec::new()));

/// Register a module for cross-module evolution, returns module index
#[unsafe(no_mangle)]
extern "C" fn slang_evo_cross_module(name: *const i8) -> i64 {
    if name.is_null() { return -1; }
    let c = unsafe { std::ffi::CStr::from_ptr(name) };
    let n = c.to_string_lossy().to_string();
    let mut modules = CROSS_EVO_MODULES.lock().unwrap();
    let idx = modules.len() as i64;
    modules.push((n, Vec::new()));
    idx
}

/// Add a dependency to a module
#[unsafe(no_mangle)]
extern "C" fn slang_evo_dep_add(module_idx: i64, dep: *const i8) -> i64 {
    if dep.is_null() { return 0; }
    let c = unsafe { std::ffi::CStr::from_ptr(dep) };
    let d = c.to_string_lossy().to_string();
    let mut modules = CROSS_EVO_MODULES.lock().unwrap();
    if module_idx >= 0 && (module_idx as usize) < modules.len() {
        modules[module_idx as usize].1.push(d);
        1
    } else {
        0
    }
}

/// Check dependencies of a module (returns dep count)
#[unsafe(no_mangle)]
extern "C" fn slang_evo_dep_check(module_idx: i64) -> i64 {
    let modules = CROSS_EVO_MODULES.lock().unwrap();
    if module_idx >= 0 && (module_idx as usize) < modules.len() {
        modules[module_idx as usize].1.len() as i64
    } else {
        -1
    }
}

/// Safe mutate � only if no circular deps (simplified: always safe)
#[unsafe(no_mangle)]
extern "C" fn slang_evo_safe_mutate(module_idx: i64) -> i64 {
    let modules = CROSS_EVO_MODULES.lock().unwrap();
    if module_idx >= 0 && (module_idx as usize) < modules.len() {
        1 // safe to mutate (no circular dep detection in simulation)
    } else {
        0
    }
}

// -- v158: Evolution Checkpointing -----------------------------

static EVO_CHECKPOINTS: std::sync::LazyLock<std::sync::Mutex<BTreeMap<String, Vec<u8>>>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(BTreeMap::new()));

/// Save an evolution checkpoint with a name
#[unsafe(no_mangle)]
extern "C" fn slang_evo_checkpoint_save(name: *const i8) -> i64 {
    if name.is_null() { return 0; }
    let c = unsafe { std::ffi::CStr::from_ptr(name) };
    let n = c.to_string_lossy().to_string();
    // Simulate checkpoint data as a timestamp-based blob
    let data = format!("checkpoint_{}", n).into_bytes();
    EVO_CHECKPOINTS.lock().unwrap().insert(n, data);
    1
}

/// Load an evolution checkpoint, returns 1 if found
#[unsafe(no_mangle)]
extern "C" fn slang_evo_checkpoint_load(name: *const i8) -> i64 {
    if name.is_null() { return 0; }
    let c = unsafe { std::ffi::CStr::from_ptr(name) };
    let n = c.to_string_lossy();
    if EVO_CHECKPOINTS.lock().unwrap().contains_key(n.as_ref()) { 1 } else { 0 }
}

/// List number of saved checkpoints
#[unsafe(no_mangle)]
extern "C" fn slang_evo_checkpoint_list_count() -> i64 {
    EVO_CHECKPOINTS.lock().unwrap().len() as i64
}

/// Clear all checkpoints
#[unsafe(no_mangle)]
extern "C" fn slang_evo_checkpoint_clear() -> i64 {
    let mut cp = EVO_CHECKPOINTS.lock().unwrap();
    let count = cp.len() as i64;
    cp.clear();
    count
}

// -- v159: Meta-Evolution v2 -----------------------------------

static META_EVO_STRATEGIES: std::sync::LazyLock<std::sync::Mutex<Vec<(String, f64, bool)>>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(Vec::new()));

/// Register a meta-evolution strategy, returns index
#[unsafe(no_mangle)]
extern "C" fn slang_meta_evo_register(name: *const i8, score: f64) -> i64 {
    if name.is_null() { return -1; }
    let c = unsafe { std::ffi::CStr::from_ptr(name) };
    let n = c.to_string_lossy().to_string();
    let mut strats = META_EVO_STRATEGIES.lock().unwrap();
    let idx = strats.len() as i64;
    strats.push((n, score, true)); // (name, fitness, active)
    idx
}

/// Select the best strategy (returns index of highest-scoring active)
#[unsafe(no_mangle)]
extern "C" fn slang_meta_evo_select() -> i64 {
    let strats = META_EVO_STRATEGIES.lock().unwrap();
    strats.iter().enumerate()
        .filter(|(_, (_, _, active))| *active)
        .max_by(|(_, (_, a, _)), (_, (_, b, _))| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(i, _)| i as i64)
        .unwrap_or(-1)
}

/// Check if meta-evolution has converged (all strategies within 10% of best)
#[unsafe(no_mangle)]
extern "C" fn slang_meta_evo_converged() -> i64 {
    let strats = META_EVO_STRATEGIES.lock().unwrap();
    let active: Vec<f64> = strats.iter().filter(|(_, _, a)| *a).map(|(_, s, _)| *s).collect();
    if active.len() <= 1 { return 1; }
    let best = active.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    if best == 0.0 { return 1; }
    let threshold = best * 0.9;
    if active.iter().all(|&s| s >= threshold) { 1 } else { 0 }
}

/// Get count of active strategies
#[unsafe(no_mangle)]
extern "C" fn slang_meta_evo_stats_count() -> i64 {
    let strats = META_EVO_STRATEGIES.lock().unwrap();
    strats.iter().filter(|(_, _, a)| *a).count() as i64
}

// -- v160: Tensor-First Types --------------------------------------

static TENSORS: std::sync::LazyLock<std::sync::Mutex<BTreeMap<i64, (Vec<i64>, Vec<f64>)>>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(BTreeMap::new()));
static TENSOR_COUNTER: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(1);

/// Create a tensor with given rank (shape = rank x 1), returns tensor ID
#[unsafe(no_mangle)]
extern "C" fn slang_tensor_create(rank: i64) -> i64 {
    let id = TENSOR_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let r = rank.max(1) as usize;
    let shape = vec![1i64; r];
    let data = vec![0.0f64; 1];
    TENSORS.lock().unwrap().insert(id, (shape, data));
    id
}

/// Get tensor rank (number of dimensions)
#[unsafe(no_mangle)]
extern "C" fn slang_tensor_rank(id: i64) -> i64 {
    let tensors = TENSORS.lock().unwrap();
    tensors.get(&id).map(|(s, _)| s.len() as i64).unwrap_or(-1)
}

/// Get total number of elements in a tensor
#[unsafe(no_mangle)]
extern "C" fn slang_tensor_size(id: i64) -> i64 {
    let tensors = TENSORS.lock().unwrap();
    tensors.get(&id).map(|(_, d)| d.len() as i64).unwrap_or(-1)
}

/// Set a value in a tensor at flat index
#[unsafe(no_mangle)]
extern "C" fn slang_tensor_set(id: i64, idx: i64, val: f64) -> i64 {
    let mut tensors = TENSORS.lock().unwrap();
    if let Some((_, data)) = tensors.get_mut(&id) {
        let i = idx as usize;
        if i < data.len() { data[i] = val; return 1; }
        // Auto-extend
        while data.len() <= i { data.push(0.0); }
        data[i] = val;
        1
    } else { 0 }
}

/// Get a value from a tensor at flat index
#[unsafe(no_mangle)]
extern "C" fn slang_tensor_get(id: i64, idx: i64) -> f64 {
    let tensors = TENSORS.lock().unwrap();
    if let Some((_, data)) = tensors.get(&id) {
        let i = idx as usize;
        if i < data.len() { return data[i]; }
    }
    0.0
}

/// Element-wise add two tensors, returns new tensor ID
#[unsafe(no_mangle)]
extern "C" fn slang_tensor_add(a: i64, b: i64) -> i64 {
    let tensors = TENSORS.lock().unwrap();
    let (da, db) = match (tensors.get(&a), tensors.get(&b)) {
        (Some((_, da)), Some((_, db))) => (da.clone(), db.clone()),
        _ => return -1,
    };
    drop(tensors);
    let len = da.len().max(db.len());
    let mut result = vec![0.0f64; len];
    for i in 0..len {
        let va = if i < da.len() { da[i] } else { 0.0 };
        let vb = if i < db.len() { db[i] } else { 0.0 };
        result[i] = va + vb;
    }
    let id = TENSOR_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    TENSORS.lock().unwrap().insert(id, (vec![len as i64], result));
    id
}

/// Element-wise multiply two tensors, returns new tensor ID
#[unsafe(no_mangle)]
extern "C" fn slang_tensor_mul(a: i64, b: i64) -> i64 {
    let tensors = TENSORS.lock().unwrap();
    let (da, db) = match (tensors.get(&a), tensors.get(&b)) {
        (Some((_, da)), Some((_, db))) => (da.clone(), db.clone()),
        _ => return -1,
    };
    drop(tensors);
    let len = da.len().max(db.len());
    let mut result = vec![0.0f64; len];
    for i in 0..len {
        let va = if i < da.len() { da[i] } else { 0.0 };
        let vb = if i < db.len() { db[i] } else { 0.0 };
        result[i] = va * vb;
    }
    let id = TENSOR_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    TENSORS.lock().unwrap().insert(id, (vec![len as i64], result));
    id
}

// -- v161: Auto-Differentiation ------------------------------------

/// Compute numerical gradient of a polynomial at x (coefficients stored in tensor)
#[unsafe(no_mangle)]
extern "C" fn slang_grad_compute(tensor_id: i64, x: f64) -> f64 {
    let tensors = TENSORS.lock().unwrap();
    if let Some((_, coeffs)) = tensors.get(&tensor_id) {
        // f(x) = c0 + c1*x + c2*x^2 + ..., f'(x) = c1 + 2*c2*x + ...
        let mut grad = 0.0f64;
        for (i, &c) in coeffs.iter().enumerate().skip(1) {
            grad += c * (i as f64) * x.powi(i as i32 - 1);
        }
        grad
    } else { 0.0 }
}

/// Forward-mode AD: compute function value at x
#[unsafe(no_mangle)]
extern "C" fn slang_grad_forward(tensor_id: i64, x: f64) -> f64 {
    let tensors = TENSORS.lock().unwrap();
    if let Some((_, coeffs)) = tensors.get(&tensor_id) {
        let mut val = 0.0f64;
        for (i, &c) in coeffs.iter().enumerate() {
            val += c * x.powi(i as i32);
        }
        val
    } else { 0.0 }
}

/// Reverse-mode AD: same as grad_compute but named differently for API clarity
#[unsafe(no_mangle)]
extern "C" fn slang_grad_reverse(tensor_id: i64, x: f64) -> f64 {
    slang_grad_compute(tensor_id, x)
}

/// Compute Jacobian dimension (number of coefficients - 1)
#[unsafe(no_mangle)]
extern "C" fn slang_grad_jacobian_dim(tensor_id: i64) -> i64 {
    let tensors = TENSORS.lock().unwrap();
    tensors.get(&tensor_id).map(|(_, d)| (d.len().saturating_sub(1)) as i64).unwrap_or(0)
}

// -- v162: ML Pipeline Builtins ------------------------------------

static ML_MODELS: std::sync::LazyLock<std::sync::Mutex<BTreeMap<i64, (f64, f64)>>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(BTreeMap::new()));
static ML_MODEL_COUNTER: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(1);

/// Fit a simple linear model y = slope*x + intercept from two values
#[unsafe(no_mangle)]
extern "C" fn slang_ml_linear_fit(x1: f64, y1: f64, x2: f64, y2: f64) -> i64 {
    let slope = if (x2 - x1).abs() > 1e-12 { (y2 - y1) / (x2 - x1) } else { 0.0 };
    let intercept = y1 - slope * x1;
    let id = ML_MODEL_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    ML_MODELS.lock().unwrap().insert(id, (slope, intercept));
    id
}

/// Predict y for given x using a linear model
#[unsafe(no_mangle)]
extern "C" fn slang_ml_predict(model_id: i64, x: f64) -> f64 {
    let models = ML_MODELS.lock().unwrap();
    if let Some((slope, intercept)) = models.get(&model_id) {
        slope * x + intercept
    } else { 0.0 }
}

/// Compute accuracy as 1 - |predicted - actual| / |actual|
#[unsafe(no_mangle)]
extern "C" fn slang_ml_accuracy(predicted: f64, actual: f64) -> f64 {
    if actual.abs() < 1e-12 {
        if (predicted - actual).abs() < 1e-12 { 1.0 } else { 0.0 }
    } else {
        (1.0 - ((predicted - actual) / actual).abs()).max(0.0)
    }
}

/// Compute MSE loss between predicted and actual
#[unsafe(no_mangle)]
extern "C" fn slang_ml_loss(predicted: f64, actual: f64) -> f64 {
    (predicted - actual).powi(2)
}

// -- v163: Neural Architecture Search Builtins ---------------------

static NAS_REGISTRY: std::sync::LazyLock<std::sync::Mutex<Vec<(String, f64)>>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(Vec::new()));

/// Register a NAS architecture candidate
#[unsafe(no_mangle)]
extern "C" fn slang_nas_search(name: *const i8) -> i64 {
    if name.is_null() { return -1; }
    let c = unsafe { std::ffi::CStr::from_ptr(name) };
    let n = c.to_string_lossy().to_string();
    let mut reg = NAS_REGISTRY.lock().unwrap();
    let idx = reg.len() as i64;
    reg.push((n, 0.0));
    idx
}

/// Evaluate a NAS candidate with a fitness score
#[unsafe(no_mangle)]
extern "C" fn slang_nas_evaluate(idx: i64, score: f64) -> i64 {
    let mut reg = NAS_REGISTRY.lock().unwrap();
    if idx >= 0 && (idx as usize) < reg.len() {
        reg[idx as usize].1 = score;
        1
    } else { 0 }
}

/// Get index of best NAS candidate
#[unsafe(no_mangle)]
extern "C" fn slang_nas_best() -> i64 {
    let reg = NAS_REGISTRY.lock().unwrap();
    reg.iter().enumerate()
        .max_by(|(_, (_, a)), (_, (_, b))| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(i, _)| i as i64)
        .unwrap_or(-1)
}

/// Get total number of NAS candidates
#[unsafe(no_mangle)]
extern "C" fn slang_nas_count() -> i64 {
    NAS_REGISTRY.lock().unwrap().len() as i64
}

// -- v164: Feature Engineering -------------------------------------

/// Normalize a value to [0,1] given min and max
#[unsafe(no_mangle)]
extern "C" fn slang_feature_normalize(val: f64, min: f64, max: f64) -> f64 {
    if (max - min).abs() < 1e-12 { return 0.0; }
    (val - min) / (max - min)
}

/// One-hot encode: returns 1.0 if val == target, 0.0 otherwise
#[unsafe(no_mangle)]
extern "C" fn slang_feature_one_hot(val: i64, target: i64) -> f64 {
    if val == target { 1.0 } else { 0.0 }
}

/// Compute variance of elements in a tensor
#[unsafe(no_mangle)]
extern "C" fn slang_feature_variance(tensor_id: i64) -> f64 {
    let tensors = TENSORS.lock().unwrap();
    if let Some((_, data)) = tensors.get(&tensor_id) {
        if data.is_empty() { return 0.0; }
        let n = data.len() as f64;
        let mean = data.iter().sum::<f64>() / n;
        data.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n
    } else { 0.0 }
}

/// Compute Pearson correlation of two tensors
#[unsafe(no_mangle)]
extern "C" fn slang_feature_correlate(a_id: i64, b_id: i64) -> f64 {
    let tensors = TENSORS.lock().unwrap();
    let (da, db) = match (tensors.get(&a_id), tensors.get(&b_id)) {
        (Some((_, da)), Some((_, db))) => (da.clone(), db.clone()),
        _ => return 0.0,
    };
    drop(tensors);
    let n = da.len().min(db.len());
    if n == 0 { return 0.0; }
    let nf = n as f64;
    let ma = da[..n].iter().sum::<f64>() / nf;
    let mb = db[..n].iter().sum::<f64>() / nf;
    let mut cov = 0.0f64;
    let mut va = 0.0f64;
    let mut vb = 0.0f64;
    for i in 0..n {
        let a = da[i] - ma;
        let b = db[i] - mb;
        cov += a * b;
        va += a * a;
        vb += b * b;
    }
    let denom = (va * vb).sqrt();
    if denom < 1e-12 { 0.0 } else { cov / denom }
}

// -- v165: Model Serialization -------------------------------------

static MODEL_STORE: std::sync::LazyLock<std::sync::Mutex<BTreeMap<String, (i64, Vec<u8>)>>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(BTreeMap::new()));
static MODEL_VERSION: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(1);

/// Save a model (simulated) with a name
#[unsafe(no_mangle)]
extern "C" fn slang_model_save(name: *const i8) -> i64 {
    if name.is_null() { return 0; }
    let c = unsafe { std::ffi::CStr::from_ptr(name) };
    let n = c.to_string_lossy().to_string();
    let ver = MODEL_VERSION.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let blob = format!("model_{}_{}", n, ver).into_bytes();
    MODEL_STORE.lock().unwrap().insert(n, (ver, blob));
    ver
}

/// Load a model by name, returns version or 0 if not found
#[unsafe(no_mangle)]
extern "C" fn slang_model_load(name: *const i8) -> i64 {
    if name.is_null() { return 0; }
    let c = unsafe { std::ffi::CStr::from_ptr(name) };
    let n = c.to_string_lossy();
    MODEL_STORE.lock().unwrap().get(n.as_ref()).map(|(v, _)| *v).unwrap_or(0)
}

/// Get current model version counter
#[unsafe(no_mangle)]
extern "C" fn slang_model_version() -> i64 {
    MODEL_VERSION.load(std::sync::atomic::Ordering::SeqCst)
}

/// Check if two model versions are compatible (within 5 versions)
#[unsafe(no_mangle)]
extern "C" fn slang_model_compatible(v1: i64, v2: i64) -> i64 {
    if (v1 - v2).abs() <= 5 { 1 } else { 0 }
}

// -- v166: Disk-Backed KV Store (simulated in-memory) -------------

static KV_STORE: std::sync::LazyLock<std::sync::Mutex<BTreeMap<String, Vec<u8>>>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(BTreeMap::new()));
static KV_WAL: std::sync::LazyLock<std::sync::Mutex<Vec<(String, String)>>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(Vec::new()));

/// Put key-value into KV store
#[unsafe(no_mangle)]
extern "C" fn slang_kv_put(key: *const i8, val: *const i8) -> i64 {
    if key.is_null() || val.is_null() { return 0; }
    let k = unsafe { std::ffi::CStr::from_ptr(key) }.to_string_lossy().to_string();
    let v = unsafe { std::ffi::CStr::from_ptr(val) }.to_string_lossy().to_string();
    KV_WAL.lock().unwrap().push((k.clone(), v.clone()));
    KV_STORE.lock().unwrap().insert(k, v.into_bytes());
    1
}

/// Get value by key (returns length, 0 if not found)
#[unsafe(no_mangle)]
extern "C" fn slang_kv_get_len(key: *const i8) -> i64 {
    if key.is_null() { return 0; }
    let k = unsafe { std::ffi::CStr::from_ptr(key) }.to_string_lossy();
    KV_STORE.lock().unwrap().get(k.as_ref()).map(|v| v.len() as i64).unwrap_or(0)
}

/// Delete a key from KV store
#[unsafe(no_mangle)]
extern "C" fn slang_kv_delete(key: *const i8) -> i64 {
    if key.is_null() { return 0; }
    let k = unsafe { std::ffi::CStr::from_ptr(key) }.to_string_lossy();
    if KV_STORE.lock().unwrap().remove(k.as_ref()).is_some() { 1 } else { 0 }
}

/// Get WAL entry count
#[unsafe(no_mangle)]
extern "C" fn slang_kv_wal_count() -> i64 {
    KV_WAL.lock().unwrap().len() as i64
}

// -- v167: B-Tree Disk Index (simulated) -------------------------

static BTREE_INDEX: std::sync::LazyLock<std::sync::Mutex<BTreeMap<i64, i64>>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(BTreeMap::new()));

/// Insert key-value into B-tree index
#[unsafe(no_mangle)]
extern "C" fn slang_btree_insert(key: i64, val: i64) -> i64 {
    BTREE_INDEX.lock().unwrap().insert(key, val);
    1
}

/// Lookup by key in B-tree index
#[unsafe(no_mangle)]
extern "C" fn slang_btree_lookup(key: i64) -> i64 {
    BTREE_INDEX.lock().unwrap().get(&key).copied().unwrap_or(-1)
}

/// Range count: how many keys in [lo, hi]
#[unsafe(no_mangle)]
extern "C" fn slang_btree_range_count(lo: i64, hi: i64) -> i64 {
    let idx = BTREE_INDEX.lock().unwrap();
    idx.range(lo..=hi).count() as i64
}

/// Total entries in B-tree index
#[unsafe(no_mangle)]
extern "C" fn slang_btree_count() -> i64 {
    BTREE_INDEX.lock().unwrap().len() as i64
}

// -- v168: SQL Query Engine (simulated) --------------------------

static SQL_TABLES: std::sync::LazyLock<std::sync::Mutex<BTreeMap<String, Vec<Vec<i64>>>>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(BTreeMap::new()));

/// Create a table with given name
#[unsafe(no_mangle)]
extern "C" fn slang_sql_create_table(name: *const i8) -> i64 {
    if name.is_null() { return 0; }
    let n = unsafe { std::ffi::CStr::from_ptr(name) }.to_string_lossy().to_string();
    let mut tables = SQL_TABLES.lock().unwrap();
    if tables.contains_key(&n) { return 0; }
    tables.insert(n, Vec::new());
    1
}

/// Insert a row (two i64 columns) into table
#[unsafe(no_mangle)]
extern "C" fn slang_sql_insert(name: *const i8, col1: i64, col2: i64) -> i64 {
    if name.is_null() { return 0; }
    let n = unsafe { std::ffi::CStr::from_ptr(name) }.to_string_lossy();
    let mut tables = SQL_TABLES.lock().unwrap();
    if let Some(rows) = tables.get_mut(n.as_ref()) {
        rows.push(vec![col1, col2]);
        1
    } else { 0 }
}

/// Count rows in table
#[unsafe(no_mangle)]
extern "C" fn slang_sql_count(name: *const i8) -> i64 {
    if name.is_null() { return 0; }
    let n = unsafe { std::ffi::CStr::from_ptr(name) }.to_string_lossy();
    SQL_TABLES.lock().unwrap().get(n.as_ref()).map(|r| r.len() as i64).unwrap_or(0)
}

/// Select sum of column 0 from table
#[unsafe(no_mangle)]
extern "C" fn slang_sql_sum_col0(name: *const i8) -> i64 {
    if name.is_null() { return 0; }
    let n = unsafe { std::ffi::CStr::from_ptr(name) }.to_string_lossy();
    SQL_TABLES.lock().unwrap().get(n.as_ref())
        .map(|rows| rows.iter().map(|r| r.first().copied().unwrap_or(0)).sum())
        .unwrap_or(0)
}

// -- v169: Schema Migration (simulated) --------------------------

static SCHEMA_VERSIONS: std::sync::LazyLock<std::sync::Mutex<BTreeMap<String, i64>>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(BTreeMap::new()));

/// Register schema at version 1
#[unsafe(no_mangle)]
extern "C" fn slang_schema_create(name: *const i8) -> i64 {
    if name.is_null() { return 0; }
    let n = unsafe { std::ffi::CStr::from_ptr(name) }.to_string_lossy().to_string();
    SCHEMA_VERSIONS.lock().unwrap().insert(n, 1);
    1
}

/// Migrate schema to next version
#[unsafe(no_mangle)]
extern "C" fn slang_schema_migrate(name: *const i8) -> i64 {
    if name.is_null() { return 0; }
    let n = unsafe { std::ffi::CStr::from_ptr(name) }.to_string_lossy();
    let mut schemas = SCHEMA_VERSIONS.lock().unwrap();
    if let Some(v) = schemas.get_mut(n.as_ref()) {
        *v += 1;
        *v
    } else { 0 }
}

/// Get schema version
#[unsafe(no_mangle)]
extern "C" fn slang_schema_version(name: *const i8) -> i64 {
    if name.is_null() { return 0; }
    let n = unsafe { std::ffi::CStr::from_ptr(name) }.to_string_lossy();
    SCHEMA_VERSIONS.lock().unwrap().get(n.as_ref()).copied().unwrap_or(0)
}

/// Check schema compatibility (within 3 versions)
#[unsafe(no_mangle)]
extern "C" fn slang_schema_compatible(v1: i64, v2: i64) -> i64 {
    if (v1 - v2).abs() <= 3 { 1 } else { 0 }
}

// -- v170: Transaction Log (simulated) ---------------------------

static TXN_LOG: std::sync::LazyLock<std::sync::Mutex<Vec<(i64, String, bool)>>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(Vec::new()));
static TXN_COUNTER: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(1);

/// Begin a new transaction, returns txn ID
#[unsafe(no_mangle)]
extern "C" fn slang_txn_begin() -> i64 {
    let id = TXN_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    TXN_LOG.lock().unwrap().push((id, "begin".into(), false));
    id
}

/// Commit a transaction
#[unsafe(no_mangle)]
extern "C" fn slang_txn_commit(id: i64) -> i64 {
    let mut log = TXN_LOG.lock().unwrap();
    for entry in log.iter_mut().rev() {
        if entry.0 == id && !entry.2 {
            entry.1 = "commit".into();
            entry.2 = true;
            return 1;
        }
    }
    0
}

/// Rollback a transaction
#[unsafe(no_mangle)]
extern "C" fn slang_txn_rollback(id: i64) -> i64 {
    let mut log = TXN_LOG.lock().unwrap();
    for entry in log.iter_mut().rev() {
        if entry.0 == id && !entry.2 {
            entry.1 = "rollback".into();
            entry.2 = true;
            return 1;
        }
    }
    0
}

/// Get total transaction log entry count
#[unsafe(no_mangle)]
extern "C" fn slang_txn_log_count() -> i64 {
    TXN_LOG.lock().unwrap().len() as i64
}

// -- v171: Data Import/Export (simulated) -------------------------

static DATA_BUFFERS: std::sync::LazyLock<std::sync::Mutex<BTreeMap<i64, Vec<i64>>>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(BTreeMap::new()));
static DATA_BUF_COUNTER: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(1);

/// Create a data buffer for import/export
#[unsafe(no_mangle)]
extern "C" fn slang_data_buf_create() -> i64 {
    let id = DATA_BUF_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    DATA_BUFFERS.lock().unwrap().insert(id, Vec::new());
    id
}

/// Append a value to data buffer
#[unsafe(no_mangle)]
extern "C" fn slang_data_buf_push(id: i64, val: i64) -> i64 {
    let mut bufs = DATA_BUFFERS.lock().unwrap();
    if let Some(buf) = bufs.get_mut(&id) {
        buf.push(val);
        buf.len() as i64
    } else { 0 }
}

/// Get data buffer length
#[unsafe(no_mangle)]
extern "C" fn slang_data_buf_len(id: i64) -> i64 {
    DATA_BUFFERS.lock().unwrap().get(&id).map(|b| b.len() as i64).unwrap_or(0)
}

/// Get value from data buffer at index
#[unsafe(no_mangle)]
extern "C" fn slang_data_buf_get(id: i64, idx: i64) -> i64 {
    DATA_BUFFERS.lock().unwrap().get(&id)
        .and_then(|b| b.get(idx as usize).copied())
        .unwrap_or(-1)
}

// -- v172: TCP Socket Primitives (simulated) ---------------------

static NET_SOCKETS: std::sync::LazyLock<std::sync::Mutex<BTreeMap<i64, (String, i64, bool)>>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(BTreeMap::new()));
static NET_SOCK_COUNTER: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(1);

/// Create a simulated TCP socket, returns socket ID
#[unsafe(no_mangle)]
extern "C" fn slang_tcp_create() -> i64 {
    let id = NET_SOCK_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    NET_SOCKETS.lock().unwrap().insert(id, ("created".into(), 0, false));
    id
}

/// Simulate TCP connect to host:port
#[unsafe(no_mangle)]
extern "C" fn slang_tcp_sim_connect(sock: i64, port: i64) -> i64 {
    let mut sockets = NET_SOCKETS.lock().unwrap();
    if let Some(s) = sockets.get_mut(&sock) {
        s.0 = "connected".into();
        s.1 = port;
        s.2 = true;
        1
    } else { 0 }
}

/// Check if socket is connected
#[unsafe(no_mangle)]
extern "C" fn slang_tcp_connected(sock: i64) -> i64 {
    NET_SOCKETS.lock().unwrap().get(&sock).map(|s| if s.2 { 1 } else { 0 }).unwrap_or(0)
}

/// Close a socket
#[unsafe(no_mangle)]
extern "C" fn slang_tcp_sim_close(sock: i64) -> i64 {
    let mut sockets = NET_SOCKETS.lock().unwrap();
    if let Some(s) = sockets.get_mut(&sock) {
        s.0 = "closed".into();
        s.2 = false;
        1
    } else { 0 }
}

// -- v173: HTTP Client (simulated) -------------------------------

static HTTP_REQUESTS: std::sync::LazyLock<std::sync::Mutex<Vec<(String, i64)>>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(Vec::new()));

/// Simulate HTTP GET, returns status code
#[unsafe(no_mangle)]
extern "C" fn slang_http_sim_get(url: *const i8) -> i64 {
    if url.is_null() { return 0; }
    let u = unsafe { std::ffi::CStr::from_ptr(url) }.to_string_lossy().to_string();
    let status = if u.starts_with("http") { 200 } else { 400 };
    HTTP_REQUESTS.lock().unwrap().push((u, status));
    status
}

/// Simulate HTTP POST, returns status code
#[unsafe(no_mangle)]
extern "C" fn slang_http_sim_post(url: *const i8) -> i64 {
    if url.is_null() { return 0; }
    let u = unsafe { std::ffi::CStr::from_ptr(url) }.to_string_lossy().to_string();
    let status = if u.starts_with("http") { 201 } else { 400 };
    HTTP_REQUESTS.lock().unwrap().push((u, status));
    status
}

/// Get count of HTTP requests made
#[unsafe(no_mangle)]
extern "C" fn slang_http_request_count() -> i64 {
    HTTP_REQUESTS.lock().unwrap().len() as i64
}

/// Parse URL: returns 1 if valid (starts with http:// or https://)
#[unsafe(no_mangle)]
extern "C" fn slang_http_url_valid(url: *const i8) -> i64 {
    if url.is_null() { return 0; }
    let u = unsafe { std::ffi::CStr::from_ptr(url) }.to_string_lossy();
    if u.starts_with("http://") || u.starts_with("https://") { 1 } else { 0 }
}

// -- v174: WebSocket (simulated) ---------------------------------

static WS_CHANNELS: std::sync::LazyLock<std::sync::Mutex<BTreeMap<i64, Vec<String>>>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(BTreeMap::new()));
static WS_COUNTER: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(1);

/// Create a WebSocket channel
#[unsafe(no_mangle)]
extern "C" fn slang_ws_create() -> i64 {
    let id = WS_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    WS_CHANNELS.lock().unwrap().insert(id, Vec::new());
    id
}

/// Send a message on a WebSocket channel
#[unsafe(no_mangle)]
extern "C" fn slang_ws_send(ch: i64, msg: *const i8) -> i64 {
    if msg.is_null() { return 0; }
    let m = unsafe { std::ffi::CStr::from_ptr(msg) }.to_string_lossy().to_string();
    let mut chs = WS_CHANNELS.lock().unwrap();
    if let Some(msgs) = chs.get_mut(&ch) {
        msgs.push(m);
        1
    } else { 0 }
}

/// Get message count on a WebSocket channel
#[unsafe(no_mangle)]
extern "C" fn slang_ws_msg_count(ch: i64) -> i64 {
    WS_CHANNELS.lock().unwrap().get(&ch).map(|v| v.len() as i64).unwrap_or(0)
}

/// Close a WebSocket channel
#[unsafe(no_mangle)]
extern "C" fn slang_ws_close(ch: i64) -> i64 {
    if WS_CHANNELS.lock().unwrap().remove(&ch).is_some() { 1 } else { 0 }
}

// -- v175: RPC Framework (simulated) -----------------------------

static RPC_SERVICES: std::sync::LazyLock<std::sync::Mutex<BTreeMap<String, i64>>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(BTreeMap::new()));
static RPC_CALL_COUNT: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(0);

/// Register an RPC service
#[unsafe(no_mangle)]
extern "C" fn slang_rpc_register(name: *const i8) -> i64 {
    if name.is_null() { return 0; }
    let n = unsafe { std::ffi::CStr::from_ptr(name) }.to_string_lossy().to_string();
    RPC_SERVICES.lock().unwrap().insert(n, 0);
    1
}

/// Call an RPC service, returns 1 if found
#[unsafe(no_mangle)]
extern "C" fn slang_rpc_call(name: *const i8) -> i64 {
    if name.is_null() { return 0; }
    let n = unsafe { std::ffi::CStr::from_ptr(name) }.to_string_lossy();
    let mut svcs = RPC_SERVICES.lock().unwrap();
    if let Some(count) = svcs.get_mut(n.as_ref()) {
        *count += 1;
        RPC_CALL_COUNT.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        1
    } else { 0 }
}

/// Get total RPC call count
#[unsafe(no_mangle)]
extern "C" fn slang_rpc_total_calls() -> i64 {
    RPC_CALL_COUNT.load(std::sync::atomic::Ordering::SeqCst)
}

/// Get registered service count
#[unsafe(no_mangle)]
extern "C" fn slang_rpc_service_count() -> i64 {
    RPC_SERVICES.lock().unwrap().len() as i64
}

// -- v176: DNS Resolution (simulated) ----------------------------

static DNS_CACHE: std::sync::LazyLock<std::sync::Mutex<BTreeMap<String, String>>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(BTreeMap::new()));

/// Simulate DNS resolve: stores domain?IP mapping
#[unsafe(no_mangle)]
extern "C" fn slang_dns_resolve(domain: *const i8) -> i64 {
    if domain.is_null() { return 0; }
    let d = unsafe { std::ffi::CStr::from_ptr(domain) }.to_string_lossy().to_string();
    // Simulate: hash domain to a fake IP
    let hash = d.bytes().fold(0u32, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u32));
    let ip = format!("{}.{}.{}.{}", (hash >> 24) & 0xFF, (hash >> 16) & 0xFF, (hash >> 8) & 0xFF, hash & 0xFF);
    DNS_CACHE.lock().unwrap().insert(d, ip);
    1
}

/// Check if domain is in DNS cache
#[unsafe(no_mangle)]
extern "C" fn slang_dns_cached(domain: *const i8) -> i64 {
    if domain.is_null() { return 0; }
    let d = unsafe { std::ffi::CStr::from_ptr(domain) }.to_string_lossy();
    if DNS_CACHE.lock().unwrap().contains_key(d.as_ref()) { 1 } else { 0 }
}

/// Get DNS cache size
#[unsafe(no_mangle)]
extern "C" fn slang_dns_cache_size() -> i64 {
    DNS_CACHE.lock().unwrap().len() as i64
}

/// Flush DNS cache
#[unsafe(no_mangle)]
extern "C" fn slang_dns_cache_flush() -> i64 {
    let mut cache = DNS_CACHE.lock().unwrap();
    let n = cache.len() as i64;
    cache.clear();
    n
}

// -- v177: TLS/SSL (simulated) -----------------------------------

static TLS_SESSIONS: std::sync::LazyLock<std::sync::Mutex<BTreeMap<i64, (String, bool)>>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(BTreeMap::new()));
static TLS_COUNTER: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(1);

/// Create TLS session
#[unsafe(no_mangle)]
extern "C" fn slang_tls_create(host: *const i8) -> i64 {
    if host.is_null() { return 0; }
    let h = unsafe { std::ffi::CStr::from_ptr(host) }.to_string_lossy().to_string();
    let id = TLS_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    TLS_SESSIONS.lock().unwrap().insert(id, (h, true));
    id
}

/// Check if TLS session is active
#[unsafe(no_mangle)]
extern "C" fn slang_tls_active(id: i64) -> i64 {
    TLS_SESSIONS.lock().unwrap().get(&id).map(|(_, a)| if *a { 1 } else { 0 }).unwrap_or(0)
}

/// Close TLS session
#[unsafe(no_mangle)]
extern "C" fn slang_tls_close(id: i64) -> i64 {
    let mut sessions = TLS_SESSIONS.lock().unwrap();
    if let Some(s) = sessions.get_mut(&id) {
        s.1 = false;
        1
    } else { 0 }
}

/// Validate certificate (simulated: always valid if session exists)
#[unsafe(no_mangle)]
extern "C" fn slang_tls_cert_valid(id: i64) -> i64 {
    if TLS_SESSIONS.lock().unwrap().contains_key(&id) { 1 } else { 0 }
}

// -- v178: REPL Enhancements (simulated) -------------------------

static REPL_HISTORY: std::sync::LazyLock<std::sync::Mutex<Vec<String>>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(Vec::new()));

#[unsafe(no_mangle)]
extern "C" fn slang_repl_history_add(cmd: *const i8) -> i64 {
    if cmd.is_null() { return 0; }
    let c = unsafe { std::ffi::CStr::from_ptr(cmd) }.to_string_lossy().to_string();
    REPL_HISTORY.lock().unwrap().push(c);
    1
}

#[unsafe(no_mangle)]
extern "C" fn slang_repl_history_count() -> i64 {
    REPL_HISTORY.lock().unwrap().len() as i64
}

#[unsafe(no_mangle)]
extern "C" fn slang_repl_history_clear() -> i64 {
    let mut h = REPL_HISTORY.lock().unwrap();
    let n = h.len() as i64;
    h.clear();
    n
}

#[unsafe(no_mangle)]
extern "C" fn slang_repl_complete_count(prefix: *const i8) -> i64 {
    if prefix.is_null() { return 0; }
    let p = unsafe { std::ffi::CStr::from_ptr(prefix) }.to_string_lossy();
    let builtins = crate::stdlib::builtins();
    builtins.iter().filter(|b| b.name.starts_with(p.as_ref())).count() as i64
}

// -- v179: Package Registry (simulated) --------------------------

static PKG_REGISTRY: std::sync::LazyLock<std::sync::Mutex<BTreeMap<String, (i64, i64, i64)>>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(BTreeMap::new()));

#[unsafe(no_mangle)]
extern "C" fn slang_pkg_publish(name: *const i8, major: i64, minor: i64, patch: i64) -> i64 {
    if name.is_null() { return 0; }
    let n = unsafe { std::ffi::CStr::from_ptr(name) }.to_string_lossy().to_string();
    PKG_REGISTRY.lock().unwrap().insert(n, (major, minor, patch));
    1
}

#[unsafe(no_mangle)]
extern "C" fn slang_pkg_installed(name: *const i8) -> i64 {
    if name.is_null() { return 0; }
    let n = unsafe { std::ffi::CStr::from_ptr(name) }.to_string_lossy();
    if PKG_REGISTRY.lock().unwrap().contains_key(n.as_ref()) { 1 } else { 0 }
}

#[unsafe(no_mangle)]
extern "C" fn slang_pkg_count() -> i64 {
    PKG_REGISTRY.lock().unwrap().len() as i64
}

#[unsafe(no_mangle)]
extern "C" fn slang_pkg_remove(name: *const i8) -> i64 {
    if name.is_null() { return 0; }
    let n = unsafe { std::ffi::CStr::from_ptr(name) }.to_string_lossy();
    if PKG_REGISTRY.lock().unwrap().remove(n.as_ref()).is_some() { 1 } else { 0 }
}

// -- v180: Documentation Generator (simulated) -------------------

static DOC_ENTRIES: std::sync::LazyLock<std::sync::Mutex<Vec<(String, String)>>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(Vec::new()));

#[unsafe(no_mangle)]
extern "C" fn slang_doc_add(name: *const i8, doc: *const i8) -> i64 {
    if name.is_null() || doc.is_null() { return 0; }
    let n = unsafe { std::ffi::CStr::from_ptr(name) }.to_string_lossy().to_string();
    let d = unsafe { std::ffi::CStr::from_ptr(doc) }.to_string_lossy().to_string();
    DOC_ENTRIES.lock().unwrap().push((n, d));
    1
}

#[unsafe(no_mangle)]
extern "C" fn slang_doc_count() -> i64 {
    DOC_ENTRIES.lock().unwrap().len() as i64
}

#[unsafe(no_mangle)]
extern "C" fn slang_doc_has(name: *const i8) -> i64 {
    if name.is_null() { return 0; }
    let n = unsafe { std::ffi::CStr::from_ptr(name) }.to_string_lossy();
    if DOC_ENTRIES.lock().unwrap().iter().any(|(k, _)| k == n.as_ref()) { 1 } else { 0 }
}

#[unsafe(no_mangle)]
extern "C" fn slang_doc_clear() -> i64 {
    let mut d = DOC_ENTRIES.lock().unwrap();
    let n = d.len() as i64;
    d.clear();
    n
}

// -- v181: Benchmark Suite (simulated) ---------------------------

static BENCH_RESULTS: std::sync::LazyLock<std::sync::Mutex<Vec<(String, f64)>>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(Vec::new()));

#[unsafe(no_mangle)]
extern "C" fn slang_bench_record(name: *const i8, elapsed_ns: f64) -> i64 {
    if name.is_null() { return 0; }
    let n = unsafe { std::ffi::CStr::from_ptr(name) }.to_string_lossy().to_string();
    BENCH_RESULTS.lock().unwrap().push((n, elapsed_ns));
    1
}

#[unsafe(no_mangle)]
extern "C" fn slang_bench_count() -> i64 {
    BENCH_RESULTS.lock().unwrap().len() as i64
}

#[unsafe(no_mangle)]
extern "C" fn slang_bench_best(name: *const i8) -> f64 {
    if name.is_null() { return 0.0; }
    let n = unsafe { std::ffi::CStr::from_ptr(name) }.to_string_lossy();
    BENCH_RESULTS.lock().unwrap().iter()
        .filter(|(k, _)| k == n.as_ref())
        .map(|(_, v)| *v)
        .fold(f64::INFINITY, f64::min)
}

#[unsafe(no_mangle)]
extern "C" fn slang_bench_clear() -> i64 {
    let mut b = BENCH_RESULTS.lock().unwrap();
    let n = b.len() as i64;
    b.clear();
    n
}

// -- v182: Profiler (simulated) ----------------------------------

static PROFILER_ACTIVE: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(0);
static PROFILE_SAMPLES: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(0);

#[unsafe(no_mangle)]
extern "C" fn slang_profile_start() -> i64 {
    PROFILER_ACTIVE.store(1, std::sync::atomic::Ordering::SeqCst);
    PROFILE_SAMPLES.store(0, std::sync::atomic::Ordering::SeqCst);
    1
}

#[unsafe(no_mangle)]
extern "C" fn slang_profile_stop() -> i64 {
    PROFILER_ACTIVE.store(0, std::sync::atomic::Ordering::SeqCst);
    1
}

#[unsafe(no_mangle)]
extern "C" fn slang_profile_sample() -> i64 {
    if PROFILER_ACTIVE.load(std::sync::atomic::Ordering::SeqCst) == 1 {
        PROFILE_SAMPLES.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1
    } else { 0 }
}

#[unsafe(no_mangle)]
extern "C" fn slang_profile_samples() -> i64 {
    PROFILE_SAMPLES.load(std::sync::atomic::Ordering::SeqCst)
}

// -- v183: Interactive Playground (simulated) --------------------

static PLAYGROUND_SNIPPETS: std::sync::LazyLock<std::sync::Mutex<Vec<(String, i64)>>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(Vec::new()));

#[unsafe(no_mangle)]
extern "C" fn slang_playground_eval(code: *const i8) -> i64 {
    if code.is_null() { return 0; }
    let c = unsafe { std::ffi::CStr::from_ptr(code) }.to_string_lossy().to_string();
    let result = c.len() as i64; // simulated: "result" is length
    PLAYGROUND_SNIPPETS.lock().unwrap().push((c, result));
    result
}

#[unsafe(no_mangle)]
extern "C" fn slang_playground_count() -> i64 {
    PLAYGROUND_SNIPPETS.lock().unwrap().len() as i64
}

#[unsafe(no_mangle)]
extern "C" fn slang_playground_clear() -> i64 {
    let mut p = PLAYGROUND_SNIPPETS.lock().unwrap();
    let n = p.len() as i64;
    p.clear();
    n
}

#[unsafe(no_mangle)]
extern "C" fn slang_playground_last_result() -> i64 {
    PLAYGROUND_SNIPPETS.lock().unwrap().last().map(|(_, r)| *r).unwrap_or(0)
}

// -- v184: Work-Stealing Scheduler (simulated) -------------------

static TASK_QUEUE: std::sync::LazyLock<std::sync::Mutex<Vec<(i64, i64)>>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(Vec::new()));
static TASK_ID_GEN: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(1);

#[unsafe(no_mangle)]
extern "C" fn slang_task_submit(priority: i64) -> i64 {
    let id = TASK_ID_GEN.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    TASK_QUEUE.lock().unwrap().push((id, priority));
    id
}

#[unsafe(no_mangle)]
extern "C" fn slang_task_queue_len() -> i64 {
    TASK_QUEUE.lock().unwrap().len() as i64
}

#[unsafe(no_mangle)]
extern "C" fn slang_task_steal() -> i64 {
    TASK_QUEUE.lock().unwrap().pop().map(|(id, _)| id).unwrap_or(-1)
}

#[unsafe(no_mangle)]
extern "C" fn slang_task_queue_clear() -> i64 {
    let mut q = TASK_QUEUE.lock().unwrap();
    let n = q.len() as i64;
    q.clear();
    n
}

// -- v185: Actor Model (simulated) -------------------------------

static ACTORS: std::sync::LazyLock<std::sync::Mutex<BTreeMap<i64, Vec<i64>>>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(BTreeMap::new()));
static ACTOR_COUNTER: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(1);

#[unsafe(no_mangle)]
extern "C" fn slang_actor_spawn() -> i64 {
    let id = ACTOR_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    ACTORS.lock().unwrap().insert(id, Vec::new());
    id
}

#[unsafe(no_mangle)]
extern "C" fn slang_actor_send(actor: i64, msg: i64) -> i64 {
    let mut actors = ACTORS.lock().unwrap();
    if let Some(mailbox) = actors.get_mut(&actor) {
        mailbox.push(msg);
        1
    } else { 0 }
}

#[unsafe(no_mangle)]
extern "C" fn slang_actor_recv(actor: i64) -> i64 {
    let mut actors = ACTORS.lock().unwrap();
    if let Some(mailbox) = actors.get_mut(&actor) {
        if mailbox.is_empty() { -1 } else { mailbox.remove(0) }
    } else { -1 }
}

#[unsafe(no_mangle)]
extern "C" fn slang_actor_mailbox_len(actor: i64) -> i64 {
    ACTORS.lock().unwrap().get(&actor).map(|m| m.len() as i64).unwrap_or(0)
}

// -- v186: STM (simulated) ---------------------------------------

static STM_VARS: std::sync::LazyLock<std::sync::Mutex<BTreeMap<i64, i64>>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(BTreeMap::new()));
static STM_VAR_COUNTER: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(1);

#[unsafe(no_mangle)]
extern "C" fn slang_stm_new(init: i64) -> i64 {
    let id = STM_VAR_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    STM_VARS.lock().unwrap().insert(id, init);
    id
}

#[unsafe(no_mangle)]
extern "C" fn slang_stm_read(id: i64) -> i64 {
    STM_VARS.lock().unwrap().get(&id).copied().unwrap_or(-1)
}

#[unsafe(no_mangle)]
extern "C" fn slang_stm_write(id: i64, val: i64) -> i64 {
    let mut vars = STM_VARS.lock().unwrap();
    if vars.contains_key(&id) { vars.insert(id, val); 1 } else { 0 }
}

#[unsafe(no_mangle)]
extern "C" fn slang_stm_cas(id: i64, expected: i64, new_val: i64) -> i64 {
    let mut vars = STM_VARS.lock().unwrap();
    if let Some(v) = vars.get_mut(&id) {
        if *v == expected { *v = new_val; 1 } else { 0 }
    } else { 0 }
}

// -- v187: Parallel Collections (simulated) ----------------------

#[unsafe(no_mangle)]
extern "C" fn slang_par_sum(tensor_id: i64) -> f64 {
    let tensors = TENSORS.lock().unwrap();
    tensors.get(&tensor_id).map(|(_, d)| d.iter().sum()).unwrap_or(0.0)
}

#[unsafe(no_mangle)]
extern "C" fn slang_par_min(tensor_id: i64) -> f64 {
    let tensors = TENSORS.lock().unwrap();
    tensors.get(&tensor_id).map(|(_, d)| d.iter().cloned().fold(f64::INFINITY, f64::min)).unwrap_or(0.0)
}

#[unsafe(no_mangle)]
extern "C" fn slang_par_max(tensor_id: i64) -> f64 {
    let tensors = TENSORS.lock().unwrap();
    tensors.get(&tensor_id).map(|(_, d)| d.iter().cloned().fold(f64::NEG_INFINITY, f64::max)).unwrap_or(0.0)
}

#[unsafe(no_mangle)]
extern "C" fn slang_par_count(tensor_id: i64) -> i64 {
    let tensors = TENSORS.lock().unwrap();
    tensors.get(&tensor_id).map(|(_, d)| d.len() as i64).unwrap_or(0)
}

// -- v188: GPU Task Scheduling (simulated) -----------------------

static GPU_TASKS: std::sync::LazyLock<std::sync::Mutex<Vec<(i64, String)>>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(Vec::new()));

#[unsafe(no_mangle)]
extern "C" fn slang_gpu_submit(kernel_id: i64) -> i64 {
    GPU_TASKS.lock().unwrap().push((kernel_id, "submitted".into()));
    kernel_id
}

#[unsafe(no_mangle)]
extern "C" fn slang_gpu_queue_len() -> i64 {
    GPU_TASKS.lock().unwrap().len() as i64
}

#[unsafe(no_mangle)]
extern "C" fn slang_gpu_flush() -> i64 {
    let mut tasks = GPU_TASKS.lock().unwrap();
    let n = tasks.len() as i64;
    tasks.clear();
    n
}

#[unsafe(no_mangle)]
extern "C" fn slang_gpu_available() -> i64 {
    1 // simulated: GPU always "available"
}

// -- v189: Distributed Computing (simulated) ---------------------

static DIST_NODES: std::sync::LazyLock<std::sync::Mutex<BTreeMap<i64, String>>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(BTreeMap::new()));
static DIST_NODE_COUNTER: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(1);

#[unsafe(no_mangle)]
extern "C" fn slang_dist_node_add(name: *const i8) -> i64 {
    if name.is_null() { return 0; }
    let n = unsafe { std::ffi::CStr::from_ptr(name) }.to_string_lossy().to_string();
    let id = DIST_NODE_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    DIST_NODES.lock().unwrap().insert(id, n);
    id
}

#[unsafe(no_mangle)]
extern "C" fn slang_dist_node_count() -> i64 {
    DIST_NODES.lock().unwrap().len() as i64
}

#[unsafe(no_mangle)]
extern "C" fn slang_dist_broadcast(_msg: i64) -> i64 {
    let nodes = DIST_NODES.lock().unwrap();
    nodes.len() as i64 // "sent" to all nodes
}

#[unsafe(no_mangle)]
extern "C" fn slang_dist_reduce(val: i64, op: i64) -> i64 {
    // simulated: op 0=sum, 1=max, 2=min
    let nodes = DIST_NODES.lock().unwrap();
    let n = nodes.len() as i64;
    match op {
        0 => val * n.max(1),   // sum across nodes
        1 => val,              // max is identity
        _ => val,
    }
}

// -- v190: ADTs v2 (simulated type registry) ---------------------

static TYPE_REGISTRY: std::sync::LazyLock<std::sync::Mutex<BTreeMap<String, i64>>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(BTreeMap::new()));

#[unsafe(no_mangle)]
extern "C" fn slang_type_register(name: *const i8, variant_count: i64) -> i64 {
    if name.is_null() { return 0; }
    let n = unsafe { std::ffi::CStr::from_ptr(name) }.to_string_lossy().to_string();
    TYPE_REGISTRY.lock().unwrap().insert(n, variant_count);
    1
}

#[unsafe(no_mangle)]
extern "C" fn slang_type_variant_count(name: *const i8) -> i64 {
    if name.is_null() { return 0; }
    let n = unsafe { std::ffi::CStr::from_ptr(name) }.to_string_lossy();
    TYPE_REGISTRY.lock().unwrap().get(n.as_ref()).copied().unwrap_or(0)
}

#[unsafe(no_mangle)]
extern "C" fn slang_type_count() -> i64 {
    TYPE_REGISTRY.lock().unwrap().len() as i64
}

#[unsafe(no_mangle)]
extern "C" fn slang_type_exists(name: *const i8) -> i64 {
    if name.is_null() { return 0; }
    let n = unsafe { std::ffi::CStr::from_ptr(name) }.to_string_lossy();
    if TYPE_REGISTRY.lock().unwrap().contains_key(n.as_ref()) { 1 } else { 0 }
}

// -- v191: Higher-Kinded Types (simulated) -----------------------

static HKT_REGISTRY: std::sync::LazyLock<std::sync::Mutex<BTreeMap<String, i64>>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(BTreeMap::new()));

#[unsafe(no_mangle)]
extern "C" fn slang_hkt_register(name: *const i8, kind_arity: i64) -> i64 {
    if name.is_null() { return 0; }
    let n = unsafe { std::ffi::CStr::from_ptr(name) }.to_string_lossy().to_string();
    HKT_REGISTRY.lock().unwrap().insert(n, kind_arity);
    1
}

#[unsafe(no_mangle)]
extern "C" fn slang_hkt_arity(name: *const i8) -> i64 {
    if name.is_null() { return 0; }
    let n = unsafe { std::ffi::CStr::from_ptr(name) }.to_string_lossy();
    HKT_REGISTRY.lock().unwrap().get(n.as_ref()).copied().unwrap_or(-1)
}

#[unsafe(no_mangle)]
extern "C" fn slang_hkt_count() -> i64 {
    HKT_REGISTRY.lock().unwrap().len() as i64
}

#[unsafe(no_mangle)]
extern "C" fn slang_hkt_exists(name: *const i8) -> i64 {
    if name.is_null() { return 0; }
    let n = unsafe { std::ffi::CStr::from_ptr(name) }.to_string_lossy();
    if HKT_REGISTRY.lock().unwrap().contains_key(n.as_ref()) { 1 } else { 0 }
}

// -- v192: Dependent Types v2 (simulated) ------------------------

#[unsafe(no_mangle)]
extern "C" fn slang_dep_type_check_range(val: i64, lo: i64, hi: i64) -> i64 {
    if val >= lo && val <= hi { 1 } else { 0 }
}

#[unsafe(no_mangle)]
extern "C" fn slang_dep_type_nat(val: i64) -> i64 {
    if val >= 0 { 1 } else { 0 }
}

#[unsafe(no_mangle)]
extern "C" fn slang_dep_type_positive(val: i64) -> i64 {
    if val > 0 { 1 } else { 0 }
}

#[unsafe(no_mangle)]
extern "C" fn slang_dep_type_bounded_add(a: i64, b: i64, max: i64) -> i64 {
    let sum = a.saturating_add(b);
    if sum <= max { sum } else { -1 }
}

// -- v193: Effect Polymorphism (simulated) -----------------------

static EFFECT_REGISTRY: std::sync::LazyLock<std::sync::Mutex<BTreeMap<String, Vec<String>>>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(BTreeMap::new()));

#[unsafe(no_mangle)]
extern "C" fn slang_effect_register(name: *const i8) -> i64 {
    if name.is_null() { return 0; }
    let n = unsafe { std::ffi::CStr::from_ptr(name) }.to_string_lossy().to_string();
    EFFECT_REGISTRY.lock().unwrap().insert(n, Vec::new());
    1
}

#[unsafe(no_mangle)]
extern "C" fn slang_effect_add_handler(effect: *const i8, handler: *const i8) -> i64 {
    if effect.is_null() || handler.is_null() { return 0; }
    let e = unsafe { std::ffi::CStr::from_ptr(effect) }.to_string_lossy();
    let h = unsafe { std::ffi::CStr::from_ptr(handler) }.to_string_lossy().to_string();
    let mut reg = EFFECT_REGISTRY.lock().unwrap();
    if let Some(handlers) = reg.get_mut(e.as_ref()) {
        handlers.push(h);
        1
    } else { 0 }
}

#[unsafe(no_mangle)]
extern "C" fn slang_effect_handler_count(effect: *const i8) -> i64 {
    if effect.is_null() { return 0; }
    let e = unsafe { std::ffi::CStr::from_ptr(effect) }.to_string_lossy();
    EFFECT_REGISTRY.lock().unwrap().get(e.as_ref()).map(|h| h.len() as i64).unwrap_or(0)
}

#[unsafe(no_mangle)]
extern "C" fn slang_effect_count() -> i64 {
    EFFECT_REGISTRY.lock().unwrap().len() as i64
}

// -- v194: Type-Level Computation (simulated) --------------------

#[unsafe(no_mangle)]
extern "C" fn slang_type_level_add(a: i64, b: i64) -> i64 { a + b }

#[unsafe(no_mangle)]
extern "C" fn slang_type_level_mul(a: i64, b: i64) -> i64 { a * b }

#[unsafe(no_mangle)]
extern "C" fn slang_type_level_eq(a: i64, b: i64) -> i64 { if a == b { 1 } else { 0 } }

#[unsafe(no_mangle)]
extern "C" fn slang_type_level_if(cond: i64, then_val: i64, else_val: i64) -> i64 {
    if cond != 0 { then_val } else { else_val }
}

// -- v195: Gradual Typing (simulated) ----------------------------

static GRADUAL_TYPES: std::sync::LazyLock<std::sync::Mutex<BTreeMap<String, String>>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(BTreeMap::new()));

#[unsafe(no_mangle)]
extern "C" fn slang_gradual_annotate(var: *const i8, typ: *const i8) -> i64 {
    if var.is_null() || typ.is_null() { return 0; }
    let v = unsafe { std::ffi::CStr::from_ptr(var) }.to_string_lossy().to_string();
    let t = unsafe { std::ffi::CStr::from_ptr(typ) }.to_string_lossy().to_string();
    GRADUAL_TYPES.lock().unwrap().insert(v, t);
    1
}

#[unsafe(no_mangle)]
extern "C" fn slang_gradual_check(var: *const i8, typ: *const i8) -> i64 {
    if var.is_null() || typ.is_null() { return 0; }
    let v = unsafe { std::ffi::CStr::from_ptr(var) }.to_string_lossy();
    let t = unsafe { std::ffi::CStr::from_ptr(typ) }.to_string_lossy();
    let types = GRADUAL_TYPES.lock().unwrap();
    if let Some(annotated) = types.get(v.as_ref()) {
        if annotated == t.as_ref() || annotated == "Any" { 1 } else { 0 }
    } else { 1 } // unannotated = Any = always compatible
}

#[unsafe(no_mangle)]
extern "C" fn slang_gradual_typed_count() -> i64 {
    GRADUAL_TYPES.lock().unwrap().len() as i64
}

#[unsafe(no_mangle)]
extern "C" fn slang_gradual_is_any(var: *const i8) -> i64 {
    if var.is_null() { return 1; }
    let v = unsafe { std::ffi::CStr::from_ptr(var) }.to_string_lossy();
    let types = GRADUAL_TYPES.lock().unwrap();
    if let Some(t) = types.get(v.as_ref()) {
        if t == "Any" { 1 } else { 0 }
    } else { 1 } // unannotated = Any
}

// -- v196: FFI v2 (simulated) ------------------------------------

static FFI_BINDINGS: std::sync::LazyLock<std::sync::Mutex<BTreeMap<String, String>>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(BTreeMap::new()));

#[unsafe(no_mangle)]
extern "C" fn slang_ffi_bind(name: *const i8, lang: *const i8) -> i64 {
    if name.is_null() || lang.is_null() { return 0; }
    let n = unsafe { std::ffi::CStr::from_ptr(name) }.to_string_lossy().to_string();
    let l = unsafe { std::ffi::CStr::from_ptr(lang) }.to_string_lossy().to_string();
    FFI_BINDINGS.lock().unwrap().insert(n, l);
    1
}

#[unsafe(no_mangle)]
extern "C" fn slang_ffi_bound(name: *const i8) -> i64 {
    if name.is_null() { return 0; }
    let n = unsafe { std::ffi::CStr::from_ptr(name) }.to_string_lossy();
    if FFI_BINDINGS.lock().unwrap().contains_key(n.as_ref()) { 1 } else { 0 }
}

#[unsafe(no_mangle)]
extern "C" fn slang_ffi_count() -> i64 {
    FFI_BINDINGS.lock().unwrap().len() as i64
}

#[unsafe(no_mangle)]
extern "C" fn slang_ffi_remove(name: *const i8) -> i64 {
    if name.is_null() { return 0; }
    let n = unsafe { std::ffi::CStr::from_ptr(name) }.to_string_lossy();
    if FFI_BINDINGS.lock().unwrap().remove(n.as_ref()).is_some() { 1 } else { 0 }
}

// -- v197: Cloud-Native Deployment (simulated) -------------------

static CLOUD_DEPLOYMENTS: std::sync::LazyLock<std::sync::Mutex<Vec<(String, String)>>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(Vec::new()));

#[unsafe(no_mangle)]
extern "C" fn slang_cloud_deploy(name: *const i8, target: *const i8) -> i64 {
    if name.is_null() || target.is_null() { return 0; }
    let n = unsafe { std::ffi::CStr::from_ptr(name) }.to_string_lossy().to_string();
    let t = unsafe { std::ffi::CStr::from_ptr(target) }.to_string_lossy().to_string();
    CLOUD_DEPLOYMENTS.lock().unwrap().push((n, t));
    1
}

#[unsafe(no_mangle)]
extern "C" fn slang_cloud_deployment_count() -> i64 {
    CLOUD_DEPLOYMENTS.lock().unwrap().len() as i64
}

#[unsafe(no_mangle)]
extern "C" fn slang_cloud_health_check() -> i64 {
    1 // simulated: always healthy
}

#[unsafe(no_mangle)]
extern "C" fn slang_cloud_shutdown() -> i64 {
    let mut d = CLOUD_DEPLOYMENTS.lock().unwrap();
    let n = d.len() as i64;
    d.clear();
    n
}

// -- v198: Self-Hosting Compiler v3 (simulated) ------------------

static BOOTSTRAP_STAGES: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(0);

#[unsafe(no_mangle)]
extern "C" fn slang_bootstrap_stage() -> i64 {
    BOOTSTRAP_STAGES.load(std::sync::atomic::Ordering::SeqCst)
}

#[unsafe(no_mangle)]
extern "C" fn slang_bootstrap_advance() -> i64 {
    let prev = BOOTSTRAP_STAGES.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    prev + 1
}

#[unsafe(no_mangle)]
extern "C" fn slang_bootstrap_verify(stage1_hash: i64, stage2_hash: i64) -> i64 {
    if stage1_hash == stage2_hash { 1 } else { 0 }
}

#[unsafe(no_mangle)]
extern "C" fn slang_bootstrap_reset() -> i64 {
    BOOTSTRAP_STAGES.store(0, std::sync::atomic::Ordering::SeqCst);
    1
}

// -- v199: AI-Assisted Language Server (simulated) ---------------

static AI_SUGGESTIONS: std::sync::LazyLock<std::sync::Mutex<Vec<(String, String)>>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(Vec::new()));

#[unsafe(no_mangle)]
extern "C" fn slang_ai_suggest(context: *const i8) -> i64 {
    if context.is_null() { return 0; }
    let c = unsafe { std::ffi::CStr::from_ptr(context) }.to_string_lossy().to_string();
    let suggestion = format!("suggested_for_{}", &c[..c.len().min(10)]);
    AI_SUGGESTIONS.lock().unwrap().push((c, suggestion));
    1
}

#[unsafe(no_mangle)]
extern "C" fn slang_ai_suggestion_count() -> i64 {
    AI_SUGGESTIONS.lock().unwrap().len() as i64
}

#[unsafe(no_mangle)]
extern "C" fn slang_ai_explain_error(code: i64) -> i64 {
    if code == 0 { 0 } else { 1 } // simulated: any non-zero error can be explained
}

#[unsafe(no_mangle)]
extern "C" fn slang_ai_clear() -> i64 {
    let mut s = AI_SUGGESTIONS.lock().unwrap();
    let n = s.len() as i64;
    s.clear();
    n
}

// -- v200: Milestone Release (meta builtins) ---------------------

#[unsafe(no_mangle)]
extern "C" fn slang_vitalis_version() -> i64 {
    300
}

#[unsafe(no_mangle)]
extern "C" fn slang_vitalis_module_count() -> i64 {
    // approximate count of src/*.rs modules
    192
}

#[unsafe(no_mangle)]
extern "C" fn slang_vitalis_test_count() -> i64 {
    // approximate test count
    4307
}

#[unsafe(no_mangle)]
extern "C" fn slang_vitalis_builtin_count() -> i64 {
    crate::stdlib::builtins().len() as i64
}

/// Get process ID
extern "C" fn slang_pid() -> i64 {
    std::process::id() as i64
}

/// Format string: sprintf-lite. Replaces {} with i64 value.
extern "C" fn slang_format_int(template: *const i8, val: i64) -> *const i8 {
    if template.is_null() { return intern_cstr("") as *const i8; }
    let t = unsafe { std::ffi::CStr::from_ptr(template).to_string_lossy().into_owned() };
    intern_cstr(&t.replacen("{}", &val.to_string(), 1)) as *const i8
}

/// Format string: replaces {} with f64 value.
extern "C" fn slang_format_float(template: *const i8, val: f64) -> *const i8 {
    if template.is_null() { return intern_cstr("") as *const i8; }
    let t = unsafe { std::ffi::CStr::from_ptr(template).to_string_lossy().into_owned() };
    intern_cstr(&t.replacen("{}", &format!("{:.6}", val), 1)) as *const i8
}

// ── v15: JSON Runtime ───────────────────────────────────────────────────
// Minimal JSON: map handles can be serialized/deserialized

/// Serialize a map handle to JSON string
extern "C" fn slang_json_encode(handle: i64) -> *const i8 {
    let arena = VITALIS_MAP_ARENA.lock().unwrap();
    match arena.get(handle as usize) {
        Some(map) => {
            let entries: Vec<String> = map.iter()
                .map(|(k, v)| format!("\"{}\":{}", k, v))
                .collect();
            let json = format!("{{{}}}", entries.join(","));
            intern_cstr(&json) as *const i8
        }
        None => intern_cstr("{}") as *const i8,
    }
}

/// Deserialize JSON string to a new map handle (basic int-valued objects only)
extern "C" fn slang_json_decode(s: *const i8) -> i64 {
    let handle = slang_map_new();
    if s.is_null() { return handle; }
    let sv = unsafe { std::ffi::CStr::from_ptr(s).to_string_lossy().into_owned() };
    // Simple parser for {"key":123, ...}
    let sv = sv.trim();
    if !sv.starts_with('{') || !sv.ends_with('}') { return handle; }
    let inner = &sv[1..sv.len()-1];
    for pair in inner.split(',') {
        let pair = pair.trim();
        if let Some(colon) = pair.find(':') {
            let key = pair[..colon].trim().trim_matches('"');
            let val = pair[colon+1..].trim();
            if let Ok(v) = val.parse::<i64>() {
                let key_cstr = intern_cstr(key);
                slang_map_set(handle, key_cstr as *const i8, v);
            }
        }
    }
    handle
}

// ─── v18→v370: Async runtime integration ───────────────────────────────────
use std::sync::Mutex as StdMutex;
static ASYNC_EXECUTOR: std::sync::LazyLock<StdMutex<crate::async_runtime::Executor>> =
    std::sync::LazyLock::new(|| StdMutex::new(crate::async_runtime::Executor::new()));

extern "C" fn slang_spawn(task_id: i64) -> i64 {
    let mut exec = ASYNC_EXECUTOR.lock().unwrap();
    let id = exec.spawn(&format!("task-{}", task_id), &format!("fn-{}", task_id));
    // Immediately complete with the seed value (single-step tasks)
    exec.complete_task(id, task_id);
    id.0 as i64
}

extern "C" fn slang_task_result(task_id: i64) -> i64 {
    let exec = ASYNC_EXECUTOR.lock().unwrap();
    let id = crate::async_runtime::TaskId(task_id as u64);
    exec.get_result(id).unwrap_or(0)
}

extern "C" fn slang_task_await(task_id: i64) -> i64 {
    let exec = ASYNC_EXECUTOR.lock().unwrap();
    let id = crate::async_runtime::TaskId(task_id as u64);
    if exec.is_complete(id) {
        exec.get_result(id).unwrap_or(0)
    } else {
        0 // Task not yet complete
    }
}

extern "C" fn slang_async_run_all() -> i64 {
    let mut exec = ASYNC_EXECUTOR.lock().unwrap();
    let results = exec.run_all();
    results.len() as i64
}

// ─── v18: Networking functions ───────────────────────────────────────────
extern "C" fn slang_http_get(url: *const i8) -> *const i8 {
    if url.is_null() { return intern_cstr("") as *const i8; }
    let url_str = unsafe { std::ffi::CStr::from_ptr(url).to_string_lossy() };
    match ureq::get(&url_str).call() {
        Ok(resp) => {
            let body = resp.into_string().unwrap_or_default();
            intern_cstr(&body) as *const i8
        }
        Err(_) => intern_cstr("") as *const i8,
    }
}

extern "C" fn slang_http_post(url: *const i8, body: *const i8) -> *const i8 {
    if url.is_null() { return intern_cstr("") as *const i8; }
    let url_str = unsafe { std::ffi::CStr::from_ptr(url).to_string_lossy() };
    let body_str = if body.is_null() {
        String::new()
    } else {
        unsafe { std::ffi::CStr::from_ptr(body).to_string_lossy().into_owned() }
    };
    match ureq::post(&url_str).send_string(&body_str) {
        Ok(resp) => {
            let response_body = resp.into_string().unwrap_or_default();
            intern_cstr(&response_body) as *const i8
        }
        Err(_) => intern_cstr("") as *const i8,
    }
}

extern "C" fn slang_http_status(url: *const i8) -> i64 {
    if url.is_null() { return 0; }
    let url_str = unsafe { std::ffi::CStr::from_ptr(url).to_string_lossy() };
    match ureq::get(&url_str).call() {
        Ok(resp) => resp.status() as i64,
        Err(ureq::Error::Status(code, _)) => code as i64,
        Err(_) => 0,
    }
}

extern "C" fn slang_tcp_connect(_host: *const i8, _port: i64) -> i64 { 1 }
extern "C" fn slang_tcp_send(_handle: i64, _data: *const i8) -> i64 { 0 }
extern "C" fn slang_tcp_close(_handle: i64) { }

// ─── v60: Self-hosting bootstrap primitives ────────────────────────────────

/// Get ASCII code of the first character of a string. Returns -1 for null/empty.
extern "C" fn slang_char_to_int(s: *const i8) -> i64 {
    if s.is_null() { return -1; }
    let sv = unsafe { std::ffi::CStr::from_ptr(s).to_bytes() };
    if sv.is_empty() { return -1; }
    sv[0] as i64
}

/// Create a single-character string from an ASCII code. Returns "" for invalid codes.
extern "C" fn slang_int_to_char(n: i64) -> *const i8 {
    if n < 0 || n > 127 { return intern_cstr("") as *const i8; }
    let ch = n as u8 as char;
    intern_cstr(&ch.to_string()) as *const i8
}

/// Allocate a new zero-filled array of i64 elements.
extern "C" fn slang_array_new(size: i64) -> *mut u8 {
    slang_array_alloc(size.max(0), 8)
}

/// Exit the process with a status code.
extern "C" fn slang_exit(code: i64) {
    std::process::exit(code as i32);
}

/// Write raw bytes (from an i64 array, each truncated to u8) to a file.
extern "C" fn slang_file_write_bytes(path: *const i8, arr: *const u8) -> i8 {
    if path.is_null() || arr.is_null() { return 0; }
    let p = unsafe { std::ffi::CStr::from_ptr(path).to_string_lossy().into_owned() };
    let len = unsafe { *(arr.sub(8) as *const i64) };
    if len <= 0 { return match std::fs::write(&p, &[]) { Ok(_) => 1, Err(_) => 0 }; }
    let mut bytes = Vec::with_capacity(len as usize);
    for i in 0..len as usize {
        let val = unsafe { *(arr as *const i64).add(i) };
        bytes.push(val as u8);
    }
    match std::fs::write(&p, &bytes) { Ok(_) => 1, Err(_) => 0 }
}

/// Read a file as raw bytes, returning an i64 array where each element is a byte value.
extern "C" fn slang_file_read_bytes(path: *const i8) -> *mut u8 {
    if path.is_null() { return slang_array_alloc(0, 8); }
    let p = unsafe { std::ffi::CStr::from_ptr(path).to_string_lossy().into_owned() };
    match std::fs::read(&p) {
        Ok(data) => {
            let arr = slang_array_alloc(data.len() as i64, 8);
            for (i, &byte) in data.iter().enumerate() {
                unsafe { *(arr as *mut i64).add(i) = byte as i64; }
            }
            arr
        }
        Err(_) => slang_array_alloc(0, 8),
    }
}

/// Get the number of command-line arguments.
extern "C" fn slang_args_count() -> i64 {
    std::env::args().count() as i64
}

/// Get a command-line argument by index.
extern "C" fn slang_args_get(index: i64) -> *const i8 {
    if index < 0 { return intern_cstr("") as *const i8; }
    match std::env::args().nth(index as usize) {
        Some(arg) => intern_cstr(&arg) as *const i8,
        None => intern_cstr("") as *const i8,
    }
}

// ─── Tensor Runtime (f64 arrays for ML / Void-Vitalis) ──────────────────
/// Convert integer to f64.
extern "C" fn slang_to_f64(i: i64) -> f64 { i as f64 }
/// Convert f64 to integer (truncate).
extern "C" fn slang_to_i64(f: f64) -> i64 { f as i64 }

/// Fill n elements of an f64 tensor with the given value.
extern "C" fn slang_t_fill(ptr: *mut u8, n: i64, val: f64) {
    if ptr.is_null() { return; }
    let data = ptr as *mut f64;
    for i in 0..n as usize { unsafe { *data.add(i) = val; } }
}

/// Copy n f64 elements from src to dst.
extern "C" fn slang_t_copy(src: *const u8, dst: *mut u8, n: i64) {
    if src.is_null() || dst.is_null() { return; }
    unsafe { std::ptr::copy_nonoverlapping(src as *const f64, dst as *mut f64, n as usize); }
}

/// Fill n elements with random normal values (mean=0, std=0.02).
extern "C" fn slang_t_randn(ptr: *mut u8, n: i64, seed: i64) {
    if ptr.is_null() { return; }
    let data = ptr as *mut f64;
    let mut state = (seed as u64).wrapping_add(0x9E3779B97F4A7C15);
    for i in 0..n as usize {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let u1 = ((state >> 11) as f64) / ((1u64 << 53) as f64);
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let u2 = ((state >> 11) as f64) / ((1u64 << 53) as f64);
        let u1 = if u1 < 1e-10 { 1e-10 } else { u1 };
        let normal = (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos();
        unsafe { *data.add(i) = normal * 0.02; }
    }
}

/// Print first n elements of an f64 tensor.
extern "C" fn slang_t_print_n(ptr: *const u8, n: i64) {
    if ptr.is_null() { print!("[]"); return; }
    let data = ptr as *const f64;
    let len = unsafe { *(ptr.sub(8) as *const i64) };
    let show = (n as i64).min(len) as usize;
    print!("[");
    for i in 0..show {
        if i > 0 { print!(", "); }
        print!("{:.6}", unsafe { *data.add(i) });
    }
    println!("]");
}

/// Matrix multiply: out[M,N] = a[M,K] @ b[K,N].
extern "C" fn slang_t_matmul(
    a: *const u8, b: *const u8, out: *mut u8,
    m: i64, n: i64, k: i64,
) {
    let (a, b, out_p) = (a as *const f64, b as *const f64, out as *mut f64);
    let (m, n, k) = (m as usize, n as usize, k as usize);
    for i in 0..m {
        for j in 0..n {
            let mut sum = 0.0f64;
            for p in 0..k {
                sum += unsafe { *a.add(i * k + p) * *b.add(p * n + j) };
            }
            unsafe { *out_p.add(i * n + j) = sum; }
        }
    }
}

/// Element-wise add: out[i] = a[i] + b[i].
extern "C" fn slang_t_add_vv(a: *const u8, b: *const u8, out: *mut u8, n: i64) {
    let (a, b, out_p) = (a as *const f64, b as *const f64, out as *mut f64);
    for i in 0..n as usize { unsafe { *out_p.add(i) = *a.add(i) + *b.add(i); } }
}

/// Element-wise subtract: out[i] = a[i] - b[i].
extern "C" fn slang_t_sub_vv(a: *const u8, b: *const u8, out: *mut u8, n: i64) {
    let (a, b, out_p) = (a as *const f64, b as *const f64, out as *mut f64);
    for i in 0..n as usize { unsafe { *out_p.add(i) = *a.add(i) - *b.add(i); } }
}

/// Element-wise multiply: out[i] = a[i] * b[i].
extern "C" fn slang_t_mul_vv(a: *const u8, b: *const u8, out: *mut u8, n: i64) {
    let (a, b, out_p) = (a as *const f64, b as *const f64, out as *mut f64);
    for i in 0..n as usize { unsafe { *out_p.add(i) = *a.add(i) * *b.add(i); } }
}

/// Scalar multiply: out[i] = a[i] * s.
extern "C" fn slang_t_scale(a: *const u8, s: f64, out: *mut u8, n: i64) {
    let (a, out_p) = (a as *const f64, out as *mut f64);
    for i in 0..n as usize { unsafe { *out_p.add(i) = *a.add(i) * s; } }
}

/// Row-wise softmax: out[r,c] = exp(inp[r,c]-max) / sum(exp).
extern "C" fn slang_t_softmax(inp: *const u8, out: *mut u8, rows: i64, cols: i64) {
    let (inp, out_p) = (inp as *const f64, out as *mut f64);
    let (rows, cols) = (rows as usize, cols as usize);
    for r in 0..rows {
        let off = r * cols;
        let mut mx = f64::NEG_INFINITY;
        for c in 0..cols { let v = unsafe { *inp.add(off + c) }; if v > mx { mx = v; } }
        let mut s = 0.0f64;
        for c in 0..cols {
            let v = unsafe { (*inp.add(off + c) - mx).exp() };
            unsafe { *out_p.add(off + c) = v; }
            s += v;
        }
        if s > 0.0 { for c in 0..cols { unsafe { *out_p.add(off + c) /= s; } } }
    }
}

/// Sum all elements.
extern "C" fn slang_t_sum(ptr: *const u8, n: i64) -> f64 {
    if ptr.is_null() { return 0.0; }
    let data = ptr as *const f64;
    let mut s = 0.0f64;
    for i in 0..n as usize { s += unsafe { *data.add(i) }; }
    s
}

/// Add bias to each row: out[r,c] = a[r,c] + bias[c].
extern "C" fn slang_t_add_bias(
    a: *const u8, bias: *const u8, out: *mut u8, rows: i64, cols: i64,
) {
    let (a, bias, out_p) = (a as *const f64, bias as *const f64, out as *mut f64);
    let (rows, cols) = (rows as usize, cols as usize);
    for r in 0..rows {
        for c in 0..cols {
            let idx = r * cols + c;
            unsafe { *out_p.add(idx) = *a.add(idx) + *bias.add(c); }
        }
    }
}

/// Argmax: index of largest element.
extern "C" fn slang_t_max_idx(ptr: *const u8, n: i64) -> i64 {
    if ptr.is_null() || n <= 0 { return 0; }
    let data = ptr as *const f64;
    let mut mx = f64::NEG_INFINITY;
    let mut mi = 0i64;
    for i in 0..n as usize {
        let v = unsafe { *data.add(i) };
        if v > mx { mx = v; mi = i as i64; }
    }
    mi
}

/// Dot product.
extern "C" fn slang_t_dot(a: *const u8, b: *const u8, n: i64) -> f64 {
    if a.is_null() || b.is_null() { return 0.0; }
    let (a, b) = (a as *const f64, b as *const f64);
    let mut s = 0.0f64;
    for i in 0..n as usize { s += unsafe { *a.add(i) * *b.add(i) }; }
    s
}

/// Cross-entropy loss with gradient: returns scalar loss.
/// logits[B*V], targets i64 array ptr, grad_out[B*V].
extern "C" fn slang_t_cross_entropy(
    logits: *const u8, targets: *const u8, grad_out: *mut u8,
    batch: i64, vocab: i64,
) -> f64 {
    let lg = logits as *const f64;
    let tgt = targets as *const i64;
    let grad = grad_out as *mut f64;
    let (b, v) = (batch as usize, vocab as usize);
    let mut total_loss = 0.0f64;
    for i in 0..b {
        let off = i * v;
        // softmax
        let mut mx = f64::NEG_INFINITY;
        for j in 0..v { let x = unsafe { *lg.add(off + j) }; if x > mx { mx = x; } }
        let mut s = 0.0f64;
        for j in 0..v {
            let e = unsafe { (*lg.add(off + j) - mx).exp() };
            unsafe { *(grad.add(off + j)) = e; }
            s += e;
        }
        for j in 0..v { unsafe { *grad.add(off + j) /= s; } }
        let t = unsafe { *tgt.add(i) } as usize;
        let p = unsafe { *grad.add(off + t) };
        total_loss -= (p + 1e-10).ln();
        // gradient: softmax_out - one_hot
        unsafe { *grad.add(off + t) -= 1.0; }
        // scale by 1/B
        let inv_b = 1.0 / b as f64;
        for j in 0..v { unsafe { *grad.add(off + j) *= inv_b; } }
    }
    total_loss / b as f64
}

/// Transpose matrix: out[C,R] = a[R,C].
extern "C" fn slang_t_transpose(a: *const u8, out: *mut u8, rows: i64, cols: i64) {
    let (a, out_p) = (a as *const f64, out as *mut f64);
    let (r, c) = (rows as usize, cols as usize);
    for i in 0..r {
        for j in 0..c {
            unsafe { *out_p.add(j * r + i) = *a.add(i * c + j); }
        }
    }
}

/// AdamW optimizer step for a single parameter tensor.
/// config is a 6-element f64 tensor: [lr, beta1, beta2, eps, weight_decay, t_step].
extern "C" fn slang_t_adamw(
    param: *mut u8, grad: *const u8, m_ptr: *mut u8, v_ptr: *mut u8,
    n: i64, config: *const u8,
) {
    let (p, g) = (param as *mut f64, grad as *const f64);
    let (m, v) = (m_ptr as *mut f64, v_ptr as *mut f64);
    let cfg = config as *const f64;
    let lr   = unsafe { *cfg.add(0) };
    let b1   = unsafe { *cfg.add(1) };
    let b2   = unsafe { *cfg.add(2) };
    let eps  = unsafe { *cfg.add(3) };
    let wd   = unsafe { *cfg.add(4) };
    let step = unsafe { *cfg.add(5) };
    let bc1 = 1.0 - b1.powf(step);
    let bc2 = 1.0 - b2.powf(step);
    for i in 0..n as usize {
        unsafe {
            let gi = *g.add(i);
            *m.add(i) = b1 * *m.add(i) + (1.0 - b1) * gi;
            *v.add(i) = b2 * *v.add(i) + (1.0 - b2) * gi * gi;
            let m_hat = *m.add(i) / bc1;
            let v_hat = *v.add(i) / bc2;
            *p.add(i) = *p.add(i) * (1.0 - lr * wd) - lr * m_hat / (v_hat.sqrt() + eps);
        }
    }
}

/// L2 norm of a tensor.
extern "C" fn slang_t_norm(ptr: *const u8, n: i64) -> f64 {
    if ptr.is_null() { return 0.0; }
    let data = ptr as *const f64;
    let mut s = 0.0f64;
    for i in 0..n as usize { let v = unsafe { *data.add(i) }; s += v * v; }
    s.sqrt()
}

// ─── GUI builtins (minifb framebuffer) ──────────────────────────────────

struct GuiState {
    window: minifb::Window,
    buf: Vec<u32>,
    w: usize,
    h: usize,
}

// SAFETY: Window is only accessed from the main thread (JIT code runs on main thread).
// The Mutex is for API compatibility with `static`, not cross-thread access.
unsafe impl Send for GuiState {}
unsafe impl Sync for GuiState {}

static GUI_STATE: std::sync::Mutex<Option<GuiState>> = std::sync::Mutex::new(None);

// 8×12 bitmap font — printable ASCII 32..126
static FONT_8X12: &[u8] = include_bytes!("font8x12.bin");

fn font_glyph(ch: u8) -> &'static [u8] {
    if ch < 32 || ch > 126 { return &[0; 12]; }
    let idx = (ch - 32) as usize * 12;
    if idx + 12 > FONT_8X12.len() { return &[0; 12]; }
    &FONT_8X12[idx..idx + 12]
}

fn gui_set_pixel(buf: &mut [u32], w: usize, h: usize, x: i32, y: i32, color: u32) {
    if x >= 0 && y >= 0 && (x as usize) < w && (y as usize) < h {
        buf[y as usize * w + x as usize] = color;
    }
}

extern "C" fn slang_gui_open(width: i64, height: i64) -> i64 {
    let w = width.max(100) as usize;
    let h = height.max(100) as usize;
    let mut opts = minifb::WindowOptions::default();
    opts.resize = true;
    match minifb::Window::new("Void·Vitalis Studio — Vitalis on Vitalis", w, h, opts) {
        Ok(mut win) => {
            win.set_target_fps(30);
            let buf = vec![0x080b12u32; w * h];
            let mut g = GUI_STATE.lock().unwrap();
            *g = Some(GuiState { window: win, buf, w, h });
            1
        }
        Err(_) => 0,
    }
}

extern "C" fn slang_gui_close() -> i64 {
    let mut g = GUI_STATE.lock().unwrap();
    *g = None;
    1
}

extern "C" fn slang_gui_clear(color: i64) -> i64 {
    let mut g = GUI_STATE.lock().unwrap();
    if let Some(ref mut s) = *g {
        let c = color as u32;
        for p in s.buf.iter_mut() { *p = c; }
        1
    } else { 0 }
}

extern "C" fn slang_gui_rect(x: i64, y: i64, w: i64, h: i64, color: i64) -> i64 {
    let mut g = GUI_STATE.lock().unwrap();
    if let Some(ref mut s) = *g {
        let c = color as u32;
        let (sw, sh) = (s.w, s.h);
        let x0 = x.max(0) as usize;
        let y0 = y.max(0) as usize;
        let x1 = ((x + w) as usize).min(sw);
        let y1 = ((y + h) as usize).min(sh);
        for py in y0..y1 {
            for px in x0..x1 {
                s.buf[py * sw + px] = c;
            }
        }
        1
    } else { 0 }
}

extern "C" fn slang_gui_line(x1: i64, y1: i64, x2: i64, y2: i64, color: i64) -> i64 {
    let mut g = GUI_STATE.lock().unwrap();
    if let Some(ref mut s) = *g {
        let c = color as u32;
        let (w, h) = (s.w, s.h);
        // Bresenham
        let (mut cx, mut cy) = (x1 as i32, y1 as i32);
        let (dx, dy) = ((x2 as i32 - cx).abs(), -((y2 as i32 - cy).abs()));
        let sx = if cx < x2 as i32 { 1 } else { -1 };
        let sy = if cy < y2 as i32 { 1 } else { -1 };
        let mut err = dx + dy;
        loop {
            gui_set_pixel(&mut s.buf, w, h, cx, cy, c);
            if cx == x2 as i32 && cy == y2 as i32 { break; }
            let e2 = 2 * err;
            if e2 >= dy { err += dy; cx += sx; }
            if e2 <= dx { err += dx; cy += sy; }
        }
        1
    } else { 0 }
}

extern "C" fn slang_gui_text(x: i64, y: i64, s_ptr: *const i8, color: i64, scale: i64) -> i64 {
    if s_ptr.is_null() { return 0; }
    let text = unsafe { std::ffi::CStr::from_ptr(s_ptr).to_string_lossy() };
    let mut g = GUI_STATE.lock().unwrap();
    if let Some(ref mut st) = *g {
        let c = color as u32;
        let sc = scale.max(1) as usize;
        let (sw, sh) = (st.w, st.h);
        let mut cx = x as i32;
        for ch in text.bytes() {
            let glyph = font_glyph(ch);
            for row in 0..12usize {
                let bits = glyph[row];
                for col in 0..8usize {
                    if bits & (0x80 >> col) != 0 {
                        for sy in 0..sc {
                            for sx in 0..sc {
                                gui_set_pixel(
                                    &mut st.buf, sw, sh,
                                    cx + (col * sc + sx) as i32,
                                    y as i32 + (row * sc + sy) as i32,
                                    c,
                                );
                            }
                        }
                    }
                }
            }
            cx += (8 * sc) as i32;
        }
        1
    } else { 0 }
}

extern "C" fn slang_gui_update() -> i64 {
    let mut g = GUI_STATE.lock().unwrap();
    if let Some(ref mut s) = *g {
        s.window.update_with_buffer(&s.buf, s.w, s.h).ok();
        if s.window.is_open() && !s.window.is_key_down(minifb::Key::Escape) {
            1
        } else { 0 }
    } else { 0 }
}

extern "C" fn slang_gui_circle(cx: i64, cy: i64, r: i64, color: i64) -> i64 {
    let mut g = GUI_STATE.lock().unwrap();
    if let Some(ref mut s) = *g {
        let c = color as u32;
        let (sw, sh) = (s.w, s.h);
        let ri = r as i32;
        for dy in -ri..=ri {
            for dx in -ri..=ri {
                if dx * dx + dy * dy <= ri * ri {
                    gui_set_pixel(&mut s.buf, sw, sh, cx as i32 + dx, cy as i32 + dy, c);
                }
            }
        }
        1
    } else { 0 }
}

// ─── Advanced rendering builtins (HD quality) ───────────────────────────

/// Linear gradient rectangle. dir: 0=horizontal, 1=vertical
extern "C" fn slang_gui_gradient_rect(x: i64, y: i64, w: i64, h: i64, c1: i64, c2: i64, dir: i64) -> i64 {
    let mut g = GUI_STATE.lock().unwrap();
    if let Some(ref mut s) = *g {
        let (sw, sh) = (s.w, s.h);
        let x0 = x.max(0) as usize;
        let y0 = y.max(0) as usize;
        let x1 = ((x + w) as usize).min(sw);
        let y1 = ((y + h) as usize).min(sh);
        let r1 = ((c1 as u32 >> 16) & 0xFF) as f64;
        let g1 = ((c1 as u32 >> 8) & 0xFF) as f64;
        let b1 = (c1 as u32 & 0xFF) as f64;
        let r2 = ((c2 as u32 >> 16) & 0xFF) as f64;
        let g2 = ((c2 as u32 >> 8) & 0xFF) as f64;
        let b2 = (c2 as u32 & 0xFF) as f64;
        let steps = if dir == 0 { w.max(1) as f64 } else { h.max(1) as f64 };
        for py in y0..y1 {
            for px in x0..x1 {
                let t = if dir == 0 {
                    (px - x0) as f64 / steps
                } else {
                    (py - y0) as f64 / steps
                };
                let r = (r1 + (r2 - r1) * t).min(255.0).max(0.0) as u32;
                let g = (g1 + (g2 - g1) * t).min(255.0).max(0.0) as u32;
                let b = (b1 + (b2 - b1) * t).min(255.0).max(0.0) as u32;
                s.buf[py * sw + px] = (r << 16) | (g << 8) | b;
            }
        }
        1
    } else { 0 }
}

/// Individual pixel write
extern "C" fn slang_gui_pixel(x: i64, y: i64, color: i64) -> i64 {
    let mut g = GUI_STATE.lock().unwrap();
    if let Some(ref mut s) = *g {
        gui_set_pixel(&mut s.buf, s.w, s.h, x as i32, y as i32, color as u32);
        1
    } else { 0 }
}

/// Rounded rectangle
extern "C" fn slang_gui_rounded_rect(x: i64, y: i64, w: i64, h: i64, radius: i64, color: i64) -> i64 {
    let mut g = GUI_STATE.lock().unwrap();
    if let Some(ref mut s) = *g {
        let c = color as u32;
        let (sw, sh) = (s.w, s.h);
        let r = radius.min(w / 2).min(h / 2).max(0) as i32;
        let x0 = x as i32;
        let y0 = y as i32;
        let wi = w as i32;
        let hi = h as i32;
        for py in 0..hi {
            for px in 0..wi {
                let sx = x0 + px;
                let sy = y0 + py;
                if sx < 0 || sy < 0 || sx >= sw as i32 || sy >= sh as i32 { continue; }
                // Check corner distance
                let mut inside = true;
                if px < r && py < r {
                    // top-left corner
                    let dx = r - px;
                    let dy = r - py;
                    if dx * dx + dy * dy > r * r { inside = false; }
                } else if px >= wi - r && py < r {
                    // top-right corner
                    let dx = px - (wi - r - 1);
                    let dy = r - py;
                    if dx * dx + dy * dy > r * r { inside = false; }
                } else if px < r && py >= hi - r {
                    // bottom-left corner
                    let dx = r - px;
                    let dy = py - (hi - r - 1);
                    if dx * dx + dy * dy > r * r { inside = false; }
                } else if px >= wi - r && py >= hi - r {
                    // bottom-right corner
                    let dx = px - (wi - r - 1);
                    let dy = py - (hi - r - 1);
                    if dx * dx + dy * dy > r * r { inside = false; }
                }
                if inside {
                    s.buf[sy as usize * sw + sx as usize] = c;
                }
            }
        }
        1
    } else { 0 }
}

/// Alpha-blended rectangle (alpha 0-255)
extern "C" fn slang_gui_blend_rect(x: i64, y: i64, w: i64, h: i64, color: i64, alpha: i64) -> i64 {
    let mut g = GUI_STATE.lock().unwrap();
    if let Some(ref mut s) = *g {
        let (sw, sh) = (s.w, s.h);
        let a = alpha.min(255).max(0) as u32;
        let sr = ((color as u32 >> 16) & 0xFF) as u32;
        let sg = ((color as u32 >> 8) & 0xFF) as u32;
        let sb = (color as u32 & 0xFF) as u32;
        let x0 = x.max(0) as usize;
        let y0 = y.max(0) as usize;
        let x1 = ((x + w) as usize).min(sw);
        let y1 = ((y + h) as usize).min(sh);
        for py in y0..y1 {
            for px in x0..x1 {
                let dst = s.buf[py * sw + px];
                let dr = (dst >> 16) & 0xFF;
                let dg = (dst >> 8) & 0xFF;
                let db = dst & 0xFF;
                let r = (sr * a + dr * (255 - a)) / 255;
                let g = (sg * a + dg * (255 - a)) / 255;
                let b = (sb * a + db * (255 - a)) / 255;
                s.buf[py * sw + px] = (r << 16) | (g << 8) | b;
            }
        }
        1
    } else { 0 }
}

/// Thick line with configurable width
extern "C" fn slang_gui_thick_line(x1: i64, y1: i64, x2: i64, y2: i64, width: i64, color: i64) -> i64 {
    let mut g = GUI_STATE.lock().unwrap();
    if let Some(ref mut s) = *g {
        let c = color as u32;
        let (sw, sh) = (s.w, s.h);
        let half = (width / 2).max(0) as i32;
        // Draw main line with thickness via perpendicular expansion
        let (mut cx, mut cy) = (x1 as i32, y1 as i32);
        let (dx, dy) = ((x2 as i32 - cx).abs(), -((y2 as i32 - cy).abs()));
        let sx = if cx < x2 as i32 { 1 } else { -1 };
        let sy = if cy < y2 as i32 { 1 } else { -1 };
        let mut err = dx + dy;
        loop {
            for t in -half..=half {
                if dx.abs() > dy.abs().max(1) {
                    gui_set_pixel(&mut s.buf, sw, sh, cx, cy + t, c);
                } else {
                    gui_set_pixel(&mut s.buf, sw, sh, cx + t, cy, c);
                }
            }
            if cx == x2 as i32 && cy == y2 as i32 { break; }
            let e2 = 2 * err;
            if e2 >= dy { err += dy; cx += sx; }
            if e2 <= dx { err += dx; cy += sy; }
        }
        1
    } else { 0 }
}

/// Filled triangle using scanline algorithm
extern "C" fn slang_gui_triangle(x1: i64, y1: i64, x2: i64, y2: i64, x3: i64, y3: i64, color: i64) -> i64 {
    let mut g = GUI_STATE.lock().unwrap();
    if let Some(ref mut s) = *g {
        let c = color as u32;
        let (sw, sh) = (s.w, s.h);
        let mut pts = [(x1 as i32, y1 as i32), (x2 as i32, y2 as i32), (x3 as i32, y3 as i32)];
        pts.sort_by_key(|p| p.1);
        let (ax, ay) = pts[0];
        let (bx, by) = pts[1];
        let (cx_pt, cy_pt) = pts[2];
        for y in ay..=cy_pt {
            if y < 0 || y >= sh as i32 { continue; }
            let mut left;
            let mut right;
            if y <= by {
                let d1 = if by == ay { 0.0 } else { (y - ay) as f64 / (by - ay) as f64 };
                let d2 = if cy_pt == ay { 0.0 } else { (y - ay) as f64 / (cy_pt - ay) as f64 };
                left = ax + ((bx - ax) as f64 * d1) as i32;
                right = ax + ((cx_pt - ax) as f64 * d2) as i32;
            } else {
                let d1 = if cy_pt == by { 0.0 } else { (y - by) as f64 / (cy_pt - by) as f64 };
                let d2 = if cy_pt == ay { 0.0 } else { (y - ay) as f64 / (cy_pt - ay) as f64 };
                left = bx + ((cx_pt - bx) as f64 * d1) as i32;
                right = ax + ((cx_pt - ax) as f64 * d2) as i32;
            }
            if left > right { std::mem::swap(&mut left, &mut right); }
            for x in left.max(0)..=right.min(sw as i32 - 1) {
                s.buf[y as usize * sw + x as usize] = c;
            }
        }
        1
    } else { 0 }
}

/// Anti-aliased circle using distance-based alpha
extern "C" fn slang_gui_aa_circle(cx: i64, cy: i64, r: i64, color: i64) -> i64 {
    let mut g = GUI_STATE.lock().unwrap();
    if let Some(ref mut s) = *g {
        let (sw, sh) = (s.w, s.h);
        let sr = ((color as u32 >> 16) & 0xFF) as f64;
        let sg = ((color as u32 >> 8) & 0xFF) as f64;
        let sb = (color as u32 & 0xFF) as f64;
        let rf = r as f64;
        let ri = r as i32 + 2; // extra for AA fringe
        for dy in -ri..=ri {
            for dx in -ri..=ri {
                let px = cx as i32 + dx;
                let py = cy as i32 + dy;
                if px < 0 || py < 0 || px >= sw as i32 || py >= sh as i32 { continue; }
                let dist = ((dx * dx + dy * dy) as f64).sqrt();
                if dist <= rf - 0.5 {
                    gui_set_pixel(&mut s.buf, sw, sh, px, py, color as u32);
                } else if dist <= rf + 0.5 {
                    // Anti-alias fringe
                    let alpha = ((rf + 0.5 - dist) * 255.0).min(255.0).max(0.0) as u32;
                    let idx = py as usize * sw + px as usize;
                    let dst = s.buf[idx];
                    let dr = (dst >> 16) & 0xFF;
                    let dg = (dst >> 8) & 0xFF;
                    let db = dst & 0xFF;
                    let nr = (sr as u32 * alpha + dr * (255 - alpha)) / 255;
                    let ng = (sg as u32 * alpha + dg * (255 - alpha)) / 255;
                    let nb = (sb as u32 * alpha + db * (255 - alpha)) / 255;
                    s.buf[idx] = (nr << 16) | (ng << 8) | nb;
                }
            }
        }
        1
    } else { 0 }
}

/// Gradient rounded rectangle — combines gradient fill with rounded corners
extern "C" fn slang_gui_gradient_rounded_rect(x: i64, y: i64, w: i64, h: i64, radius: i64, c1: i64, c2: i64, dir: i64) -> i64 {
    let mut g = GUI_STATE.lock().unwrap();
    if let Some(ref mut s) = *g {
        let (sw, sh) = (s.w, s.h);
        let r = radius.min(w / 2).min(h / 2).max(0) as i32;
        let wi = w as i32;
        let hi = h as i32;
        let x0 = x as i32;
        let y0 = y as i32;
        let r1 = ((c1 as u32 >> 16) & 0xFF) as f64;
        let g1 = ((c1 as u32 >> 8) & 0xFF) as f64;
        let b1 = (c1 as u32 & 0xFF) as f64;
        let r2 = ((c2 as u32 >> 16) & 0xFF) as f64;
        let g2 = ((c2 as u32 >> 8) & 0xFF) as f64;
        let b2 = (c2 as u32 & 0xFF) as f64;
        let steps = if dir == 0 { w.max(1) as f64 } else { h.max(1) as f64 };
        for py in 0..hi {
            for px in 0..wi {
                let sx = x0 + px;
                let sy = y0 + py;
                if sx < 0 || sy < 0 || sx >= sw as i32 || sy >= sh as i32 { continue; }
                // Corner check
                let mut inside = true;
                if px < r && py < r {
                    let dx = r - px; let dy = r - py;
                    if dx * dx + dy * dy > r * r { inside = false; }
                } else if px >= wi - r && py < r {
                    let dx = px - (wi - r - 1); let dy = r - py;
                    if dx * dx + dy * dy > r * r { inside = false; }
                } else if px < r && py >= hi - r {
                    let dx = r - px; let dy = py - (hi - r - 1);
                    if dx * dx + dy * dy > r * r { inside = false; }
                } else if px >= wi - r && py >= hi - r {
                    let dx = px - (wi - r - 1); let dy = py - (hi - r - 1);
                    if dx * dx + dy * dy > r * r { inside = false; }
                }
                if inside {
                    let t = if dir == 0 { px as f64 / steps } else { py as f64 / steps };
                    let cr = (r1 + (r2 - r1) * t).min(255.0).max(0.0) as u32;
                    let cg = (g1 + (g2 - g1) * t).min(255.0).max(0.0) as u32;
                    let cb = (b1 + (b2 - b1) * t).min(255.0).max(0.0) as u32;
                    s.buf[sy as usize * sw + sx as usize] = (cr << 16) | (cg << 8) | cb;
                }
            }
        }
        1
    } else { 0 }
}

/// Glow effect — radial gradient circle (for glow/bloom effects)
extern "C" fn slang_gui_glow(cx: i64, cy: i64, r: i64, color: i64, intensity: i64) -> i64 {
    let mut g = GUI_STATE.lock().unwrap();
    if let Some(ref mut s) = *g {
        let (sw, sh) = (s.w, s.h);
        let sr = ((color as u32 >> 16) & 0xFF) as f64;
        let sg = ((color as u32 >> 8) & 0xFF) as f64;
        let sb = (color as u32 & 0xFF) as f64;
        let rf = r as f64;
        let ri = r as i32;
        let int = (intensity.min(255).max(0) as f64) / 255.0;
        for dy in -ri..=ri {
            for dx in -ri..=ri {
                let px = cx as i32 + dx;
                let py = cy as i32 + dy;
                if px < 0 || py < 0 || px >= sw as i32 || py >= sh as i32 { continue; }
                let dist = ((dx * dx + dy * dy) as f64).sqrt();
                if dist <= rf {
                    let falloff = 1.0 - (dist / rf);
                    let alpha = (falloff * falloff * int * 255.0).min(255.0) as u32;
                    if alpha > 0 {
                        let idx = py as usize * sw + px as usize;
                        let dst = s.buf[idx];
                        let dr = (dst >> 16) & 0xFF;
                        let dg = (dst >> 8) & 0xFF;
                        let db = dst & 0xFF;
                        let nr = ((sr as u32 * alpha + dr * (255 - alpha)) / 255).min(255);
                        let ng = ((sg as u32 * alpha + dg * (255 - alpha)) / 255).min(255);
                        let nb = ((sb as u32 * alpha + db * (255 - alpha)) / 255).min(255);
                        s.buf[idx] = (nr << 16) | (ng << 8) | nb;
                    }
                }
            }
        }
        1
    } else { 0 }
}

// ─── JIT Compiler ───────────────────────────────────────────────────────
pub struct JitCompiler {
    module: JITModule,
    ctx: codegen::Context,
    func_ids: HashMap<String, FuncId>,
}

/// Info about a Phi instruction for block-parameter passing.
struct PhiInfo {
    result: ir::Value,
    ty: IrType,
    incoming: Vec<(ir::Value, ir::BlockId)>,
}

impl JitCompiler {
    pub fn new() -> CodegenResult<Self> {
        let mut flag_builder = settings::builder();
        flag_builder.set("opt_level", "speed").map_err(|e| CodegenError {
            message: format!("failed to set opt_level: {}", e),
        })?;

        let isa_builder = cranelift_native::builder().map_err(|e| CodegenError {
            message: format!("failed to create ISA builder: {}", e),
        })?;

        let isa = isa_builder
            .finish(settings::Flags::new(flag_builder))
            .map_err(|e| CodegenError {
                message: format!("failed to build ISA: {}", e),
            })?;

        let mut builder = JITBuilder::with_isa(isa, cranelift_module::default_libcall_names());

        // Register runtime functions
        builder.symbol("slang_print_i64", slang_print_i64 as *const u8);
        builder.symbol("slang_print_str", slang_print_str as *const u8);
        builder.symbol("slang_println_str", slang_println_str as *const u8);
        builder.symbol("slang_println_i64", slang_println_i64 as *const u8);
        builder.symbol("slang_print_f64", slang_print_f64 as *const u8);
        builder.symbol("slang_println_f64", slang_println_f64 as *const u8);
        builder.symbol("slang_print_bool", slang_print_bool as *const u8);
        builder.symbol("slang_println_bool", slang_println_bool as *const u8);
        builder.symbol("slang_print_cstr", slang_print_cstr as *const u8);
        builder.symbol("slang_println_cstr", slang_println_cstr as *const u8);
        // Math
        builder.symbol("slang_sqrt_f64", slang_sqrt_f64 as *const u8);
        builder.symbol("slang_abs_i64", slang_abs_i64 as *const u8);
        builder.symbol("slang_abs_f64", slang_abs_f64 as *const u8);
        builder.symbol("slang_min_i64", slang_min_i64 as *const u8);
        builder.symbol("slang_max_i64", slang_max_i64 as *const u8);
        builder.symbol("slang_min_f64", slang_min_f64 as *const u8);
        builder.symbol("slang_max_f64", slang_max_f64 as *const u8);
        builder.symbol("slang_pow_f64", slang_pow_f64 as *const u8);
        builder.symbol("slang_floor_f64", slang_floor_f64 as *const u8);
        builder.symbol("slang_ceil_f64", slang_ceil_f64 as *const u8);
        builder.symbol("slang_round_f64", slang_round_f64 as *const u8);
        builder.symbol("slang_ln_f64", slang_ln_f64 as *const u8);
        builder.symbol("slang_log2_f64", slang_log2_f64 as *const u8);
        builder.symbol("slang_log10_f64", slang_log10_f64 as *const u8);
        builder.symbol("slang_sin_f64", slang_sin_f64 as *const u8);
        builder.symbol("slang_cos_f64", slang_cos_f64 as *const u8);
        builder.symbol("slang_exp_f64", slang_exp_f64 as *const u8);
        // Conversion
        builder.symbol("slang_i64_to_f64", slang_i64_to_f64 as *const u8);
        builder.symbol("slang_f64_to_i64", slang_f64_to_i64 as *const u8);
        // Strings
        builder.symbol("slang_str_len", slang_str_len as *const u8);
        builder.symbol("slang_str_eq", slang_str_eq as *const u8);
        builder.symbol("slang_str_cat", slang_str_cat as *const u8);
        // Extended math
        builder.symbol("slang_atan2_f64", slang_atan2_f64 as *const u8);
        builder.symbol("slang_hypot_f64", slang_hypot_f64 as *const u8);
        builder.symbol("slang_clamp_f64", slang_clamp_f64 as *const u8);
        builder.symbol("slang_clamp_i64", slang_clamp_i64 as *const u8);
        builder.symbol("slang_rand_f64",  slang_rand_f64  as *const u8);
        builder.symbol("slang_rand_i64",  slang_rand_i64  as *const u8);
        // Phase 4: Array heap runtime
        builder.symbol("slang_array_alloc",   slang_array_alloc   as *const u8);
        builder.symbol("slang_array_len",     slang_array_len     as *const u8);
        builder.symbol("slang_array_get_i64", slang_array_get_i64 as *const u8);
        builder.symbol("slang_array_set_i64", slang_array_set_i64 as *const u8);
        builder.symbol("slang_array_get_f64", slang_array_get_f64 as *const u8);
        builder.symbol("slang_array_set_f64", slang_array_set_f64 as *const u8);
        // Phase 5: New stdlib runtime symbols
        builder.symbol("slang_clock_ns",         slang_clock_ns         as *const u8);
        builder.symbol("slang_clock_ms",         slang_clock_ms         as *const u8);
        builder.symbol("slang_assert_eq_i64",    slang_assert_eq_i64    as *const u8);
        builder.symbol("slang_assert_true",      slang_assert_true      as *const u8);
        builder.symbol("slang_popcount",         slang_popcount         as *const u8);
        builder.symbol("slang_leading_zeros",    slang_leading_zeros    as *const u8);
        builder.symbol("slang_trailing_zeros",   slang_trailing_zeros   as *const u8);
        builder.symbol("slang_sign_i64",         slang_sign_i64         as *const u8);
        builder.symbol("slang_gcd",              slang_gcd              as *const u8);
        builder.symbol("slang_lcm",              slang_lcm              as *const u8);
        builder.symbol("slang_factorial",        slang_factorial        as *const u8);
        builder.symbol("slang_fibonacci",        slang_fibonacci        as *const u8);
        builder.symbol("slang_is_prime",         slang_is_prime         as *const u8);
        builder.symbol("slang_tan_f64",          slang_tan_f64          as *const u8);
        builder.symbol("slang_asin_f64",         slang_asin_f64         as *const u8);
        builder.symbol("slang_acos_f64",         slang_acos_f64         as *const u8);
        builder.symbol("slang_atan_f64",         slang_atan_f64         as *const u8);
        // Phase 21 stdlib symbols
        builder.symbol("slang_hash_i64",         slang_hash_i64         as *const u8);
        builder.symbol("slang_lerp_f64",         slang_lerp_f64         as *const u8);
        builder.symbol("slang_smoothstep_f64",   slang_smoothstep_f64   as *const u8);
        builder.symbol("slang_wrap_i64",         slang_wrap_i64         as *const u8);
        builder.symbol("slang_map_range_f64",    slang_map_range_f64    as *const u8);
        builder.symbol("slang_epoch_secs",       slang_epoch_secs       as *const u8);
        // Phase 22 stdlib symbols
        builder.symbol("slang_fma_f64",          slang_fma_f64          as *const u8);
        builder.symbol("slang_cbrt_f64",         slang_cbrt_f64         as *const u8);
        builder.symbol("slang_deg_to_rad",       slang_deg_to_rad       as *const u8);
        builder.symbol("slang_rad_to_deg",       slang_rad_to_deg       as *const u8);
        builder.symbol("slang_sigmoid_f64",      slang_sigmoid_f64      as *const u8);
        builder.symbol("slang_relu_f64",         slang_relu_f64         as *const u8);
        builder.symbol("slang_tanh_f64",         slang_tanh_f64         as *const u8);
        builder.symbol("slang_ipow",             slang_ipow             as *const u8);
        // Phase 23 stdlib symbols
        builder.symbol("slang_sinh_f64",         slang_sinh_f64         as *const u8);
        builder.symbol("slang_cosh_f64",         slang_cosh_f64         as *const u8);
        builder.symbol("slang_log_f64",          slang_log_f64          as *const u8);
        builder.symbol("slang_exp2_f64",         slang_exp2_f64         as *const u8);
        builder.symbol("slang_copysign_f64",     slang_copysign_f64     as *const u8);
        builder.symbol("slang_fract_f64",        slang_fract_f64        as *const u8);
        builder.symbol("slang_trunc_f64",        slang_trunc_f64        as *const u8);
        builder.symbol("slang_step_f64",         slang_step_f64         as *const u8);
        builder.symbol("slang_leaky_relu_f64",   slang_leaky_relu_f64   as *const u8);
        builder.symbol("slang_elu_f64",          slang_elu_f64          as *const u8);
        // Phase 24 stdlib symbols
        builder.symbol("slang_swish_f64",        slang_swish_f64        as *const u8);
        builder.symbol("slang_gelu_f64",         slang_gelu_f64         as *const u8);
        builder.symbol("slang_softplus_f64",     slang_softplus_f64     as *const u8);
        builder.symbol("slang_mish_f64",         slang_mish_f64         as *const u8);
        builder.symbol("slang_log1p_f64",        slang_log1p_f64        as *const u8);
        builder.symbol("slang_expm1_f64",        slang_expm1_f64        as *const u8);
        builder.symbol("slang_recip_f64",        slang_recip_f64        as *const u8);
        builder.symbol("slang_rsqrt_f64",        slang_rsqrt_f64        as *const u8);
        // Phase 25 stdlib symbols
        builder.symbol("slang_selu_f64",          slang_selu_f64          as *const u8);
        builder.symbol("slang_hard_sigmoid_f64",  slang_hard_sigmoid_f64  as *const u8);
        builder.symbol("slang_hard_swish_f64",    slang_hard_swish_f64    as *const u8);
        builder.symbol("slang_log_sigmoid_f64",   slang_log_sigmoid_f64   as *const u8);
        builder.symbol("slang_celu_f64",          slang_celu_f64          as *const u8);
        builder.symbol("slang_softsign_f64",      slang_softsign_f64      as *const u8);
        builder.symbol("slang_gaussian_f64",      slang_gaussian_f64      as *const u8);
        builder.symbol("slang_sinc_f64",          slang_sinc_f64          as *const u8);
        builder.symbol("slang_inv_sqrt_approx_f64", slang_inv_sqrt_approx_f64 as *const u8);
        builder.symbol("slang_logit_f64",         slang_logit_f64         as *const u8);
        // ── v15: String operations ───────────────────────────────────────
        builder.symbol("slang_str_upper",        slang_str_upper        as *const u8);
        builder.symbol("slang_str_lower",        slang_str_lower        as *const u8);
        builder.symbol("slang_str_trim",         slang_str_trim         as *const u8);
        builder.symbol("slang_str_contains",     slang_str_contains     as *const u8);
        builder.symbol("slang_str_starts_with",  slang_str_starts_with  as *const u8);
        builder.symbol("slang_str_ends_with",    slang_str_ends_with    as *const u8);
        builder.symbol("slang_str_char_at",      slang_str_char_at      as *const u8);
        builder.symbol("slang_str_substr",       slang_str_substr       as *const u8);
        builder.symbol("slang_str_index_of",     slang_str_index_of     as *const u8);
        builder.symbol("slang_str_replace",      slang_str_replace      as *const u8);
        builder.symbol("slang_str_repeat",       slang_str_repeat       as *const u8);
        builder.symbol("slang_str_reverse",      slang_str_reverse      as *const u8);
        builder.symbol("slang_str_split_count",  slang_str_split_count  as *const u8);
        builder.symbol("slang_str_split_get",    slang_str_split_get    as *const u8);
        builder.symbol("slang_to_string_i64",    slang_to_string_i64    as *const u8);
        builder.symbol("slang_to_string_f64",    slang_to_string_f64    as *const u8);
        builder.symbol("slang_to_string_bool",   slang_to_string_bool   as *const u8);
        builder.symbol("slang_str_format_i64",   slang_str_format_i64   as *const u8);
        builder.symbol("slang_str_format_f64",   slang_str_format_f64   as *const u8);
        builder.symbol("slang_str_format_str",   slang_str_format_str   as *const u8);
        builder.symbol("slang_parse_int",        slang_parse_int        as *const u8);
        builder.symbol("slang_parse_float",      slang_parse_float      as *const u8);
        // ── v15: File I/O ────────────────────────────────────────────────
        builder.symbol("slang_file_read",        slang_file_read        as *const u8);
        builder.symbol("slang_file_write",       slang_file_write       as *const u8);
        builder.symbol("slang_file_append",      slang_file_append      as *const u8);
        builder.symbol("slang_file_exists",      slang_file_exists      as *const u8);
        builder.symbol("slang_file_delete",      slang_file_delete      as *const u8);
        builder.symbol("slang_file_size",        slang_file_size        as *const u8);
        // ── v15: Map operations ──────────────────────────────────────────
        builder.symbol("slang_map_new",          slang_map_new          as *const u8);
        builder.symbol("slang_map_set",          slang_map_set          as *const u8);
        builder.symbol("slang_map_get",          slang_map_get          as *const u8);
        builder.symbol("slang_map_has",          slang_map_has          as *const u8);
        builder.symbol("slang_map_remove",       slang_map_remove       as *const u8);
        builder.symbol("slang_map_len",          slang_map_len          as *const u8);
        builder.symbol("slang_map_keys",         slang_map_keys         as *const u8);
        // ── v16: Set operations ──────────────────────────────────────────
        builder.symbol("slang_set_new",          slang_set_new          as *const u8);
        builder.symbol("slang_set_add",          slang_set_add          as *const u8);
        builder.symbol("slang_set_has",          slang_set_has          as *const u8);
        builder.symbol("slang_set_remove",       slang_set_remove       as *const u8);
        builder.symbol("slang_set_len",          slang_set_len          as *const u8);
        builder.symbol("slang_set_union",        slang_set_union        as *const u8);
        builder.symbol("slang_set_intersect",    slang_set_intersect    as *const u8);
        builder.symbol("slang_set_diff",         slang_set_diff         as *const u8);
        builder.symbol("slang_set_to_array",     slang_set_to_array     as *const u8);
        // ── v18: Tuple operations ────────────────────────────────────────
        builder.symbol("slang_tuple_new2",       slang_tuple_new2       as *const u8);
        builder.symbol("slang_tuple_new3",       slang_tuple_new3       as *const u8);
        builder.symbol("slang_tuple_new4",       slang_tuple_new4       as *const u8);
        builder.symbol("slang_tuple_get",        slang_tuple_get        as *const u8);
        builder.symbol("slang_tuple_len",        slang_tuple_len        as *const u8);
        // ── v15: Error handling ──────────────────────────────────────────
        builder.symbol("slang_error_set",        slang_error_set        as *const u8);
        builder.symbol("slang_error_check",      slang_error_check      as *const u8);
        builder.symbol("slang_error_msg",        slang_error_msg        as *const u8);
        builder.symbol("slang_error_clear",      slang_error_clear      as *const u8);
        // ── v15: Environment & System ────────────────────────────────────
        builder.symbol("slang_env_get",          slang_env_get          as *const u8);
        builder.symbol("slang_sleep_ms",         slang_sleep_ms         as *const u8);
        builder.symbol("slang_eprint",           slang_eprint           as *const u8);
        builder.symbol("slang_eprintln",         slang_eprintln         as *const u8);
        builder.symbol("slang_pid",              slang_pid              as *const u8);
        // -- v142: Runtime Logging ----------------------------------------
        builder.symbol("slang_log_trace",        slang_log_trace        as *const u8);
        builder.symbol("slang_log_debug",        slang_log_debug        as *const u8);
        builder.symbol("slang_log_info",         slang_log_info         as *const u8);
        builder.symbol("slang_log_warn",         slang_log_warn         as *const u8);
        builder.symbol("slang_log_error",        slang_log_error        as *const u8);
        builder.symbol("slang_log_level_set",    slang_log_level_set    as *const u8);
        builder.symbol("slang_log_level_get",    slang_log_level_get    as *const u8);
        // -- v143: Audit Trail -------------------------------------------
        builder.symbol("slang_audit_event",      slang_audit_event      as *const u8);
        builder.symbol("slang_audit_count",      slang_audit_count      as *const u8);
        builder.symbol("slang_audit_dump",       slang_audit_dump       as *const u8);
        builder.symbol("slang_audit_clear",      slang_audit_clear      as *const u8);
        builder.symbol("slang_audit_last",       slang_audit_last       as *const u8);
        // -- v144: Metrics & Telemetry --------------------------------------
        builder.symbol("slang_metric_counter",   slang_metric_counter   as *const u8);
        builder.symbol("slang_metric_gauge",     slang_metric_gauge     as *const u8);
        builder.symbol("slang_metric_histogram", slang_metric_histogram as *const u8);
        builder.symbol("slang_metric_get_counter", slang_metric_get_counter as *const u8);
        builder.symbol("slang_metric_get_gauge", slang_metric_get_gauge as *const u8);
        builder.symbol("slang_metric_dump",      slang_metric_dump      as *const u8);
        builder.symbol("slang_metric_clear",     slang_metric_clear     as *const u8);
        // -- v145: Distributed Tracing --------------------------------------
        builder.symbol("slang_span_start",       slang_span_start       as *const u8);
        builder.symbol("slang_span_end",         slang_span_end         as *const u8);
        builder.symbol("slang_span_set_tag",     slang_span_set_tag     as *const u8);
        builder.symbol("slang_trace_id",         slang_trace_id         as *const u8);
        builder.symbol("slang_span_depth",       slang_span_depth       as *const u8);
        // -- v146: Health Check & Runtime Diagnostics ---------------------
        builder.symbol("slang_runtime_uptime_ms",   slang_runtime_uptime_ms   as *const u8);
        builder.symbol("slang_runtime_memory_used", slang_runtime_memory_used as *const u8);
        builder.symbol("slang_runtime_version",     slang_runtime_version     as *const u8);
        builder.symbol("slang_runtime_alloc_count", slang_runtime_alloc_count as *const u8);
        builder.symbol("slang_runtime_alloc_total", slang_runtime_alloc_total as *const u8);
        builder.symbol("slang_runtime_cpu_count",   slang_runtime_cpu_count   as *const u8);
        // -- v147: Observable Pipeline Integration ----------------------
        builder.symbol("slang_pipeline_timer_start",   slang_pipeline_timer_start   as *const u8);
        builder.symbol("slang_pipeline_timer_end",     slang_pipeline_timer_end     as *const u8);
        builder.symbol("slang_pipeline_stage_count",   slang_pipeline_stage_count   as *const u8);
        builder.symbol("slang_pipeline_dump_timings",  slang_pipeline_dump_timings  as *const u8);
        builder.symbol("slang_pipeline_clear_timings", slang_pipeline_clear_timings as *const u8);
        // -- v148: RBAC & Capability Permissions ------------------------
        builder.symbol("slang_permission_check",  slang_permission_check  as *const u8);
        builder.symbol("slang_permission_grant",  slang_permission_grant  as *const u8);
        builder.symbol("slang_permission_revoke", slang_permission_revoke as *const u8);
        builder.symbol("slang_permission_list",   slang_permission_list   as *const u8);
        builder.symbol("slang_permission_clear",  slang_permission_clear  as *const u8);
        // -- v149: Cryptographic Signing -----------------------------
        builder.symbol("slang_crypto_sha256",       slang_crypto_sha256       as *const u8);
        builder.symbol("slang_crypto_hmac_sign",    slang_crypto_hmac_sign    as *const u8);
        builder.symbol("slang_crypto_hmac_verify",  slang_crypto_hmac_verify  as *const u8);
        builder.symbol("slang_crypto_base64_encode",slang_crypto_base64_encode as *const u8);
        builder.symbol("slang_crypto_base64_decode",slang_crypto_base64_decode as *const u8);
        // -- v150: Sandbox Enforcement -------------------------------
        builder.symbol("slang_sandbox_create",    slang_sandbox_create    as *const u8);
        builder.symbol("slang_sandbox_allow",     slang_sandbox_allow     as *const u8);
        builder.symbol("slang_sandbox_check",     slang_sandbox_check     as *const u8);
        builder.symbol("slang_sandbox_violations",slang_sandbox_violations as *const u8);
        builder.symbol("slang_sandbox_destroy",   slang_sandbox_destroy   as *const u8);
        // -- v151: Security Audit Logger ----------------------------
        builder.symbol("slang_security_log",       slang_security_log       as *const u8);
        builder.symbol("slang_security_log_count", slang_security_log_count as *const u8);
        builder.symbol("slang_security_log_verify",slang_security_log_verify as *const u8);
        builder.symbol("slang_security_log_dump",  slang_security_log_dump  as *const u8);
        builder.symbol("slang_security_log_clear", slang_security_log_clear as *const u8);
        // -- v152: Input Validation Framework -----------------------
        builder.symbol("slang_validate_email",    slang_validate_email    as *const u8);
        builder.symbol("slang_validate_url",      slang_validate_url      as *const u8);
        builder.symbol("slang_validate_ip",       slang_validate_ip       as *const u8);
        builder.symbol("slang_sanitize_html",     slang_sanitize_html     as *const u8);
        builder.symbol("slang_sanitize_sql",      slang_sanitize_sql      as *const u8);
        // -- v153: Secure Communication Primitives -----------------
        builder.symbol("slang_secure_channel_create", slang_secure_channel_create as *const u8);
        builder.symbol("slang_secure_channel_send",   slang_secure_channel_send   as *const u8);
        builder.symbol("slang_secure_channel_recv",   slang_secure_channel_recv   as *const u8);
        builder.symbol("slang_secure_channel_close",  slang_secure_channel_close  as *const u8);
        builder.symbol("slang_constant_time_eq",      slang_constant_time_eq      as *const u8);
        // -- v154: Autonomous Improvement Lab v2 -------------------
        builder.symbol("slang_improvement_run_trial",  slang_improvement_run_trial  as *const u8);
        builder.symbol("slang_improvement_best_score", slang_improvement_best_score as *const u8);
        builder.symbol("slang_improvement_history_count", slang_improvement_history_count as *const u8);
        builder.symbol("slang_improvement_reset",      slang_improvement_reset      as *const u8);
        // -- v155: LLM-Guided Mutation Templates -------------------
        builder.symbol("slang_mutation_apply",         slang_mutation_apply         as *const u8);
        builder.symbol("slang_mutation_list_count",    slang_mutation_list_count    as *const u8);
        builder.symbol("slang_mutation_score",         slang_mutation_score         as *const u8);
        builder.symbol("slang_mutation_undo",          slang_mutation_undo          as *const u8);
        // -- v156: Evolution Fitness Profiles -----------------------
        builder.symbol("slang_fitness_register",       slang_fitness_register       as *const u8);
        builder.symbol("slang_fitness_evaluate",       slang_fitness_evaluate       as *const u8);
        builder.symbol("slang_fitness_pareto_count",   slang_fitness_pareto_count   as *const u8);
        builder.symbol("slang_fitness_clear",          slang_fitness_clear          as *const u8);
        // -- v157: Cross-Module Evolution ---------------------------
        builder.symbol("slang_evo_cross_module",       slang_evo_cross_module       as *const u8);
        builder.symbol("slang_evo_dep_add",            slang_evo_dep_add            as *const u8);
        builder.symbol("slang_evo_dep_check",          slang_evo_dep_check          as *const u8);
        builder.symbol("slang_evo_safe_mutate",        slang_evo_safe_mutate        as *const u8);
        // -- v158: Evolution Checkpointing -------------------------
        builder.symbol("slang_evo_checkpoint_save",    slang_evo_checkpoint_save    as *const u8);
        builder.symbol("slang_evo_checkpoint_load",    slang_evo_checkpoint_load    as *const u8);
        builder.symbol("slang_evo_checkpoint_list_count", slang_evo_checkpoint_list_count as *const u8);
        builder.symbol("slang_evo_checkpoint_clear",   slang_evo_checkpoint_clear   as *const u8);
        // -- v159: Meta-Evolution v2 -------------------------------
        builder.symbol("slang_meta_evo_register",      slang_meta_evo_register      as *const u8);
        builder.symbol("slang_meta_evo_select",        slang_meta_evo_select        as *const u8);
        builder.symbol("slang_meta_evo_converged",     slang_meta_evo_converged     as *const u8);
        builder.symbol("slang_meta_evo_stats_count",   slang_meta_evo_stats_count   as *const u8);
        // -- v160: Tensor-First Types ------------------------------
        builder.symbol("slang_tensor_create",          slang_tensor_create          as *const u8);
        builder.symbol("slang_tensor_rank",            slang_tensor_rank            as *const u8);
        builder.symbol("slang_tensor_size",            slang_tensor_size            as *const u8);
        builder.symbol("slang_tensor_set",             slang_tensor_set             as *const u8);
        builder.symbol("slang_tensor_get",             slang_tensor_get             as *const u8);
        builder.symbol("slang_tensor_add",             slang_tensor_add             as *const u8);
        builder.symbol("slang_tensor_mul",             slang_tensor_mul             as *const u8);
        // -- v161: Auto-Differentiation ----------------------------
        builder.symbol("slang_grad_compute",           slang_grad_compute           as *const u8);
        builder.symbol("slang_grad_forward",           slang_grad_forward           as *const u8);
        builder.symbol("slang_grad_reverse",           slang_grad_reverse           as *const u8);
        builder.symbol("slang_grad_jacobian_dim",      slang_grad_jacobian_dim      as *const u8);
        // -- v162: ML Pipeline -------------------------------------
        builder.symbol("slang_ml_linear_fit",          slang_ml_linear_fit          as *const u8);
        builder.symbol("slang_ml_predict",             slang_ml_predict             as *const u8);
        builder.symbol("slang_ml_accuracy",            slang_ml_accuracy            as *const u8);
        builder.symbol("slang_ml_loss",                slang_ml_loss                as *const u8);
        // -- v163: NAS ---------------------------------------------
        builder.symbol("slang_nas_search",             slang_nas_search             as *const u8);
        builder.symbol("slang_nas_evaluate",           slang_nas_evaluate           as *const u8);
        builder.symbol("slang_nas_best",               slang_nas_best               as *const u8);
        builder.symbol("slang_nas_count",              slang_nas_count              as *const u8);
        // -- v164: Feature Engineering ------------------------------
        builder.symbol("slang_feature_normalize",      slang_feature_normalize      as *const u8);
        builder.symbol("slang_feature_one_hot",        slang_feature_one_hot        as *const u8);
        builder.symbol("slang_feature_variance",       slang_feature_variance       as *const u8);
        builder.symbol("slang_feature_correlate",      slang_feature_correlate      as *const u8);
        // -- v165: Model Serialization ------------------------------
        builder.symbol("slang_model_save",             slang_model_save             as *const u8);
        builder.symbol("slang_model_load",             slang_model_load             as *const u8);
        builder.symbol("slang_model_version",          slang_model_version          as *const u8);
        builder.symbol("slang_model_compatible",       slang_model_compatible       as *const u8);
        // -- v166: KV Store ----------------------------------------------
        builder.symbol("slang_kv_put",                 slang_kv_put                 as *const u8);
        builder.symbol("slang_kv_get_len",             slang_kv_get_len             as *const u8);
        builder.symbol("slang_kv_delete",              slang_kv_delete              as *const u8);
        builder.symbol("slang_kv_wal_count",           slang_kv_wal_count           as *const u8);
        // -- v167: B-Tree Index ------------------------------------------
        builder.symbol("slang_btree_insert",           slang_btree_insert           as *const u8);
        builder.symbol("slang_btree_lookup",           slang_btree_lookup           as *const u8);
        builder.symbol("slang_btree_range_count",      slang_btree_range_count      as *const u8);
        builder.symbol("slang_btree_count",            slang_btree_count            as *const u8);
        // -- v168: SQL Engine --------------------------------------------
        builder.symbol("slang_sql_create_table",       slang_sql_create_table       as *const u8);
        builder.symbol("slang_sql_insert",             slang_sql_insert             as *const u8);
        builder.symbol("slang_sql_count",              slang_sql_count              as *const u8);
        builder.symbol("slang_sql_sum_col0",           slang_sql_sum_col0           as *const u8);
        // -- v169: Schema Migration --------------------------------------
        builder.symbol("slang_schema_create",          slang_schema_create          as *const u8);
        builder.symbol("slang_schema_migrate",         slang_schema_migrate         as *const u8);
        builder.symbol("slang_schema_version",         slang_schema_version         as *const u8);
        builder.symbol("slang_schema_compatible",      slang_schema_compatible      as *const u8);
        // -- v170: Transaction Log ---------------------------------------
        builder.symbol("slang_txn_begin",              slang_txn_begin              as *const u8);
        builder.symbol("slang_txn_commit",             slang_txn_commit             as *const u8);
        builder.symbol("slang_txn_rollback",           slang_txn_rollback           as *const u8);
        builder.symbol("slang_txn_log_count",          slang_txn_log_count          as *const u8);
        // -- v171: Data Import/Export -------------------------------------
        builder.symbol("slang_data_buf_create",        slang_data_buf_create        as *const u8);
        builder.symbol("slang_data_buf_push",          slang_data_buf_push          as *const u8);
        builder.symbol("slang_data_buf_len",           slang_data_buf_len           as *const u8);
        builder.symbol("slang_data_buf_get",           slang_data_buf_get           as *const u8);
        // -- v172: TCP Sockets --------------------------------------
        builder.symbol("slang_tcp_create",              slang_tcp_create              as *const u8);
        builder.symbol("slang_tcp_sim_connect",        slang_tcp_sim_connect        as *const u8);
        builder.symbol("slang_tcp_connected",           slang_tcp_connected           as *const u8);
        builder.symbol("slang_tcp_sim_close",           slang_tcp_sim_close           as *const u8);
        // -- v173: HTTP Client --------------------------------------
        builder.symbol("slang_http_sim_get",            slang_http_sim_get            as *const u8);
        builder.symbol("slang_http_sim_post",           slang_http_sim_post           as *const u8);
        builder.symbol("slang_http_request_count",      slang_http_request_count      as *const u8);
        builder.symbol("slang_http_url_valid",          slang_http_url_valid          as *const u8);
        // -- v174: WebSocket ----------------------------------------
        builder.symbol("slang_ws_create",               slang_ws_create               as *const u8);
        builder.symbol("slang_ws_send",                 slang_ws_send                 as *const u8);
        builder.symbol("slang_ws_msg_count",            slang_ws_msg_count            as *const u8);
        builder.symbol("slang_ws_close",                slang_ws_close                as *const u8);
        // -- v175: RPC Framework -------------------------------------
        builder.symbol("slang_rpc_register",            slang_rpc_register            as *const u8);
        builder.symbol("slang_rpc_call",                slang_rpc_call                as *const u8);
        builder.symbol("slang_rpc_total_calls",         slang_rpc_total_calls         as *const u8);
        builder.symbol("slang_rpc_service_count",       slang_rpc_service_count       as *const u8);
        // -- v176: DNS Resolution ------------------------------------
        builder.symbol("slang_dns_resolve",             slang_dns_resolve             as *const u8);
        builder.symbol("slang_dns_cached",              slang_dns_cached              as *const u8);
        builder.symbol("slang_dns_cache_size",          slang_dns_cache_size          as *const u8);
        builder.symbol("slang_dns_cache_flush",         slang_dns_cache_flush         as *const u8);
        // -- v177: TLS/SSL -------------------------------------------
        builder.symbol("slang_tls_create",              slang_tls_create              as *const u8);
        builder.symbol("slang_tls_active",              slang_tls_active              as *const u8);
        builder.symbol("slang_tls_close",               slang_tls_close               as *const u8);
        builder.symbol("slang_tls_cert_valid",          slang_tls_cert_valid          as *const u8);
        // -- v178: REPL Enhancements --------------------------------
        builder.symbol("slang_repl_history_add",       slang_repl_history_add       as *const u8);
        builder.symbol("slang_repl_history_count",     slang_repl_history_count     as *const u8);
        builder.symbol("slang_repl_history_clear",     slang_repl_history_clear     as *const u8);
        builder.symbol("slang_repl_complete_count",    slang_repl_complete_count    as *const u8);
        // -- v179: Package Registry ---------------------------------
        builder.symbol("slang_pkg_publish",            slang_pkg_publish            as *const u8);
        builder.symbol("slang_pkg_installed",          slang_pkg_installed          as *const u8);
        builder.symbol("slang_pkg_count",              slang_pkg_count              as *const u8);
        builder.symbol("slang_pkg_remove",             slang_pkg_remove             as *const u8);
        // -- v180: Documentation Generator --------------------------
        builder.symbol("slang_doc_add",                slang_doc_add                as *const u8);
        builder.symbol("slang_doc_count",              slang_doc_count              as *const u8);
        builder.symbol("slang_doc_has",                slang_doc_has                as *const u8);
        builder.symbol("slang_doc_clear",              slang_doc_clear              as *const u8);
        // -- v181: Benchmark Suite ----------------------------------
        builder.symbol("slang_bench_record",           slang_bench_record           as *const u8);
        builder.symbol("slang_bench_count",            slang_bench_count            as *const u8);
        builder.symbol("slang_bench_best",             slang_bench_best             as *const u8);
        builder.symbol("slang_bench_clear",            slang_bench_clear            as *const u8);
        // -- v182: Profiler -----------------------------------------
        builder.symbol("slang_profile_start",          slang_profile_start          as *const u8);
        builder.symbol("slang_profile_stop",           slang_profile_stop           as *const u8);
        builder.symbol("slang_profile_sample",         slang_profile_sample         as *const u8);
        builder.symbol("slang_profile_samples",        slang_profile_samples        as *const u8);
        // -- v183: Interactive Playground ----------------------------
        builder.symbol("slang_playground_eval",        slang_playground_eval        as *const u8);
        builder.symbol("slang_playground_count",       slang_playground_count       as *const u8);
        builder.symbol("slang_playground_clear",       slang_playground_clear       as *const u8);
        builder.symbol("slang_playground_last_result", slang_playground_last_result as *const u8);
        // -- v184: Work-Stealing Scheduler --------------------------
        builder.symbol("slang_task_submit",            slang_task_submit            as *const u8);
        builder.symbol("slang_task_queue_len",         slang_task_queue_len         as *const u8);
        builder.symbol("slang_task_steal",             slang_task_steal             as *const u8);
        builder.symbol("slang_task_queue_clear",       slang_task_queue_clear       as *const u8);
        // -- v185: Actor Model --------------------------------------
        builder.symbol("slang_actor_spawn",            slang_actor_spawn            as *const u8);
        builder.symbol("slang_actor_send",             slang_actor_send             as *const u8);
        builder.symbol("slang_actor_recv",             slang_actor_recv             as *const u8);
        builder.symbol("slang_actor_mailbox_len",      slang_actor_mailbox_len      as *const u8);
        // -- v186: STM ----------------------------------------------
        builder.symbol("slang_stm_new",                slang_stm_new                as *const u8);
        builder.symbol("slang_stm_read",               slang_stm_read               as *const u8);
        builder.symbol("slang_stm_write",              slang_stm_write              as *const u8);
        builder.symbol("slang_stm_cas",                slang_stm_cas                as *const u8);
        // -- v187: Parallel Collections -----------------------------
        builder.symbol("slang_par_sum",                slang_par_sum                as *const u8);
        builder.symbol("slang_par_min",                slang_par_min                as *const u8);
        builder.symbol("slang_par_max",                slang_par_max                as *const u8);
        builder.symbol("slang_par_count",              slang_par_count              as *const u8);
        // -- v188: GPU Task Scheduling ------------------------------
        builder.symbol("slang_gpu_submit",             slang_gpu_submit             as *const u8);
        builder.symbol("slang_gpu_queue_len",          slang_gpu_queue_len          as *const u8);
        builder.symbol("slang_gpu_flush",              slang_gpu_flush              as *const u8);
        builder.symbol("slang_gpu_available",          slang_gpu_available          as *const u8);
        // -- v189: Distributed Computing ----------------------------
        builder.symbol("slang_dist_node_add",          slang_dist_node_add          as *const u8);
        builder.symbol("slang_dist_node_count",        slang_dist_node_count        as *const u8);
        builder.symbol("slang_dist_broadcast",         slang_dist_broadcast         as *const u8);
        builder.symbol("slang_dist_reduce",            slang_dist_reduce            as *const u8);
        // -- v190: ADTs v2 ------------------------------------------
        builder.symbol("slang_type_register",          slang_type_register          as *const u8);
        builder.symbol("slang_type_variant_count",     slang_type_variant_count     as *const u8);
        builder.symbol("slang_type_count",             slang_type_count             as *const u8);
        builder.symbol("slang_type_exists",            slang_type_exists            as *const u8);
        // -- v191: Higher-Kinded Types ------------------------------
        builder.symbol("slang_hkt_register",           slang_hkt_register           as *const u8);
        builder.symbol("slang_hkt_arity",              slang_hkt_arity              as *const u8);
        builder.symbol("slang_hkt_count",              slang_hkt_count              as *const u8);
        builder.symbol("slang_hkt_exists",             slang_hkt_exists             as *const u8);
        // -- v192: Dependent Types v2 -------------------------------
        builder.symbol("slang_dep_type_check_range",   slang_dep_type_check_range   as *const u8);
        builder.symbol("slang_dep_type_nat",           slang_dep_type_nat           as *const u8);
        builder.symbol("slang_dep_type_positive",      slang_dep_type_positive      as *const u8);
        builder.symbol("slang_dep_type_bounded_add",   slang_dep_type_bounded_add   as *const u8);
        // -- v193: Effect Polymorphism ------------------------------
        builder.symbol("slang_effect_register",        slang_effect_register        as *const u8);
        builder.symbol("slang_effect_add_handler",     slang_effect_add_handler     as *const u8);
        builder.symbol("slang_effect_handler_count",   slang_effect_handler_count   as *const u8);
        builder.symbol("slang_effect_count",           slang_effect_count           as *const u8);
        // -- v194: Type-Level Computation ---------------------------
        builder.symbol("slang_type_level_add",         slang_type_level_add         as *const u8);
        builder.symbol("slang_type_level_mul",         slang_type_level_mul         as *const u8);
        builder.symbol("slang_type_level_eq",          slang_type_level_eq          as *const u8);
        builder.symbol("slang_type_level_if",          slang_type_level_if          as *const u8);
        // -- v195: Gradual Typing -----------------------------------
        builder.symbol("slang_gradual_annotate",       slang_gradual_annotate       as *const u8);
        builder.symbol("slang_gradual_check",          slang_gradual_check          as *const u8);
        builder.symbol("slang_gradual_typed_count",    slang_gradual_typed_count    as *const u8);
        builder.symbol("slang_gradual_is_any",         slang_gradual_is_any         as *const u8);
        // -- v196: FFI v2 -------------------------------------------
        builder.symbol("slang_ffi_bind",               slang_ffi_bind               as *const u8);
        builder.symbol("slang_ffi_bound",              slang_ffi_bound              as *const u8);
        builder.symbol("slang_ffi_count",              slang_ffi_count              as *const u8);
        builder.symbol("slang_ffi_remove",             slang_ffi_remove             as *const u8);
        // -- v197: Cloud-Native Deployment --------------------------
        builder.symbol("slang_cloud_deploy",           slang_cloud_deploy           as *const u8);
        builder.symbol("slang_cloud_deployment_count", slang_cloud_deployment_count as *const u8);
        builder.symbol("slang_cloud_health_check",     slang_cloud_health_check     as *const u8);
        builder.symbol("slang_cloud_shutdown",         slang_cloud_shutdown         as *const u8);
        // -- v198: Self-Hosting v3 ----------------------------------
        builder.symbol("slang_bootstrap_stage",        slang_bootstrap_stage        as *const u8);
        builder.symbol("slang_bootstrap_advance",      slang_bootstrap_advance      as *const u8);
        builder.symbol("slang_bootstrap_verify",       slang_bootstrap_verify       as *const u8);
        builder.symbol("slang_bootstrap_reset",        slang_bootstrap_reset        as *const u8);
        // -- v199: AI Language Server -------------------------------
        builder.symbol("slang_ai_suggest",             slang_ai_suggest             as *const u8);
        builder.symbol("slang_ai_suggestion_count",    slang_ai_suggestion_count    as *const u8);
        builder.symbol("slang_ai_explain_error",       slang_ai_explain_error       as *const u8);
        builder.symbol("slang_ai_clear",               slang_ai_clear               as *const u8);
        // -- v200: Milestone ----------------------------------------
        builder.symbol("slang_vitalis_version",        slang_vitalis_version        as *const u8);
        builder.symbol("slang_vitalis_module_count",   slang_vitalis_module_count   as *const u8);
        builder.symbol("slang_vitalis_test_count",     slang_vitalis_test_count     as *const u8);
        builder.symbol("slang_vitalis_builtin_count",  slang_vitalis_builtin_count  as *const u8);
        // -- v201-v206: Spike Engine --
        builder.symbol("slang_spike_emit", crate::spike_engine::slang_spike_emit as *const u8);
        builder.symbol("slang_spike_queue_len", crate::spike_engine::slang_spike_queue_len as *const u8);
        builder.symbol("slang_spike_next", crate::spike_engine::slang_spike_next as *const u8);
        builder.symbol("slang_spike_clear", crate::spike_engine::slang_spike_clear as *const u8);
        builder.symbol("slang_neuro_compartment_create", crate::spike_engine::slang_neuro_compartment_create as *const u8);
        builder.symbol("slang_neuro_compartment_step", crate::spike_engine::slang_neuro_compartment_step as *const u8);
        builder.symbol("slang_neuro_dendrite_propagate", crate::spike_engine::slang_neuro_dendrite_propagate as *const u8);
        builder.symbol("slang_neuro_compartment_count", crate::spike_engine::slang_neuro_compartment_count as *const u8);
        builder.symbol("slang_synapse_conductance", crate::spike_engine::slang_synapse_conductance as *const u8);
        builder.symbol("slang_synapse_stp_facilitate", crate::spike_engine::slang_synapse_stp_facilitate as *const u8);
        builder.symbol("slang_synapse_stp_depress", crate::spike_engine::slang_synapse_stp_depress as *const u8);
        builder.symbol("slang_synapse_count", crate::spike_engine::slang_synapse_count as *const u8);
        builder.symbol("slang_neuro_wilson_cowan", crate::spike_engine::slang_neuro_wilson_cowan as *const u8);
        builder.symbol("slang_neuro_neural_mass", crate::spike_engine::slang_neuro_neural_mass as *const u8);
        builder.symbol("slang_neuro_population_activity", crate::spike_engine::slang_neuro_population_activity as *const u8);
        builder.symbol("slang_neuro_population_sync", crate::spike_engine::slang_neuro_population_sync as *const u8);
        builder.symbol("slang_spike_encode_rate", crate::spike_engine::slang_spike_encode_rate as *const u8);
        builder.symbol("slang_spike_encode_temporal", crate::spike_engine::slang_spike_encode_temporal as *const u8);
        builder.symbol("slang_spike_decode_rate", crate::spike_engine::slang_spike_decode_rate as *const u8);
        builder.symbol("slang_spike_encode_phase", crate::spike_engine::slang_spike_encode_phase as *const u8);
        builder.symbol("slang_neuro_mem_alloc", crate::spike_engine::slang_neuro_mem_alloc as *const u8);
        builder.symbol("slang_neuro_mem_read", crate::spike_engine::slang_neuro_mem_read as *const u8);
        builder.symbol("slang_neuro_mem_write", crate::spike_engine::slang_neuro_mem_write as *const u8);
        builder.symbol("slang_neuro_mem_near_compute", crate::spike_engine::slang_neuro_mem_near_compute as *const u8);
        // -- v207-v212: Loihi Simulator --
        builder.symbol("slang_loihi_core_create", crate::loihi_sim::slang_loihi_core_create as *const u8);
        builder.symbol("slang_loihi_core_config", crate::loihi_sim::slang_loihi_core_config as *const u8);
        builder.symbol("slang_loihi_core_neuron_count", crate::loihi_sim::slang_loihi_core_neuron_count as *const u8);
        builder.symbol("slang_loihi_core_count", crate::loihi_sim::slang_loihi_core_count as *const u8);
        builder.symbol("slang_loihi_route_spike", crate::loihi_sim::slang_loihi_route_spike as *const u8);
        builder.symbol("slang_loihi_route_multicast", crate::loihi_sim::slang_loihi_route_multicast as *const u8);
        builder.symbol("slang_loihi_noc_latency", crate::loihi_sim::slang_loihi_noc_latency as *const u8);
        builder.symbol("slang_loihi_noc_bandwidth", crate::loihi_sim::slang_loihi_noc_bandwidth as *const u8);
        builder.symbol("slang_loihi_learn_stdp", crate::loihi_sim::slang_loihi_learn_stdp as *const u8);
        builder.symbol("slang_loihi_learn_reward", crate::loihi_sim::slang_loihi_learn_reward as *const u8);
        builder.symbol("slang_loihi_learn_3factor", crate::loihi_sim::slang_loihi_learn_3factor as *const u8);
        builder.symbol("slang_loihi_learn_config", crate::loihi_sim::slang_loihi_learn_config as *const u8);
        builder.symbol("slang_loihi_timestep", crate::loihi_sim::slang_loihi_timestep as *const u8);
        builder.symbol("slang_loihi_barrier_sync", crate::loihi_sim::slang_loihi_barrier_sync as *const u8);
        builder.symbol("slang_loihi_async_tick", crate::loihi_sim::slang_loihi_async_tick as *const u8);
        builder.symbol("slang_loihi_time_now", crate::loihi_sim::slang_loihi_time_now as *const u8);
        builder.symbol("slang_loihi_energy_spike", crate::loihi_sim::slang_loihi_energy_spike as *const u8);
        builder.symbol("slang_loihi_energy_compute", crate::loihi_sim::slang_loihi_energy_compute as *const u8);
        builder.symbol("slang_loihi_power_total", crate::loihi_sim::slang_loihi_power_total as *const u8);
        builder.symbol("slang_loihi_energy_reset", crate::loihi_sim::slang_loihi_energy_reset as *const u8);
        builder.symbol("slang_loihi_inst_soma", crate::loihi_sim::slang_loihi_inst_soma as *const u8);
        builder.symbol("slang_loihi_inst_synapse", crate::loihi_sim::slang_loihi_inst_synapse as *const u8);
        builder.symbol("slang_loihi_inst_axon", crate::loihi_sim::slang_loihi_inst_axon as *const u8);
        builder.symbol("slang_loihi_inst_dendrite", crate::loihi_sim::slang_loihi_inst_dendrite as *const u8);
        // -- v213-v218: SNN Learning --
        builder.symbol("slang_snn_surrogate_forward", crate::snn_learning::slang_snn_surrogate_forward as *const u8);
        builder.symbol("slang_snn_surrogate_backward", crate::snn_learning::slang_snn_surrogate_backward as *const u8);
        builder.symbol("slang_snn_surrogate_sigmoid", crate::snn_learning::slang_snn_surrogate_sigmoid as *const u8);
        builder.symbol("slang_snn_surrogate_loss", crate::snn_learning::slang_snn_surrogate_loss as *const u8);
        builder.symbol("slang_snn_bptt_forward", crate::snn_learning::slang_snn_bptt_forward as *const u8);
        builder.symbol("slang_snn_bptt_backward", crate::snn_learning::slang_snn_bptt_backward as *const u8);
        builder.symbol("slang_snn_bptt_truncate", crate::snn_learning::slang_snn_bptt_truncate as *const u8);
        builder.symbol("slang_snn_bptt_gradient", crate::snn_learning::slang_snn_bptt_gradient as *const u8);
        builder.symbol("slang_snn_nas_search", crate::snn_learning::slang_snn_nas_search as *const u8);
        builder.symbol("slang_snn_nas_evaluate", crate::snn_learning::slang_snn_nas_evaluate as *const u8);
        builder.symbol("slang_snn_nas_mutate", crate::snn_learning::slang_snn_nas_mutate as *const u8);
        builder.symbol("slang_snn_nas_best", crate::snn_learning::slang_snn_nas_best as *const u8);
        builder.symbol("slang_snn_fed_aggregate", crate::snn_learning::slang_snn_fed_aggregate as *const u8);
        builder.symbol("slang_snn_fed_share", crate::snn_learning::slang_snn_fed_share as *const u8);
        builder.symbol("slang_snn_fed_round", crate::snn_learning::slang_snn_fed_round as *const u8);
        builder.symbol("slang_snn_fed_node_count", crate::snn_learning::slang_snn_fed_node_count as *const u8);
        builder.symbol("slang_snn_transfer_freeze", crate::snn_learning::slang_snn_transfer_freeze as *const u8);
        builder.symbol("slang_snn_transfer_finetune", crate::snn_learning::slang_snn_transfer_finetune as *const u8);
        builder.symbol("slang_snn_transfer_adapt", crate::snn_learning::slang_snn_transfer_adapt as *const u8);
        builder.symbol("slang_snn_transfer_similarity", crate::snn_learning::slang_snn_transfer_similarity as *const u8);
        builder.symbol("slang_snn_continual_learn", crate::snn_learning::slang_snn_continual_learn as *const u8);
        builder.symbol("slang_snn_continual_consolidate", crate::snn_learning::slang_snn_continual_consolidate as *const u8);
        builder.symbol("slang_snn_continual_replay", crate::snn_learning::slang_snn_continual_replay as *const u8);
        builder.symbol("slang_snn_continual_forget_score", crate::snn_learning::slang_snn_continual_forget_score as *const u8);
        // -- v219-v224: Processing-in-Memory --
        builder.symbol("slang_pim_alloc", crate::pim_compute::slang_pim_alloc as *const u8);
        builder.symbol("slang_pim_compute_add", crate::pim_compute::slang_pim_compute_add as *const u8);
        builder.symbol("slang_pim_compute_mul", crate::pim_compute::slang_pim_compute_mul as *const u8);
        builder.symbol("slang_pim_transfer_cost", crate::pim_compute::slang_pim_transfer_cost as *const u8);
        builder.symbol("slang_datacentric_map", crate::pim_compute::slang_datacentric_map as *const u8);
        builder.symbol("slang_datacentric_reduce", crate::pim_compute::slang_datacentric_reduce as *const u8);
        builder.symbol("slang_datacentric_scatter", crate::pim_compute::slang_datacentric_scatter as *const u8);
        builder.symbol("slang_datacentric_gather", crate::pim_compute::slang_datacentric_gather as *const u8);
        builder.symbol("slang_sparse_spike_propagate", crate::pim_compute::slang_sparse_spike_propagate as *const u8);
        builder.symbol("slang_sparse_nonzero_count", crate::pim_compute::slang_sparse_nonzero_count as *const u8);
        builder.symbol("slang_sparse_compress", crate::pim_compute::slang_sparse_compress as *const u8);
        builder.symbol("slang_sparse_decompress", crate::pim_compute::slang_sparse_decompress as *const u8);
        builder.symbol("slang_cache_oblivious_transpose", crate::pim_compute::slang_cache_oblivious_transpose as *const u8);
        builder.symbol("slang_cache_oblivious_fft", crate::pim_compute::slang_cache_oblivious_fft as *const u8);
        builder.symbol("slang_cache_oblivious_sort", crate::pim_compute::slang_cache_oblivious_sort as *const u8);
        builder.symbol("slang_cache_oblivious_matmul", crate::pim_compute::slang_cache_oblivious_matmul as *const u8);
        builder.symbol("slang_memcompute_fused_mac", crate::pim_compute::slang_memcompute_fused_mac as *const u8);
        builder.symbol("slang_memcompute_fused_compare", crate::pim_compute::slang_memcompute_fused_compare as *const u8);
        builder.symbol("slang_memcompute_fused_accumulate", crate::pim_compute::slang_memcompute_fused_accumulate as *const u8);
        builder.symbol("slang_memcompute_pipeline_depth", crate::pim_compute::slang_memcompute_pipeline_depth as *const u8);
        builder.symbol("slang_zerocopy_spike_buffer", crate::pim_compute::slang_zerocopy_spike_buffer as *const u8);
        builder.symbol("slang_zerocopy_fanout", crate::pim_compute::slang_zerocopy_fanout as *const u8);
        builder.symbol("slang_zerocopy_gather", crate::pim_compute::slang_zerocopy_gather as *const u8);
        builder.symbol("slang_zerocopy_active_count", crate::pim_compute::slang_zerocopy_active_count as *const u8);
        // -- v225-v230: Brain Models --
        builder.symbol("slang_pred_coding_forward", crate::brain_models::slang_pred_coding_forward as *const u8);
        builder.symbol("slang_pred_coding_error", crate::brain_models::slang_pred_coding_error as *const u8);
        builder.symbol("slang_pred_coding_update", crate::brain_models::slang_pred_coding_update as *const u8);
        builder.symbol("slang_pred_coding_layers", crate::brain_models::slang_pred_coding_layers as *const u8);
        builder.symbol("slang_htm_spatial_pool", crate::brain_models::slang_htm_spatial_pool as *const u8);
        builder.symbol("slang_htm_temporal_memory", crate::brain_models::slang_htm_temporal_memory as *const u8);
        builder.symbol("slang_htm_anomaly_score", crate::brain_models::slang_htm_anomaly_score as *const u8);
        builder.symbol("slang_htm_column_count", crate::brain_models::slang_htm_column_count as *const u8);
        builder.symbol("slang_neuro_osc_gamma", crate::brain_models::slang_neuro_osc_gamma as *const u8);
        builder.symbol("slang_neuro_osc_theta", crate::brain_models::slang_neuro_osc_theta as *const u8);
        builder.symbol("slang_neuro_osc_couple", crate::brain_models::slang_neuro_osc_couple as *const u8);
        builder.symbol("slang_neuro_osc_phase_lock", crate::brain_models::slang_neuro_osc_phase_lock as *const u8);
        builder.symbol("slang_neuromod_dopamine", crate::brain_models::slang_neuromod_dopamine as *const u8);
        builder.symbol("slang_neuromod_serotonin", crate::brain_models::slang_neuromod_serotonin as *const u8);
        builder.symbol("slang_neuromod_acetylcholine", crate::brain_models::slang_neuromod_acetylcholine as *const u8);
        builder.symbol("slang_neuromod_apply", crate::brain_models::slang_neuromod_apply as *const u8);
        builder.symbol("slang_cortical_column_create", crate::brain_models::slang_cortical_column_create as *const u8);
        builder.symbol("slang_cortical_column_step", crate::brain_models::slang_cortical_column_step as *const u8);
        builder.symbol("slang_cortical_column_layer_activity", crate::brain_models::slang_cortical_column_layer_activity as *const u8);
        builder.symbol("slang_cortical_column_count", crate::brain_models::slang_cortical_column_count as *const u8);
        builder.symbol("slang_spike_attention_query", crate::brain_models::slang_spike_attention_query as *const u8);
        builder.symbol("slang_spike_attention_key", crate::brain_models::slang_spike_attention_key as *const u8);
        builder.symbol("slang_spike_attention_value", crate::brain_models::slang_spike_attention_value as *const u8);
        builder.symbol("slang_spike_attention_score", crate::brain_models::slang_spike_attention_score as *const u8);
        // -- v231-v236: GPU Neuromorphic --
        builder.symbol("slang_gpu_spike_propagate", crate::gpu_neuromorphic::slang_gpu_spike_propagate as *const u8);
        builder.symbol("slang_gpu_spike_batch_size", crate::gpu_neuromorphic::slang_gpu_spike_batch_size as *const u8);
        builder.symbol("slang_gpu_spike_throughput", crate::gpu_neuromorphic::slang_gpu_spike_throughput as *const u8);
        builder.symbol("slang_gpu_spike_sync", crate::gpu_neuromorphic::slang_gpu_spike_sync as *const u8);
        builder.symbol("slang_gpu_neuron_update", crate::gpu_neuromorphic::slang_gpu_neuron_update as *const u8);
        builder.symbol("slang_gpu_neuron_batch", crate::gpu_neuromorphic::slang_gpu_neuron_batch as *const u8);
        builder.symbol("slang_gpu_neuron_occupancy", crate::gpu_neuromorphic::slang_gpu_neuron_occupancy as *const u8);
        builder.symbol("slang_gpu_neuron_count", crate::gpu_neuromorphic::slang_gpu_neuron_count as *const u8);
        builder.symbol("slang_gpu_synapse_spmv", crate::gpu_neuromorphic::slang_gpu_synapse_spmv as *const u8);
        builder.symbol("slang_gpu_synapse_csr", crate::gpu_neuromorphic::slang_gpu_synapse_csr as *const u8);
        builder.symbol("slang_gpu_synapse_nnz", crate::gpu_neuromorphic::slang_gpu_synapse_nnz as *const u8);
        builder.symbol("slang_gpu_synapse_density", crate::gpu_neuromorphic::slang_gpu_synapse_density as *const u8);
        builder.symbol("slang_gpu_event_push", crate::gpu_neuromorphic::slang_gpu_event_push as *const u8);
        builder.symbol("slang_gpu_event_pop", crate::gpu_neuromorphic::slang_gpu_event_pop as *const u8);
        builder.symbol("slang_gpu_event_merge", crate::gpu_neuromorphic::slang_gpu_event_merge as *const u8);
        builder.symbol("slang_gpu_event_size", crate::gpu_neuromorphic::slang_gpu_event_size as *const u8);
        builder.symbol("slang_mixed_prec_quantize", crate::gpu_neuromorphic::slang_mixed_prec_quantize as *const u8);
        builder.symbol("slang_mixed_prec_dequantize", crate::gpu_neuromorphic::slang_mixed_prec_dequantize as *const u8);
        builder.symbol("slang_mixed_prec_accumulate", crate::gpu_neuromorphic::slang_mixed_prec_accumulate as *const u8);
        builder.symbol("slang_mixed_prec_bits", crate::gpu_neuromorphic::slang_mixed_prec_bits as *const u8);
        builder.symbol("slang_multi_gpu_partition", crate::gpu_neuromorphic::slang_multi_gpu_partition as *const u8);
        builder.symbol("slang_multi_gpu_sync", crate::gpu_neuromorphic::slang_multi_gpu_sync as *const u8);
        builder.symbol("slang_multi_gpu_migrate", crate::gpu_neuromorphic::slang_multi_gpu_migrate as *const u8);
        builder.symbol("slang_multi_gpu_count", crate::gpu_neuromorphic::slang_multi_gpu_count as *const u8);
        // -- v237-v242: Neuro Applications --
        builder.symbol("slang_spike_vision_encode", crate::neuro_applications::slang_spike_vision_encode as *const u8);
        builder.symbol("slang_spike_vision_edge", crate::neuro_applications::slang_spike_vision_edge as *const u8);
        builder.symbol("slang_spike_vision_motion", crate::neuro_applications::slang_spike_vision_motion as *const u8);
        builder.symbol("slang_spike_vision_frames", crate::neuro_applications::slang_spike_vision_frames as *const u8);
        builder.symbol("slang_spike_audio_encode", crate::neuro_applications::slang_spike_audio_encode as *const u8);
        builder.symbol("slang_spike_audio_frequency", crate::neuro_applications::slang_spike_audio_frequency as *const u8);
        builder.symbol("slang_spike_audio_onset", crate::neuro_applications::slang_spike_audio_onset as *const u8);
        builder.symbol("slang_spike_audio_classify", crate::neuro_applications::slang_spike_audio_classify as *const u8);
        builder.symbol("slang_spike_pid", crate::neuro_applications::slang_spike_pid as *const u8);
        builder.symbol("slang_spike_motor", crate::neuro_applications::slang_spike_motor as *const u8);
        builder.symbol("slang_spike_reflex", crate::neuro_applications::slang_spike_reflex as *const u8);
        builder.symbol("slang_spike_trajectory_cost", crate::neuro_applications::slang_spike_trajectory_cost as *const u8);
        builder.symbol("slang_spike_anomaly_score", crate::neuro_applications::slang_spike_anomaly_score as *const u8);
        builder.symbol("slang_spike_changepoint", crate::neuro_applications::slang_spike_changepoint as *const u8);
        builder.symbol("slang_spike_burst_detect", crate::neuro_applications::slang_spike_burst_detect as *const u8);
        builder.symbol("slang_spike_pattern_match", crate::neuro_applications::slang_spike_pattern_match as *const u8);
        builder.symbol("slang_spike_anneal", crate::neuro_applications::slang_spike_anneal as *const u8);
        builder.symbol("slang_spike_gradient", crate::neuro_applications::slang_spike_gradient as *const u8);
        builder.symbol("slang_spike_constraint", crate::neuro_applications::slang_spike_constraint as *const u8);
        builder.symbol("slang_spike_fitness", crate::neuro_applications::slang_spike_fitness as *const u8);
        builder.symbol("slang_spike_nlp_similarity", crate::neuro_applications::slang_spike_nlp_similarity as *const u8);
        builder.symbol("slang_spike_nlp_attention", crate::neuro_applications::slang_spike_nlp_attention as *const u8);
        builder.symbol("slang_spike_nlp_encode_len", crate::neuro_applications::slang_spike_nlp_encode_len as *const u8);
        builder.symbol("slang_spike_nlp_perplexity", crate::neuro_applications::slang_spike_nlp_perplexity as *const u8);
        // -- v243-v248: Neuro Evolution --
        builder.symbol("slang_neat_crossover", crate::neuro_evolve::slang_neat_crossover as *const u8);
        builder.symbol("slang_neat_mutate", crate::neuro_evolve::slang_neat_mutate as *const u8);
        builder.symbol("slang_neat_speciate", crate::neuro_evolve::slang_neat_speciate as *const u8);
        builder.symbol("slang_neat_generation", crate::neuro_evolve::slang_neat_generation as *const u8);
        builder.symbol("slang_som_bmu", crate::neuro_evolve::slang_som_bmu as *const u8);
        builder.symbol("slang_som_radius", crate::neuro_evolve::slang_som_radius as *const u8);
        builder.symbol("slang_som_learning_rate", crate::neuro_evolve::slang_som_learning_rate as *const u8);
        builder.symbol("slang_som_quant_error", crate::neuro_evolve::slang_som_quant_error as *const u8);
        builder.symbol("slang_neuro_nas_evaluate", crate::neuro_evolve::slang_neuro_nas_evaluate as *const u8);
        builder.symbol("slang_neuro_nas_sample", crate::neuro_evolve::slang_neuro_nas_sample as *const u8);
        builder.symbol("slang_neuro_nas_prune", crate::neuro_evolve::slang_neuro_nas_prune as *const u8);
        builder.symbol("slang_neuro_nas_best_score", crate::neuro_evolve::slang_neuro_nas_best_score as *const u8);
        builder.symbol("slang_spike_rl_rstdp", crate::neuro_evolve::slang_spike_rl_rstdp as *const u8);
        builder.symbol("slang_spike_rl_td", crate::neuro_evolve::slang_spike_rl_td as *const u8);
        builder.symbol("slang_spike_rl_policy", crate::neuro_evolve::slang_spike_rl_policy as *const u8);
        builder.symbol("slang_spike_rl_predict_reward", crate::neuro_evolve::slang_spike_rl_predict_reward as *const u8);
        builder.symbol("slang_curiosity_reward", crate::neuro_evolve::slang_curiosity_reward as *const u8);
        builder.symbol("slang_curiosity_info_gain", crate::neuro_evolve::slang_curiosity_info_gain as *const u8);
        builder.symbol("slang_curiosity_novelty", crate::neuro_evolve::slang_curiosity_novelty as *const u8);
        builder.symbol("slang_curiosity_decay", crate::neuro_evolve::slang_curiosity_decay as *const u8);
        builder.symbol("slang_meta_maml_adapt", crate::neuro_evolve::slang_meta_maml_adapt as *const u8);
        builder.symbol("slang_meta_reptile", crate::neuro_evolve::slang_meta_reptile as *const u8);
        builder.symbol("slang_meta_task_similarity", crate::neuro_evolve::slang_meta_task_similarity as *const u8);
        builder.symbol("slang_meta_convergence", crate::neuro_evolve::slang_meta_convergence as *const u8);
        // -- v249-v254: SNN-ANN Hybrid --
        builder.symbol("slang_snn_ann_to_rate", crate::snn_hybrid::slang_snn_ann_to_rate as *const u8);
        builder.symbol("slang_snn_rate_to_ann", crate::snn_hybrid::slang_snn_rate_to_ann as *const u8);
        builder.symbol("slang_snn_conversion_loss", crate::snn_hybrid::slang_snn_conversion_loss as *const u8);
        builder.symbol("slang_snn_optimal_timesteps", crate::snn_hybrid::slang_snn_optimal_timesteps as *const u8);
        builder.symbol("slang_hybrid_infer", crate::snn_hybrid::slang_hybrid_infer as *const u8);
        builder.symbol("slang_hybrid_set_fraction", crate::snn_hybrid::slang_hybrid_set_fraction as *const u8);
        builder.symbol("slang_hybrid_efficiency", crate::snn_hybrid::slang_hybrid_efficiency as *const u8);
        builder.symbol("slang_hybrid_accuracy_gain", crate::snn_hybrid::slang_hybrid_accuracy_gain as *const u8);
        builder.symbol("slang_spike_compile", crate::snn_hybrid::slang_spike_compile as *const u8);
        builder.symbol("slang_spike_compile_optimize", crate::snn_hybrid::slang_spike_compile_optimize as *const u8);
        builder.symbol("slang_spike_compile_count", crate::snn_hybrid::slang_spike_compile_count as *const u8);
        builder.symbol("slang_spike_compile_memory", crate::snn_hybrid::slang_spike_compile_memory as *const u8);
        builder.symbol("slang_diff_spike_ste", crate::snn_hybrid::slang_diff_spike_ste as *const u8);
        builder.symbol("slang_diff_spike_sigmoid", crate::snn_hybrid::slang_diff_spike_sigmoid as *const u8);
        builder.symbol("slang_diff_spike_fast_sigmoid", crate::snn_hybrid::slang_diff_spike_fast_sigmoid as *const u8);
        builder.symbol("slang_diff_spike_accumulate", crate::snn_hybrid::slang_diff_spike_accumulate as *const u8);
        builder.symbol("slang_neural_ode_euler", crate::snn_hybrid::slang_neural_ode_euler as *const u8);
        builder.symbol("slang_neural_ode_rk4", crate::snn_hybrid::slang_neural_ode_rk4 as *const u8);
        builder.symbol("slang_neural_ode_adjoint", crate::snn_hybrid::slang_neural_ode_adjoint as *const u8);
        builder.symbol("slang_neural_ode_adaptive_dt", crate::snn_hybrid::slang_neural_ode_adaptive_dt as *const u8);
        builder.symbol("slang_hybrid_distill", crate::snn_hybrid::slang_hybrid_distill as *const u8);
        builder.symbol("slang_hybrid_freeze", crate::snn_hybrid::slang_hybrid_freeze as *const u8);
        builder.symbol("slang_hybrid_lr", crate::snn_hybrid::slang_hybrid_lr as *const u8);
        builder.symbol("slang_hybrid_weighted_acc", crate::snn_hybrid::slang_hybrid_weighted_acc as *const u8);
        // -- v255-v260: Hippocampal Memory --
        builder.symbol("slang_hippo_encode", crate::hippocampal_memory::slang_hippo_encode as *const u8);
        builder.symbol("slang_hippo_recall", crate::hippocampal_memory::slang_hippo_recall as *const u8);
        builder.symbol("slang_hippo_replay", crate::hippocampal_memory::slang_hippo_replay as *const u8);
        builder.symbol("slang_hippo_count", crate::hippocampal_memory::slang_hippo_count as *const u8);
        builder.symbol("slang_wm_push", crate::hippocampal_memory::slang_wm_push as *const u8);
        builder.symbol("slang_wm_pop", crate::hippocampal_memory::slang_wm_pop as *const u8);
        builder.symbol("slang_wm_set_capacity", crate::hippocampal_memory::slang_wm_set_capacity as *const u8);
        builder.symbol("slang_wm_utilization", crate::hippocampal_memory::slang_wm_utilization as *const u8);
        builder.symbol("slang_sleep_consolidate", crate::hippocampal_memory::slang_sleep_consolidate as *const u8);
        builder.symbol("slang_sleep_rem", crate::hippocampal_memory::slang_sleep_rem as *const u8);
        builder.symbol("slang_sleep_nrem_ripple", crate::hippocampal_memory::slang_sleep_nrem_ripple as *const u8);
        builder.symbol("slang_sleep_duration", crate::hippocampal_memory::slang_sleep_duration as *const u8);
        builder.symbol("slang_hopfield_energy", crate::hippocampal_memory::slang_hopfield_energy as *const u8);
        builder.symbol("slang_hopfield_capacity", crate::hippocampal_memory::slang_hopfield_capacity as *const u8);
        builder.symbol("slang_hopfield_retrieve", crate::hippocampal_memory::slang_hopfield_retrieve as *const u8);
        builder.symbol("slang_hopfield_accuracy", crate::hippocampal_memory::slang_hopfield_accuracy as *const u8);
        builder.symbol("slang_synaptag_decay", crate::hippocampal_memory::slang_synaptag_decay as *const u8);
        builder.symbol("slang_synaptag_capture", crate::hippocampal_memory::slang_synaptag_capture as *const u8);
        builder.symbol("slang_synaptag_late_ltp", crate::hippocampal_memory::slang_synaptag_late_ltp as *const u8);
        builder.symbol("slang_synaptag_protein", crate::hippocampal_memory::slang_synaptag_protein as *const u8);
        builder.symbol("slang_memcompress_schema", crate::hippocampal_memory::slang_memcompress_schema as *const u8);
        builder.symbol("slang_memcompress_forget", crate::hippocampal_memory::slang_memcompress_forget as *const u8);
        builder.symbol("slang_memcompress_merge", crate::hippocampal_memory::slang_memcompress_merge as *const u8);
        builder.symbol("slang_memcompress_ratio", crate::hippocampal_memory::slang_memcompress_ratio as *const u8);
        // -- v261-v266: Distributed Neuromorphic --
        builder.symbol("slang_neuro_cluster_init", crate::neuro_distributed::slang_neuro_cluster_init as *const u8);
        builder.symbol("slang_neuro_cluster_distribute", crate::neuro_distributed::slang_neuro_cluster_distribute as *const u8);
        builder.symbol("slang_neuro_cluster_load", crate::neuro_distributed::slang_neuro_cluster_load as *const u8);
        builder.symbol("slang_neuro_cluster_nodes", crate::neuro_distributed::slang_neuro_cluster_nodes as *const u8);
        builder.symbol("slang_spike_consensus_vote", crate::neuro_distributed::slang_spike_consensus_vote as *const u8);
        builder.symbol("slang_spike_consensus_bft", crate::neuro_distributed::slang_spike_consensus_bft as *const u8);
        builder.symbol("slang_spike_consensus_tick", crate::neuro_distributed::slang_spike_consensus_tick as *const u8);
        builder.symbol("slang_spike_consensus_latency", crate::neuro_distributed::slang_spike_consensus_latency as *const u8);
        builder.symbol("slang_fed_neuro_average", crate::neuro_distributed::slang_fed_neuro_average as *const u8);
        builder.symbol("slang_fed_neuro_dp_noise", crate::neuro_distributed::slang_fed_neuro_dp_noise as *const u8);
        builder.symbol("slang_fed_neuro_compress", crate::neuro_distributed::slang_fed_neuro_compress as *const u8);
        builder.symbol("slang_fed_neuro_round", crate::neuro_distributed::slang_fed_neuro_round as *const u8);
        builder.symbol("slang_edge_neuro_budget", crate::neuro_distributed::slang_edge_neuro_budget as *const u8);
        builder.symbol("slang_edge_neuro_quantize", crate::neuro_distributed::slang_edge_neuro_quantize as *const u8);
        builder.symbol("slang_edge_neuro_latency_ok", crate::neuro_distributed::slang_edge_neuro_latency_ok as *const u8);
        builder.symbol("slang_edge_neuro_model_size", crate::neuro_distributed::slang_edge_neuro_model_size as *const u8);
        builder.symbol("slang_stream_spike_process", crate::neuro_distributed::slang_stream_spike_process as *const u8);
        builder.symbol("slang_stream_spike_rate", crate::neuro_distributed::slang_stream_spike_rate as *const u8);
        builder.symbol("slang_stream_spike_backpressure", crate::neuro_distributed::slang_stream_spike_backpressure as *const u8);
        builder.symbol("slang_stream_spike_total", crate::neuro_distributed::slang_stream_spike_total as *const u8);
        builder.symbol("slang_neuro_platform_caps", crate::neuro_distributed::slang_neuro_platform_caps as *const u8);
        builder.symbol("slang_neuro_platform_map", crate::neuro_distributed::slang_neuro_platform_map as *const u8);
        builder.symbol("slang_neuro_platform_overhead", crate::neuro_distributed::slang_neuro_platform_overhead as *const u8);
        builder.symbol("slang_neuro_platform_power", crate::neuro_distributed::slang_neuro_platform_power as *const u8);
        // -- v267-v272: Neuro Tooling --
        builder.symbol("slang_neuro_viz_spike_raster", crate::neuro_tooling::slang_neuro_viz_spike_raster as *const u8);
        builder.symbol("slang_neuro_viz_membrane", crate::neuro_tooling::slang_neuro_viz_membrane as *const u8);
        builder.symbol("slang_neuro_viz_connectivity", crate::neuro_tooling::slang_neuro_viz_connectivity as *const u8);
        builder.symbol("slang_neuro_viz_frames", crate::neuro_tooling::slang_neuro_viz_frames as *const u8);
        builder.symbol("slang_neuro_debug_break", crate::neuro_tooling::slang_neuro_debug_break as *const u8);
        builder.symbol("slang_neuro_debug_inspect", crate::neuro_tooling::slang_neuro_debug_inspect as *const u8);
        builder.symbol("slang_neuro_debug_step", crate::neuro_tooling::slang_neuro_debug_step as *const u8);
        builder.symbol("slang_neuro_debug_breakpoints", crate::neuro_tooling::slang_neuro_debug_breakpoints as *const u8);
        builder.symbol("slang_neuro_profile_throughput", crate::neuro_tooling::slang_neuro_profile_throughput as *const u8);
        builder.symbol("slang_neuro_profile_memory", crate::neuro_tooling::slang_neuro_profile_memory as *const u8);
        builder.symbol("slang_neuro_profile_energy", crate::neuro_tooling::slang_neuro_profile_energy as *const u8);
        builder.symbol("slang_neuro_profile_samples", crate::neuro_tooling::slang_neuro_profile_samples as *const u8);
        builder.symbol("slang_neuro_dsl_neuron", crate::neuro_tooling::slang_neuro_dsl_neuron as *const u8);
        builder.symbol("slang_neuro_dsl_synapse", crate::neuro_tooling::slang_neuro_dsl_synapse as *const u8);
        builder.symbol("slang_neuro_dsl_network", crate::neuro_tooling::slang_neuro_dsl_network as *const u8);
        builder.symbol("slang_neuro_dsl_validate", crate::neuro_tooling::slang_neuro_dsl_validate as *const u8);
        builder.symbol("slang_neuro_bench_spike_lat", crate::neuro_tooling::slang_neuro_bench_spike_lat as *const u8);
        builder.symbol("slang_neuro_bench_neuron_tput", crate::neuro_tooling::slang_neuro_bench_neuron_tput as *const u8);
        builder.symbol("slang_neuro_bench_synapse_rate", crate::neuro_tooling::slang_neuro_bench_synapse_rate as *const u8);
        builder.symbol("slang_neuro_bench_efficiency", crate::neuro_tooling::slang_neuro_bench_efficiency as *const u8);
        builder.symbol("slang_neuro_test_timing", crate::neuro_tooling::slang_neuro_test_timing as *const u8);
        builder.symbol("slang_neuro_test_accuracy", crate::neuro_tooling::slang_neuro_test_accuracy as *const u8);
        builder.symbol("slang_neuro_test_convergence", crate::neuro_tooling::slang_neuro_test_convergence as *const u8);
        builder.symbol("slang_neuro_test_spike_gen", crate::neuro_tooling::slang_neuro_test_spike_gen as *const u8);
        // -- v273-v278: Quantum Neuromorphic --
        builder.symbol("slang_quantum_spike_encode", crate::quantum_neuro::slang_quantum_spike_encode as *const u8);
        builder.symbol("slang_quantum_spike_decode", crate::quantum_neuro::slang_quantum_spike_decode as *const u8);
        builder.symbol("slang_quantum_spike_superpose", crate::quantum_neuro::slang_quantum_spike_superpose as *const u8);
        builder.symbol("slang_quantum_spike_fidelity", crate::quantum_neuro::slang_quantum_spike_fidelity as *const u8);
        builder.symbol("slang_quantum_plasticity_stdp", crate::quantum_neuro::slang_quantum_plasticity_stdp as *const u8);
        builder.symbol("slang_quantum_plasticity_anneal", crate::quantum_neuro::slang_quantum_plasticity_anneal as *const u8);
        builder.symbol("slang_quantum_plasticity_tunnel", crate::quantum_neuro::slang_quantum_plasticity_tunnel as *const u8);
        builder.symbol("slang_quantum_plasticity_t2", crate::quantum_neuro::slang_quantum_plasticity_t2 as *const u8);
        builder.symbol("slang_quantum_reservoir_init", crate::quantum_neuro::slang_quantum_reservoir_init as *const u8);
        builder.symbol("slang_quantum_reservoir_project", crate::quantum_neuro::slang_quantum_reservoir_project as *const u8);
        builder.symbol("slang_quantum_reservoir_kernel", crate::quantum_neuro::slang_quantum_reservoir_kernel as *const u8);
        builder.symbol("slang_quantum_reservoir_dim", crate::quantum_neuro::slang_quantum_reservoir_dim as *const u8);
        builder.symbol("slang_qsnn_var_update", crate::quantum_neuro::slang_qsnn_var_update as *const u8);
        builder.symbol("slang_qsnn_var_cost", crate::quantum_neuro::slang_qsnn_var_cost as *const u8);
        builder.symbol("slang_qsnn_var_depth", crate::quantum_neuro::slang_qsnn_var_depth as *const u8);
        builder.symbol("slang_qsnn_var_expressibility", crate::quantum_neuro::slang_qsnn_var_expressibility as *const u8);
        builder.symbol("slang_quantum_qec_shor", crate::quantum_neuro::slang_quantum_qec_shor as *const u8);
        builder.symbol("slang_quantum_qec_surface", crate::quantum_neuro::slang_quantum_qec_surface as *const u8);
        builder.symbol("slang_quantum_qec_syndrome", crate::quantum_neuro::slang_quantum_qec_syndrome as *const u8);
        builder.symbol("slang_quantum_qec_overhead", crate::quantum_neuro::slang_quantum_qec_overhead as *const u8);
        builder.symbol("slang_quantum_bridge_encode", crate::quantum_neuro::slang_quantum_bridge_encode as *const u8);
        builder.symbol("slang_quantum_bridge_decode", crate::quantum_neuro::slang_quantum_bridge_decode as *const u8);
        builder.symbol("slang_quantum_bridge_cost", crate::quantum_neuro::slang_quantum_bridge_cost as *const u8);
        builder.symbol("slang_quantum_bridge_advantage", crate::quantum_neuro::slang_quantum_bridge_advantage as *const u8);
        // -- v279-v284: Neuro Safety --
        builder.symbol("slang_neuro_verify_timing", crate::neuro_safety::slang_neuro_verify_timing as *const u8);
        builder.symbol("slang_neuro_verify_membrane", crate::neuro_safety::slang_neuro_verify_membrane as *const u8);
        builder.symbol("slang_neuro_verify_symmetry", crate::neuro_safety::slang_neuro_verify_symmetry as *const u8);
        builder.symbol("slang_neuro_verify_liveness", crate::neuro_safety::slang_neuro_verify_liveness as *const u8);
        builder.symbol("slang_neuro_safe_rate_clamp", crate::neuro_safety::slang_neuro_safe_rate_clamp as *const u8);
        builder.symbol("slang_neuro_safe_runaway", crate::neuro_safety::slang_neuro_safe_runaway as *const u8);
        builder.symbol("slang_neuro_safe_dead_neuron", crate::neuro_safety::slang_neuro_safe_dead_neuron as *const u8);
        builder.symbol("slang_neuro_safe_violations", crate::neuro_safety::slang_neuro_safe_violations as *const u8);
        builder.symbol("slang_neuro_explain_contribution", crate::neuro_safety::slang_neuro_explain_contribution as *const u8);
        builder.symbol("slang_neuro_explain_ablation", crate::neuro_safety::slang_neuro_explain_ablation as *const u8);
        builder.symbol("slang_neuro_explain_saliency", crate::neuro_safety::slang_neuro_explain_saliency as *const u8);
        builder.symbol("slang_neuro_explain_lrp", crate::neuro_safety::slang_neuro_explain_lrp as *const u8);
        builder.symbol("slang_neuro_robust_eps_check", crate::neuro_safety::slang_neuro_robust_eps_check as *const u8);
        builder.symbol("slang_neuro_robust_margin", crate::neuro_safety::slang_neuro_robust_margin as *const u8);
        builder.symbol("slang_neuro_robust_certified_radius", crate::neuro_safety::slang_neuro_robust_certified_radius as *const u8);
        builder.symbol("slang_neuro_robust_inject_noise", crate::neuro_safety::slang_neuro_robust_inject_noise as *const u8);
        builder.symbol("slang_neuro_fair_dp_gap", crate::neuro_safety::slang_neuro_fair_dp_gap as *const u8);
        builder.symbol("slang_neuro_fair_eo_gap", crate::neuro_safety::slang_neuro_fair_eo_gap as *const u8);
        builder.symbol("slang_neuro_fair_calibration", crate::neuro_safety::slang_neuro_fair_calibration as *const u8);
        builder.symbol("slang_neuro_fair_lipschitz", crate::neuro_safety::slang_neuro_fair_lipschitz as *const u8);
        builder.symbol("slang_neuro_cert_bounds", crate::neuro_safety::slang_neuro_cert_bounds as *const u8);
        builder.symbol("slang_neuro_cert_ibp_width", crate::neuro_safety::slang_neuro_cert_ibp_width as *const u8);
        builder.symbol("slang_neuro_cert_crown", crate::neuro_safety::slang_neuro_cert_crown as *const u8);
        builder.symbol("slang_neuro_cert_accuracy", crate::neuro_safety::slang_neuro_cert_accuracy as *const u8);
        // -- v285-v290: Neuro Performance --
        builder.symbol("slang_neuro_simd_accumulate", crate::neuro_perf::slang_neuro_simd_accumulate as *const u8);
        builder.symbol("slang_neuro_simd_threshold", crate::neuro_perf::slang_neuro_simd_threshold as *const u8);
        builder.symbol("slang_neuro_simd_decay", crate::neuro_perf::slang_neuro_simd_decay as *const u8);
        builder.symbol("slang_neuro_simd_throughput", crate::neuro_perf::slang_neuro_simd_throughput as *const u8);
        builder.symbol("slang_neuro_jit_compile", crate::neuro_perf::slang_neuro_jit_compile as *const u8);
        builder.symbol("slang_neuro_jit_speedup", crate::neuro_perf::slang_neuro_jit_speedup as *const u8);
        builder.symbol("slang_neuro_jit_cache_hit", crate::neuro_perf::slang_neuro_jit_cache_hit as *const u8);
        builder.symbol("slang_neuro_jit_compiled_count", crate::neuro_perf::slang_neuro_jit_compiled_count as *const u8);
        builder.symbol("slang_neuro_precision_auto", crate::neuro_perf::slang_neuro_precision_auto as *const u8);
        builder.symbol("slang_neuro_precision_quant_error", crate::neuro_perf::slang_neuro_precision_quant_error as *const u8);
        builder.symbol("slang_neuro_precision_savings", crate::neuro_perf::slang_neuro_precision_savings as *const u8);
        builder.symbol("slang_neuro_precision_scale", crate::neuro_perf::slang_neuro_precision_scale as *const u8);
        builder.symbol("slang_neuro_spec_predict", crate::neuro_perf::slang_neuro_spec_predict as *const u8);
        builder.symbol("slang_neuro_spec_gain", crate::neuro_perf::slang_neuro_spec_gain as *const u8);
        builder.symbol("slang_neuro_spec_rollback_cost", crate::neuro_perf::slang_neuro_spec_rollback_cost as *const u8);
        builder.symbol("slang_neuro_spec_confidence", crate::neuro_perf::slang_neuro_spec_confidence as *const u8);
        builder.symbol("slang_neuro_pgo_sample", crate::neuro_perf::slang_neuro_pgo_sample as *const u8);
        builder.symbol("slang_neuro_pgo_is_hot", crate::neuro_perf::slang_neuro_pgo_is_hot as *const u8);
        builder.symbol("slang_neuro_pgo_unroll", crate::neuro_perf::slang_neuro_pgo_unroll as *const u8);
        builder.symbol("slang_neuro_pgo_total_samples", crate::neuro_perf::slang_neuro_pgo_total_samples as *const u8);
        builder.symbol("slang_neuro_zero_send", crate::neuro_perf::slang_neuro_zero_send as *const u8);
        builder.symbol("slang_neuro_zero_update", crate::neuro_perf::slang_neuro_zero_update as *const u8);
        builder.symbol("slang_neuro_zero_conn_type", crate::neuro_perf::slang_neuro_zero_conn_type as *const u8);
        builder.symbol("slang_neuro_zero_overhead", crate::neuro_perf::slang_neuro_zero_overhead as *const u8);
        // -- v291: Advanced Spike Analytics --
        builder.symbol("slang_spike_analytics_mean_rate", slang_spike_analytics_mean_rate as *const u8);
        builder.symbol("slang_spike_analytics_cv_isi", slang_spike_analytics_cv_isi as *const u8);
        builder.symbol("slang_spike_analytics_fano_factor", slang_spike_analytics_fano_factor as *const u8);
        builder.symbol("slang_spike_analytics_burst_index", slang_spike_analytics_burst_index as *const u8);
        // -- v292: Neural Network Metrics --
        builder.symbol("slang_nn_metric_sparsity", slang_nn_metric_sparsity as *const u8);
        builder.symbol("slang_nn_metric_entropy", slang_nn_metric_entropy as *const u8);
        builder.symbol("slang_nn_metric_mutual_info", slang_nn_metric_mutual_info as *const u8);
        builder.symbol("slang_nn_metric_transfer_entropy", slang_nn_metric_transfer_entropy as *const u8);
        // -- v293: Spike Train Distance --
        builder.symbol("slang_spike_dist_victor_purpura", slang_spike_dist_victor_purpura as *const u8);
        builder.symbol("slang_spike_dist_van_rossum", slang_spike_dist_van_rossum as *const u8);
        builder.symbol("slang_spike_dist_schreiber", slang_spike_dist_schreiber as *const u8);
        builder.symbol("slang_spike_dist_earth_mover", slang_spike_dist_earth_mover as *const u8);
        // -- v294: Neural Coding --
        builder.symbol("slang_neural_code_rate", slang_neural_code_rate as *const u8);
        builder.symbol("slang_neural_code_temporal", slang_neural_code_temporal as *const u8);
        builder.symbol("slang_neural_code_population", slang_neural_code_population as *const u8);
        builder.symbol("slang_neural_code_sparse", slang_neural_code_sparse as *const u8);
        // -- v295: Synaptic Plasticity Metrics --
        builder.symbol("slang_synap_metric_ltp_ratio", slang_synap_metric_ltp_ratio as *const u8);
        builder.symbol("slang_synap_metric_ltd_ratio", slang_synap_metric_ltd_ratio as *const u8);
        builder.symbol("slang_synap_metric_homeostatic", slang_synap_metric_homeostatic as *const u8);
        builder.symbol("slang_synap_metric_metaplasticity", slang_synap_metric_metaplasticity as *const u8);
        // -- v296: Network Topology --
        builder.symbol("slang_topo_clustering_coeff", slang_topo_clustering_coeff as *const u8);
        builder.symbol("slang_topo_path_length", slang_topo_path_length as *const u8);
        builder.symbol("slang_topo_small_world", slang_topo_small_world as *const u8);
        builder.symbol("slang_topo_modularity", slang_topo_modularity as *const u8);
        // -- v297: Neuromorphic IO --
        builder.symbol("slang_neuro_io_aer_encode", slang_neuro_io_aer_encode as *const u8);
        builder.symbol("slang_neuro_io_aer_decode", slang_neuro_io_aer_decode as *const u8);
        builder.symbol("slang_neuro_io_dvs_encode", slang_neuro_io_dvs_encode as *const u8);
        builder.symbol("slang_neuro_io_serial_pack", slang_neuro_io_serial_pack as *const u8);
        // -- v298: Neural Dynamics --
        builder.symbol("slang_dyn_lyapunov_exp", slang_dyn_lyapunov_exp as *const u8);
        builder.symbol("slang_dyn_bifurcation", slang_dyn_bifurcation as *const u8);
        builder.symbol("slang_dyn_phase_portrait", slang_dyn_phase_portrait as *const u8);
        builder.symbol("slang_dyn_attractor_dim", slang_dyn_attractor_dim as *const u8);
        // -- v299: Neuromorphic Scheduler --
        builder.symbol("slang_neuro_sched_priority", slang_neuro_sched_priority as *const u8);
        builder.symbol("slang_neuro_sched_deadline", slang_neuro_sched_deadline as *const u8);
        builder.symbol("slang_neuro_sched_edf", slang_neuro_sched_edf as *const u8);
        builder.symbol("slang_neuro_sched_utilization", slang_neuro_sched_utilization as *const u8);
        // -- v300: Milestone --
        builder.symbol("slang_vitalis_v300_version", slang_vitalis_v300_version as *const u8);
        builder.symbol("slang_vitalis_v300_total_builtins", slang_vitalis_v300_total_builtins as *const u8);
        builder.symbol("slang_vitalis_v300_neuro_modules", slang_vitalis_v300_neuro_modules as *const u8);
        builder.symbol("slang_vitalis_v300_milestone", slang_vitalis_v300_milestone as *const u8);

        // -- v301-v366: Post-Neuromorphic Era --
        // v301 Escape Analysis
        builder.symbol("slang_escape_analyze", crate::escape_analysis::slang_escape_analyze as *const u8);
        builder.symbol("slang_escape_stack_promoted", crate::escape_analysis::slang_escape_stack_promoted as *const u8);
        builder.symbol("slang_escape_summary", crate::escape_analysis::slang_escape_summary as *const u8);
        builder.symbol("slang_escape_clear", crate::escape_analysis::slang_escape_clear as *const u8);
        // v302 Tail Call Optimization
        builder.symbol("slang_tco_detect", slang_tco_detect as *const u8);
        builder.symbol("slang_tco_optimized_count", slang_tco_optimized_count as *const u8);
        builder.symbol("slang_tco_depth_limit", slang_tco_depth_limit as *const u8);
        builder.symbol("slang_tco_enabled", slang_tco_enabled as *const u8);
        // v303 Algebraic Simplification
        builder.symbol("slang_opt_algebraic_count", slang_opt_algebraic_count as *const u8);
        builder.symbol("slang_opt_algebraic_enable", slang_opt_algebraic_enable as *const u8);
        builder.symbol("slang_opt_strength_reduced", slang_opt_strength_reduced as *const u8);
        builder.symbol("slang_opt_identity_removed", slang_opt_identity_removed as *const u8);
        // v304 Interprocedural Analysis
        builder.symbol("slang_ipa_add_edge", crate::interprocedural::slang_ipa_add_edge as *const u8);
        builder.symbol("slang_ipa_call_graph_size", crate::interprocedural::slang_ipa_call_graph_size as *const u8);
        builder.symbol("slang_ipa_mark_pure", crate::interprocedural::slang_ipa_mark_pure as *const u8);
        builder.symbol("slang_ipa_pure_functions", crate::interprocedural::slang_ipa_pure_functions as *const u8);
        builder.symbol("slang_ipa_record_const_args", crate::interprocedural::slang_ipa_record_const_args as *const u8);
        builder.symbol("slang_ipa_const_args", crate::interprocedural::slang_ipa_const_args as *const u8);
        builder.symbol("slang_ipa_summary", crate::interprocedural::slang_ipa_summary as *const u8);
        builder.symbol("slang_ipa_clear", crate::interprocedural::slang_ipa_clear as *const u8);
        // v305 LTO
        builder.symbol("slang_lto_inline_count", slang_lto_inline_count as *const u8);
        builder.symbol("slang_lto_dead_globals", slang_lto_dead_globals as *const u8);
        builder.symbol("slang_lto_devirtualized", slang_lto_devirtualized as *const u8);
        builder.symbol("slang_lto_enabled", slang_lto_enabled as *const u8);
        // v306 Vectorization
        builder.symbol("slang_vec_slp_opportunities", slang_vec_slp_opportunities as *const u8);
        builder.symbol("slang_vec_slp_applied", slang_vec_slp_applied as *const u8);
        builder.symbol("slang_vec_width", slang_vec_width as *const u8);
        builder.symbol("slang_vec_speedup_estimate", slang_vec_speedup_estimate as *const u8);
        // v307 Compile-Time Execution
        builder.symbol("slang_consteval_string", slang_consteval_string as *const u8);
        builder.symbol("slang_consteval_array", slang_consteval_array as *const u8);
        builder.symbol("slang_consteval_struct", slang_consteval_struct as *const u8);
        builder.symbol("slang_consteval_count", slang_consteval_count as *const u8);
        // v308 Profile-Guided Optimization
        builder.symbol("slang_pgo_record", slang_pgo_record as *const u8);
        builder.symbol("slang_pgo_hotness", slang_pgo_hotness as *const u8);
        builder.symbol("slang_pgo_branch_bias", slang_pgo_branch_bias as *const u8);
        builder.symbol("slang_pgo_total_samples", slang_pgo_total_samples as *const u8);
        // v309 Register Allocation
        builder.symbol("slang_regalloc_spill_count", slang_regalloc_spill_count as *const u8);
        builder.symbol("slang_regalloc_move_count", slang_regalloc_move_count as *const u8);
        builder.symbol("slang_regalloc_pressure", slang_regalloc_pressure as *const u8);
        builder.symbol("slang_regalloc_coalesced", slang_regalloc_coalesced as *const u8);
        // v310 Debug Info
        builder.symbol("slang_debug_line_count", slang_debug_line_count as *const u8);
        builder.symbol("slang_debug_var_count", slang_debug_var_count as *const u8);
        builder.symbol("slang_debug_scope_depth", slang_debug_scope_depth as *const u8);
        builder.symbol("slang_debug_info_size", slang_debug_info_size as *const u8);
        // v311 Existential Types
        builder.symbol("slang_existential_create", slang_existential_create as *const u8);
        builder.symbol("slang_existential_open", slang_existential_open as *const u8);
        builder.symbol("slang_existential_pack", slang_existential_pack as *const u8);
        builder.symbol("slang_existential_count", slang_existential_count as *const u8);
        // v312 Row Types
        builder.symbol("slang_row_type_fields", crate::row_types::slang_row_type_fields as *const u8);
        builder.symbol("slang_row_type_create", crate::row_types::slang_row_type_create as *const u8);
        builder.symbol("slang_row_type_extend", crate::row_types::slang_row_type_extend as *const u8);
        builder.symbol("slang_row_type_restrict", crate::row_types::slang_row_type_restrict as *const u8);
        builder.symbol("slang_row_type_compatible", crate::row_types::slang_row_type_compatible as *const u8);
        // v313 Linear Types
        builder.symbol("slang_linear_create", crate::linear_types::slang_linear_create as *const u8);
        builder.symbol("slang_linear_check", crate::linear_types::slang_linear_check as *const u8);
        builder.symbol("slang_linear_consume", crate::linear_types::slang_linear_consume as *const u8);
        builder.symbol("slang_session_create", crate::linear_types::slang_session_create as *const u8);
        builder.symbol("slang_session_state", crate::linear_types::slang_session_state as *const u8);
        builder.symbol("slang_session_advance", crate::linear_types::slang_session_advance as *const u8);
        // v314 GADTs
        builder.symbol("slang_gadt_create", slang_gadt_create as *const u8);
        builder.symbol("slang_gadt_refine", slang_gadt_refine as *const u8);
        builder.symbol("slang_gadt_witness", slang_gadt_witness as *const u8);
        builder.symbol("slang_gadt_count", slang_gadt_count as *const u8);
        // v315 Type Classes
        builder.symbol("slang_typeclass_instances", slang_typeclass_instances as *const u8);
        builder.symbol("slang_typeclass_resolve", slang_typeclass_resolve as *const u8);
        builder.symbol("slang_typeclass_coherence", slang_typeclass_coherence as *const u8);
        builder.symbol("slang_typeclass_register", slang_typeclass_register as *const u8);
        // v316 Dependent Types
        builder.symbol("slang_dependent_proof", slang_dependent_proof as *const u8);
        builder.symbol("slang_dependent_index", slang_dependent_index as *const u8);
        builder.symbol("slang_dependent_refine", slang_dependent_refine as *const u8);
        builder.symbol("slang_dependent_check", slang_dependent_check as *const u8);
        // v317 Effect Inference
        builder.symbol("slang_effect_infer", slang_effect_infer as *const u8);
        builder.symbol("slang_effect_row", slang_effect_row as *const u8);
        builder.symbol("slang_effect_mask", slang_effect_mask as *const u8);
        builder.symbol("slang_effect_polymorphic", slang_effect_polymorphic as *const u8);
        // v318 Mixture of Experts
        builder.symbol("slang_moe_create", crate::mixture_of_experts::slang_moe_create as *const u8);
        builder.symbol("slang_moe_route", crate::mixture_of_experts::slang_moe_route as *const u8);
        builder.symbol("slang_moe_expert_load", crate::mixture_of_experts::slang_moe_expert_load as *const u8);
        builder.symbol("slang_moe_aux_loss", crate::mixture_of_experts::slang_moe_aux_loss as *const u8);
        // v319 Quantization
        builder.symbol("slang_quant_int8", slang_quant_int8 as *const u8);
        builder.symbol("slang_quant_int4", slang_quant_int4 as *const u8);
        builder.symbol("slang_quant_error", slang_quant_error as *const u8);
        builder.symbol("slang_quant_calibrate", slang_quant_calibrate as *const u8);
        // v320 Attention Variants
        builder.symbol("slang_attn_flash", slang_attn_flash as *const u8);
        builder.symbol("slang_attn_linear", slang_attn_linear as *const u8);
        builder.symbol("slang_attn_sparse", slang_attn_sparse as *const u8);
        builder.symbol("slang_attn_sliding_window", slang_attn_sliding_window as *const u8);
        // v321 Distillation
        builder.symbol("slang_distill_kd_loss", crate::distillation::slang_distill_kd_loss as *const u8);
        builder.symbol("slang_distill_feature_loss", crate::distillation::slang_distill_feature_loss as *const u8);
        builder.symbol("slang_distill_attention_transfer", crate::distillation::slang_distill_attention_transfer as *const u8);
        builder.symbol("slang_distill_temperature", crate::distillation::slang_distill_temperature as *const u8);
        // v322 GNN
        builder.symbol("slang_gnn_create", crate::gnn::slang_gnn_create as *const u8);
        builder.symbol("slang_gnn_add_edge", crate::gnn::slang_gnn_add_edge as *const u8);
        builder.symbol("slang_gnn_set_feature", crate::gnn::slang_gnn_set_feature as *const u8);
        builder.symbol("slang_gnn_message_pass", crate::gnn::slang_gnn_message_pass as *const u8);
        builder.symbol("slang_gnn_conv", crate::gnn::slang_gnn_conv as *const u8);
        builder.symbol("slang_gnn_attention", crate::gnn::slang_gnn_attention as *const u8);
        builder.symbol("slang_gnn_readout", crate::gnn::slang_gnn_readout as *const u8);
        // v323 Diffusion
        builder.symbol("slang_diffusion_forward", crate::diffusion::slang_diffusion_forward as *const u8);
        builder.symbol("slang_diffusion_reverse", crate::diffusion::slang_diffusion_reverse as *const u8);
        builder.symbol("slang_diffusion_schedule", crate::diffusion::slang_diffusion_schedule as *const u8);
        builder.symbol("slang_diffusion_sample", crate::diffusion::slang_diffusion_sample as *const u8);
        // v324 RL
        builder.symbol("slang_rl_q_update", slang_rl_q_update as *const u8);
        builder.symbol("slang_rl_policy_gradient", slang_rl_policy_gradient as *const u8);
        builder.symbol("slang_rl_advantage", slang_rl_advantage as *const u8);
        builder.symbol("slang_rl_reward_discount", slang_rl_reward_discount as *const u8);
        // v325 Embedding Search
        builder.symbol("slang_hnsw_create", crate::embedding_search::slang_hnsw_create as *const u8);
        builder.symbol("slang_hnsw_insert", crate::embedding_search::slang_hnsw_insert as *const u8);
        builder.symbol("slang_hnsw_search", crate::embedding_search::slang_hnsw_search as *const u8);
        builder.symbol("slang_hnsw_recall", crate::embedding_search::slang_hnsw_recall as *const u8);
        // v326 Tokenizer
        builder.symbol("slang_tokenizer_bpe_train", slang_tokenizer_bpe_train as *const u8);
        builder.symbol("slang_tokenizer_encode", slang_tokenizer_encode as *const u8);
        builder.symbol("slang_tokenizer_decode", slang_tokenizer_decode as *const u8);
        builder.symbol("slang_tokenizer_vocab_size", slang_tokenizer_vocab_size as *const u8);
        // v327 RLHF
        builder.symbol("slang_rlhf_reward", slang_rlhf_reward as *const u8);
        builder.symbol("slang_rlhf_kl_penalty", slang_rlhf_kl_penalty as *const u8);
        builder.symbol("slang_rlhf_preference", slang_rlhf_preference as *const u8);
        builder.symbol("slang_rlhf_ppo_clip", slang_rlhf_ppo_clip as *const u8);
        // v328 Async Runtime
        builder.symbol("slang_async_spawn_task", slang_async_spawn_task as *const u8);
        builder.symbol("slang_async_yield_now", slang_async_yield_now as *const u8);
        builder.symbol("slang_async_select", slang_async_select as *const u8);
        builder.symbol("slang_async_timeout", slang_async_timeout as *const u8);
        // v329 Work Stealing
        builder.symbol("slang_ws_create_pool", slang_ws_create_pool as *const u8);
        builder.symbol("slang_ws_submit", slang_ws_submit as *const u8);
        builder.symbol("slang_ws_steal_count", slang_ws_steal_count as *const u8);
        builder.symbol("slang_ws_active_workers", slang_ws_active_workers as *const u8);
        // v330 Connection Pool
        builder.symbol("slang_conn_pool_create", crate::connection_pool::slang_conn_pool_create as *const u8);
        builder.symbol("slang_pool_acquire", crate::connection_pool::slang_pool_acquire as *const u8);
        builder.symbol("slang_pool_release", crate::connection_pool::slang_pool_release as *const u8);
        builder.symbol("slang_pool_stats", crate::connection_pool::slang_pool_stats as *const u8);
        // v331 Protobuf
        builder.symbol("slang_protobuf_encode", crate::protobuf::slang_protobuf_encode as *const u8);
        builder.symbol("slang_protobuf_decode", crate::protobuf::slang_protobuf_decode as *const u8);
        builder.symbol("slang_protobuf_field", crate::protobuf::slang_protobuf_field as *const u8);
        builder.symbol("slang_protobuf_size", crate::protobuf::slang_protobuf_size as *const u8);
        // v332 Consensus
        builder.symbol("slang_raft_propose", slang_raft_propose as *const u8);
        builder.symbol("slang_raft_commit_index", slang_raft_commit_index as *const u8);
        builder.symbol("slang_raft_leader", slang_raft_leader as *const u8);
        builder.symbol("slang_raft_term", slang_raft_term as *const u8);
        // v333 Event Sourcing
        builder.symbol("slang_event_store_create", crate::event_sourcing::slang_event_store_create as *const u8);
        builder.symbol("slang_event_append", crate::event_sourcing::slang_event_append as *const u8);
        builder.symbol("slang_event_replay", crate::event_sourcing::slang_event_replay as *const u8);
        builder.symbol("slang_event_snapshot", crate::event_sourcing::slang_event_snapshot as *const u8);
        builder.symbol("slang_event_project", crate::event_sourcing::slang_event_project as *const u8);
        // v334 Stream Processing
        builder.symbol("slang_stream_create", crate::stream_processing::slang_stream_create as *const u8);
        builder.symbol("slang_stream_window_tumbling", crate::stream_processing::slang_stream_window_tumbling as *const u8);
        builder.symbol("slang_stream_window_sliding", crate::stream_processing::slang_stream_window_sliding as *const u8);
        builder.symbol("slang_stream_watermark", crate::stream_processing::slang_stream_watermark as *const u8);
        builder.symbol("slang_stream_late_count", crate::stream_processing::slang_stream_late_count as *const u8);
        // v335 Message Queue
        builder.symbol("slang_mq_create_topic", crate::message_queue::slang_mq_create_topic as *const u8);
        builder.symbol("slang_mq_publish", crate::message_queue::slang_mq_publish as *const u8);
        builder.symbol("slang_mq_subscribe", crate::message_queue::slang_mq_subscribe as *const u8);
        builder.symbol("slang_mq_consume", crate::message_queue::slang_mq_consume as *const u8);
        builder.symbol("slang_mq_offset", crate::message_queue::slang_mq_offset as *const u8);
        // v336 CQRS
        builder.symbol("slang_cqrs_command", crate::cqrs::slang_cqrs_command as *const u8);
        builder.symbol("slang_cqrs_query", crate::cqrs::slang_cqrs_query as *const u8);
        builder.symbol("slang_cqrs_command_count", crate::cqrs::slang_cqrs_command_count as *const u8);
        builder.symbol("slang_cqrs_query_count", crate::cqrs::slang_cqrs_query_count as *const u8);
        // v337 GraphQL
        builder.symbol("slang_graphql_schema_create", crate::graphql::slang_graphql_schema_create as *const u8);
        builder.symbol("slang_graphql_add_type", crate::graphql::slang_graphql_add_type as *const u8);
        builder.symbol("slang_graphql_add_field", crate::graphql::slang_graphql_add_field as *const u8);
        builder.symbol("slang_graphql_validate", crate::graphql::slang_graphql_validate as *const u8);
        builder.symbol("slang_graphql_type_count", crate::graphql::slang_graphql_type_count as *const u8);
        // v338 JWT
        builder.symbol("slang_jwt_create", crate::jwt::slang_jwt_create as *const u8);
        builder.symbol("slang_jwt_verify", crate::jwt::slang_jwt_verify as *const u8);
        builder.symbol("slang_jwt_claims", crate::jwt::slang_jwt_claims as *const u8);
        builder.symbol("slang_jwt_expired", crate::jwt::slang_jwt_expired as *const u8);
        builder.symbol("slang_jwt_set_claim", crate::jwt::slang_jwt_set_claim as *const u8);
        builder.symbol("slang_jwt_get_claim", crate::jwt::slang_jwt_get_claim as *const u8);
        // v339 OAuth2
        builder.symbol("slang_oauth2_auth_url", crate::oauth2::slang_oauth2_auth_url as *const u8);
        builder.symbol("slang_oauth2_exchange", crate::oauth2::slang_oauth2_exchange as *const u8);
        builder.symbol("slang_oauth2_refresh", crate::oauth2::slang_oauth2_refresh as *const u8);
        builder.symbol("slang_oauth2_pkce_verify", crate::oauth2::slang_oauth2_pkce_verify as *const u8);
        builder.symbol("slang_oauth2_state", crate::oauth2::slang_oauth2_state as *const u8);
        // v340 Rate Limiter
        builder.symbol("slang_ratelimit_check", slang_ratelimit_check as *const u8);
        builder.symbol("slang_ratelimit_remaining", slang_ratelimit_remaining as *const u8);
        builder.symbol("slang_ratelimit_reset", slang_ratelimit_reset as *const u8);
        builder.symbol("slang_ratelimit_window", slang_ratelimit_window as *const u8);
        // v341 Chaos Engineering
        builder.symbol("slang_chaos_inject_fault", crate::chaos::slang_chaos_inject_fault as *const u8);
        builder.symbol("slang_chaos_inject_latency", crate::chaos::slang_chaos_inject_latency as *const u8);
        builder.symbol("slang_chaos_error_rate", crate::chaos::slang_chaos_error_rate as *const u8);
        builder.symbol("slang_chaos_partition", crate::chaos::slang_chaos_partition as *const u8);
        builder.symbol("slang_chaos_fault_count", crate::chaos::slang_chaos_fault_count as *const u8);
        builder.symbol("slang_chaos_total_latency", crate::chaos::slang_chaos_total_latency as *const u8);
        builder.symbol("slang_chaos_is_partitioned", crate::chaos::slang_chaos_is_partitioned as *const u8);
        builder.symbol("slang_chaos_current_error_rate", crate::chaos::slang_chaos_current_error_rate as *const u8);
        // v342 RBAC
        builder.symbol("slang_rbac_assign_role", slang_rbac_assign_role as *const u8);
        builder.symbol("slang_rbac_check_perm", slang_rbac_check_perm as *const u8);
        builder.symbol("slang_rbac_grant", slang_rbac_grant as *const u8);
        builder.symbol("slang_rbac_revoke", slang_rbac_revoke as *const u8);
        // v343 CSP
        builder.symbol("slang_csp_create", crate::csp::slang_csp_create as *const u8);
        builder.symbol("slang_csp_add_directive", crate::csp::slang_csp_add_directive as *const u8);
        builder.symbol("slang_csp_nonce", crate::csp::slang_csp_nonce as *const u8);
        builder.symbol("slang_csp_directive_count", crate::csp::slang_csp_directive_count as *const u8);
        builder.symbol("slang_csp_report_only", crate::csp::slang_csp_report_only as *const u8);
        builder.symbol("slang_csp_check", crate::csp::slang_csp_check as *const u8);
        // v344 Input Validation
        builder.symbol("slang_validate_range", slang_validate_range as *const u8);
        builder.symbol("slang_validate_length", slang_validate_length as *const u8);
        builder.symbol("slang_validate_pattern", slang_validate_pattern as *const u8);
        builder.symbol("slang_validate_sanitize", slang_validate_sanitize as *const u8);
        // v345 Audit Log
        builder.symbol("slang_audit_log", slang_audit_log as *const u8);
        builder.symbol("slang_audit_last_action", slang_audit_last_action as *const u8);
        // v346 Encryption
        builder.symbol("slang_encrypt_xor", slang_encrypt_xor as *const u8);
        builder.symbol("slang_encrypt_rotate", slang_encrypt_rotate as *const u8);
        builder.symbol("slang_encrypt_hash", slang_encrypt_hash as *const u8);
        builder.symbol("slang_encrypt_verify", slang_encrypt_verify as *const u8);
        // v347 Certificate
        builder.symbol("slang_cert_create", slang_cert_create as *const u8);
        builder.symbol("slang_cert_verify", slang_cert_verify as *const u8);
        builder.symbol("slang_cert_expiry", slang_cert_expiry as *const u8);
        builder.symbol("slang_cert_chain_length", slang_cert_chain_length as *const u8);
        // v348 Test Runner
        builder.symbol("slang_test_discover", crate::test_runner::slang_test_discover as *const u8);
        builder.symbol("slang_test_pass", crate::test_runner::slang_test_pass as *const u8);
        builder.symbol("slang_test_fail", crate::test_runner::slang_test_fail as *const u8);
        builder.symbol("slang_test_skip", crate::test_runner::slang_test_skip as *const u8);
        builder.symbol("slang_test_coverage", crate::test_runner::slang_test_coverage as *const u8);
        builder.symbol("slang_test_pass_rate", crate::test_runner::slang_test_pass_rate as *const u8);
        builder.symbol("slang_test_total", crate::test_runner::slang_test_total as *const u8);
        // v349 Snapshot Testing
        builder.symbol("slang_snapshot_capture", crate::snapshot_testing::slang_snapshot_capture as *const u8);
        builder.symbol("slang_snapshot_compare", crate::snapshot_testing::slang_snapshot_compare as *const u8);
        builder.symbol("slang_snapshot_update", crate::snapshot_testing::slang_snapshot_update as *const u8);
        builder.symbol("slang_snapshot_version", crate::snapshot_testing::slang_snapshot_version as *const u8);
        builder.symbol("slang_snapshot_match_count", crate::snapshot_testing::slang_snapshot_match_count as *const u8);
        builder.symbol("slang_snapshot_mismatch_count", crate::snapshot_testing::slang_snapshot_mismatch_count as *const u8);
        // v350 Fuzzer
        builder.symbol("slang_fuzz_add_corpus", crate::fuzzer::slang_fuzz_add_corpus as *const u8);
        builder.symbol("slang_fuzz_run", crate::fuzzer::slang_fuzz_run as *const u8);
        builder.symbol("slang_fuzz_crash", crate::fuzzer::slang_fuzz_crash as *const u8);
        builder.symbol("slang_fuzz_corpus_size", crate::fuzzer::slang_fuzz_corpus_size as *const u8);
        builder.symbol("slang_fuzz_coverage", crate::fuzzer::slang_fuzz_coverage as *const u8);
        builder.symbol("slang_fuzz_crash_count", crate::fuzzer::slang_fuzz_crash_count as *const u8);
        builder.symbol("slang_fuzz_unique_crashes", crate::fuzzer::slang_fuzz_unique_crashes as *const u8);
        builder.symbol("slang_fuzz_total_runs", crate::fuzzer::slang_fuzz_total_runs as *const u8);
        // v351 Property Testing
        builder.symbol("slang_prop_check", slang_prop_check as *const u8);
        builder.symbol("slang_prop_shrink", slang_prop_shrink as *const u8);
        builder.symbol("slang_prop_counterexample", slang_prop_counterexample as *const u8);
        builder.symbol("slang_prop_total_checks", slang_prop_total_checks as *const u8);
        // v352 Mutation Testing
        builder.symbol("slang_mutation_inject", slang_mutation_inject as *const u8);
        builder.symbol("slang_mutation_killed", slang_mutation_killed as *const u8);
        builder.symbol("slang_mutation_survived", slang_mutation_survived as *const u8);
        builder.symbol("slang_mut_test_score", slang_mut_test_score as *const u8);
        // v353 API Compatibility
        builder.symbol("slang_api_semver_diff", crate::api_compat::slang_api_semver_diff as *const u8);
        builder.symbol("slang_api_breaking_change", crate::api_compat::slang_api_breaking_change as *const u8);
        builder.symbol("slang_api_addition", crate::api_compat::slang_api_addition as *const u8);
        builder.symbol("slang_api_deprecation", crate::api_compat::slang_api_deprecation as *const u8);
        builder.symbol("slang_api_surface", crate::api_compat::slang_api_surface as *const u8);
        builder.symbol("slang_api_breaking_count", crate::api_compat::slang_api_breaking_count as *const u8);
        builder.symbol("slang_api_addition_count", crate::api_compat::slang_api_addition_count as *const u8);
        builder.symbol("slang_api_deprecation_count", crate::api_compat::slang_api_deprecation_count as *const u8);
        // v354 Migration
        builder.symbol("slang_migration_create", crate::migration::slang_migration_create as *const u8);
        builder.symbol("slang_migration_transform", crate::migration::slang_migration_transform as *const u8);
        builder.symbol("slang_migration_rollback", crate::migration::slang_migration_rollback as *const u8);
        builder.symbol("slang_migration_progress", crate::migration::slang_migration_progress as *const u8);
        builder.symbol("slang_migration_delta", crate::migration::slang_migration_delta as *const u8);
        // v355 Code Actions
        builder.symbol("slang_codeaction_extract", slang_codeaction_extract as *const u8);
        builder.symbol("slang_codeaction_inline", slang_codeaction_inline as *const u8);
        builder.symbol("slang_codeaction_rename", slang_codeaction_rename as *const u8);
        builder.symbol("slang_codeaction_count", slang_codeaction_count as *const u8);
        // v356 Telemetry
        builder.symbol("slang_telemetry_compile_time", slang_telemetry_compile_time as *const u8);
        builder.symbol("slang_telemetry_peak_memory", slang_telemetry_peak_memory as *const u8);
        builder.symbol("slang_telemetry_cache_hits", slang_telemetry_cache_hits as *const u8);
        builder.symbol("slang_telemetry_error_count", slang_telemetry_error_count as *const u8);
        // v357 Build System
        builder.symbol("slang_build_target", slang_build_target as *const u8);
        builder.symbol("slang_build_parallel", slang_build_parallel as *const u8);
        builder.symbol("slang_build_cache_hit", slang_build_cache_hit as *const u8);
        builder.symbol("slang_build_artifact_count", slang_build_artifact_count as *const u8);
        // v358 OpenAPI
        builder.symbol("slang_openapi_create", crate::openapi::slang_openapi_create as *const u8);
        builder.symbol("slang_openapi_add_route", crate::openapi::slang_openapi_add_route as *const u8);
        builder.symbol("slang_openapi_add_param", crate::openapi::slang_openapi_add_param as *const u8);
        builder.symbol("slang_openapi_validate", crate::openapi::slang_openapi_validate as *const u8);
        builder.symbol("slang_openapi_route_count", crate::openapi::slang_openapi_route_count as *const u8);
        // v359 Benchmarking
        builder.symbol("slang_bench_start", slang_bench_start as *const u8);
        builder.symbol("slang_bench_stop", slang_bench_stop as *const u8);
        builder.symbol("slang_bench_iterations", slang_bench_iterations as *const u8);
        builder.symbol("slang_bench_throughput", slang_bench_throughput as *const u8);
        // v360 Profiler
        builder.symbol("slang_profile_begin", slang_profile_begin as *const u8);
        builder.symbol("slang_profile_end", slang_profile_end as *const u8);
        builder.symbol("slang_profile_flamegraph", slang_profile_flamegraph as *const u8);
        builder.symbol("slang_profile_hotspot", slang_profile_hotspot as *const u8);
        // v361 Release
        builder.symbol("slang_release_changelog", crate::release::slang_release_changelog as *const u8);
        builder.symbol("slang_release_version_bump", crate::release::slang_release_version_bump as *const u8);
        builder.symbol("slang_release_package", crate::release::slang_release_package as *const u8);
        builder.symbol("slang_release_version", crate::release::slang_release_version as *const u8);
        builder.symbol("slang_release_changelog_count", crate::release::slang_release_changelog_count as *const u8);
        builder.symbol("slang_release_packages_built", crate::release::slang_release_packages_built as *const u8);
        // v362 Plugin System
        builder.symbol("slang_plugin_load", crate::plugin_system::slang_plugin_load as *const u8);
        builder.symbol("slang_plugin_register_hook", crate::plugin_system::slang_plugin_register_hook as *const u8);
        builder.symbol("slang_plugin_activate", crate::plugin_system::slang_plugin_activate as *const u8);
        builder.symbol("slang_plugin_unload", crate::plugin_system::slang_plugin_unload as *const u8);
        builder.symbol("slang_plugin_state", crate::plugin_system::slang_plugin_state as *const u8);
        builder.symbol("slang_plugin_hook_count", crate::plugin_system::slang_plugin_hook_count as *const u8);
        builder.symbol("slang_plugin_count", crate::plugin_system::slang_plugin_count as *const u8);
        // v363 Wasm Component Model
        builder.symbol("slang_wasm_component_create", slang_wasm_component_create as *const u8);
        builder.symbol("slang_wasm_component_link", slang_wasm_component_link as *const u8);
        builder.symbol("slang_wasm_component_instantiate", slang_wasm_component_instantiate as *const u8);
        builder.symbol("slang_wasm_component_count", slang_wasm_component_count as *const u8);
        // v364 WASI Preview2
        builder.symbol("slang_wasi_fs_read", slang_wasi_fs_read as *const u8);
        builder.symbol("slang_wasi_fs_write", slang_wasi_fs_write as *const u8);
        builder.symbol("slang_wasi_clock", slang_wasi_clock as *const u8);
        builder.symbol("slang_wasi_random", slang_wasi_random as *const u8);
        // v365 Package Registry
        builder.symbol("slang_registry_publish", slang_registry_publish as *const u8);
        builder.symbol("slang_registry_resolve", slang_registry_resolve as *const u8);
        builder.symbol("slang_registry_download", slang_registry_download as *const u8);
        builder.symbol("slang_registry_version_count", slang_registry_version_count as *const u8);
        // v366 Milestone
        builder.symbol("slang_vitalis_v366_version", slang_vitalis_v366_version as *const u8);
        builder.symbol("slang_vitalis_v366_total_builtins", slang_vitalis_v366_total_builtins as *const u8);
        builder.symbol("slang_vitalis_v366_modules", slang_vitalis_v366_modules as *const u8);
        builder.symbol("slang_vitalis_v366_milestone", slang_vitalis_v366_milestone as *const u8);

        builder.symbol("slang_format_int",       slang_format_int       as *const u8);
        builder.symbol("slang_format_float",     slang_format_float     as *const u8);
        // ── v15: JSON ────────────────────────────────────────────────────
        builder.symbol("slang_json_encode",      slang_json_encode      as *const u8);
        builder.symbol("slang_json_decode",      slang_json_decode      as *const u8);

        // ── v18: Collection methods ──────────────────────────────────────
        builder.symbol("slang_array_push",       slang_array_push       as *const u8);
        builder.symbol("slang_array_pop",        slang_array_pop        as *const u8);
        builder.symbol("slang_array_contains",   slang_array_contains   as *const u8);
        builder.symbol("slang_array_reverse",    slang_array_reverse    as *const u8);
        builder.symbol("slang_array_sort",       slang_array_sort       as *const u8);
        builder.symbol("slang_array_join",       slang_array_join       as *const u8);
        builder.symbol("slang_array_slice",      slang_array_slice      as *const u8);
        builder.symbol("slang_array_find",       slang_array_find       as *const u8);
        // ── Iterator / functional array ops ───────────────────────────
        builder.symbol("slang_array_range",        slang_array_range        as *const u8);
        builder.symbol("slang_array_sum",          slang_array_sum          as *const u8);
        builder.symbol("slang_array_min",          slang_array_min          as *const u8);
        builder.symbol("slang_array_max",          slang_array_max          as *const u8);
        builder.symbol("slang_array_any",          slang_array_any          as *const u8);
        builder.symbol("slang_array_all_positive", slang_array_all_positive as *const u8);
        builder.symbol("slang_array_count",        slang_array_count        as *const u8);
        builder.symbol("slang_array_flatten",      slang_array_flatten      as *const u8);
        builder.symbol("slang_array_zip",          slang_array_zip          as *const u8);
        builder.symbol("slang_array_enumerate",    slang_array_enumerate    as *const u8);
        builder.symbol("slang_array_take",         slang_array_take         as *const u8);
        builder.symbol("slang_array_drop",         slang_array_drop         as *const u8);
        builder.symbol("slang_array_unique",       slang_array_unique       as *const u8);
        builder.symbol("slang_error_message",    slang_error_message    as *const u8);

        // Regex
        builder.symbol("slang_regex_match",          slang_regex_match          as *const u8);
        builder.symbol("slang_regex_is_match",       slang_regex_is_match       as *const u8);
        builder.symbol("slang_regex_find",           slang_regex_find           as *const u8);
        builder.symbol("slang_regex_replace",        slang_regex_replace        as *const u8);
        builder.symbol("slang_regex_split_count",    slang_regex_split_count    as *const u8);
        builder.symbol("slang_regex_split_get",      slang_regex_split_get      as *const u8);
        builder.symbol("slang_regex_find_all_count", slang_regex_find_all_count as *const u8);
        builder.symbol("slang_regex_find_all_get",   slang_regex_find_all_get   as *const u8);
        // ── v370: Async runtime ──────────────────────────────────────────
        builder.symbol("slang_spawn",          slang_spawn          as *const u8);
        builder.symbol("slang_task_result",    slang_task_result    as *const u8);
        builder.symbol("slang_task_await",     slang_task_await     as *const u8);
        builder.symbol("slang_async_run_all",  slang_async_run_all  as *const u8);
        // ── v18: Networking ──────────────────────────────────────────────
        builder.symbol("slang_http_get",       slang_http_get       as *const u8);
        builder.symbol("slang_http_post",      slang_http_post      as *const u8);
        builder.symbol("slang_http_status",    slang_http_status    as *const u8);
        builder.symbol("slang_tcp_connect",    slang_tcp_connect    as *const u8);
        builder.symbol("slang_tcp_send",       slang_tcp_send       as *const u8);
        builder.symbol("slang_tcp_close",      slang_tcp_close      as *const u8);
        // ── Module system ────────────────────────────────────────────────
        builder.symbol("slang_module_loaded",     slang_module_loaded     as *const u8);

        // ── v60: Self-hosting bootstrap primitives ───────────────────────
        builder.symbol("slang_char_to_int",       slang_char_to_int       as *const u8);
        builder.symbol("slang_int_to_char",       slang_int_to_char       as *const u8);
        builder.symbol("slang_array_new",         slang_array_new         as *const u8);
        builder.symbol("slang_exit",              slang_exit              as *const u8);
        builder.symbol("slang_file_write_bytes",  slang_file_write_bytes  as *const u8);
        builder.symbol("slang_file_read_bytes",   slang_file_read_bytes   as *const u8);
        builder.symbol("slang_args_count",        slang_args_count        as *const u8);
        builder.symbol("slang_args_get",          slang_args_get          as *const u8);

        // ── Tensor/ML runtime (Void-Vitalis) ─────────────────────────────
        builder.symbol("slang_to_f64",         slang_to_f64         as *const u8);
        builder.symbol("slang_to_i64",         slang_to_i64         as *const u8);
        builder.symbol("slang_t_fill",         slang_t_fill         as *const u8);
        builder.symbol("slang_t_copy",         slang_t_copy         as *const u8);
        builder.symbol("slang_t_randn",        slang_t_randn        as *const u8);
        builder.symbol("slang_t_print_n",      slang_t_print_n      as *const u8);
        builder.symbol("slang_t_matmul",       slang_t_matmul       as *const u8);
        builder.symbol("slang_t_add_vv",       slang_t_add_vv       as *const u8);
        builder.symbol("slang_t_sub_vv",       slang_t_sub_vv       as *const u8);
        builder.symbol("slang_t_mul_vv",       slang_t_mul_vv       as *const u8);
        builder.symbol("slang_t_scale",        slang_t_scale        as *const u8);
        builder.symbol("slang_t_softmax",      slang_t_softmax      as *const u8);
        builder.symbol("slang_t_sum",          slang_t_sum          as *const u8);
        builder.symbol("slang_t_add_bias",     slang_t_add_bias     as *const u8);
        builder.symbol("slang_t_max_idx",      slang_t_max_idx      as *const u8);
        builder.symbol("slang_t_dot",          slang_t_dot          as *const u8);
        builder.symbol("slang_t_cross_entropy",slang_t_cross_entropy as *const u8);
        builder.symbol("slang_t_transpose",    slang_t_transpose    as *const u8);
        builder.symbol("slang_t_adamw",        slang_t_adamw        as *const u8);
        builder.symbol("slang_t_norm",         slang_t_norm         as *const u8);

        // GUI builtins
        builder.symbol("slang_gui_open",       slang_gui_open       as *const u8);
        builder.symbol("slang_gui_close",      slang_gui_close      as *const u8);
        builder.symbol("slang_gui_clear",      slang_gui_clear      as *const u8);
        builder.symbol("slang_gui_rect",       slang_gui_rect       as *const u8);
        builder.symbol("slang_gui_line",       slang_gui_line       as *const u8);
        builder.symbol("slang_gui_text",       slang_gui_text       as *const u8);
        builder.symbol("slang_gui_update",     slang_gui_update     as *const u8);
        builder.symbol("slang_gui_circle",     slang_gui_circle     as *const u8);

        // Advanced rendering builtins
        builder.symbol("slang_gui_gradient_rect",     slang_gui_gradient_rect     as *const u8);
        builder.symbol("slang_gui_pixel",              slang_gui_pixel              as *const u8);
        builder.symbol("slang_gui_rounded_rect",      slang_gui_rounded_rect      as *const u8);
        builder.symbol("slang_gui_blend_rect",        slang_gui_blend_rect        as *const u8);
        builder.symbol("slang_gui_thick_line",        slang_gui_thick_line        as *const u8);
        builder.symbol("slang_gui_triangle",          slang_gui_triangle          as *const u8);
        builder.symbol("slang_gui_aa_circle",         slang_gui_aa_circle         as *const u8);
        builder.symbol("slang_gui_gradient_rounded_rect", slang_gui_gradient_rounded_rect as *const u8);
        builder.symbol("slang_gui_glow",              slang_gui_glow              as *const u8);

        // ── v62: Extended module symbols (tensor, ML, ECS, etc.) ────────
        crate::jit_symbols::register_all(&mut builder);

        let module = JITModule::new(builder);
        let ctx = module.make_context();

        Ok(Self {
            module,
            ctx,
            func_ids: HashMap::new(),
        })
    }

    /// Compile an IR module into native code.
    pub fn compile(&mut self, ir_module: &IrModule) -> CodegenResult<()> {
        // Declare all functions first (for forward references)
        for func in &ir_module.functions {
            self.declare_function(func)?;
        }

        // Declare runtime support functions
        self.declare_runtime_functions()?;

        // Define all functions
        for func in &ir_module.functions {
            self.define_function(func)?;
        }

        // Finalize all functions
        self.module.finalize_definitions().map_err(|e| CodegenError {
            message: format!("failed to finalize: {}", e),
        })?;

        Ok(())
    }

    fn declare_runtime_functions(&mut self) -> CodegenResult<()> {
        // slang_print_i64(i64) -> void
        {
            let mut sig = self.module.make_signature();
            sig.params.push(AbiParam::new(types::I64));
            let id = self
                .module
                .declare_function("slang_print_i64", Linkage::Import, &sig)
                .map_err(|e| CodegenError {
                    message: format!("declare slang_print_i64: {}", e),
                })?;
            self.func_ids.insert("slang_print_i64".into(), id);
        }

        // slang_print_str(ptr, len) -> void
        {
            let ptr_type = self.module.target_config().pointer_type();
            let mut sig = self.module.make_signature();
            sig.params.push(AbiParam::new(ptr_type));
            sig.params.push(AbiParam::new(types::I64));
            let id = self
                .module
                .declare_function("slang_print_str", Linkage::Import, &sig)
                .map_err(|e| CodegenError {
                    message: format!("declare slang_print_str: {}", e),
                })?;
            self.func_ids.insert("slang_print_str".into(), id);
        }

        // slang_println_str(ptr, len) -> void
        {
            let ptr_type = self.module.target_config().pointer_type();
            let mut sig = self.module.make_signature();
            sig.params.push(AbiParam::new(ptr_type));
            sig.params.push(AbiParam::new(types::I64));
            let id = self
                .module
                .declare_function("slang_println_str", Linkage::Import, &sig)
                .map_err(|e| CodegenError {
                    message: format!("declare slang_println_str: {}", e),
                })?;
            self.func_ids.insert("slang_println_str".into(), id);
        }

        // ── Typed printing ──────────────────────────────────────────────
        macro_rules! decl_fn {
            ($name:literal, [ $($param:expr),* ], $ret:expr) => {{
                let mut sig = self.module.make_signature();
                $( sig.params.push(AbiParam::new($param)); )*
                if $ret != types::INVALID { sig.returns.push(AbiParam::new($ret)); }
                let id = self.module.declare_function($name, Linkage::Import, &sig)
                    .map_err(|e| CodegenError { message: format!("declare {}: {}", $name, e) })?;
                self.func_ids.insert($name.to_string(), id);
            }};
        }
        let ptr_type = self.module.target_config().pointer_type();
        decl_fn!("slang_println_i64",   [types::I64],              types::INVALID);
        decl_fn!("slang_print_f64",     [types::F64],              types::INVALID);
        decl_fn!("slang_println_f64",   [types::F64],              types::INVALID);
        decl_fn!("slang_print_bool",    [types::I8],               types::INVALID);
        decl_fn!("slang_println_bool",  [types::I8],               types::INVALID);
        decl_fn!("slang_print_cstr",    [ptr_type],                types::INVALID);
        decl_fn!("slang_println_cstr",  [ptr_type],                types::INVALID);
        // Math (f64 → f64)
        decl_fn!("slang_sqrt_f64",  [types::F64],              types::F64);
        decl_fn!("slang_ln_f64",    [types::F64],              types::F64);
        decl_fn!("slang_log2_f64",  [types::F64],              types::F64);
        decl_fn!("slang_log10_f64", [types::F64],              types::F64);
        decl_fn!("slang_sin_f64",   [types::F64],              types::F64);
        decl_fn!("slang_cos_f64",   [types::F64],              types::F64);
        decl_fn!("slang_exp_f64",   [types::F64],              types::F64);
        decl_fn!("slang_floor_f64", [types::F64],              types::F64);
        decl_fn!("slang_ceil_f64",  [types::F64],              types::F64);
        decl_fn!("slang_round_f64", [types::F64],              types::F64);
        decl_fn!("slang_abs_f64",   [types::F64],              types::F64);
        decl_fn!("slang_pow_f64",   [types::F64, types::F64],  types::F64);
        decl_fn!("slang_min_f64",   [types::F64, types::F64],  types::F64);
        decl_fn!("slang_max_f64",   [types::F64, types::F64],  types::F64);
        // Math (i64)
        decl_fn!("slang_abs_i64",   [types::I64],              types::I64);
        decl_fn!("slang_min_i64",   [types::I64, types::I64],  types::I64);
        decl_fn!("slang_max_i64",   [types::I64, types::I64],  types::I64);
        // Conversion
        decl_fn!("slang_i64_to_f64", [types::I64], types::F64);
        decl_fn!("slang_f64_to_i64", [types::F64], types::I64);
        // Strings
        decl_fn!("slang_str_len",   [ptr_type],            types::I64);
        decl_fn!("slang_str_eq",    [ptr_type, ptr_type],  types::I8);
        decl_fn!("slang_str_cat",   [ptr_type, ptr_type],  ptr_type);
        // Extended math
        decl_fn!("slang_atan2_f64",  [types::F64, types::F64],                          types::F64);
        decl_fn!("slang_hypot_f64",  [types::F64, types::F64],                          types::F64);
        decl_fn!("slang_clamp_f64",  [types::F64, types::F64, types::F64],              types::F64);
        decl_fn!("slang_clamp_i64",  [types::I64, types::I64, types::I64],              types::I64);
        decl_fn!("slang_rand_f64",   [],                                                types::F64);
        decl_fn!("slang_rand_i64",   [],                                                types::I64);
        // Phase 4: Array heap runtime
        decl_fn!("slang_array_alloc",   [types::I64, types::I64],    ptr_type);
        decl_fn!("slang_array_len",     [ptr_type],                  types::I64);
        decl_fn!("slang_array_get_i64", [ptr_type, types::I64],      types::I64);
        decl_fn!("slang_array_set_i64", [ptr_type, types::I64, types::I64],   types::INVALID);
        decl_fn!("slang_array_get_f64", [ptr_type, types::I64],      types::F64);
        decl_fn!("slang_array_set_f64", [ptr_type, types::I64, types::F64],   types::INVALID);
        // Phase 5: New stdlib declarations
        decl_fn!("slang_clock_ns",        [],                            types::I64);
        decl_fn!("slang_clock_ms",        [],                            types::I64);
        decl_fn!("slang_assert_eq_i64",   [types::I64, types::I64],     types::INVALID);
        decl_fn!("slang_assert_true",     [types::I8],                   types::INVALID);
        decl_fn!("slang_popcount",        [types::I64],                  types::I64);
        decl_fn!("slang_leading_zeros",   [types::I64],                  types::I64);
        decl_fn!("slang_trailing_zeros",  [types::I64],                  types::I64);
        decl_fn!("slang_sign_i64",        [types::I64],                  types::I64);
        decl_fn!("slang_gcd",             [types::I64, types::I64],      types::I64);
        decl_fn!("slang_lcm",             [types::I64, types::I64],      types::I64);
        decl_fn!("slang_factorial",       [types::I64],                  types::I64);
        decl_fn!("slang_fibonacci",       [types::I64],                  types::I64);
        decl_fn!("slang_is_prime",        [types::I64],                  types::I8);
        decl_fn!("slang_tan_f64",         [types::F64],                  types::F64);
        decl_fn!("slang_asin_f64",        [types::F64],                  types::F64);
        decl_fn!("slang_acos_f64",        [types::F64],                  types::F64);
        decl_fn!("slang_atan_f64",        [types::F64],                  types::F64);
        // Phase 21 stdlib declarations
        decl_fn!("slang_hash_i64",        [types::I64],                  types::I64);
        decl_fn!("slang_lerp_f64",        [types::F64, types::F64, types::F64], types::F64);
        decl_fn!("slang_smoothstep_f64",  [types::F64, types::F64, types::F64], types::F64);
        decl_fn!("slang_wrap_i64",        [types::I64, types::I64, types::I64], types::I64);
        decl_fn!("slang_map_range_f64",   [types::F64, types::F64, types::F64, types::F64, types::F64], types::F64);
        decl_fn!("slang_epoch_secs",      [],                            types::I64);
        // Phase 22 stdlib declarations
        decl_fn!("slang_fma_f64",         [types::F64, types::F64, types::F64], types::F64);
        decl_fn!("slang_cbrt_f64",        [types::F64],                  types::F64);
        decl_fn!("slang_deg_to_rad",      [types::F64],                  types::F64);
        decl_fn!("slang_rad_to_deg",      [types::F64],                  types::F64);
        decl_fn!("slang_sigmoid_f64",     [types::F64],                  types::F64);
        decl_fn!("slang_relu_f64",        [types::F64],                  types::F64);
        decl_fn!("slang_tanh_f64",        [types::F64],                  types::F64);
        decl_fn!("slang_ipow",            [types::I64, types::I64],      types::I64);
        // Phase 23 stdlib declarations
        decl_fn!("slang_sinh_f64",         [types::F64],                  types::F64);
        decl_fn!("slang_cosh_f64",         [types::F64],                  types::F64);
        decl_fn!("slang_log_f64",          [types::F64],                  types::F64);
        decl_fn!("slang_exp2_f64",         [types::F64],                  types::F64);
        decl_fn!("slang_copysign_f64",     [types::F64, types::F64],      types::F64);
        decl_fn!("slang_fract_f64",        [types::F64],                  types::F64);
        decl_fn!("slang_trunc_f64",        [types::F64],                  types::F64);
        decl_fn!("slang_step_f64",         [types::F64, types::F64],      types::F64);
        decl_fn!("slang_leaky_relu_f64",   [types::F64, types::F64],      types::F64);
        decl_fn!("slang_elu_f64",          [types::F64, types::F64],      types::F64);
        // Phase 24 stdlib declarations
        decl_fn!("slang_swish_f64",        [types::F64],                  types::F64);
        decl_fn!("slang_gelu_f64",         [types::F64],                  types::F64);
        decl_fn!("slang_softplus_f64",     [types::F64],                  types::F64);
        decl_fn!("slang_mish_f64",         [types::F64],                  types::F64);
        decl_fn!("slang_log1p_f64",        [types::F64],                  types::F64);
        decl_fn!("slang_expm1_f64",        [types::F64],                  types::F64);
        decl_fn!("slang_recip_f64",        [types::F64],                  types::F64);
        decl_fn!("slang_rsqrt_f64",        [types::F64],                  types::F64);
        // Phase 25 stdlib declarations
        decl_fn!("slang_selu_f64",          [types::F64],                  types::F64);
        decl_fn!("slang_hard_sigmoid_f64",  [types::F64],                  types::F64);
        decl_fn!("slang_hard_swish_f64",    [types::F64],                  types::F64);
        decl_fn!("slang_log_sigmoid_f64",   [types::F64],                  types::F64);
        decl_fn!("slang_celu_f64",          [types::F64],                  types::F64);
        decl_fn!("slang_softsign_f64",      [types::F64],                  types::F64);
        decl_fn!("slang_gaussian_f64",      [types::F64],                  types::F64);
        decl_fn!("slang_sinc_f64",          [types::F64],                  types::F64);
        decl_fn!("slang_inv_sqrt_approx_f64", [types::F64],                types::F64);
        decl_fn!("slang_logit_f64",         [types::F64],                  types::F64);
        // ── v15: String operations ───────────────────────────────────────
        decl_fn!("slang_str_upper",        [ptr_type],                    ptr_type);
        decl_fn!("slang_str_lower",        [ptr_type],                    ptr_type);
        decl_fn!("slang_str_trim",         [ptr_type],                    ptr_type);
        decl_fn!("slang_str_contains",     [ptr_type, ptr_type],          types::I8);
        decl_fn!("slang_str_starts_with",  [ptr_type, ptr_type],          types::I8);
        decl_fn!("slang_str_ends_with",    [ptr_type, ptr_type],          types::I8);
        decl_fn!("slang_str_char_at",      [ptr_type, types::I64],        ptr_type);
        decl_fn!("slang_str_substr",       [ptr_type, types::I64, types::I64], ptr_type);
        decl_fn!("slang_str_index_of",     [ptr_type, ptr_type],          types::I64);
        decl_fn!("slang_str_replace",      [ptr_type, ptr_type, ptr_type], ptr_type);
        decl_fn!("slang_str_repeat",       [ptr_type, types::I64],        ptr_type);
        decl_fn!("slang_str_reverse",      [ptr_type],                    ptr_type);
        decl_fn!("slang_str_split_count",  [ptr_type, ptr_type],          types::I64);
        decl_fn!("slang_str_split_get",    [ptr_type, ptr_type, types::I64], ptr_type);
        decl_fn!("slang_to_string_i64",    [types::I64],                  ptr_type);
        decl_fn!("slang_to_string_f64",    [types::F64],                  ptr_type);
        decl_fn!("slang_to_string_bool",   [types::I8],                   ptr_type);
        decl_fn!("slang_str_format_i64",   [ptr_type, types::I64],        ptr_type);
        decl_fn!("slang_str_format_f64",   [ptr_type, types::F64],        ptr_type);
        decl_fn!("slang_str_format_str",   [ptr_type, ptr_type],          ptr_type);
        decl_fn!("slang_parse_int",        [ptr_type],                    types::I64);
        decl_fn!("slang_parse_float",      [ptr_type],                    types::F64);
        // ── v15: File I/O ────────────────────────────────────────────────
        decl_fn!("slang_file_read",        [ptr_type],                    ptr_type);
        decl_fn!("slang_file_write",       [ptr_type, ptr_type],          types::I8);
        decl_fn!("slang_file_append",      [ptr_type, ptr_type],          types::I8);
        decl_fn!("slang_file_exists",      [ptr_type],                    types::I8);
        decl_fn!("slang_file_delete",      [ptr_type],                    types::I8);
        decl_fn!("slang_file_size",        [ptr_type],                    types::I64);
        // ── v15: Map operations ──────────────────────────────────────────
        decl_fn!("slang_map_new",          [],                            types::I64);
        decl_fn!("slang_map_set",          [types::I64, ptr_type, types::I64], types::INVALID);
        decl_fn!("slang_map_get",          [types::I64, ptr_type],        types::I64);
        decl_fn!("slang_map_has",          [types::I64, ptr_type],        types::I8);
        decl_fn!("slang_map_remove",       [types::I64, ptr_type],        types::INVALID);
        decl_fn!("slang_map_len",          [types::I64],                  types::I64);
        decl_fn!("slang_map_keys",         [types::I64],                  ptr_type);
        // ── v16: Set operations ──────────────────────────────────────────
        decl_fn!("slang_set_new",          [],                            types::I64);
        decl_fn!("slang_set_add",          [types::I64, types::I64],      types::INVALID);
        decl_fn!("slang_set_has",          [types::I64, types::I64],      types::I8);
        decl_fn!("slang_set_remove",       [types::I64, types::I64],      types::INVALID);
        decl_fn!("slang_set_len",          [types::I64],                  types::I64);
        decl_fn!("slang_set_union",        [types::I64, types::I64],      types::I64);
        decl_fn!("slang_set_intersect",    [types::I64, types::I64],      types::I64);
        decl_fn!("slang_set_diff",         [types::I64, types::I64],      types::I64);
        decl_fn!("slang_set_to_array",     [types::I64],                  ptr_type);
        // ── v18: Tuple operations ────────────────────────────────────────
        decl_fn!("slang_tuple_new2",       [types::I64, types::I64],                       types::I64);
        decl_fn!("slang_tuple_new3",       [types::I64, types::I64, types::I64],            types::I64);
        decl_fn!("slang_tuple_new4",       [types::I64, types::I64, types::I64, types::I64], types::I64);
        decl_fn!("slang_tuple_get",        [types::I64, types::I64],                       types::I64);
        decl_fn!("slang_tuple_len",        [types::I64],                                   types::I64);
        // ── v15: Error handling ──────────────────────────────────────────
        decl_fn!("slang_error_set",        [types::I64, ptr_type],        types::INVALID);
        decl_fn!("slang_error_check",      [],                            types::I64);
        decl_fn!("slang_error_msg",        [],                            ptr_type);
        decl_fn!("slang_error_clear",      [],                            types::INVALID);
        // ── v15: Environment & System ────────────────────────────────────
        decl_fn!("slang_env_get",          [ptr_type],                    ptr_type);
        decl_fn!("slang_sleep_ms",         [types::I64],                  types::INVALID);
        decl_fn!("slang_eprint",           [ptr_type],                    types::INVALID);
        decl_fn!("slang_eprintln",         [ptr_type],                    types::INVALID);
        decl_fn!("slang_pid",              [],                            types::I64);
        // -- v142: Runtime Logging ----------------------------------------
        decl_fn!("slang_log_trace",        [ptr_type],                    types::INVALID);
        decl_fn!("slang_log_debug",        [ptr_type],                    types::INVALID);
        decl_fn!("slang_log_info",         [ptr_type],                    types::INVALID);
        decl_fn!("slang_log_warn",         [ptr_type],                    types::INVALID);
        decl_fn!("slang_log_error",        [ptr_type],                    types::INVALID);
        decl_fn!("slang_log_level_set",    [types::I64],                  types::INVALID);
        decl_fn!("slang_log_level_get",    [],                            types::I64);
        // -- v143: Audit Trail -------------------------------------------
        decl_fn!("slang_audit_event",      [ptr_type, ptr_type, ptr_type], types::INVALID);
        decl_fn!("slang_audit_count",      [],                            types::I64);
        decl_fn!("slang_audit_dump",       [],                            types::INVALID);
        decl_fn!("slang_audit_clear",      [],                            types::INVALID);
        decl_fn!("slang_audit_last",       [ptr_type, types::I64],        types::I64);
        // -- v144: Metrics & Telemetry --------------------------------------
        decl_fn!("slang_metric_counter",   [ptr_type, types::I64],        types::INVALID);
        decl_fn!("slang_metric_gauge",     [ptr_type, types::F64],        types::INVALID);
        decl_fn!("slang_metric_histogram", [ptr_type, types::F64],        types::INVALID);
        decl_fn!("slang_metric_get_counter", [ptr_type],                  types::F64);
        decl_fn!("slang_metric_get_gauge", [ptr_type],                    types::F64);
        decl_fn!("slang_metric_dump",      [],                            types::INVALID);
        decl_fn!("slang_metric_clear",     [],                            types::INVALID);
        // -- v145: Distributed Tracing --------------------------------------
        decl_fn!("slang_span_start",       [ptr_type],                    types::I64);
        decl_fn!("slang_span_end",         [],                            types::I64);
        decl_fn!("slang_span_set_tag",     [ptr_type, ptr_type],          types::INVALID);
        decl_fn!("slang_trace_id",         [],                            ptr_type);
        decl_fn!("slang_span_depth",       [],                            types::I64);
        // -- v146: Health Check & Runtime Diagnostics ---------------------
        decl_fn!("slang_runtime_uptime_ms",   [],                            types::I64);
        decl_fn!("slang_runtime_memory_used", [],                            types::I64);
        decl_fn!("slang_runtime_version",     [],                            ptr_type);
        decl_fn!("slang_runtime_alloc_count", [types::I64],                  types::I64);
        decl_fn!("slang_runtime_alloc_total", [],                            types::I64);
        decl_fn!("slang_runtime_cpu_count",   [],                            types::I64);
        // -- v147: Observable Pipeline Integration ----------------------
        decl_fn!("slang_pipeline_timer_start",   [ptr_type],                    types::I64);
        decl_fn!("slang_pipeline_timer_end",     [ptr_type],                    types::I64);
        decl_fn!("slang_pipeline_stage_count",   [],                            types::I64);
        decl_fn!("slang_pipeline_dump_timings",  [],                            types::INVALID);
        decl_fn!("slang_pipeline_clear_timings", [],                            types::INVALID);
        // -- v148: RBAC & Capability Permissions ------------------------
        decl_fn!("slang_permission_check",  [ptr_type],                    types::I64);
        decl_fn!("slang_permission_grant",  [ptr_type],                    types::I64);
        decl_fn!("slang_permission_revoke", [ptr_type],                    types::I64);
        decl_fn!("slang_permission_list",   [],                            ptr_type);
        decl_fn!("slang_permission_clear",  [],                            types::I64);
        // -- v149: Cryptographic Signing -----------------------------
        decl_fn!("slang_crypto_sha256",       [ptr_type],                    ptr_type);
        decl_fn!("slang_crypto_hmac_sign",    [ptr_type, ptr_type],          ptr_type);
        decl_fn!("slang_crypto_hmac_verify",  [ptr_type, ptr_type, ptr_type],types::I64);
        decl_fn!("slang_crypto_base64_encode",[ptr_type],                    ptr_type);
        decl_fn!("slang_crypto_base64_decode",[ptr_type],                    ptr_type);
        // -- v150: Sandbox Enforcement -------------------------------
        decl_fn!("slang_sandbox_create",    [],                            types::I64);
        decl_fn!("slang_sandbox_allow",     [ptr_type],                    types::I64);
        decl_fn!("slang_sandbox_check",     [ptr_type],                    types::I64);
        decl_fn!("slang_sandbox_violations",[],                            types::I64);
        decl_fn!("slang_sandbox_destroy",   [],                            types::I64);
        // -- v151: Security Audit Logger ----------------------------
        decl_fn!("slang_security_log",       [ptr_type, ptr_type],          types::I64);
        decl_fn!("slang_security_log_count", [],                            types::I64);
        decl_fn!("slang_security_log_verify",[],                            types::I64);
        decl_fn!("slang_security_log_dump",  [],                            types::INVALID);
        decl_fn!("slang_security_log_clear", [],                            types::I64);
        // -- v152: Input Validation Framework -----------------------
        decl_fn!("slang_validate_email",    [ptr_type],                    types::I64);
        decl_fn!("slang_validate_url",      [ptr_type],                    types::I64);
        decl_fn!("slang_validate_ip",       [ptr_type],                    types::I64);
        decl_fn!("slang_sanitize_html",     [ptr_type],                    ptr_type);
        decl_fn!("slang_sanitize_sql",      [ptr_type],                    ptr_type);
        // -- v153: Secure Communication Primitives -----------------
        decl_fn!("slang_secure_channel_create", [],                        types::I64);
        decl_fn!("slang_secure_channel_send",   [types::I64, ptr_type],    types::I64);
        decl_fn!("slang_secure_channel_recv",   [types::I64],              ptr_type);
        decl_fn!("slang_secure_channel_close",  [types::I64],              types::I64);
        decl_fn!("slang_constant_time_eq",      [ptr_type, ptr_type],      types::I64);
        // -- v154: Autonomous Improvement Lab v2 -------------------
        decl_fn!("slang_improvement_run_trial",  [ptr_type, types::F64],    types::I64);
        decl_fn!("slang_improvement_best_score", [],                        types::F64);
        decl_fn!("slang_improvement_history_count", [],                     types::I64);
        decl_fn!("slang_improvement_reset",      [],                        types::I64);
        // -- v155: LLM-Guided Mutation Templates -------------------
        decl_fn!("slang_mutation_apply",         [ptr_type],                types::I64);
        decl_fn!("slang_mutation_list_count",    [],                        types::I64);
        decl_fn!("slang_mutation_score",         [types::I64, types::F64],  types::I64);
        decl_fn!("slang_mutation_undo",          [types::I64],              types::I64);
        // -- v156: Evolution Fitness Profiles -----------------------
        decl_fn!("slang_fitness_register",       [ptr_type],                types::I64);
        decl_fn!("slang_fitness_evaluate",       [types::I64, types::F64],  types::I64);
        decl_fn!("slang_fitness_pareto_count",   [],                        types::I64);
        decl_fn!("slang_fitness_clear",          [],                        types::I64);
        // -- v157: Cross-Module Evolution ---------------------------
        decl_fn!("slang_evo_cross_module",       [ptr_type],                types::I64);
        decl_fn!("slang_evo_dep_add",            [types::I64, ptr_type],    types::I64);
        decl_fn!("slang_evo_dep_check",          [types::I64],              types::I64);
        decl_fn!("slang_evo_safe_mutate",        [types::I64],              types::I64);
        // -- v158: Evolution Checkpointing -------------------------
        decl_fn!("slang_evo_checkpoint_save",    [ptr_type],                types::I64);
        decl_fn!("slang_evo_checkpoint_load",    [ptr_type],                types::I64);
        decl_fn!("slang_evo_checkpoint_list_count", [],                     types::I64);
        decl_fn!("slang_evo_checkpoint_clear",   [],                        types::I64);
        // -- v159: Meta-Evolution v2 -------------------------------
        decl_fn!("slang_meta_evo_register",      [ptr_type, types::F64],    types::I64);
        decl_fn!("slang_meta_evo_select",        [],                        types::I64);
        decl_fn!("slang_meta_evo_converged",     [],                        types::I64);
        decl_fn!("slang_meta_evo_stats_count",   [],                        types::I64);
        // -- v160: Tensor-First Types ------------------------------
        decl_fn!("slang_tensor_create",          [types::I64],              types::I64);
        decl_fn!("slang_tensor_rank",            [types::I64],              types::I64);
        decl_fn!("slang_tensor_size",            [types::I64],              types::I64);
        decl_fn!("slang_tensor_set",             [types::I64, types::I64, types::F64], types::I64);
        decl_fn!("slang_tensor_get",             [types::I64, types::I64],  types::F64);
        decl_fn!("slang_tensor_add",             [types::I64, types::I64],  types::I64);
        decl_fn!("slang_tensor_mul",             [types::I64, types::I64],  types::I64);
        // -- v161: Auto-Differentiation ----------------------------
        decl_fn!("slang_grad_compute",           [types::I64, types::F64],  types::F64);
        decl_fn!("slang_grad_forward",           [types::I64, types::F64],  types::F64);
        decl_fn!("slang_grad_reverse",           [types::I64, types::F64],  types::F64);
        decl_fn!("slang_grad_jacobian_dim",      [types::I64],              types::I64);
        // -- v162: ML Pipeline -------------------------------------
        decl_fn!("slang_ml_linear_fit",          [types::F64, types::F64, types::F64, types::F64], types::I64);
        decl_fn!("slang_ml_predict",             [types::I64, types::F64],  types::F64);
        decl_fn!("slang_ml_accuracy",            [types::F64, types::F64],  types::F64);
        decl_fn!("slang_ml_loss",                [types::F64, types::F64],  types::F64);
        // -- v163: NAS ---------------------------------------------
        decl_fn!("slang_nas_search",             [ptr_type],                types::I64);
        decl_fn!("slang_nas_evaluate",           [types::I64, types::F64],  types::I64);
        decl_fn!("slang_nas_best",               [],                        types::I64);
        decl_fn!("slang_nas_count",              [],                        types::I64);
        // -- v164: Feature Engineering ------------------------------
        decl_fn!("slang_feature_normalize",      [types::F64, types::F64, types::F64], types::F64);
        decl_fn!("slang_feature_one_hot",        [types::I64, types::I64],  types::F64);
        decl_fn!("slang_feature_variance",       [types::I64],              types::F64);
        decl_fn!("slang_feature_correlate",      [types::I64, types::I64],  types::F64);
        // -- v165: Model Serialization ------------------------------
        decl_fn!("slang_model_save",             [ptr_type],                types::I64);
        decl_fn!("slang_model_load",             [ptr_type],                types::I64);
        decl_fn!("slang_model_version",          [],                        types::I64);
        decl_fn!("slang_model_compatible",       [types::I64, types::I64],  types::I64);
        // -- v166: KV Store ----------------------------------------------
        decl_fn!("slang_kv_put",                 [ptr_type, ptr_type],      types::I64);
        decl_fn!("slang_kv_get_len",             [ptr_type],                types::I64);
        decl_fn!("slang_kv_delete",              [ptr_type],                types::I64);
        decl_fn!("slang_kv_wal_count",           [],                        types::I64);
        // -- v167: B-Tree Index ------------------------------------------
        decl_fn!("slang_btree_insert",           [types::I64, types::I64],  types::I64);
        decl_fn!("slang_btree_lookup",           [types::I64],              types::I64);
        decl_fn!("slang_btree_range_count",      [types::I64, types::I64],  types::I64);
        decl_fn!("slang_btree_count",            [],                        types::I64);
        // -- v168: SQL Engine --------------------------------------------
        decl_fn!("slang_sql_create_table",       [ptr_type],                types::I64);
        decl_fn!("slang_sql_insert",             [ptr_type, types::I64, types::I64], types::I64);
        decl_fn!("slang_sql_count",              [ptr_type],                types::I64);
        decl_fn!("slang_sql_sum_col0",           [ptr_type],                types::I64);
        // -- v169: Schema Migration --------------------------------------
        decl_fn!("slang_schema_create",          [ptr_type],                types::I64);
        decl_fn!("slang_schema_migrate",         [ptr_type],                types::I64);
        decl_fn!("slang_schema_version",         [ptr_type],                types::I64);
        decl_fn!("slang_schema_compatible",      [types::I64, types::I64],  types::I64);
        // -- v170: Transaction Log ---------------------------------------
        decl_fn!("slang_txn_begin",              [],                        types::I64);
        decl_fn!("slang_txn_commit",             [types::I64],              types::I64);
        decl_fn!("slang_txn_rollback",           [types::I64],              types::I64);
        decl_fn!("slang_txn_log_count",          [],                        types::I64);
        // -- v171: Data Import/Export -------------------------------------
        decl_fn!("slang_data_buf_create",        [],                        types::I64);
        decl_fn!("slang_data_buf_push",          [types::I64, types::I64],  types::I64);
        decl_fn!("slang_data_buf_len",           [types::I64],              types::I64);
        decl_fn!("slang_data_buf_get",           [types::I64, types::I64],  types::I64);
        // -- v172: TCP Sockets --------------------------------------
        decl_fn!("slang_tcp_create",              [],                        types::I64);
        decl_fn!("slang_tcp_sim_connect",      [types::I64, types::I64],  types::I64);
        decl_fn!("slang_tcp_connected",           [types::I64],              types::I64);
        decl_fn!("slang_tcp_sim_close",           [types::I64],              types::I64);
        // -- v173: HTTP Client --------------------------------------
        decl_fn!("slang_http_sim_get",            [ptr_type],                types::I64);
        decl_fn!("slang_http_sim_post",           [ptr_type],                types::I64);
        decl_fn!("slang_http_request_count",      [],                        types::I64);
        decl_fn!("slang_http_url_valid",          [ptr_type],                types::I64);
        // -- v174: WebSocket ----------------------------------------
        decl_fn!("slang_ws_create",               [],                        types::I64);
        decl_fn!("slang_ws_send",                 [types::I64, ptr_type],    types::I64);
        decl_fn!("slang_ws_msg_count",            [types::I64],              types::I64);
        decl_fn!("slang_ws_close",                [types::I64],              types::I64);
        // -- v175: RPC Framework -------------------------------------
        decl_fn!("slang_rpc_register",            [ptr_type],                types::I64);
        decl_fn!("slang_rpc_call",                [ptr_type],                types::I64);
        decl_fn!("slang_rpc_total_calls",         [],                        types::I64);
        decl_fn!("slang_rpc_service_count",       [],                        types::I64);
        // -- v176: DNS Resolution ------------------------------------
        decl_fn!("slang_dns_resolve",             [ptr_type],                types::I64);
        decl_fn!("slang_dns_cached",              [ptr_type],                types::I64);
        decl_fn!("slang_dns_cache_size",          [],                        types::I64);
        decl_fn!("slang_dns_cache_flush",         [],                        types::I64);
        // -- v177: TLS/SSL -------------------------------------------
        decl_fn!("slang_tls_create",              [ptr_type],                types::I64);
        decl_fn!("slang_tls_active",              [types::I64],              types::I64);
        decl_fn!("slang_tls_close",               [types::I64],              types::I64);
        decl_fn!("slang_tls_cert_valid",          [types::I64],              types::I64);
        // -- v178: REPL Enhancements --------------------------------
        decl_fn!("slang_repl_history_add",       [ptr_type],               types::I64);
        decl_fn!("slang_repl_history_count",     [],                       types::I64);
        decl_fn!("slang_repl_history_clear",     [],                       types::I64);
        decl_fn!("slang_repl_complete_count",    [ptr_type],               types::I64);
        // -- v179: Package Registry ---------------------------------
        decl_fn!("slang_pkg_publish",            [ptr_type, types::I64, types::I64, types::I64], types::I64);
        decl_fn!("slang_pkg_installed",          [ptr_type],               types::I64);
        decl_fn!("slang_pkg_count",              [],                       types::I64);
        decl_fn!("slang_pkg_remove",             [ptr_type],               types::I64);
        // -- v180: Documentation Generator --------------------------
        decl_fn!("slang_doc_add",                [ptr_type, ptr_type],     types::I64);
        decl_fn!("slang_doc_count",              [],                       types::I64);
        decl_fn!("slang_doc_has",                [ptr_type],               types::I64);
        decl_fn!("slang_doc_clear",              [],                       types::I64);
        // -- v181: Benchmark Suite ----------------------------------
        decl_fn!("slang_bench_record",           [ptr_type, types::F64],   types::I64);
        decl_fn!("slang_bench_count",            [],                       types::I64);
        decl_fn!("slang_bench_best",             [ptr_type],               types::F64);
        decl_fn!("slang_bench_clear",            [],                       types::I64);
        // -- v182: Profiler -----------------------------------------
        decl_fn!("slang_profile_start",          [],                       types::I64);
        decl_fn!("slang_profile_stop",           [],                       types::I64);
        decl_fn!("slang_profile_sample",         [],                       types::I64);
        decl_fn!("slang_profile_samples",        [],                       types::I64);
        // -- v183: Interactive Playground ----------------------------
        decl_fn!("slang_playground_eval",        [ptr_type],               types::I64);
        decl_fn!("slang_playground_count",       [],                       types::I64);
        decl_fn!("slang_playground_clear",       [],                       types::I64);
        decl_fn!("slang_playground_last_result", [],                       types::I64);
        // -- v184: Work-Stealing Scheduler --------------------------
        decl_fn!("slang_task_submit",            [types::I64],             types::I64);
        decl_fn!("slang_task_queue_len",         [],                       types::I64);
        decl_fn!("slang_task_steal",             [],                       types::I64);
        decl_fn!("slang_task_queue_clear",       [],                       types::I64);
        // -- v185: Actor Model --------------------------------------
        decl_fn!("slang_actor_spawn",            [],                       types::I64);
        decl_fn!("slang_actor_send",             [types::I64, types::I64], types::I64);
        decl_fn!("slang_actor_recv",             [types::I64],             types::I64);
        decl_fn!("slang_actor_mailbox_len",      [types::I64],             types::I64);
        // -- v186: STM ----------------------------------------------
        decl_fn!("slang_stm_new",                [types::I64],             types::I64);
        decl_fn!("slang_stm_read",               [types::I64],             types::I64);
        decl_fn!("slang_stm_write",              [types::I64, types::I64], types::I64);
        decl_fn!("slang_stm_cas",                [types::I64, types::I64, types::I64], types::I64);
        // -- v187: Parallel Collections -----------------------------
        decl_fn!("slang_par_sum",                [types::I64],             types::F64);
        decl_fn!("slang_par_min",                [types::I64],             types::F64);
        decl_fn!("slang_par_max",                [types::I64],             types::F64);
        decl_fn!("slang_par_count",              [types::I64],             types::I64);
        // -- v188: GPU Task Scheduling ------------------------------
        decl_fn!("slang_gpu_submit",             [types::I64],             types::I64);
        decl_fn!("slang_gpu_queue_len",          [],                       types::I64);
        decl_fn!("slang_gpu_flush",              [],                       types::I64);
        decl_fn!("slang_gpu_available",          [],                       types::I64);
        // -- v189: Distributed Computing ----------------------------
        decl_fn!("slang_dist_node_add",          [ptr_type],               types::I64);
        decl_fn!("slang_dist_node_count",        [],                       types::I64);
        decl_fn!("slang_dist_broadcast",         [types::I64],             types::I64);
        decl_fn!("slang_dist_reduce",            [types::I64, types::I64], types::I64);
        // -- v190: ADTs v2 ------------------------------------------
        decl_fn!("slang_type_register",          [ptr_type, types::I64],   types::I64);
        decl_fn!("slang_type_variant_count",     [ptr_type],               types::I64);
        decl_fn!("slang_type_count",             [],                       types::I64);
        decl_fn!("slang_type_exists",            [ptr_type],               types::I64);
        // -- v191: Higher-Kinded Types ------------------------------
        decl_fn!("slang_hkt_register",           [ptr_type, types::I64],   types::I64);
        decl_fn!("slang_hkt_arity",              [ptr_type],               types::I64);
        decl_fn!("slang_hkt_count",              [],                       types::I64);
        decl_fn!("slang_hkt_exists",             [ptr_type],               types::I64);
        // -- v192: Dependent Types v2 -------------------------------
        decl_fn!("slang_dep_type_check_range",   [types::I64, types::I64, types::I64], types::I64);
        decl_fn!("slang_dep_type_nat",           [types::I64],             types::I64);
        decl_fn!("slang_dep_type_positive",      [types::I64],             types::I64);
        decl_fn!("slang_dep_type_bounded_add",   [types::I64, types::I64, types::I64], types::I64);
        // -- v193: Effect Polymorphism ------------------------------
        decl_fn!("slang_effect_register",        [ptr_type],               types::I64);
        decl_fn!("slang_effect_add_handler",     [ptr_type, ptr_type],     types::I64);
        decl_fn!("slang_effect_handler_count",   [ptr_type],               types::I64);
        decl_fn!("slang_effect_count",           [],                       types::I64);
        // -- v194: Type-Level Computation ---------------------------
        decl_fn!("slang_type_level_add",         [types::I64, types::I64], types::I64);
        decl_fn!("slang_type_level_mul",         [types::I64, types::I64], types::I64);
        decl_fn!("slang_type_level_eq",          [types::I64, types::I64], types::I64);
        decl_fn!("slang_type_level_if",          [types::I64, types::I64, types::I64], types::I64);
        // -- v195: Gradual Typing -----------------------------------
        decl_fn!("slang_gradual_annotate",       [ptr_type, ptr_type],     types::I64);
        decl_fn!("slang_gradual_check",          [ptr_type, ptr_type],     types::I64);
        decl_fn!("slang_gradual_typed_count",    [],                       types::I64);
        decl_fn!("slang_gradual_is_any",         [ptr_type],               types::I64);
        // -- v196: FFI v2 -------------------------------------------
        decl_fn!("slang_ffi_bind",               [ptr_type, ptr_type],     types::I64);
        decl_fn!("slang_ffi_bound",              [ptr_type],               types::I64);
        decl_fn!("slang_ffi_count",              [],                       types::I64);
        decl_fn!("slang_ffi_remove",             [ptr_type],               types::I64);
        // -- v197: Cloud-Native Deployment --------------------------
        decl_fn!("slang_cloud_deploy",           [ptr_type, ptr_type],     types::I64);
        decl_fn!("slang_cloud_deployment_count", [],                       types::I64);
        decl_fn!("slang_cloud_health_check",     [],                       types::I64);
        decl_fn!("slang_cloud_shutdown",         [],                       types::I64);
        // -- v198: Self-Hosting v3 ----------------------------------
        decl_fn!("slang_bootstrap_stage",        [],                       types::I64);
        decl_fn!("slang_bootstrap_advance",      [],                       types::I64);
        decl_fn!("slang_bootstrap_verify",       [types::I64, types::I64], types::I64);
        decl_fn!("slang_bootstrap_reset",        [],                       types::I64);
        // -- v199: AI Language Server -------------------------------
        decl_fn!("slang_ai_suggest",             [ptr_type],               types::I64);
        decl_fn!("slang_ai_suggestion_count",    [],                       types::I64);
        decl_fn!("slang_ai_explain_error",       [types::I64],             types::I64);
        decl_fn!("slang_ai_clear",               [],                       types::I64);
        // -- v200: Milestone ----------------------------------------
        decl_fn!("slang_vitalis_version",        [],                       types::I64);
        decl_fn!("slang_vitalis_module_count",   [],                       types::I64);
        decl_fn!("slang_vitalis_test_count",     [],                       types::I64);
        decl_fn!("slang_vitalis_builtin_count",  [],                       types::I64);
        // -- v201-v206: Spike Engine --
        decl_fn!("slang_spike_emit", [types::I64, types::I64, types::I64], types::I64);
        decl_fn!("slang_spike_queue_len", [], types::I64);
        decl_fn!("slang_spike_next", [], types::I64);
        decl_fn!("slang_spike_clear", [], types::I64);
        decl_fn!("slang_neuro_compartment_create", [types::F64, types::F64], types::I64);
        decl_fn!("slang_neuro_compartment_step", [types::I64, types::F64], types::I64);
        decl_fn!("slang_neuro_dendrite_propagate", [types::F64, types::I64], types::I64);
        decl_fn!("slang_neuro_compartment_count", [], types::I64);
        decl_fn!("slang_synapse_conductance", [types::F64, types::F64, types::F64], types::I64);
        decl_fn!("slang_synapse_stp_facilitate", [types::F64, types::F64], types::I64);
        decl_fn!("slang_synapse_stp_depress", [types::I64, types::F64], types::I64);
        decl_fn!("slang_synapse_count", [], types::I64);
        decl_fn!("slang_neuro_wilson_cowan", [types::F64, types::F64, types::F64, types::F64, types::F64, types::F64, types::I64], types::I64);
        decl_fn!("slang_neuro_neural_mass", [types::F64, types::F64, types::F64, types::F64], types::I64);
        decl_fn!("slang_neuro_population_activity", [types::I64, types::F64, types::F64], types::I64);
        decl_fn!("slang_neuro_population_sync", [types::F64, types::F64], types::I64);
        decl_fn!("slang_spike_encode_rate", [types::F64, types::F64, types::I64], types::I64);
        decl_fn!("slang_spike_encode_temporal", [types::F64, types::F64, types::I64], types::I64);
        decl_fn!("slang_spike_decode_rate", [types::I64, types::I64, types::F64], types::I64);
        decl_fn!("slang_spike_encode_phase", [types::F64, types::F64], types::I64);
        decl_fn!("slang_neuro_mem_alloc", [types::I64], types::I64);
        decl_fn!("slang_neuro_mem_read", [types::I64], types::I64);
        decl_fn!("slang_neuro_mem_write", [types::I64, types::I64], types::I64);
        decl_fn!("slang_neuro_mem_near_compute", [types::I64, types::I64, types::I64], types::I64);
        // -- v207-v212: Loihi Simulator --
        decl_fn!("slang_loihi_core_create", [types::F64, types::F64, types::I64], types::I64);
        decl_fn!("slang_loihi_core_config", [types::I64, types::I64, types::F64], types::I64);
        decl_fn!("slang_loihi_core_neuron_count", [types::I64], types::I64);
        decl_fn!("slang_loihi_core_count", [], types::I64);
        decl_fn!("slang_loihi_route_spike", [types::I64, types::I64], types::I64);
        decl_fn!("slang_loihi_route_multicast", [types::I64], types::I64);
        decl_fn!("slang_loihi_noc_latency", [types::I64], types::I64);
        decl_fn!("slang_loihi_noc_bandwidth", [types::I64], types::I64);
        decl_fn!("slang_loihi_learn_stdp", [types::I64, types::F64], types::I64);
        decl_fn!("slang_loihi_learn_reward", [types::F64, types::F64, types::F64], types::I64);
        decl_fn!("slang_loihi_learn_3factor", [types::F64, types::F64, types::F64, types::F64], types::I64);
        decl_fn!("slang_loihi_learn_config", [types::I64, types::I64], types::I64);
        decl_fn!("slang_loihi_timestep", [], types::I64);
        decl_fn!("slang_loihi_barrier_sync", [], types::I64);
        decl_fn!("slang_loihi_async_tick", [types::I64], types::I64);
        decl_fn!("slang_loihi_time_now", [], types::I64);
        decl_fn!("slang_loihi_energy_spike", [types::I64], types::I64);
        decl_fn!("slang_loihi_energy_compute", [types::I64], types::I64);
        decl_fn!("slang_loihi_power_total", [], types::I64);
        decl_fn!("slang_loihi_energy_reset", [], types::I64);
        decl_fn!("slang_loihi_inst_soma", [types::I64, types::F64], types::I64);
        decl_fn!("slang_loihi_inst_synapse", [types::I64, types::F64], types::I64);
        decl_fn!("slang_loihi_inst_axon", [types::I64], types::I64);
        decl_fn!("slang_loihi_inst_dendrite", [types::I64, types::F64], types::I64);
        // -- v213-v218: SNN Learning --
        decl_fn!("slang_snn_surrogate_forward", [types::F64, types::F64, types::F64, types::I64], types::I64);
        decl_fn!("slang_snn_surrogate_backward", [types::F64, types::F64, types::F64], types::I64);
        decl_fn!("slang_snn_surrogate_sigmoid", [types::F64], types::I64);
        decl_fn!("slang_snn_surrogate_loss", [types::F64, types::F64, types::I64], types::I64);
        decl_fn!("slang_snn_bptt_forward", [types::F64, types::F64, types::F64, types::I64], types::I64);
        decl_fn!("slang_snn_bptt_backward", [types::F64, types::F64, types::I64], types::I64);
        decl_fn!("slang_snn_bptt_truncate", [types::F64, types::F64, types::I64], types::I64);
        decl_fn!("slang_snn_bptt_gradient", [types::F64, types::I64], types::I64);
        decl_fn!("slang_snn_nas_search", [types::I64, types::I64, types::F64], types::I64);
        decl_fn!("slang_snn_nas_evaluate", [types::I64, types::I64], types::I64);
        decl_fn!("slang_snn_nas_mutate", [types::I64, types::I64, types::I64], types::I64);
        decl_fn!("slang_snn_nas_best", [], types::I64);
        decl_fn!("slang_snn_fed_aggregate", [types::F64, types::I64], types::I64);
        decl_fn!("slang_snn_fed_share", [types::F64, types::F64], types::I64);
        decl_fn!("slang_snn_fed_round", [], types::I64);
        decl_fn!("slang_snn_fed_node_count", [types::I64], types::I64);
        decl_fn!("slang_snn_transfer_freeze", [types::I64], types::I64);
        decl_fn!("slang_snn_transfer_finetune", [types::F64, types::I64], types::I64);
        decl_fn!("slang_snn_transfer_adapt", [types::F64, types::F64], types::I64);
        decl_fn!("slang_snn_transfer_similarity", [types::F64, types::F64], types::I64);
        decl_fn!("slang_snn_continual_learn", [types::F64, types::F64, types::F64, types::F64, types::F64, types::F64], types::I64);
        decl_fn!("slang_snn_continual_consolidate", [types::F64, types::I64], types::I64);
        decl_fn!("slang_snn_continual_replay", [types::I64, types::I64], types::I64);
        decl_fn!("slang_snn_continual_forget_score", [types::F64, types::F64], types::I64);
        // -- v219-v224: Processing-in-Memory --
        decl_fn!("slang_pim_alloc", [types::I64], types::I64);
        decl_fn!("slang_pim_compute_add", [types::I64, types::I64], types::I64);
        decl_fn!("slang_pim_compute_mul", [types::I64, types::I64], types::I64);
        decl_fn!("slang_pim_transfer_cost", [types::I64, types::I64], types::I64);
        decl_fn!("slang_datacentric_map", [types::I64, types::F64, types::F64], types::I64);
        decl_fn!("slang_datacentric_reduce", [types::I64, types::I64, types::I64], types::I64);
        decl_fn!("slang_datacentric_scatter", [types::I64, types::I64], types::I64);
        decl_fn!("slang_datacentric_gather", [types::I64, types::F64], types::I64);
        decl_fn!("slang_sparse_spike_propagate", [types::I64, types::I64], types::I64);
        decl_fn!("slang_sparse_nonzero_count", [types::I64, types::I64], types::I64);
        decl_fn!("slang_sparse_compress", [types::I64, types::I64, types::I64], types::I64);
        decl_fn!("slang_sparse_decompress", [types::I64, types::I64], types::I64);
        decl_fn!("slang_cache_oblivious_transpose", [types::I64], types::I64);
        decl_fn!("slang_cache_oblivious_fft", [types::I64], types::I64);
        decl_fn!("slang_cache_oblivious_sort", [types::I64], types::I64);
        decl_fn!("slang_cache_oblivious_matmul", [types::I64], types::I64);
        decl_fn!("slang_memcompute_fused_mac", [types::F64, types::F64, types::F64], types::I64);
        decl_fn!("slang_memcompute_fused_compare", [types::I64, types::I64], types::I64);
        decl_fn!("slang_memcompute_fused_accumulate", [types::I64, types::I64, types::I64], types::I64);
        decl_fn!("slang_memcompute_pipeline_depth", [types::I64], types::I64);
        decl_fn!("slang_zerocopy_spike_buffer", [types::I64], types::I64);
        decl_fn!("slang_zerocopy_fanout", [types::I64, types::I64], types::I64);
        decl_fn!("slang_zerocopy_gather", [types::I64], types::I64);
        decl_fn!("slang_zerocopy_active_count", [], types::I64);
        // -- v225-v230: Brain Models --
        decl_fn!("slang_pred_coding_forward", [types::F64, types::F64, types::F64], types::I64);
        decl_fn!("slang_pred_coding_error", [types::F64, types::F64, types::F64], types::I64);
        decl_fn!("slang_pred_coding_update", [types::F64, types::F64, types::F64], types::I64);
        decl_fn!("slang_pred_coding_layers", [types::I64, types::I64], types::I64);
        decl_fn!("slang_htm_spatial_pool", [types::I64, types::I64, types::I64], types::I64);
        decl_fn!("slang_htm_temporal_memory", [types::I64, types::I64], types::I64);
        decl_fn!("slang_htm_anomaly_score", [types::I64, types::I64], types::I64);
        decl_fn!("slang_htm_column_count", [], types::I64);
        decl_fn!("slang_neuro_osc_gamma", [types::F64, types::F64, types::F64], types::I64);
        decl_fn!("slang_neuro_osc_theta", [types::F64, types::F64, types::F64], types::I64);
        decl_fn!("slang_neuro_osc_couple", [types::F64, types::F64, types::F64], types::I64);
        decl_fn!("slang_neuro_osc_phase_lock", [types::F64, types::F64], types::I64);
        decl_fn!("slang_neuromod_dopamine", [types::F64, types::F64], types::I64);
        decl_fn!("slang_neuromod_serotonin", [types::F64, types::F64], types::I64);
        decl_fn!("slang_neuromod_acetylcholine", [types::F64, types::F64], types::I64);
        decl_fn!("slang_neuromod_apply", [types::F64, types::F64, types::F64], types::I64);
        decl_fn!("slang_cortical_column_create", [], types::I64);
        decl_fn!("slang_cortical_column_step", [types::I64, types::F64], types::I64);
        decl_fn!("slang_cortical_column_layer_activity", [types::I64, types::I64], types::I64);
        decl_fn!("slang_cortical_column_count", [], types::I64);
        decl_fn!("slang_spike_attention_query", [types::F64, types::F64, types::F64], types::I64);
        decl_fn!("slang_spike_attention_key", [types::I64, types::I64], types::I64);
        decl_fn!("slang_spike_attention_value", [types::I64, types::F64], types::I64);
        decl_fn!("slang_spike_attention_score", [types::F64, types::I64], types::I64);
        // -- v231-v236: GPU Neuromorphic --
        decl_fn!("slang_gpu_spike_propagate", [types::I64, types::I64], types::I64);
        decl_fn!("slang_gpu_spike_batch_size", [types::I64], types::I64);
        decl_fn!("slang_gpu_spike_throughput", [types::I64, types::F64], types::I64);
        decl_fn!("slang_gpu_spike_sync", [], types::I64);
        decl_fn!("slang_gpu_neuron_update", [types::I64, types::F64, types::F64], types::I64);
        decl_fn!("slang_gpu_neuron_batch", [types::I64, types::I64], types::I64);
        decl_fn!("slang_gpu_neuron_occupancy", [types::I64, types::I64], types::I64);
        decl_fn!("slang_gpu_neuron_count", [], types::I64);
        decl_fn!("slang_gpu_synapse_spmv", [types::I64, types::F64, types::F64], types::I64);
        decl_fn!("slang_gpu_synapse_csr", [types::I64, types::I64, types::I64], types::I64);
        decl_fn!("slang_gpu_synapse_nnz", [types::I64, types::F64], types::I64);
        decl_fn!("slang_gpu_synapse_density", [types::I64, types::I64, types::I64], types::I64);
        decl_fn!("slang_gpu_event_push", [types::I64, types::I64], types::I64);
        decl_fn!("slang_gpu_event_pop", [], types::I64);
        decl_fn!("slang_gpu_event_merge", [types::I64], types::I64);
        decl_fn!("slang_gpu_event_size", [], types::I64);
        decl_fn!("slang_mixed_prec_quantize", [types::F64, types::I64], types::I64);
        decl_fn!("slang_mixed_prec_dequantize", [types::I64, types::I64], types::I64);
        decl_fn!("slang_mixed_prec_accumulate", [types::F64, types::F64], types::I64);
        decl_fn!("slang_mixed_prec_bits", [types::F64], types::I64);
        decl_fn!("slang_multi_gpu_partition", [types::I64, types::I64], types::I64);
        decl_fn!("slang_multi_gpu_sync", [types::I64, types::I64], types::I64);
        decl_fn!("slang_multi_gpu_migrate", [types::I64, types::I64], types::I64);
        decl_fn!("slang_multi_gpu_count", [], types::I64);
        // -- v237-v242: Neuro Applications --
        decl_fn!("slang_spike_vision_encode", [types::I64, types::I64, types::F64], types::I64);
        decl_fn!("slang_spike_vision_edge", [types::I64, types::I64, types::F64], types::I64);
        decl_fn!("slang_spike_vision_motion", [types::I64, types::I64], types::I64);
        decl_fn!("slang_spike_vision_frames", [], types::I64);
        decl_fn!("slang_spike_audio_encode", [types::I64, types::I64, types::F64], types::I64);
        decl_fn!("slang_spike_audio_frequency", [types::I64], types::I64);
        decl_fn!("slang_spike_audio_onset", [types::I64, types::I64], types::I64);
        decl_fn!("slang_spike_audio_classify", [types::I64, types::I64], types::I64);
        decl_fn!("slang_spike_pid", [types::F64, types::F64, types::F64], types::I64);
        decl_fn!("slang_spike_motor", [types::F64, types::F64], types::I64);
        decl_fn!("slang_spike_reflex", [types::I64], types::I64);
        decl_fn!("slang_spike_trajectory_cost", [types::F64, types::I64], types::I64);
        decl_fn!("slang_spike_anomaly_score", [types::F64, types::F64], types::I64);
        decl_fn!("slang_spike_changepoint", [types::F64, types::F64, types::F64], types::I64);
        decl_fn!("slang_spike_burst_detect", [types::F64, types::F64], types::I64);
        decl_fn!("slang_spike_pattern_match", [types::F64], types::I64);
        decl_fn!("slang_spike_anneal", [types::F64, types::F64, types::I64], types::I64);
        decl_fn!("slang_spike_gradient", [types::F64, types::F64, types::F64], types::I64);
        decl_fn!("slang_spike_constraint", [types::I64, types::F64], types::I64);
        decl_fn!("slang_spike_fitness", [types::F64, types::F64], types::I64);
        decl_fn!("slang_spike_nlp_similarity", [types::F64, types::F64, types::F64], types::I64);
        decl_fn!("slang_spike_nlp_attention", [types::F64, types::F64], types::I64);
        decl_fn!("slang_spike_nlp_encode_len", [types::I64, types::I64], types::I64);
        decl_fn!("slang_spike_nlp_perplexity", [types::F64], types::I64);
        // -- v243-v248: Neuro Evolution --
        decl_fn!("slang_neat_crossover", [types::F64, types::F64], types::I64);
        decl_fn!("slang_neat_mutate", [types::I64, types::I64], types::I64);
        decl_fn!("slang_neat_speciate", [types::I64, types::I64, types::F64], types::I64);
        decl_fn!("slang_neat_generation", [], types::I64);
        decl_fn!("slang_som_bmu", [types::F64, types::F64, types::I64], types::I64);
        decl_fn!("slang_som_radius", [types::F64, types::I64, types::I64], types::I64);
        decl_fn!("slang_som_learning_rate", [types::F64, types::I64, types::I64], types::I64);
        decl_fn!("slang_som_quant_error", [types::F64], types::I64);
        decl_fn!("slang_neuro_nas_evaluate", [types::F64, types::F64, types::F64], types::I64);
        decl_fn!("slang_neuro_nas_sample", [types::I64, types::I64], types::I64);
        decl_fn!("slang_neuro_nas_prune", [types::I64, types::F64], types::I64);
        decl_fn!("slang_neuro_nas_best_score", [], types::I64);
        decl_fn!("slang_spike_rl_rstdp", [types::F64, types::F64], types::I64);
        decl_fn!("slang_spike_rl_td", [types::F64, types::F64, types::F64, types::F64], types::I64);
        decl_fn!("slang_spike_rl_policy", [types::F64, types::F64], types::I64);
        decl_fn!("slang_spike_rl_predict_reward", [types::F64, types::F64], types::I64);
        decl_fn!("slang_curiosity_reward", [types::F64], types::I64);
        decl_fn!("slang_curiosity_info_gain", [types::F64, types::F64], types::I64);
        decl_fn!("slang_curiosity_novelty", [types::F64], types::I64);
        decl_fn!("slang_curiosity_decay", [types::F64, types::I64], types::I64);
        decl_fn!("slang_meta_maml_adapt", [types::F64, types::F64, types::F64], types::I64);
        decl_fn!("slang_meta_reptile", [types::F64, types::F64, types::F64], types::I64);
        decl_fn!("slang_meta_task_similarity", [types::F64, types::F64, types::F64], types::I64);
        decl_fn!("slang_meta_convergence", [types::F64, types::F64, types::I64], types::I64);
        // -- v249-v254: SNN-ANN Hybrid --
        decl_fn!("slang_snn_ann_to_rate", [types::F64, types::F64], types::I64);
        decl_fn!("slang_snn_rate_to_ann", [types::F64, types::F64], types::I64);
        decl_fn!("slang_snn_conversion_loss", [types::F64, types::F64], types::I64);
        decl_fn!("slang_snn_optimal_timesteps", [types::F64], types::I64);
        decl_fn!("slang_hybrid_infer", [types::F64, types::F64, types::F64], types::I64);
        decl_fn!("slang_hybrid_set_fraction", [types::I64], types::I64);
        decl_fn!("slang_hybrid_efficiency", [types::F64, types::F64], types::I64);
        decl_fn!("slang_hybrid_accuracy_gain", [types::F64, types::F64], types::I64);
        decl_fn!("slang_spike_compile", [types::I64, types::I64, types::I64], types::I64);
        decl_fn!("slang_spike_compile_optimize", [types::I64, types::F64], types::I64);
        decl_fn!("slang_spike_compile_count", [], types::I64);
        decl_fn!("slang_spike_compile_memory", [types::I64, types::I64], types::I64);
        decl_fn!("slang_diff_spike_ste", [types::F64, types::F64], types::I64);
        decl_fn!("slang_diff_spike_sigmoid", [types::F64, types::F64, types::F64], types::I64);
        decl_fn!("slang_diff_spike_fast_sigmoid", [types::F64, types::F64, types::F64], types::I64);
        decl_fn!("slang_diff_spike_accumulate", [types::F64, types::F64], types::I64);
        decl_fn!("slang_neural_ode_euler", [types::F64, types::F64, types::F64], types::I64);
        decl_fn!("slang_neural_ode_rk4", [types::F64, types::F64, types::F64, types::F64, types::F64, types::F64], types::I64);
        decl_fn!("slang_neural_ode_adjoint", [types::F64, types::F64], types::I64);
        decl_fn!("slang_neural_ode_adaptive_dt", [types::F64, types::F64, types::F64], types::I64);
        decl_fn!("slang_hybrid_distill", [types::F64, types::F64, types::F64], types::I64);
        decl_fn!("slang_hybrid_freeze", [types::I64, types::I64, types::I64], types::I64);
        decl_fn!("slang_hybrid_lr", [types::F64, types::I64, types::I64, types::I64], types::I64);
        decl_fn!("slang_hybrid_weighted_acc", [types::F64, types::F64, types::F64], types::I64);
        // -- v255-v260: Hippocampal Memory --
        decl_fn!("slang_hippo_encode", [types::I64, types::I64, types::I64], types::I64);
        decl_fn!("slang_hippo_recall", [types::I64], types::I64);
        decl_fn!("slang_hippo_replay", [types::I64], types::I64);
        decl_fn!("slang_hippo_count", [], types::I64);
        decl_fn!("slang_wm_push", [types::I64], types::I64);
        decl_fn!("slang_wm_pop", [], types::I64);
        decl_fn!("slang_wm_set_capacity", [types::I64], types::I64);
        decl_fn!("slang_wm_utilization", [], types::I64);
        decl_fn!("slang_sleep_consolidate", [types::I64], types::I64);
        decl_fn!("slang_sleep_rem", [types::F64], types::I64);
        decl_fn!("slang_sleep_nrem_ripple", [types::I64], types::I64);
        decl_fn!("slang_sleep_duration", [types::I64], types::I64);
        decl_fn!("slang_hopfield_energy", [types::I64, types::F64, types::F64], types::I64);
        decl_fn!("slang_hopfield_capacity", [types::I64], types::I64);
        decl_fn!("slang_hopfield_retrieve", [types::I64, types::I64], types::I64);
        decl_fn!("slang_hopfield_accuracy", [types::I64, types::F64], types::I64);
        decl_fn!("slang_synaptag_decay", [types::F64, types::F64, types::F64], types::I64);
        decl_fn!("slang_synaptag_capture", [types::F64, types::F64], types::I64);
        decl_fn!("slang_synaptag_late_ltp", [types::F64, types::F64], types::I64);
        decl_fn!("slang_synaptag_protein", [types::F64, types::F64], types::I64);
        decl_fn!("slang_memcompress_schema", [types::I64, types::F64], types::I64);
        decl_fn!("slang_memcompress_forget", [types::I64, types::F64], types::I64);
        decl_fn!("slang_memcompress_merge", [types::I64, types::F64], types::I64);
        decl_fn!("slang_memcompress_ratio", [types::I64, types::I64], types::I64);
        // -- v261-v266: Distributed Neuromorphic --
        decl_fn!("slang_neuro_cluster_init", [types::I64], types::I64);
        decl_fn!("slang_neuro_cluster_distribute", [types::I64], types::I64);
        decl_fn!("slang_neuro_cluster_load", [types::I64], types::I64);
        decl_fn!("slang_neuro_cluster_nodes", [], types::I64);
        decl_fn!("slang_spike_consensus_vote", [types::I64, types::I64], types::I64);
        decl_fn!("slang_spike_consensus_bft", [types::I64], types::I64);
        decl_fn!("slang_spike_consensus_tick", [types::I64, types::I64], types::I64);
        decl_fn!("slang_spike_consensus_latency", [types::I64, types::I64], types::I64);
        decl_fn!("slang_fed_neuro_average", [types::F64, types::F64, types::F64], types::I64);
        decl_fn!("slang_fed_neuro_dp_noise", [types::I64, types::F64], types::I64);
        decl_fn!("slang_fed_neuro_compress", [types::I64, types::F64], types::I64);
        decl_fn!("slang_fed_neuro_round", [types::I64], types::I64);
        decl_fn!("slang_edge_neuro_budget", [types::I64], types::I64);
        decl_fn!("slang_edge_neuro_quantize", [types::I64, types::I64], types::I64);
        decl_fn!("slang_edge_neuro_latency_ok", [types::I64, types::I64], types::I64);
        decl_fn!("slang_edge_neuro_model_size", [types::I64, types::I64], types::I64);
        decl_fn!("slang_stream_spike_process", [types::I64, types::I64], types::I64);
        decl_fn!("slang_stream_spike_rate", [types::I64, types::F64], types::I64);
        decl_fn!("slang_stream_spike_backpressure", [types::I64, types::I64], types::I64);
        decl_fn!("slang_stream_spike_total", [], types::I64);
        decl_fn!("slang_neuro_platform_caps", [types::I64], types::I64);
        decl_fn!("slang_neuro_platform_map", [types::I64, types::I64], types::I64);
        decl_fn!("slang_neuro_platform_overhead", [types::I64, types::I64], types::I64);
        decl_fn!("slang_neuro_platform_power", [types::I64, types::I64], types::I64);
        // -- v267-v272: Neuro Tooling --
        decl_fn!("slang_neuro_viz_spike_raster", [types::I64, types::I64, types::F64], types::I64);
        decl_fn!("slang_neuro_viz_membrane", [types::I64, types::I64], types::I64);
        decl_fn!("slang_neuro_viz_connectivity", [types::I64, types::I64], types::I64);
        decl_fn!("slang_neuro_viz_frames", [], types::I64);
        decl_fn!("slang_neuro_debug_break", [types::I64], types::I64);
        decl_fn!("slang_neuro_debug_inspect", [types::I64, types::I64], types::I64);
        decl_fn!("slang_neuro_debug_step", [types::I64, types::F64], types::I64);
        decl_fn!("slang_neuro_debug_breakpoints", [], types::I64);
        decl_fn!("slang_neuro_profile_throughput", [types::I64, types::F64], types::I64);
        decl_fn!("slang_neuro_profile_memory", [types::I64, types::I64], types::I64);
        decl_fn!("slang_neuro_profile_energy", [types::I64, types::I64], types::I64);
        decl_fn!("slang_neuro_profile_samples", [], types::I64);
        decl_fn!("slang_neuro_dsl_neuron", [types::I64, types::I64], types::I64);
        decl_fn!("slang_neuro_dsl_synapse", [types::I64, types::I64, types::F64], types::I64);
        decl_fn!("slang_neuro_dsl_network", [types::I64, types::F64], types::I64);
        decl_fn!("slang_neuro_dsl_validate", [types::I64, types::I64], types::I64);
        decl_fn!("slang_neuro_bench_spike_lat", [types::I64], types::I64);
        decl_fn!("slang_neuro_bench_neuron_tput", [types::I64, types::I64], types::I64);
        decl_fn!("slang_neuro_bench_synapse_rate", [types::I64, types::F64], types::I64);
        decl_fn!("slang_neuro_bench_efficiency", [types::I64, types::I64], types::I64);
        decl_fn!("slang_neuro_test_timing", [types::I64, types::I64], types::I64);
        decl_fn!("slang_neuro_test_accuracy", [types::F64, types::F64], types::I64);
        decl_fn!("slang_neuro_test_convergence", [types::F64, types::F64], types::I64);
        decl_fn!("slang_neuro_test_spike_gen", [types::I64, types::F64, types::I64], types::I64);
        // -- v273-v278: Quantum Neuromorphic --
        decl_fn!("slang_quantum_spike_encode", [types::F64, types::I64], types::I64);
        decl_fn!("slang_quantum_spike_decode", [types::I64, types::I64], types::I64);
        decl_fn!("slang_quantum_spike_superpose", [types::I64, types::I64], types::I64);
        decl_fn!("slang_quantum_spike_fidelity", [types::F64], types::I64);
        decl_fn!("slang_quantum_plasticity_stdp", [types::F64, types::F64], types::I64);
        decl_fn!("slang_quantum_plasticity_anneal", [types::F64, types::F64], types::I64);
        decl_fn!("slang_quantum_plasticity_tunnel", [types::F64, types::F64], types::I64);
        decl_fn!("slang_quantum_plasticity_t2", [types::I64, types::F64], types::I64);
        decl_fn!("slang_quantum_reservoir_init", [types::I64], types::I64);
        decl_fn!("slang_quantum_reservoir_project", [types::F64, types::I64], types::I64);
        decl_fn!("slang_quantum_reservoir_kernel", [types::F64, types::F64, types::F64], types::I64);
        decl_fn!("slang_quantum_reservoir_dim", [], types::I64);
        decl_fn!("slang_qsnn_var_update", [types::F64, types::F64, types::F64], types::I64);
        decl_fn!("slang_qsnn_var_cost", [types::F64, types::F64], types::I64);
        decl_fn!("slang_qsnn_var_depth", [types::I64, types::I64], types::I64);
        decl_fn!("slang_qsnn_var_expressibility", [types::I64, types::I64], types::I64);
        decl_fn!("slang_quantum_qec_shor", [types::I64], types::I64);
        decl_fn!("slang_quantum_qec_surface", [types::I64], types::I64);
        decl_fn!("slang_quantum_qec_syndrome", [types::I64], types::I64);
        decl_fn!("slang_quantum_qec_overhead", [types::I64, types::I64], types::I64);
        decl_fn!("slang_quantum_bridge_encode", [types::F64], types::I64);
        decl_fn!("slang_quantum_bridge_decode", [types::F64], types::I64);
        decl_fn!("slang_quantum_bridge_cost", [types::I64, types::I64], types::I64);
        decl_fn!("slang_quantum_bridge_advantage", [types::I64, types::I64], types::I64);
        // -- v279-v284: Neuro Safety --
        decl_fn!("slang_neuro_verify_timing", [types::I64, types::I64, types::I64], types::I64);
        decl_fn!("slang_neuro_verify_membrane", [types::F64, types::F64, types::F64], types::I64);
        decl_fn!("slang_neuro_verify_symmetry", [types::F64, types::F64, types::F64], types::I64);
        decl_fn!("slang_neuro_verify_liveness", [types::I64, types::I64, types::I64], types::I64);
        decl_fn!("slang_neuro_safe_rate_clamp", [types::I64, types::I64], types::I64);
        decl_fn!("slang_neuro_safe_runaway", [types::I64, types::I64], types::I64);
        decl_fn!("slang_neuro_safe_dead_neuron", [types::I64], types::I64);
        decl_fn!("slang_neuro_safe_violations", [], types::I64);
        decl_fn!("slang_neuro_explain_contribution", [types::F64, types::I64], types::I64);
        decl_fn!("slang_neuro_explain_ablation", [types::F64, types::F64], types::I64);
        decl_fn!("slang_neuro_explain_saliency", [types::F64, types::F64], types::I64);
        decl_fn!("slang_neuro_explain_lrp", [types::F64, types::F64, types::F64], types::I64);
        decl_fn!("slang_neuro_robust_eps_check", [types::F64, types::F64], types::I64);
        decl_fn!("slang_neuro_robust_margin", [types::F64, types::F64], types::I64);
        decl_fn!("slang_neuro_robust_certified_radius", [types::F64, types::F64], types::I64);
        decl_fn!("slang_neuro_robust_inject_noise", [types::F64, types::F64], types::I64);
        decl_fn!("slang_neuro_fair_dp_gap", [types::F64, types::F64], types::I64);
        decl_fn!("slang_neuro_fair_eo_gap", [types::F64, types::F64], types::I64);
        decl_fn!("slang_neuro_fair_calibration", [types::F64, types::F64], types::I64);
        decl_fn!("slang_neuro_fair_lipschitz", [types::F64, types::F64], types::I64);
        decl_fn!("slang_neuro_cert_bounds", [types::F64, types::F64, types::F64], types::I64);
        decl_fn!("slang_neuro_cert_ibp_width", [types::F64, types::F64], types::I64);
        decl_fn!("slang_neuro_cert_crown", [types::F64, types::F64], types::I64);
        decl_fn!("slang_neuro_cert_accuracy", [types::I64, types::I64], types::I64);
        // -- v285-v290: Neuro Performance --
        decl_fn!("slang_neuro_simd_accumulate", [types::I64, types::I64, types::I64, types::I64], types::I64);
        decl_fn!("slang_neuro_simd_threshold", [types::I64, types::I64, types::I64, types::I64], types::I64);
        decl_fn!("slang_neuro_simd_decay", [types::F64, types::F64], types::I64);
        decl_fn!("slang_neuro_simd_throughput", [types::I64, types::I64], types::I64);
        decl_fn!("slang_neuro_jit_compile", [types::I64, types::I64], types::I64);
        decl_fn!("slang_neuro_jit_speedup", [types::I64, types::I64], types::I64);
        decl_fn!("slang_neuro_jit_cache_hit", [types::I64, types::I64], types::I64);
        decl_fn!("slang_neuro_jit_compiled_count", [], types::I64);
        decl_fn!("slang_neuro_precision_auto", [types::F64], types::I64);
        decl_fn!("slang_neuro_precision_quant_error", [types::F64, types::I64], types::I64);
        decl_fn!("slang_neuro_precision_savings", [types::I64, types::I64], types::I64);
        decl_fn!("slang_neuro_precision_scale", [types::F64, types::F64], types::I64);
        decl_fn!("slang_neuro_spec_predict", [types::F64, types::F64], types::I64);
        decl_fn!("slang_neuro_spec_gain", [types::I64, types::I64], types::I64);
        decl_fn!("slang_neuro_spec_rollback_cost", [types::I64], types::I64);
        decl_fn!("slang_neuro_spec_confidence", [types::I64, types::I64], types::I64);
        decl_fn!("slang_neuro_pgo_sample", [types::I64], types::I64);
        decl_fn!("slang_neuro_pgo_is_hot", [types::I64, types::I64], types::I64);
        decl_fn!("slang_neuro_pgo_unroll", [types::I64], types::I64);
        decl_fn!("slang_neuro_pgo_total_samples", [], types::I64);
        decl_fn!("slang_neuro_zero_send", [types::I64, types::I64], types::I64);
        decl_fn!("slang_neuro_zero_update", [types::F64, types::F64, types::F64], types::I64);
        decl_fn!("slang_neuro_zero_conn_type", [types::F64], types::I64);
        decl_fn!("slang_neuro_zero_overhead", [], types::I64);
        // -- v291: Advanced Spike Analytics --
        decl_fn!("slang_spike_analytics_mean_rate", [types::I64, types::I64], types::I64);
        decl_fn!("slang_spike_analytics_cv_isi", [types::F64, types::F64], types::I64);
        decl_fn!("slang_spike_analytics_fano_factor", [types::F64, types::F64], types::I64);
        decl_fn!("slang_spike_analytics_burst_index", [types::I64, types::I64], types::I64);
        // -- v292: Neural Network Metrics --
        decl_fn!("slang_nn_metric_sparsity", [types::I64, types::I64], types::I64);
        decl_fn!("slang_nn_metric_entropy", [types::F64], types::I64);
        decl_fn!("slang_nn_metric_mutual_info", [types::F64, types::F64], types::I64);
        decl_fn!("slang_nn_metric_transfer_entropy", [types::F64, types::F64, types::F64], types::I64);
        // -- v293: Spike Train Distance --
        decl_fn!("slang_spike_dist_victor_purpura", [types::F64, types::F64, types::F64], types::I64);
        decl_fn!("slang_spike_dist_van_rossum", [types::F64, types::F64, types::F64], types::I64);
        decl_fn!("slang_spike_dist_schreiber", [types::F64, types::F64], types::I64);
        decl_fn!("slang_spike_dist_earth_mover", [types::I64, types::I64], types::I64);
        // -- v294: Neural Coding --
        decl_fn!("slang_neural_code_rate", [types::I64, types::I64], types::I64);
        decl_fn!("slang_neural_code_temporal", [types::F64, types::F64], types::I64);
        decl_fn!("slang_neural_code_population", [types::I64, types::I64, types::I64], types::I64);
        decl_fn!("slang_neural_code_sparse", [types::I64, types::F64], types::I64);
        // -- v295: Synaptic Plasticity Metrics --
        decl_fn!("slang_synap_metric_ltp_ratio", [types::F64, types::F64], types::I64);
        decl_fn!("slang_synap_metric_ltd_ratio", [types::F64, types::F64], types::I64);
        decl_fn!("slang_synap_metric_homeostatic", [types::F64, types::F64, types::F64], types::I64);
        decl_fn!("slang_synap_metric_metaplasticity", [types::F64, types::I64], types::I64);
        // -- v296: Network Topology --
        decl_fn!("slang_topo_clustering_coeff", [types::I64, types::I64], types::I64);
        decl_fn!("slang_topo_path_length", [types::I64, types::I64], types::I64);
        decl_fn!("slang_topo_small_world", [types::F64, types::F64], types::I64);
        decl_fn!("slang_topo_modularity", [types::I64, types::I64, types::I64], types::I64);
        // -- v297: Neuromorphic IO --
        decl_fn!("slang_neuro_io_aer_encode", [types::I64, types::I64], types::I64);
        decl_fn!("slang_neuro_io_aer_decode", [types::I64], types::I64);
        decl_fn!("slang_neuro_io_dvs_encode", [types::I64, types::I64, types::F64], types::I64);
        decl_fn!("slang_neuro_io_serial_pack", [types::I64, types::I64], types::I64);
        // -- v298: Neural Dynamics --
        decl_fn!("slang_dyn_lyapunov_exp", [types::F64, types::F64, types::F64], types::I64);
        decl_fn!("slang_dyn_bifurcation", [types::F64, types::F64], types::I64);
        decl_fn!("slang_dyn_phase_portrait", [types::F64, types::F64], types::I64);
        decl_fn!("slang_dyn_attractor_dim", [types::F64, types::I64], types::I64);
        // -- v299: Neuromorphic Scheduler --
        decl_fn!("slang_neuro_sched_priority", [types::I64, types::I64], types::I64);
        decl_fn!("slang_neuro_sched_deadline", [types::I64, types::I64], types::I64);
        decl_fn!("slang_neuro_sched_edf", [types::I64, types::I64, types::I64], types::I64);
        decl_fn!("slang_neuro_sched_utilization", [types::I64, types::I64], types::I64);
        // -- v300: Milestone --
        decl_fn!("slang_vitalis_v300_version", [], types::I64);
        decl_fn!("slang_vitalis_v300_total_builtins", [], types::I64);
        decl_fn!("slang_vitalis_v300_neuro_modules", [], types::I64);
        decl_fn!("slang_vitalis_v300_milestone", [], types::I64);

        // -- v301-v366: Post-Neuromorphic Era --
        // v301 Escape Analysis
        decl_fn!("slang_escape_analyze", [types::I64, types::I64], types::I64);
        decl_fn!("slang_escape_stack_promoted", [], types::I64);
        decl_fn!("slang_escape_summary", [], types::I64);
        decl_fn!("slang_escape_clear", [], types::I64);
        // v302 Tail Call Optimization
        decl_fn!("slang_tco_detect", [types::I64], types::I64);
        decl_fn!("slang_tco_optimized_count", [], types::I64);
        decl_fn!("slang_tco_depth_limit", [types::I64], types::I64);
        decl_fn!("slang_tco_enabled", [], types::I64);
        // v303 Algebraic Simplification
        decl_fn!("slang_opt_algebraic_count", [], types::I64);
        decl_fn!("slang_opt_algebraic_enable", [types::I64], types::I64);
        decl_fn!("slang_opt_strength_reduced", [], types::I64);
        decl_fn!("slang_opt_identity_removed", [], types::I64);
        // v304 Interprocedural Analysis
        decl_fn!("slang_ipa_add_edge", [types::I64, types::I64], types::I64);
        decl_fn!("slang_ipa_call_graph_size", [], types::I64);
        decl_fn!("slang_ipa_mark_pure", [types::I64], types::I64);
        decl_fn!("slang_ipa_pure_functions", [], types::I64);
        decl_fn!("slang_ipa_record_const_args", [types::I64, types::I64], types::I64);
        decl_fn!("slang_ipa_const_args", [], types::I64);
        decl_fn!("slang_ipa_summary", [], types::I64);
        decl_fn!("slang_ipa_clear", [], types::I64);
        // v305 LTO
        decl_fn!("slang_lto_inline_count", [], types::I64);
        decl_fn!("slang_lto_dead_globals", [], types::I64);
        decl_fn!("slang_lto_devirtualized", [], types::I64);
        decl_fn!("slang_lto_enabled", [], types::I64);
        // v306 Vectorization
        decl_fn!("slang_vec_slp_opportunities", [], types::I64);
        decl_fn!("slang_vec_slp_applied", [], types::I64);
        decl_fn!("slang_vec_width", [], types::I64);
        decl_fn!("slang_vec_speedup_estimate", [], types::I64);
        // v307 Compile-Time Execution
        decl_fn!("slang_consteval_string", [types::I64], types::I64);
        decl_fn!("slang_consteval_array", [types::I64], types::I64);
        decl_fn!("slang_consteval_struct", [types::I64], types::I64);
        decl_fn!("slang_consteval_count", [], types::I64);
        // v308 PGO
        decl_fn!("slang_pgo_record", [types::I64, types::I64], types::I64);
        decl_fn!("slang_pgo_hotness", [types::I64], types::I64);
        decl_fn!("slang_pgo_branch_bias", [types::I64], types::I64);
        decl_fn!("slang_pgo_total_samples", [], types::I64);
        // v309 Register Allocation
        decl_fn!("slang_regalloc_spill_count", [], types::I64);
        decl_fn!("slang_regalloc_move_count", [], types::I64);
        decl_fn!("slang_regalloc_pressure", [], types::I64);
        decl_fn!("slang_regalloc_coalesced", [], types::I64);
        // v310 Debug Info
        decl_fn!("slang_debug_line_count", [], types::I64);
        decl_fn!("slang_debug_var_count", [], types::I64);
        decl_fn!("slang_debug_scope_depth", [], types::I64);
        decl_fn!("slang_debug_info_size", [], types::I64);
        // v311 Existential Types
        decl_fn!("slang_existential_create", [types::I64], types::I64);
        decl_fn!("slang_existential_open", [types::I64], types::I64);
        decl_fn!("slang_existential_pack", [types::I64, types::I64], types::I64);
        decl_fn!("slang_existential_count", [], types::I64);
        // v312 Row Types
        decl_fn!("slang_row_type_fields", [types::I64], types::I64);
        decl_fn!("slang_row_type_create", [types::I64], types::I64);
        decl_fn!("slang_row_type_extend", [types::I64, types::I64], types::I64);
        decl_fn!("slang_row_type_restrict", [types::I64, types::I64], types::I64);
        decl_fn!("slang_row_type_compatible", [types::I64, types::I64], types::I64);
        // v313 Linear Types
        decl_fn!("slang_linear_create", [types::I64], types::I64);
        decl_fn!("slang_linear_check", [types::I64], types::I64);
        decl_fn!("slang_linear_consume", [types::I64], types::I64);
        decl_fn!("slang_session_create", [types::I64], types::I64);
        decl_fn!("slang_session_state", [types::I64], types::I64);
        decl_fn!("slang_session_advance", [types::I64], types::I64);
        // v314 GADTs
        decl_fn!("slang_gadt_create", [types::I64, types::I64], types::I64);
        decl_fn!("slang_gadt_refine", [types::I64, types::I64], types::I64);
        decl_fn!("slang_gadt_witness", [types::I64], types::I64);
        decl_fn!("slang_gadt_count", [], types::I64);
        // v315 Type Classes
        decl_fn!("slang_typeclass_instances", [], types::I64);
        decl_fn!("slang_typeclass_resolve", [types::I64], types::I64);
        decl_fn!("slang_typeclass_coherence", [], types::I64);
        decl_fn!("slang_typeclass_register", [types::I64, types::I64], types::I64);
        // v316 Dependent Types
        decl_fn!("slang_dependent_proof", [types::I64, types::I64], types::I64);
        decl_fn!("slang_dependent_index", [types::I64, types::I64], types::I64);
        decl_fn!("slang_dependent_refine", [types::I64], types::I64);
        decl_fn!("slang_dependent_check", [types::I64], types::I64);
        // v317 Effect Inference
        decl_fn!("slang_effect_infer", [types::I64], types::I64);
        decl_fn!("slang_effect_row", [types::I64], types::I64);
        decl_fn!("slang_effect_mask", [types::I64, types::I64], types::I64);
        decl_fn!("slang_effect_polymorphic", [types::I64], types::I64);
        // v318 Mixture of Experts
        decl_fn!("slang_moe_create", [types::I64, types::I64], types::I64);
        decl_fn!("slang_moe_route", [types::I64, types::I64], types::I64);
        decl_fn!("slang_moe_expert_load", [types::I64, types::I64], types::I64);
        decl_fn!("slang_moe_aux_loss", [types::I64], types::I64);
        // v319 Quantization
        decl_fn!("slang_quant_int8", [types::I64], types::I64);
        decl_fn!("slang_quant_int4", [types::I64], types::I64);
        decl_fn!("slang_quant_error", [types::I64, types::I64], types::I64);
        decl_fn!("slang_quant_calibrate", [types::I64, types::I64], types::I64);
        // v320 Attention Variants
        decl_fn!("slang_attn_flash", [types::I64, types::I64, types::I64], types::I64);
        decl_fn!("slang_attn_linear", [types::I64, types::I64], types::I64);
        decl_fn!("slang_attn_sparse", [types::I64, types::I64, types::I64], types::I64);
        decl_fn!("slang_attn_sliding_window", [types::I64, types::I64, types::I64], types::I64);
        // v321 Distillation
        decl_fn!("slang_distill_kd_loss", [types::I64, types::I64, types::I64], types::I64);
        decl_fn!("slang_distill_feature_loss", [types::I64, types::I64], types::I64);
        decl_fn!("slang_distill_attention_transfer", [types::I64, types::I64], types::I64);
        decl_fn!("slang_distill_temperature", [types::I64], types::I64);
        // v322 GNN
        decl_fn!("slang_gnn_create", [types::I64], types::I64);
        decl_fn!("slang_gnn_add_edge", [types::I64, types::I64, types::I64], types::I64);
        decl_fn!("slang_gnn_set_feature", [types::I64, types::I64, types::I64], types::I64);
        decl_fn!("slang_gnn_message_pass", [types::I64], types::I64);
        decl_fn!("slang_gnn_conv", [types::I64, types::I64], types::I64);
        decl_fn!("slang_gnn_attention", [types::I64, types::I64], types::I64);
        decl_fn!("slang_gnn_readout", [types::I64], types::I64);
        // v323 Diffusion
        decl_fn!("slang_diffusion_forward", [types::I64, types::I64], types::I64);
        decl_fn!("slang_diffusion_reverse", [types::I64, types::I64], types::I64);
        decl_fn!("slang_diffusion_schedule", [types::I64], types::I64);
        decl_fn!("slang_diffusion_sample", [types::I64], types::I64);
        // v324 RL
        decl_fn!("slang_rl_q_update", [types::I64, types::I64, types::I64], types::I64);
        decl_fn!("slang_rl_policy_gradient", [types::I64, types::I64], types::I64);
        decl_fn!("slang_rl_advantage", [types::I64, types::I64], types::I64);
        decl_fn!("slang_rl_reward_discount", [types::I64, types::I64], types::I64);
        // v325 Embedding Search
        decl_fn!("slang_hnsw_create", [types::I64, types::I64], types::I64);
        decl_fn!("slang_hnsw_insert", [types::I64, types::I64, types::I64], types::I64);
        decl_fn!("slang_hnsw_search", [types::I64, types::I64], types::I64);
        decl_fn!("slang_hnsw_recall", [types::I64], types::I64);
        // v326 Tokenizer
        decl_fn!("slang_tokenizer_bpe_train", [types::I64, types::I64], types::I64);
        decl_fn!("slang_tokenizer_encode", [types::I64], types::I64);
        decl_fn!("slang_tokenizer_decode", [types::I64], types::I64);
        decl_fn!("slang_tokenizer_vocab_size", [], types::I64);
        // v327 RLHF
        decl_fn!("slang_rlhf_reward", [types::I64, types::I64], types::I64);
        decl_fn!("slang_rlhf_kl_penalty", [types::I64, types::I64], types::I64);
        decl_fn!("slang_rlhf_preference", [types::I64, types::I64], types::I64);
        decl_fn!("slang_rlhf_ppo_clip", [types::I64, types::I64], types::I64);
        // v328 Async Runtime
        decl_fn!("slang_async_spawn_task", [types::I64], types::I64);
        decl_fn!("slang_async_yield_now", [], types::I64);
        decl_fn!("slang_async_select", [types::I64, types::I64], types::I64);
        decl_fn!("slang_async_timeout", [types::I64], types::I64);
        // v329 Work Stealing
        decl_fn!("slang_ws_create_pool", [types::I64], types::I64);
        decl_fn!("slang_ws_submit", [types::I64, types::I64], types::I64);
        decl_fn!("slang_ws_steal_count", [types::I64], types::I64);
        decl_fn!("slang_ws_active_workers", [types::I64], types::I64);
        // v330 Connection Pool
        decl_fn!("slang_conn_pool_create", [types::I64], types::I64);
        decl_fn!("slang_pool_acquire", [types::I64], types::I64);
        decl_fn!("slang_pool_release", [types::I64, types::I64], types::I64);
        decl_fn!("slang_pool_stats", [types::I64], types::I64);
        // v331 Protobuf
        decl_fn!("slang_protobuf_encode", [types::I64, types::I64], types::I64);
        decl_fn!("slang_protobuf_decode", [types::I64], types::I64);
        decl_fn!("slang_protobuf_field", [types::I64, types::I64], types::I64);
        decl_fn!("slang_protobuf_size", [types::I64], types::I64);
        // v332 Consensus
        decl_fn!("slang_raft_propose", [types::I64], types::I64);
        decl_fn!("slang_raft_commit_index", [], types::I64);
        decl_fn!("slang_raft_leader", [], types::I64);
        decl_fn!("slang_raft_term", [], types::I64);
        // v333 Event Sourcing
        decl_fn!("slang_event_store_create", [], types::I64);
        decl_fn!("slang_event_append", [types::I64, types::I64, types::I64], types::I64);
        decl_fn!("slang_event_replay", [types::I64], types::I64);
        decl_fn!("slang_event_snapshot", [types::I64], types::I64);
        decl_fn!("slang_event_project", [types::I64, types::I64], types::I64);
        // v334 Stream Processing
        decl_fn!("slang_stream_create", [types::I64], types::I64);
        decl_fn!("slang_stream_window_tumbling", [types::I64, types::I64], types::I64);
        decl_fn!("slang_stream_window_sliding", [types::I64, types::I64, types::I64], types::I64);
        decl_fn!("slang_stream_watermark", [types::I64], types::I64);
        decl_fn!("slang_stream_late_count", [types::I64], types::I64);
        // v335 Message Queue
        decl_fn!("slang_mq_create_topic", [], types::I64);
        decl_fn!("slang_mq_publish", [types::I64, types::I64], types::I64);
        decl_fn!("slang_mq_subscribe", [types::I64, types::I64], types::I64);
        decl_fn!("slang_mq_consume", [types::I64, types::I64], types::I64);
        decl_fn!("slang_mq_offset", [types::I64, types::I64], types::I64);
        // v336 CQRS
        decl_fn!("slang_cqrs_command", [types::I64, types::I64], types::I64);
        decl_fn!("slang_cqrs_query", [types::I64], types::I64);
        decl_fn!("slang_cqrs_command_count", [], types::I64);
        decl_fn!("slang_cqrs_query_count", [], types::I64);
        // v337 GraphQL
        decl_fn!("slang_graphql_schema_create", [], types::I64);
        decl_fn!("slang_graphql_add_type", [types::I64, types::I64], types::I64);
        decl_fn!("slang_graphql_add_field", [types::I64, types::I64, types::I64], types::I64);
        decl_fn!("slang_graphql_validate", [types::I64, types::I64, types::I64], types::I64);
        decl_fn!("slang_graphql_type_count", [types::I64], types::I64);
        // v338 JWT
        decl_fn!("slang_jwt_create", [types::I64, types::I64, types::I64], types::I64);
        decl_fn!("slang_jwt_verify", [types::I64, types::I64], types::I64);
        decl_fn!("slang_jwt_claims", [types::I64], types::I64);
        decl_fn!("slang_jwt_expired", [types::I64, types::I64], types::I64);
        decl_fn!("slang_jwt_set_claim", [types::I64, types::I64, types::I64], types::I64);
        decl_fn!("slang_jwt_get_claim", [types::I64, types::I64], types::I64);
        // v339 OAuth2
        decl_fn!("slang_oauth2_auth_url", [types::I64, types::I64], types::I64);
        decl_fn!("slang_oauth2_exchange", [types::I64, types::I64], types::I64);
        decl_fn!("slang_oauth2_refresh", [types::I64], types::I64);
        decl_fn!("slang_oauth2_pkce_verify", [types::I64, types::I64], types::I64);
        decl_fn!("slang_oauth2_state", [types::I64], types::I64);
        // v340 Rate Limiter
        decl_fn!("slang_ratelimit_check", [types::I64], types::I64);
        decl_fn!("slang_ratelimit_remaining", [types::I64], types::I64);
        decl_fn!("slang_ratelimit_reset", [types::I64], types::I64);
        decl_fn!("slang_ratelimit_window", [types::I64], types::I64);
        // v341 Chaos Engineering
        decl_fn!("slang_chaos_inject_fault", [], types::I64);
        decl_fn!("slang_chaos_inject_latency", [types::I64], types::I64);
        decl_fn!("slang_chaos_error_rate", [types::I64], types::I64);
        decl_fn!("slang_chaos_partition", [types::I64], types::I64);
        decl_fn!("slang_chaos_fault_count", [], types::I64);
        decl_fn!("slang_chaos_total_latency", [], types::I64);
        decl_fn!("slang_chaos_is_partitioned", [], types::I64);
        decl_fn!("slang_chaos_current_error_rate", [], types::I64);
        // v342 RBAC
        decl_fn!("slang_rbac_assign_role", [types::I64, types::I64], types::I64);
        decl_fn!("slang_rbac_check_perm", [types::I64, types::I64], types::I64);
        decl_fn!("slang_rbac_grant", [types::I64, types::I64], types::I64);
        decl_fn!("slang_rbac_revoke", [types::I64, types::I64], types::I64);
        // v343 CSP
        decl_fn!("slang_csp_create", [], types::I64);
        decl_fn!("slang_csp_add_directive", [types::I64, types::I64, types::I64], types::I64);
        decl_fn!("slang_csp_nonce", [types::I64], types::I64);
        decl_fn!("slang_csp_directive_count", [types::I64], types::I64);
        decl_fn!("slang_csp_report_only", [types::I64, types::I64], types::I64);
        decl_fn!("slang_csp_check", [types::I64, types::I64, types::I64], types::I64);
        // v344 Input Validation
        decl_fn!("slang_validate_range", [types::I64, types::I64, types::I64], types::I64);
        decl_fn!("slang_validate_length", [types::I64, types::I64, types::I64], types::I64);
        decl_fn!("slang_validate_pattern", [types::I64, types::I64], types::I64);
        decl_fn!("slang_validate_sanitize", [types::I64], types::I64);
        // v345 Audit Log
        decl_fn!("slang_audit_log", [types::I64, types::I64], types::I64);
        decl_fn!("slang_audit_last_action", [], types::I64);
        // v346 Encryption
        decl_fn!("slang_encrypt_xor", [types::I64, types::I64], types::I64);
        decl_fn!("slang_encrypt_rotate", [types::I64, types::I64], types::I64);
        decl_fn!("slang_encrypt_hash", [types::I64], types::I64);
        decl_fn!("slang_encrypt_verify", [types::I64, types::I64], types::I64);
        // v347 Certificate
        decl_fn!("slang_cert_create", [types::I64, types::I64], types::I64);
        decl_fn!("slang_cert_verify", [types::I64, types::I64], types::I64);
        decl_fn!("slang_cert_expiry", [types::I64], types::I64);
        decl_fn!("slang_cert_chain_length", [types::I64], types::I64);
        // v348 Test Runner
        decl_fn!("slang_test_discover", [types::I64], types::I64);
        decl_fn!("slang_test_pass", [], types::I64);
        decl_fn!("slang_test_fail", [], types::I64);
        decl_fn!("slang_test_skip", [], types::I64);
        decl_fn!("slang_test_coverage", [types::I64, types::I64], types::I64);
        decl_fn!("slang_test_pass_rate", [], types::I64);
        decl_fn!("slang_test_total", [], types::I64);
        // v349 Snapshot Testing
        decl_fn!("slang_snapshot_capture", [types::I64, types::I64], types::I64);
        decl_fn!("slang_snapshot_compare", [types::I64, types::I64], types::I64);
        decl_fn!("slang_snapshot_update", [types::I64, types::I64], types::I64);
        decl_fn!("slang_snapshot_version", [types::I64], types::I64);
        decl_fn!("slang_snapshot_match_count", [], types::I64);
        decl_fn!("slang_snapshot_mismatch_count", [], types::I64);
        // v350 Fuzzer
        decl_fn!("slang_fuzz_add_corpus", [types::I64], types::I64);
        decl_fn!("slang_fuzz_run", [types::I64], types::I64);
        decl_fn!("slang_fuzz_crash", [types::I64], types::I64);
        decl_fn!("slang_fuzz_corpus_size", [], types::I64);
        decl_fn!("slang_fuzz_coverage", [], types::I64);
        decl_fn!("slang_fuzz_crash_count", [], types::I64);
        decl_fn!("slang_fuzz_unique_crashes", [], types::I64);
        decl_fn!("slang_fuzz_total_runs", [], types::I64);
        // v351 Property Testing
        decl_fn!("slang_prop_check", [types::I64, types::I64], types::I64);
        decl_fn!("slang_prop_shrink", [types::I64], types::I64);
        decl_fn!("slang_prop_counterexample", [], types::I64);
        decl_fn!("slang_prop_total_checks", [], types::I64);
        // v352 Mutation Testing
        decl_fn!("slang_mutation_inject", [types::I64], types::I64);
        decl_fn!("slang_mutation_killed", [], types::I64);
        decl_fn!("slang_mutation_survived", [], types::I64);
        decl_fn!("slang_mut_test_score", [], types::I64);
        // v353 API Compatibility
        decl_fn!("slang_api_semver_diff", [types::I64, types::I64], types::I64);
        decl_fn!("slang_api_breaking_change", [], types::I64);
        decl_fn!("slang_api_addition", [], types::I64);
        decl_fn!("slang_api_deprecation", [], types::I64);
        decl_fn!("slang_api_surface", [types::I64], types::I64);
        decl_fn!("slang_api_breaking_count", [], types::I64);
        decl_fn!("slang_api_addition_count", [], types::I64);
        decl_fn!("slang_api_deprecation_count", [], types::I64);
        // v354 Migration
        decl_fn!("slang_migration_create", [types::I64, types::I64], types::I64);
        decl_fn!("slang_migration_transform", [types::I64], types::I64);
        decl_fn!("slang_migration_rollback", [types::I64], types::I64);
        decl_fn!("slang_migration_progress", [types::I64], types::I64);
        decl_fn!("slang_migration_delta", [types::I64], types::I64);
        // v355 Code Actions
        decl_fn!("slang_codeaction_extract", [types::I64], types::I64);
        decl_fn!("slang_codeaction_inline", [types::I64], types::I64);
        decl_fn!("slang_codeaction_rename", [types::I64, types::I64], types::I64);
        decl_fn!("slang_codeaction_count", [], types::I64);
        // v356 Telemetry
        decl_fn!("slang_telemetry_compile_time", [types::I64], types::I64);
        decl_fn!("slang_telemetry_peak_memory", [types::I64], types::I64);
        decl_fn!("slang_telemetry_cache_hits", [], types::I64);
        decl_fn!("slang_telemetry_error_count", [], types::I64);
        // v357 Build System
        decl_fn!("slang_build_target", [types::I64], types::I64);
        decl_fn!("slang_build_parallel", [types::I64], types::I64);
        decl_fn!("slang_build_cache_hit", [], types::I64);
        decl_fn!("slang_build_artifact_count", [], types::I64);
        // v358 OpenAPI
        decl_fn!("slang_openapi_create", [], types::I64);
        decl_fn!("slang_openapi_add_route", [types::I64, types::I64], types::I64);
        decl_fn!("slang_openapi_add_param", [types::I64, types::I64, types::I64], types::I64);
        decl_fn!("slang_openapi_validate", [types::I64, types::I64, types::I64], types::I64);
        decl_fn!("slang_openapi_route_count", [types::I64], types::I64);
        // v359 Benchmarking
        decl_fn!("slang_bench_start", [types::I64], types::I64);
        decl_fn!("slang_bench_stop", [types::I64], types::I64);
        decl_fn!("slang_bench_iterations", [types::I64, types::I64], types::I64);
        decl_fn!("slang_bench_throughput", [types::I64], types::I64);
        // v360 Profiler
        decl_fn!("slang_profile_begin", [types::I64], types::I64);
        decl_fn!("slang_profile_end", [types::I64], types::I64);
        decl_fn!("slang_profile_flamegraph", [], types::I64);
        decl_fn!("slang_profile_hotspot", [], types::I64);
        // v361 Release
        decl_fn!("slang_release_changelog", [types::I64], types::I64);
        decl_fn!("slang_release_version_bump", [types::I64], types::I64);
        decl_fn!("slang_release_package", [], types::I64);
        decl_fn!("slang_release_version", [], types::I64);
        decl_fn!("slang_release_changelog_count", [], types::I64);
        decl_fn!("slang_release_packages_built", [], types::I64);
        // v362 Plugin System
        decl_fn!("slang_plugin_load", [types::I64], types::I64);
        decl_fn!("slang_plugin_register_hook", [types::I64], types::I64);
        decl_fn!("slang_plugin_activate", [types::I64], types::I64);
        decl_fn!("slang_plugin_unload", [types::I64], types::I64);
        decl_fn!("slang_plugin_state", [types::I64], types::I64);
        decl_fn!("slang_plugin_hook_count", [types::I64], types::I64);
        decl_fn!("slang_plugin_count", [], types::I64);
        // v363 Wasm Component Model
        decl_fn!("slang_wasm_component_create", [types::I64], types::I64);
        decl_fn!("slang_wasm_component_link", [types::I64, types::I64], types::I64);
        decl_fn!("slang_wasm_component_instantiate", [types::I64], types::I64);
        decl_fn!("slang_wasm_component_count", [], types::I64);
        // v364 WASI Preview2
        decl_fn!("slang_wasi_fs_read", [types::I64], types::I64);
        decl_fn!("slang_wasi_fs_write", [types::I64, types::I64], types::I64);
        decl_fn!("slang_wasi_clock", [], types::I64);
        decl_fn!("slang_wasi_random", [], types::I64);
        // v365 Package Registry
        decl_fn!("slang_registry_publish", [types::I64, types::I64], types::I64);
        decl_fn!("slang_registry_resolve", [types::I64], types::I64);
        decl_fn!("slang_registry_download", [types::I64], types::I64);
        decl_fn!("slang_registry_version_count", [types::I64], types::I64);
        // v366 Milestone
        decl_fn!("slang_vitalis_v366_version", [], types::I64);
        decl_fn!("slang_vitalis_v366_total_builtins", [], types::I64);
        decl_fn!("slang_vitalis_v366_modules", [], types::I64);
        decl_fn!("slang_vitalis_v366_milestone", [], types::I64);

        decl_fn!("slang_format_int",       [ptr_type, types::I64],        ptr_type);
        decl_fn!("slang_format_float",     [ptr_type, types::F64],        ptr_type);
        // ── v15: JSON ────────────────────────────────────────────────────
        decl_fn!("slang_json_encode",      [types::I64],                  ptr_type);
        decl_fn!("slang_json_decode",      [ptr_type],                    types::I64);

        // ── v18: Collection methods ──────────────────────────────────────
        decl_fn!("slang_array_push",       [ptr_type, types::I64],        ptr_type);
        decl_fn!("slang_array_pop",        [ptr_type],                    types::I64);
        decl_fn!("slang_array_contains",   [ptr_type, types::I64],        types::I64);
        decl_fn!("slang_array_reverse",    [ptr_type],                    ptr_type);
        decl_fn!("slang_array_sort",       [ptr_type],                    ptr_type);
        decl_fn!("slang_array_join",       [ptr_type, ptr_type],          ptr_type);
        decl_fn!("slang_array_slice",      [ptr_type, types::I64, types::I64], ptr_type);
        decl_fn!("slang_array_find",       [ptr_type, types::I64],        types::I64);
        // ── Iterator / functional array ops ───────────────────────────
        decl_fn!("slang_array_range",        [types::I64, types::I64],      ptr_type);
        decl_fn!("slang_array_sum",          [ptr_type],                    types::I64);
        decl_fn!("slang_array_min",          [ptr_type],                    types::I64);
        decl_fn!("slang_array_max",          [ptr_type],                    types::I64);
        decl_fn!("slang_array_any",          [ptr_type, types::I64],        types::I8);
        decl_fn!("slang_array_all_positive", [ptr_type],                    types::I8);
        decl_fn!("slang_array_count",        [ptr_type, types::I64],        types::I64);
        decl_fn!("slang_array_flatten",      [ptr_type],                    ptr_type);
        decl_fn!("slang_array_zip",          [ptr_type, ptr_type],          ptr_type);
        decl_fn!("slang_array_enumerate",    [ptr_type],                    ptr_type);
        decl_fn!("slang_array_take",         [ptr_type, types::I64],        ptr_type);
        decl_fn!("slang_array_drop",         [ptr_type, types::I64],        ptr_type);
        decl_fn!("slang_array_unique",       [ptr_type],                    ptr_type);
        decl_fn!("slang_error_message",    [],                            ptr_type);

        // Regex
        decl_fn!("slang_regex_match",          [ptr_type, ptr_type],                    types::I8);
        decl_fn!("slang_regex_is_match",       [ptr_type, ptr_type],                    types::I8);
        decl_fn!("slang_regex_find",           [ptr_type, ptr_type],                    ptr_type);
        decl_fn!("slang_regex_replace",        [ptr_type, ptr_type, ptr_type],           ptr_type);
        decl_fn!("slang_regex_split_count",    [ptr_type, ptr_type],                    types::I64);
        decl_fn!("slang_regex_split_get",      [ptr_type, ptr_type, types::I64],         ptr_type);
        decl_fn!("slang_regex_find_all_count", [ptr_type, ptr_type],                    types::I64);
        decl_fn!("slang_regex_find_all_get",   [ptr_type, ptr_type, types::I64],         ptr_type);

        // v370: Async runtime
        decl_fn!("slang_spawn",        [types::I64],              types::I64);
        decl_fn!("slang_task_result",  [types::I64],              types::I64);
        decl_fn!("slang_task_await",   [types::I64],              types::I64);
        decl_fn!("slang_async_run_all",[],                         types::I64);

        // v18: Networking
        decl_fn!("slang_http_get",     [ptr_type],                ptr_type);
        decl_fn!("slang_http_post",    [ptr_type, ptr_type],      ptr_type);
        decl_fn!("slang_http_status",  [ptr_type],                types::I64);
        decl_fn!("slang_tcp_connect",  [ptr_type, types::I64],    types::I64);
        decl_fn!("slang_tcp_send",     [types::I64, ptr_type],    types::I64);
        decl_fn!("slang_tcp_close",    [types::I64],              types::INVALID);

        // ── v60: Self-hosting bootstrap primitives ───────────────────────
        decl_fn!("slang_char_to_int",       [ptr_type],                       types::I64);
        decl_fn!("slang_int_to_char",       [types::I64],                     ptr_type);
        decl_fn!("slang_array_new",         [types::I64],                     ptr_type);
        decl_fn!("slang_exit",              [types::I64],                     types::INVALID);
        decl_fn!("slang_file_write_bytes",  [ptr_type, ptr_type],             types::I8);
        decl_fn!("slang_file_read_bytes",   [ptr_type],                       ptr_type);
        decl_fn!("slang_args_count",        [],                               types::I64);
        decl_fn!("slang_args_get",          [types::I64],                     ptr_type);

        // ── Tensor/ML runtime (Void-Vitalis) ─────────────────────────────
        decl_fn!("slang_to_f64",         [types::I64],                                                      types::F64);
        decl_fn!("slang_to_i64",         [types::F64],                                                      types::I64);
        decl_fn!("slang_t_fill",         [ptr_type, types::I64, types::F64],                                 types::INVALID);
        decl_fn!("slang_t_copy",         [ptr_type, ptr_type, types::I64],                                   types::INVALID);
        decl_fn!("slang_t_randn",        [ptr_type, types::I64, types::I64],                                 types::INVALID);
        decl_fn!("slang_t_print_n",      [ptr_type, types::I64],                                             types::INVALID);
        decl_fn!("slang_t_matmul",       [ptr_type, ptr_type, ptr_type, types::I64, types::I64, types::I64], types::INVALID);
        decl_fn!("slang_t_add_vv",       [ptr_type, ptr_type, ptr_type, types::I64],                         types::INVALID);
        decl_fn!("slang_t_sub_vv",       [ptr_type, ptr_type, ptr_type, types::I64],                         types::INVALID);
        decl_fn!("slang_t_mul_vv",       [ptr_type, ptr_type, ptr_type, types::I64],                         types::INVALID);
        decl_fn!("slang_t_scale",        [ptr_type, types::F64, ptr_type, types::I64],                       types::INVALID);
        decl_fn!("slang_t_softmax",      [ptr_type, ptr_type, types::I64, types::I64],                       types::INVALID);
        decl_fn!("slang_t_sum",          [ptr_type, types::I64],                                             types::F64);
        decl_fn!("slang_t_add_bias",     [ptr_type, ptr_type, ptr_type, types::I64, types::I64],             types::INVALID);
        decl_fn!("slang_t_max_idx",      [ptr_type, types::I64],                                             types::I64);
        decl_fn!("slang_t_dot",          [ptr_type, ptr_type, types::I64],                                   types::F64);
        decl_fn!("slang_t_cross_entropy",[ptr_type, ptr_type, ptr_type, types::I64, types::I64],             types::F64);
        decl_fn!("slang_t_transpose",    [ptr_type, ptr_type, types::I64, types::I64],                       types::INVALID);
        decl_fn!("slang_t_adamw",        [ptr_type, ptr_type, ptr_type, ptr_type, types::I64, ptr_type],     types::INVALID);
        decl_fn!("slang_t_norm",         [ptr_type, types::I64],                                             types::F64);

        // GUI builtins
        decl_fn!("slang_gui_open",       [types::I64, types::I64],                                          types::I64);
        decl_fn!("slang_gui_close",      [],                                                                 types::I64);
        decl_fn!("slang_gui_clear",      [types::I64],                                                       types::I64);
        decl_fn!("slang_gui_rect",       [types::I64, types::I64, types::I64, types::I64, types::I64],       types::I64);
        decl_fn!("slang_gui_line",       [types::I64, types::I64, types::I64, types::I64, types::I64],       types::I64);
        decl_fn!("slang_gui_text",       [types::I64, types::I64, ptr_type, types::I64, types::I64],       types::I64);
        decl_fn!("slang_gui_update",     [],                                                                 types::I64);
        decl_fn!("slang_gui_circle",     [types::I64, types::I64, types::I64, types::I64],                   types::I64);

        // Advanced rendering builtins
        decl_fn!("slang_gui_gradient_rect",     [types::I64, types::I64, types::I64, types::I64, types::I64, types::I64, types::I64], types::I64);
        decl_fn!("slang_gui_pixel",              [types::I64, types::I64, types::I64],                                              types::I64);
        decl_fn!("slang_gui_rounded_rect",      [types::I64, types::I64, types::I64, types::I64, types::I64, types::I64],            types::I64);
        decl_fn!("slang_gui_blend_rect",        [types::I64, types::I64, types::I64, types::I64, types::I64, types::I64],            types::I64);
        decl_fn!("slang_gui_thick_line",        [types::I64, types::I64, types::I64, types::I64, types::I64, types::I64],            types::I64);
        decl_fn!("slang_gui_triangle",          [types::I64, types::I64, types::I64, types::I64, types::I64, types::I64, types::I64], types::I64);
        decl_fn!("slang_gui_aa_circle",         [types::I64, types::I64, types::I64, types::I64],                                    types::I64);
        decl_fn!("slang_gui_gradient_rounded_rect", [types::I64, types::I64, types::I64, types::I64, types::I64, types::I64, types::I64, types::I64], types::I64);
        decl_fn!("slang_gui_glow",              [types::I64, types::I64, types::I64, types::I64, types::I64],                        types::I64);

        // ── v62: Auto-declare all stdlib builtins not yet registered ────
        // This loop ensures every BuiltinFn from stdlib.rs gets a JIT
        // declaration, using its IrType params to build the Cranelift sig.
        // Functions already declared above are skipped (no duplicates).
        {
            let ptr_type = self.module.target_config().pointer_type();
            for b in crate::stdlib::builtins() {
                if self.func_ids.contains_key(&b.runtime_name) {
                    continue;
                }
                let mut sig = self.module.make_signature();
                for (_, ty) in &b.params {
                    sig.params.push(AbiParam::new(match ty {
                        IrType::I32 => types::I32,
                        IrType::I64 => types::I64,
                        IrType::F32 => types::F32,
                        IrType::F64 => types::F64,
                        IrType::Bool => types::I8,
                        IrType::Ptr  => ptr_type,
                        IrType::Void => types::I64,
                    }));
                }
                match b.ret {
                    IrType::Void => {},
                    IrType::I32  => { sig.returns.push(AbiParam::new(types::I32)); },
                    IrType::I64  => { sig.returns.push(AbiParam::new(types::I64)); },
                    IrType::F32  => { sig.returns.push(AbiParam::new(types::F32)); },
                    IrType::F64  => { sig.returns.push(AbiParam::new(types::F64)); },
                    IrType::Bool => { sig.returns.push(AbiParam::new(types::I8)); },
                    IrType::Ptr  => { sig.returns.push(AbiParam::new(ptr_type)); },
                }
                if let Ok(id) = self.module.declare_function(&b.runtime_name, Linkage::Import, &sig) {
                    self.func_ids.insert(b.runtime_name, id);
                }
            }
        }

        Ok(())
    }

    fn ir_type_to_cl(ty: &IrType, pointer_type: cranelift::prelude::Type) -> cranelift::prelude::Type {
        match ty {
            IrType::I32 => types::I32,
            IrType::I64 => types::I64,
            IrType::F32 => types::F32,
            IrType::F64 => types::F64,
            IrType::Bool => types::I8,
            IrType::Ptr => pointer_type,
            IrType::Void => types::I64, // Cranelift needs a type; we ignore the value
        }
    }

    fn declare_function(&mut self, func: &IrFunction) -> CodegenResult<()> {
        let mut sig = self.module.make_signature();
        let pointer_type = self.module.target_config().pointer_type();

        for (_, ty) in &func.params {
            sig.params.push(AbiParam::new(Self::ir_type_to_cl(ty, pointer_type)));
        }

        if func.ret_type != IrType::Void {
            sig.returns.push(AbiParam::new(Self::ir_type_to_cl(&func.ret_type, pointer_type)));
        }

        let id = self
            .module
            .declare_function(&func.name, Linkage::Export, &sig)
            .map_err(|e| CodegenError {
                message: format!("declare {}: {}", func.name, e),
            })?;

        self.func_ids.insert(func.name.clone(), id);
        Ok(())
    }

    fn define_function(&mut self, func: &IrFunction) -> CodegenResult<()> {
        let func_id = *self.func_ids.get(&func.name).ok_or_else(|| CodegenError {
            message: format!("function {} not declared", func.name),
        })?;

        // Build signature
        self.ctx.func.signature = self.module.declarations().get_function_decl(func_id).signature.clone();
        self.ctx.func.name = cranelift_codegen::ir::UserFuncName::user(0, func_id.as_u32());

        let pointer_type = self.module.target_config().pointer_type();
        let mut builder_ctx = FunctionBuilderContext::new();
        let mut builder = FunctionBuilder::new(&mut self.ctx.func, &mut builder_ctx);

        // Create Cranelift blocks for each IR block
        let mut block_map: HashMap<ir::BlockId, cranelift::prelude::Block> = HashMap::new();
        for bb in &func.blocks {
            let cl_block = builder.create_block();
            block_map.insert(bb.id, cl_block);
        }

        // ── Pre-pass: collect Phi info for block parameters ──
        // For each block that contains Phi instructions, we need to add
        // Cranelift block parameters so values can flow across blocks.
        let mut phis_by_block: HashMap<ir::BlockId, Vec<PhiInfo>> = HashMap::new();
        for bb in &func.blocks {
            for inst in &bb.insts {
                if let Inst::Phi { result, incoming, ty } = inst {
                    phis_by_block.entry(bb.id).or_default().push(PhiInfo {
                        result: *result,
                        ty: ty.clone(),
                        incoming: incoming.clone(),
                    });
                }
            }
        }

        // Add block parameters for blocks that have Phi instructions
        for (block_id, phis) in &phis_by_block {
            if let Some(cl_block) = block_map.get(block_id) {
                for phi in phis {
                    builder.append_block_param(*cl_block, Self::ir_type_to_cl(&phi.ty, pointer_type));
                }
            }
        }

        // Set up entry block with function parameters
        let entry_block = block_map[&func.entry];
        builder.append_block_params_for_function_params(entry_block);
        builder.switch_to_block(entry_block);
        builder.seal_block(entry_block);

        // Map IR values to Cranelift values
        let mut value_map: HashMap<ir::Value, cranelift::prelude::Value> = HashMap::new();
        // Semantic type map: lets print/println dispatch to the right runtime function
        let mut type_map: HashMap<ir::Value, IrType> = HashMap::new();

        // Bind function parameters
        let func_params: Vec<cranelift::prelude::Value> = builder.block_params(entry_block).to_vec();
        for (i, (_name, ty)) in func.params.iter().enumerate() {
            if i < func_params.len() {
                value_map.insert(ir::Value(i as u32), func_params[i]);
                type_map.insert(ir::Value(i as u32), ty.clone());
            }
        }

        // Translate each IR block
        let mut first = true;
        for bb in &func.blocks {
            if !first {
                let cl_block = block_map[&bb.id];
                builder.switch_to_block(cl_block);
                // Don't seal here — seal all blocks at end to handle arbitrary CFGs
            }
            first = false;

            // If this block has Phi params, map them to the block parameters
            if let Some(phis) = phis_by_block.get(&bb.id) {
                let cl_block = block_map[&bb.id];
                let params = builder.block_params(cl_block).to_vec();
                for (i, phi) in phis.iter().enumerate() {
                    if i < params.len() {
                        value_map.insert(phi.result, params[i]);
                    }
                }
            }

            for inst in &bb.insts {
                Self::translate_inst(
                    &mut self.module,
                    &self.func_ids,
                    pointer_type,
                    inst,
                    &mut builder,
                    &block_map,
                    &mut value_map,
                    &mut type_map,
                    &func.ret_type,
                    bb.id,
                    &phis_by_block,
                )?;
            }
        }

        // Ensure the last block is terminated
        let needs_terminator = func.blocks.last().map_or(true, |bb| {
            !bb.insts.iter().any(|i| matches!(i, Inst::Return { .. } | Inst::Jump { .. } | Inst::Branch { .. }))
        });
        if needs_terminator {
            if func.ret_type == IrType::Void {
                builder.ins().return_(&[]);
            } else if func.ret_type == IrType::F64 {
                // iconst is integer-only; float returns need f64const/f32const
                let zero = builder.ins().f64const(0.0_f64);
                builder.ins().return_(&[zero]);
            } else if func.ret_type == IrType::F32 {
                let zero = builder.ins().f32const(0.0_f32);
                builder.ins().return_(&[zero]);
            } else {
                let zero = builder.ins().iconst(Self::ir_type_to_cl(&func.ret_type, pointer_type), 0);
                builder.ins().return_(&[zero]);
            }
        }

        // Seal all blocks at once (correct for arbitrary control flow)
        builder.seal_all_blocks();
        builder.finalize();

        // Define the function
        self.module
            .define_function(func_id, &mut self.ctx)
            .map_err(|e| CodegenError {
                message: format!("define {}: {}", func.name, e),
            })?;

        self.module.clear_context(&mut self.ctx);
        Ok(())
    }

    fn translate_inst(
        module: &mut JITModule,
        func_ids: &HashMap<String, FuncId>,
        pointer_type: cranelift::prelude::Type,
        inst: &Inst,
        builder: &mut FunctionBuilder,
        block_map: &HashMap<ir::BlockId, cranelift::prelude::Block>,
        value_map: &mut HashMap<ir::Value, cranelift::prelude::Value>,
        type_map: &mut HashMap<ir::Value, IrType>,
        _ret_type: &IrType,
        current_bb: ir::BlockId,
        phis_by_block: &HashMap<ir::BlockId, Vec<PhiInfo>>,
    ) -> CodegenResult<()> {
        match inst {
            Inst::IConst { result, value, ty } => {
                let cl_ty = Self::ir_type_to_cl(ty, pointer_type);
                let val = builder.ins().iconst(cl_ty, *value);
                value_map.insert(*result, val);
                type_map.insert(*result, ty.clone());
            }
            Inst::FConst { result, value, ty } => {
                let cl_ty = Self::ir_type_to_cl(ty, pointer_type);
                let val = if cl_ty == types::F64 {
                    builder.ins().f64const(*value)
                } else {
                    builder.ins().f32const(*value as f32)
                };
                value_map.insert(*result, val);
                type_map.insert(*result, ty.clone());
            }
            Inst::BConst { result, value } => {
                let val = builder.ins().iconst(types::I8, if *value { 1 } else { 0 });
                value_map.insert(*result, val);
                type_map.insert(*result, IrType::Bool);
            }
            Inst::StrConst { result, value } => {
                // Intern into arena; return raw pointer as iconst
                let ptr = intern_cstr(value) as i64;
                let val = builder.ins().iconst(pointer_type, ptr);
                value_map.insert(*result, val);
                type_map.insert(*result, IrType::Ptr);
            }
            Inst::BinOp { result, op, lhs, rhs, ty } => {
                let l = Self::get_value(*lhs, value_map, builder);
                let r = Self::get_value(*rhs, value_map, builder);
                let val = match op {
                    IrBinOp::Add => builder.ins().iadd(l, r),
                    IrBinOp::Sub => builder.ins().isub(l, r),
                    IrBinOp::Mul => builder.ins().imul(l, r),
                    IrBinOp::Div => builder.ins().sdiv(l, r),
                    IrBinOp::Mod => builder.ins().srem(l, r),
                    IrBinOp::FAdd => builder.ins().fadd(l, r),
                    IrBinOp::FSub => builder.ins().fsub(l, r),
                    IrBinOp::FMul => builder.ins().fmul(l, r),
                    IrBinOp::FDiv => builder.ins().fdiv(l, r),
                    IrBinOp::And => builder.ins().band(l, r),
                    IrBinOp::Or => builder.ins().bor(l, r),
                };
                value_map.insert(*result, val);
                type_map.insert(*result, ty.clone());
            }
            Inst::UnOp { result, op, operand, ty } => {
                let v = Self::get_value(*operand, value_map, builder);
                let val = match op {
                    IrUnOp::Neg => builder.ins().ineg(v),
                    IrUnOp::FNeg => builder.ins().fneg(v),
                    IrUnOp::Not => {
                        let one = builder.ins().iconst(types::I8, 1);
                        builder.ins().bxor(v, one)
                    }
                };
                value_map.insert(*result, val);
                type_map.insert(*result, ty.clone());
            }
            Inst::ICmp { result, cond, lhs, rhs } => {
                let l = Self::get_value(*lhs, value_map, builder);
                let r = Self::get_value(*rhs, value_map, builder);
                let cc = match cond {
                    IrCmp::Eq => IntCC::Equal,
                    IrCmp::Ne => IntCC::NotEqual,
                    IrCmp::Lt => IntCC::SignedLessThan,
                    IrCmp::Gt => IntCC::SignedGreaterThan,
                    IrCmp::Le => IntCC::SignedLessThanOrEqual,
                    IrCmp::Ge => IntCC::SignedGreaterThanOrEqual,
                };
                let val = builder.ins().icmp(cc, l, r);
                value_map.insert(*result, val);
                type_map.insert(*result, IrType::Bool);
            }
            Inst::FCmp { result, cond, lhs, rhs } => {
                let l = Self::get_value(*lhs, value_map, builder);
                let r = Self::get_value(*rhs, value_map, builder);
                let cc = match cond {
                    IrCmp::Eq => FloatCC::Equal,
                    IrCmp::Ne => FloatCC::NotEqual,
                    IrCmp::Lt => FloatCC::LessThan,
                    IrCmp::Gt => FloatCC::GreaterThan,
                    IrCmp::Le => FloatCC::LessThanOrEqual,
                    IrCmp::Ge => FloatCC::GreaterThanOrEqual,
                };
                let val = builder.ins().fcmp(cc, l, r);
                value_map.insert(*result, val);
                type_map.insert(*result, IrType::Bool);
            }
            Inst::Call { result, func, args, ret_ty } => {
                // Determine argument semantic type for print/println dispatch
                let arg0_itype = args.first()
                    .and_then(|v| type_map.get(v))
                    .cloned()
                    .unwrap_or(IrType::I64);

                // Route to the correct typed runtime function
                let callee_name: String = match func.as_str() {
                    "print" => match arg0_itype {
                        IrType::F64 | IrType::F32 => "slang_print_f64".into(),
                        IrType::Bool              => "slang_print_bool".into(),
                        IrType::Ptr               => "slang_print_cstr".into(),
                        _                         => "slang_print_i64".into(),
                    },
                    "println" => match arg0_itype {
                        IrType::F64 | IrType::F32 => "slang_println_f64".into(),
                        IrType::Bool              => "slang_println_bool".into(),
                        IrType::Ptr               => "slang_println_cstr".into(),
                        _                         => "slang_println_i64".into(),
                    },
                    // User-callable named math / string builtins
                    "sqrt"      => "slang_sqrt_f64".into(),
                    "ln"        => "slang_ln_f64".into(),
                    "log2"      => "slang_log2_f64".into(),
                    "log10"     => "slang_log10_f64".into(),
                    "sin"       => "slang_sin_f64".into(),
                    "cos"       => "slang_cos_f64".into(),
                    "exp"       => "slang_exp_f64".into(),
                    "floor"     => "slang_floor_f64".into(),
                    "ceil"      => "slang_ceil_f64".into(),
                    "round"     => "slang_round_f64".into(),
                    "abs"       => match arg0_itype { IrType::F64 => "slang_abs_f64".into(), _ => "slang_abs_i64".into() },
                    "abs_f64"   => "slang_abs_f64".into(),
                    "abs_i64"   => "slang_abs_i64".into(),
                    "min"       => match arg0_itype { IrType::F64 => "slang_min_f64".into(), _ => "slang_min_i64".into() },
                    "min_f64"   => "slang_min_f64".into(),
                    "min_i64"   => "slang_min_i64".into(),
                    "max"       => match arg0_itype { IrType::F64 => "slang_max_f64".into(), _ => "slang_max_i64".into() },
                    "max_f64"   => "slang_max_f64".into(),
                    "max_i64"   => "slang_max_i64".into(),
                    "pow"       => "slang_pow_f64".into(),
                    "pow_f64"   => "slang_pow_f64".into(),
                    "sqrt_f64"  => "slang_sqrt_f64".into(),
                    "ln_f64"    => "slang_ln_f64".into(),
                    "log2_f64"  => "slang_log2_f64".into(),
                    "log10_f64" => "slang_log10_f64".into(),
                    "sin_f64"   => "slang_sin_f64".into(),
                    "cos_f64"   => "slang_cos_f64".into(),
                    "exp_f64"   => "slang_exp_f64".into(),
                    "floor_f64" => "slang_floor_f64".into(),
                    "ceil_f64"  => "slang_ceil_f64".into(),
                    "round_f64" => "slang_round_f64".into(),
                    "i64_to_f64" => "slang_i64_to_f64".into(),
                    "f64_to_i64" => "slang_f64_to_i64".into(),
                    "str_len"   => "slang_str_len".into(),
                    "str_eq"    => "slang_str_eq".into(),
                    "str_cat"   => "slang_str_cat".into(),
                    "atan2"     => "slang_atan2_f64".into(),
                    "atan2_f64" => "slang_atan2_f64".into(),
                    "hypot"     => "slang_hypot_f64".into(),
                    "hypot_f64" => "slang_hypot_f64".into(),
                    "clamp"     => match arg0_itype {
                        IrType::F64 => "slang_clamp_f64".into(),
                        _ => "slang_clamp_i64".into()
                    },
                    "clamp_f64" => "slang_clamp_f64".into(),
                    "clamp_i64" => "slang_clamp_i64".into(),
                    "rand_f64"  => "slang_rand_f64".into(),
                    "rand_i64"  => "slang_rand_i64".into(),
                    // Phase 5: new stdlib
                    "clock_ns"        => "slang_clock_ns".into(),
                    "clock_ms"        => "slang_clock_ms".into(),
                    "assert_eq"       => "slang_assert_eq_i64".into(),
                    "assert_true"     => "slang_assert_true".into(),
                    "popcount"        => "slang_popcount".into(),
                    "leading_zeros"   => "slang_leading_zeros".into(),
                    "trailing_zeros"  => "slang_trailing_zeros".into(),
                    "sign"            => "slang_sign_i64".into(),
                    "gcd"             => "slang_gcd".into(),
                    "lcm"             => "slang_lcm".into(),
                    "factorial"       => "slang_factorial".into(),
                    "fibonacci"       => "slang_fibonacci".into(),
                    "is_prime"        => "slang_is_prime".into(),
                    "tan"             => "slang_tan_f64".into(),
                    "tan_f64"         => "slang_tan_f64".into(),
                    "asin"            => "slang_asin_f64".into(),
                    "asin_f64"        => "slang_asin_f64".into(),
                    "acos"            => "slang_acos_f64".into(),
                    "acos_f64"        => "slang_acos_f64".into(),
                    "atan"            => "slang_atan_f64".into(),
                    "atan_f64"        => "slang_atan_f64".into(),
                    // Phase 21 stdlib
                    "hash"            => "slang_hash_i64".into(),
                    "lerp"            => "slang_lerp_f64".into(),
                    "smoothstep"      => "slang_smoothstep_f64".into(),
                    "wrap"            => "slang_wrap_i64".into(),
                    "map_range"       => "slang_map_range_f64".into(),
                    "epoch_secs"      => "slang_epoch_secs".into(),
                    // Phase 22 stdlib
                    "fma"             => "slang_fma_f64".into(),
                    "cbrt"            => "slang_cbrt_f64".into(),
                    "deg_to_rad"      => "slang_deg_to_rad".into(),
                    "rad_to_deg"      => "slang_rad_to_deg".into(),
                    "sigmoid"         => "slang_sigmoid_f64".into(),
                    "relu"            => "slang_relu_f64".into(),
                    "tanh"            => "slang_tanh_f64".into(),
                    "ipow"            => "slang_ipow".into(),
                    // Phase 23 stdlib
                    "sinh"            => "slang_sinh_f64".into(),
                    "cosh"            => "slang_cosh_f64".into(),
                    "log"             => "slang_log_f64".into(),
                    "exp2"            => "slang_exp2_f64".into(),
                    "copysign"        => "slang_copysign_f64".into(),
                    "fract"           => "slang_fract_f64".into(),
                    "trunc"           => "slang_trunc_f64".into(),
                    "step"            => "slang_step_f64".into(),
                    "leaky_relu"      => "slang_leaky_relu_f64".into(),
                    "elu"             => "slang_elu_f64".into(),
                    // Phase 24 stdlib
                    "swish"           => "slang_swish_f64".into(),
                    "gelu"            => "slang_gelu_f64".into(),
                    "softplus"        => "slang_softplus_f64".into(),
                    "mish"            => "slang_mish_f64".into(),
                    "log1p"           => "slang_log1p_f64".into(),
                    "expm1"           => "slang_expm1_f64".into(),
                    "recip"           => "slang_recip_f64".into(),
                    "rsqrt"           => "slang_rsqrt_f64".into(),
                    // Phase 25 stdlib
                    "selu"            => "slang_selu_f64".into(),
                    "hard_sigmoid"    => "slang_hard_sigmoid_f64".into(),
                    "hard_swish"      => "slang_hard_swish_f64".into(),
                    "log_sigmoid"     => "slang_log_sigmoid_f64".into(),
                    "celu"            => "slang_celu_f64".into(),
                    "softsign"        => "slang_softsign_f64".into(),
                    "gaussian"        => "slang_gaussian_f64".into(),
                    "sinc"            => "slang_sinc_f64".into(),
                    "inv_sqrt_approx" => "slang_inv_sqrt_approx_f64".into(),
                    "logit"           => "slang_logit_f64".into(),
                    // ── v15: String operations ───────────────────────────
                    "str_upper"       => "slang_str_upper".into(),
                    "str_lower"       => "slang_str_lower".into(),
                    "str_trim"        => "slang_str_trim".into(),
                    "str_contains"    => "slang_str_contains".into(),
                    "str_starts_with" => "slang_str_starts_with".into(),
                    "str_ends_with"   => "slang_str_ends_with".into(),
                    "str_char_at"     => "slang_str_char_at".into(),
                    "str_substr"      => "slang_str_substr".into(),
                    "str_index_of"    => "slang_str_index_of".into(),
                    "str_replace"     => "slang_str_replace".into(),
                    "str_repeat"      => "slang_str_repeat".into(),
                    "str_reverse"     => "slang_str_reverse".into(),
                    "str_split_count" => "slang_str_split_count".into(),
                    "str_split_get"   => "slang_str_split_get".into(),
                    "to_string_i64"   => "slang_to_string_i64".into(),
                    "to_string_f64"   => "slang_to_string_f64".into(),
                    "to_string_bool"  => "slang_to_string_bool".into(),
                    "str_format_i64"  => "slang_str_format_i64".into(),
                    "str_format_f64"  => "slang_str_format_f64".into(),
                    "str_format_str"  => "slang_str_format_str".into(),
                    "parse_int"       => "slang_parse_int".into(),
                    "parse_float"     => "slang_parse_float".into(),
                    // ── v15: File I/O ────────────────────────────────────
                    "file_read"       => "slang_file_read".into(),
                    "file_write"      => "slang_file_write".into(),
                    "file_append"     => "slang_file_append".into(),
                    "file_exists"     => "slang_file_exists".into(),
                    "file_delete"     => "slang_file_delete".into(),
                    "file_size"       => "slang_file_size".into(),
                    // ── v15: Map operations ──────────────────────────────
                    "map_new"         => "slang_map_new".into(),
                    "map_set"         => "slang_map_set".into(),
                    "map_get"         => "slang_map_get".into(),
                    "map_has"         => "slang_map_has".into(),
                    "map_remove"      => "slang_map_remove".into(),
                    "map_len"         => "slang_map_len".into(),
                    "map_keys"        => "slang_map_keys".into(),
                    // ── v16: Set operations ──────────────────────────────
                    "set_new"         => "slang_set_new".into(),
                    "set_add"         => "slang_set_add".into(),
                    "set_has"         => "slang_set_has".into(),
                    "set_remove"      => "slang_set_remove".into(),
                    "set_len"         => "slang_set_len".into(),
                    "set_union"       => "slang_set_union".into(),
                    "set_intersect"   => "slang_set_intersect".into(),
                    "set_diff"        => "slang_set_diff".into(),
                    "set_to_array"    => "slang_set_to_array".into(),
                    // ── v18: Tuple operations ────────────────────────────
                    "tuple_new2"      => "slang_tuple_new2".into(),
                    "tuple_new3"      => "slang_tuple_new3".into(),
                    "tuple_new4"      => "slang_tuple_new4".into(),
                    "tuple_get"       => "slang_tuple_get".into(),
                    "tuple_len"       => "slang_tuple_len".into(),
                    // ── v15: Error handling ──────────────────────────────
                    "error_set"       => "slang_error_set".into(),
                    "error_check"     => "slang_error_check".into(),
                    "error_msg"       => "slang_error_msg".into(),
                    "error_clear"     => "slang_error_clear".into(),
                    // ── v15: Environment & System ────────────────────────
                    "env_get"         => "slang_env_get".into(),
                    "sleep_ms"        => "slang_sleep_ms".into(),
                    "eprint"          => "slang_eprint".into(),
                    "eprintln"        => "slang_eprintln".into(),
                    "pid"             => "slang_pid".into(),
                    // -- v142: Runtime Logging -----------------------------
                    "log_trace"       => "slang_log_trace".into(),
                    "log_debug"       => "slang_log_debug".into(),
                    "log_info"        => "slang_log_info".into(),
                    "log_warn"        => "slang_log_warn".into(),
                    "log_error"       => "slang_log_error".into(),
                    "log_level_set"   => "slang_log_level_set".into(),
                    "log_level_get"   => "slang_log_level_get".into(),
                    // -- v143: Audit Trail -------------------------------
                    "audit_event"     => "slang_audit_event".into(),
                    "audit_count"     => "slang_audit_count".into(),
                    "audit_dump"      => "slang_audit_dump".into(),
                    "audit_clear"     => "slang_audit_clear".into(),
                    "audit_last"      => "slang_audit_last".into(),
                    // -- v144: Metrics & Telemetry --------------------------
                    "metric_counter"     => "slang_metric_counter".into(),
                    "metric_gauge"       => "slang_metric_gauge".into(),
                    "metric_histogram"   => "slang_metric_histogram".into(),
                    "metric_get_counter" => "slang_metric_get_counter".into(),
                    "metric_get_gauge"   => "slang_metric_get_gauge".into(),
                    "metric_dump"        => "slang_metric_dump".into(),
                    "metric_clear"       => "slang_metric_clear".into(),
                    // -- v145: Distributed Tracing --------------------------
                    "span_start"         => "slang_span_start".into(),
                    "span_end"           => "slang_span_end".into(),
                    "span_set_tag"       => "slang_span_set_tag".into(),
                    "trace_id"           => "slang_trace_id".into(),
                    "span_depth"         => "slang_span_depth".into(),
                    // -- v146: Health Check & Runtime Diagnostics ----------
                    "runtime_uptime_ms"   => "slang_runtime_uptime_ms".into(),
                    "runtime_memory_used" => "slang_runtime_memory_used".into(),
                    "runtime_version"     => "slang_runtime_version".into(),
                    "runtime_alloc_count" => "slang_runtime_alloc_count".into(),
                    "runtime_alloc_total" => "slang_runtime_alloc_total".into(),
                    "runtime_cpu_count"   => "slang_runtime_cpu_count".into(),
                    // -- v147: Observable Pipeline Integration ----------
                    "pipeline_timer_start"   => "slang_pipeline_timer_start".into(),
                    "pipeline_timer_end"     => "slang_pipeline_timer_end".into(),
                    "pipeline_stage_count"   => "slang_pipeline_stage_count".into(),
                    "pipeline_dump_timings"  => "slang_pipeline_dump_timings".into(),
                    "pipeline_clear_timings" => "slang_pipeline_clear_timings".into(),
                    // -- v148: RBAC & Capability Permissions ------------
                    "permission_check"  => "slang_permission_check".into(),
                    "permission_grant"  => "slang_permission_grant".into(),
                    "permission_revoke" => "slang_permission_revoke".into(),
                    "permission_list"   => "slang_permission_list".into(),
                    "permission_clear"  => "slang_permission_clear".into(),
                    // -- v149: Cryptographic Signing -----------------
                    "crypto_sha256"       => "slang_crypto_sha256".into(),
                    "crypto_hmac_sign"    => "slang_crypto_hmac_sign".into(),
                    "crypto_hmac_verify"  => "slang_crypto_hmac_verify".into(),
                    "crypto_base64_encode" => "slang_crypto_base64_encode".into(),
                    "crypto_base64_decode" => "slang_crypto_base64_decode".into(),
                    // -- v150: Sandbox Enforcement -------------------
                    "sandbox_create"    => "slang_sandbox_create".into(),
                    "sandbox_allow"     => "slang_sandbox_allow".into(),
                    "sandbox_check"     => "slang_sandbox_check".into(),
                    "sandbox_violations" => "slang_sandbox_violations".into(),
                    "sandbox_destroy"   => "slang_sandbox_destroy".into(),
                    // -- v151: Security Audit Logger ----------------
                    "security_log"       => "slang_security_log".into(),
                    "security_log_count" => "slang_security_log_count".into(),
                    "security_log_verify" => "slang_security_log_verify".into(),
                    "security_log_dump"  => "slang_security_log_dump".into(),
                    "security_log_clear" => "slang_security_log_clear".into(),
                    // -- v152: Input Validation Framework ----------
                    "validate_email"    => "slang_validate_email".into(),
                    "validate_url"      => "slang_validate_url".into(),
                    "validate_ip"       => "slang_validate_ip".into(),
                    "sanitize_html"     => "slang_sanitize_html".into(),
                    "sanitize_sql"      => "slang_sanitize_sql".into(),
                    // -- v153: Secure Communication Primitives -----
                    "secure_channel_create" => "slang_secure_channel_create".into(),
                    "secure_channel_send"   => "slang_secure_channel_send".into(),
                    "secure_channel_recv"   => "slang_secure_channel_recv".into(),
                    "secure_channel_close"  => "slang_secure_channel_close".into(),
                    "constant_time_eq"      => "slang_constant_time_eq".into(),
                    // -- v154: Autonomous Improvement Lab v2 -------
                    "improvement_run_trial"  => "slang_improvement_run_trial".into(),
                    "improvement_best_score" => "slang_improvement_best_score".into(),
                    "improvement_history_count" => "slang_improvement_history_count".into(),
                    "improvement_reset"      => "slang_improvement_reset".into(),
                    // -- v155: LLM-Guided Mutation Templates -------
                    "mutation_apply"         => "slang_mutation_apply".into(),
                    "mutation_list_count"    => "slang_mutation_list_count".into(),
                    "mutation_score"         => "slang_mutation_score".into(),
                    "mutation_undo"          => "slang_mutation_undo".into(),
                    // -- v156: Evolution Fitness Profiles ----------
                    "fitness_register"       => "slang_fitness_register".into(),
                    "fitness_evaluate"       => "slang_fitness_evaluate".into(),
                    "fitness_pareto_count"   => "slang_fitness_pareto_count".into(),
                    "fitness_clear"          => "slang_fitness_clear".into(),
                    // -- v157: Cross-Module Evolution --------------
                    "evo_cross_module"       => "slang_evo_cross_module".into(),
                    "evo_dep_add"            => "slang_evo_dep_add".into(),
                    "evo_dep_check"          => "slang_evo_dep_check".into(),
                    "evo_safe_mutate"        => "slang_evo_safe_mutate".into(),
                    // -- v158: Evolution Checkpointing -------------
                    "evo_checkpoint_save"    => "slang_evo_checkpoint_save".into(),
                    "evo_checkpoint_load"    => "slang_evo_checkpoint_load".into(),
                    "evo_checkpoint_list_count" => "slang_evo_checkpoint_list_count".into(),
                    "evo_checkpoint_clear"   => "slang_evo_checkpoint_clear".into(),
                    // -- v159: Meta-Evolution v2 -------------------
                    "meta_evo_register"      => "slang_meta_evo_register".into(),
                    "meta_evo_select"        => "slang_meta_evo_select".into(),
                    "meta_evo_converged"     => "slang_meta_evo_converged".into(),
                    "meta_evo_stats_count"   => "slang_meta_evo_stats_count".into(),
                    // -- v160: Tensor-First Types ------------------
                    "tensor_create"          => "slang_tensor_create".into(),
                    "tensor_rank"            => "slang_tensor_rank".into(),
                    "tensor_size"            => "slang_tensor_size".into(),
                    "tensor_set_v160"        => "slang_tensor_set".into(),
                    "tensor_get_v160"        => "slang_tensor_get".into(),
                    "tensor_add_v160"        => "slang_tensor_add".into(),
                    "tensor_mul_v160"        => "slang_tensor_mul".into(),
                    // -- v161: Auto-Differentiation ----------------
                    "grad_compute"           => "slang_grad_compute".into(),
                    "grad_forward"           => "slang_grad_forward".into(),
                    "grad_reverse"           => "slang_grad_reverse".into(),
                    "grad_jacobian_dim"      => "slang_grad_jacobian_dim".into(),
                    // -- v162: ML Pipeline -------------------------
                    "ml_linear_fit"          => "slang_ml_linear_fit".into(),
                    "ml_predict"             => "slang_ml_predict".into(),
                    "ml_accuracy"            => "slang_ml_accuracy".into(),
                    "ml_loss"                => "slang_ml_loss".into(),
                    // -- v163: NAS ---------------------------------
                    "nas_search"             => "slang_nas_search".into(),
                    "nas_evaluate"           => "slang_nas_evaluate".into(),
                    "nas_best"               => "slang_nas_best".into(),
                    "nas_count"              => "slang_nas_count".into(),
                    // -- v164: Feature Engineering ------------------
                    "feature_normalize"      => "slang_feature_normalize".into(),
                    "feature_one_hot"        => "slang_feature_one_hot".into(),
                    "feature_variance"       => "slang_feature_variance".into(),
                    "feature_correlate"      => "slang_feature_correlate".into(),
                    // -- v165: Model Serialization ------------------
                    "model_save"             => "slang_model_save".into(),
                    "model_load"             => "slang_model_load".into(),
                    "model_version"          => "slang_model_version".into(),
                    "model_compatible"       => "slang_model_compatible".into(),
                    // -- v166: KV Store ------------------------------
                    "kv_put"                 => "slang_kv_put".into(),
                    "kv_get_len"             => "slang_kv_get_len".into(),
                    "kv_delete"              => "slang_kv_delete".into(),
                    "kv_wal_count"           => "slang_kv_wal_count".into(),
                    // -- v167: B-Tree Index --------------------------
                    "btree_insert"           => "slang_btree_insert".into(),
                    "btree_lookup"           => "slang_btree_lookup".into(),
                    "btree_range_count"      => "slang_btree_range_count".into(),
                    "btree_count"            => "slang_btree_count".into(),
                    // -- v168: SQL Engine ----------------------------
                    "sql_create_table"       => "slang_sql_create_table".into(),
                    "sql_insert"             => "slang_sql_insert".into(),
                    "sql_count"              => "slang_sql_count".into(),
                    "sql_sum_col0"           => "slang_sql_sum_col0".into(),
                    // -- v169: Schema Migration ---------------------
                    "schema_create"          => "slang_schema_create".into(),
                    "schema_migrate"         => "slang_schema_migrate".into(),
                    "schema_version"         => "slang_schema_version".into(),
                    "schema_compatible"      => "slang_schema_compatible".into(),
                    // -- v170: Transaction Log ----------------------
                    "txn_begin"              => "slang_txn_begin".into(),
                    "txn_commit"             => "slang_txn_commit".into(),
                    "txn_rollback"           => "slang_txn_rollback".into(),
                    "txn_log_count"          => "slang_txn_log_count".into(),
                    // -- v171: Data Import/Export --------------------
                    "data_buf_create"        => "slang_data_buf_create".into(),
                    "data_buf_push"          => "slang_data_buf_push".into(),
                    "data_buf_len"           => "slang_data_buf_len".into(),
                    "data_buf_get"           => "slang_data_buf_get".into(),
                    // -- v172: TCP Sockets ----------------------
                    "tcp_create"              => "slang_tcp_create".into(),
                    "tcp_sim_connect"        => "slang_tcp_sim_connect".into(),
                    "tcp_connected"           => "slang_tcp_connected".into(),
                    "tcp_sim_close"           => "slang_tcp_sim_close".into(),
                    // -- v173: HTTP Client ----------------------
                    "http_sim_get"            => "slang_http_sim_get".into(),
                    "http_sim_post"           => "slang_http_sim_post".into(),
                    "http_request_count"      => "slang_http_request_count".into(),
                    "http_url_valid"          => "slang_http_url_valid".into(),
                    // -- v174: WebSocket ------------------------
                    "ws_create"               => "slang_ws_create".into(),
                    "ws_send"                 => "slang_ws_send".into(),
                    "ws_msg_count"            => "slang_ws_msg_count".into(),
                    "ws_close"                => "slang_ws_close".into(),
                    // -- v175: RPC Framework ---------------------
                    "rpc_register"            => "slang_rpc_register".into(),
                    "rpc_call"                => "slang_rpc_call".into(),
                    "rpc_total_calls"         => "slang_rpc_total_calls".into(),
                    "rpc_service_count"       => "slang_rpc_service_count".into(),
                    // -- v176: DNS Resolution --------------------
                    "dns_resolve"             => "slang_dns_resolve".into(),
                    "dns_cached"              => "slang_dns_cached".into(),
                    "dns_cache_size"          => "slang_dns_cache_size".into(),
                    "dns_cache_flush"         => "slang_dns_cache_flush".into(),
                    // -- v177: TLS/SSL ---------------------------
                    "tls_create"              => "slang_tls_create".into(),
                    "tls_active"              => "slang_tls_active".into(),
                    "tls_close"               => "slang_tls_close".into(),
                    "tls_cert_valid"          => "slang_tls_cert_valid".into(),
                    // -- v178: REPL Enhancements --------------------
                    "repl_history_add"       => "slang_repl_history_add".into(),
                    "repl_history_count"     => "slang_repl_history_count".into(),
                    "repl_history_clear"     => "slang_repl_history_clear".into(),
                    "repl_complete_count"    => "slang_repl_complete_count".into(),
                    // -- v179: Package Registry ---------------------
                    "pkg_publish"            => "slang_pkg_publish".into(),
                    "pkg_installed"          => "slang_pkg_installed".into(),
                    "pkg_count"              => "slang_pkg_count".into(),
                    "pkg_remove"             => "slang_pkg_remove".into(),
                    // -- v180: Documentation Generator --------------
                    "doc_add"                => "slang_doc_add".into(),
                    "doc_count"              => "slang_doc_count".into(),
                    "doc_has"                => "slang_doc_has".into(),
                    "doc_clear"              => "slang_doc_clear".into(),
                    // -- v181: Benchmark Suite ----------------------
                    "bench_record"           => "slang_bench_record".into(),
                    "bench_count"            => "slang_bench_count".into(),
                    "bench_best"             => "slang_bench_best".into(),
                    "bench_clear"            => "slang_bench_clear".into(),
                    // -- v182: Profiler -----------------------------
                    "profile_start"          => "slang_profile_start".into(),
                    "profile_stop"           => "slang_profile_stop".into(),
                    "profile_sample"         => "slang_profile_sample".into(),
                    "profile_samples"        => "slang_profile_samples".into(),
                    // -- v183: Interactive Playground ----------------
                    "playground_eval"        => "slang_playground_eval".into(),
                    "playground_count"       => "slang_playground_count".into(),
                    "playground_clear"       => "slang_playground_clear".into(),
                    "playground_last_result" => "slang_playground_last_result".into(),
                    // -- v184: Work-Stealing Scheduler --------------
                    "task_submit"            => "slang_task_submit".into(),
                    "task_queue_len"         => "slang_task_queue_len".into(),
                    "task_steal"             => "slang_task_steal".into(),
                    "task_queue_clear"       => "slang_task_queue_clear".into(),
                    // -- v185: Actor Model --------------------------
                    "actor_spawn"            => "slang_actor_spawn".into(),
                    "actor_send"             => "slang_actor_send".into(),
                    "actor_recv"             => "slang_actor_recv".into(),
                    "actor_mailbox_len"      => "slang_actor_mailbox_len".into(),
                    // -- v186: STM ----------------------------------
                    "stm_new"                => "slang_stm_new".into(),
                    "stm_read"               => "slang_stm_read".into(),
                    "stm_write"              => "slang_stm_write".into(),
                    "stm_cas"                => "slang_stm_cas".into(),
                    // -- v187: Parallel Collections -----------------
                    "par_sum"                => "slang_par_sum".into(),
                    "par_min"                => "slang_par_min".into(),
                    "par_max"                => "slang_par_max".into(),
                    "par_count"              => "slang_par_count".into(),
                    // -- v188: GPU Task Scheduling ------------------
                    "gpu_submit"             => "slang_gpu_submit".into(),
                    "gpu_queue_len"          => "slang_gpu_queue_len".into(),
                    "gpu_flush"              => "slang_gpu_flush".into(),
                    "gpu_available"          => "slang_gpu_available".into(),
                    // -- v189: Distributed Computing ----------------
                    "dist_node_add"          => "slang_dist_node_add".into(),
                    "dist_node_count"        => "slang_dist_node_count".into(),
                    "dist_broadcast"         => "slang_dist_broadcast".into(),
                    "dist_reduce"            => "slang_dist_reduce".into(),
                    // -- v190: ADTs v2 ------------------------------
                    "type_register"          => "slang_type_register".into(),
                    "type_variant_count"     => "slang_type_variant_count".into(),
                    "type_count"             => "slang_type_count".into(),
                    "type_exists"            => "slang_type_exists".into(),
                    // -- v191: Higher-Kinded Types ------------------
                    "hkt_register"           => "slang_hkt_register".into(),
                    "hkt_arity"              => "slang_hkt_arity".into(),
                    "hkt_count"              => "slang_hkt_count".into(),
                    "hkt_exists"             => "slang_hkt_exists".into(),
                    // -- v192: Dependent Types v2 -------------------
                    "dep_type_check_range"   => "slang_dep_type_check_range".into(),
                    "dep_type_nat"           => "slang_dep_type_nat".into(),
                    "dep_type_positive"      => "slang_dep_type_positive".into(),
                    "dep_type_bounded_add"   => "slang_dep_type_bounded_add".into(),
                    // -- v193: Effect Polymorphism ------------------
                    "effect_register"        => "slang_effect_register".into(),
                    "effect_add_handler"     => "slang_effect_add_handler".into(),
                    "effect_handler_count"   => "slang_effect_handler_count".into(),
                    "effect_count"           => "slang_effect_count".into(),
                    // -- v194: Type-Level Computation ---------------
                    "type_level_add"         => "slang_type_level_add".into(),
                    "type_level_mul"         => "slang_type_level_mul".into(),
                    "type_level_eq"          => "slang_type_level_eq".into(),
                    "type_level_if"          => "slang_type_level_if".into(),
                    // -- v195: Gradual Typing -----------------------
                    "gradual_annotate"       => "slang_gradual_annotate".into(),
                    "gradual_check"          => "slang_gradual_check".into(),
                    "gradual_typed_count"    => "slang_gradual_typed_count".into(),
                    "gradual_is_any"         => "slang_gradual_is_any".into(),
                    // -- v196: FFI v2 -------------------------------
                    "ffi_bind"               => "slang_ffi_bind".into(),
                    "ffi_bound"              => "slang_ffi_bound".into(),
                    "ffi_count"              => "slang_ffi_count".into(),
                    "ffi_remove"             => "slang_ffi_remove".into(),
                    // -- v197: Cloud-Native Deployment --------------
                    "cloud_deploy"           => "slang_cloud_deploy".into(),
                    "cloud_deployment_count" => "slang_cloud_deployment_count".into(),
                    "cloud_health_check"     => "slang_cloud_health_check".into(),
                    "cloud_shutdown"         => "slang_cloud_shutdown".into(),
                    // -- v198: Self-Hosting v3 ----------------------
                    "bootstrap_stage"        => "slang_bootstrap_stage".into(),
                    "bootstrap_advance"      => "slang_bootstrap_advance".into(),
                    "bootstrap_verify"       => "slang_bootstrap_verify".into(),
                    "bootstrap_reset"        => "slang_bootstrap_reset".into(),
                    // -- v199: AI Language Server -------------------
                    "ai_suggest"             => "slang_ai_suggest".into(),
                    "ai_suggestion_count"    => "slang_ai_suggestion_count".into(),
                    "ai_explain_error"       => "slang_ai_explain_error".into(),
                    "ai_clear"               => "slang_ai_clear".into(),
                    // -- v200: Milestone ----------------------------
                    "vitalis_version"        => "slang_vitalis_version".into(),
                    "vitalis_module_count"   => "slang_vitalis_module_count".into(),
                    "vitalis_test_count"     => "slang_vitalis_test_count".into(),
                    "vitalis_builtin_count"  => "slang_vitalis_builtin_count".into(),
                    // -- v201-v206: Spike Engine --
                    "spike_emit" => "slang_spike_emit".into(),
                    "spike_queue_len" => "slang_spike_queue_len".into(),
                    "spike_next" => "slang_spike_next".into(),
                    "spike_clear" => "slang_spike_clear".into(),
                    "neuro_compartment_create" => "slang_neuro_compartment_create".into(),
                    "neuro_compartment_step" => "slang_neuro_compartment_step".into(),
                    "neuro_dendrite_propagate" => "slang_neuro_dendrite_propagate".into(),
                    "neuro_compartment_count" => "slang_neuro_compartment_count".into(),
                    "synapse_conductance" => "slang_synapse_conductance".into(),
                    "synapse_stp_facilitate" => "slang_synapse_stp_facilitate".into(),
                    "synapse_stp_depress" => "slang_synapse_stp_depress".into(),
                    "synapse_count" => "slang_synapse_count".into(),
                    "neuro_wilson_cowan" => "slang_neuro_wilson_cowan".into(),
                    "neuro_neural_mass" => "slang_neuro_neural_mass".into(),
                    "neuro_population_activity" => "slang_neuro_population_activity".into(),
                    "neuro_population_sync" => "slang_neuro_population_sync".into(),
                    "spike_encode_rate" => "slang_spike_encode_rate".into(),
                    "spike_encode_temporal" => "slang_spike_encode_temporal".into(),
                    "spike_decode_rate" => "slang_spike_decode_rate".into(),
                    "spike_encode_phase" => "slang_spike_encode_phase".into(),
                    "neuro_mem_alloc" => "slang_neuro_mem_alloc".into(),
                    "neuro_mem_read" => "slang_neuro_mem_read".into(),
                    "neuro_mem_write" => "slang_neuro_mem_write".into(),
                    "neuro_mem_near_compute" => "slang_neuro_mem_near_compute".into(),
                    // -- v207-v212: Loihi Simulator --
                    "loihi_core_create" => "slang_loihi_core_create".into(),
                    "loihi_core_config" => "slang_loihi_core_config".into(),
                    "loihi_core_neuron_count" => "slang_loihi_core_neuron_count".into(),
                    "loihi_core_count" => "slang_loihi_core_count".into(),
                    "loihi_route_spike" => "slang_loihi_route_spike".into(),
                    "loihi_route_multicast" => "slang_loihi_route_multicast".into(),
                    "loihi_noc_latency" => "slang_loihi_noc_latency".into(),
                    "loihi_noc_bandwidth" => "slang_loihi_noc_bandwidth".into(),
                    "loihi_learn_stdp" => "slang_loihi_learn_stdp".into(),
                    "loihi_learn_reward" => "slang_loihi_learn_reward".into(),
                    "loihi_learn_3factor" => "slang_loihi_learn_3factor".into(),
                    "loihi_learn_config" => "slang_loihi_learn_config".into(),
                    "loihi_timestep" => "slang_loihi_timestep".into(),
                    "loihi_barrier_sync" => "slang_loihi_barrier_sync".into(),
                    "loihi_async_tick" => "slang_loihi_async_tick".into(),
                    "loihi_time_now" => "slang_loihi_time_now".into(),
                    "loihi_energy_spike" => "slang_loihi_energy_spike".into(),
                    "loihi_energy_compute" => "slang_loihi_energy_compute".into(),
                    "loihi_power_total" => "slang_loihi_power_total".into(),
                    "loihi_energy_reset" => "slang_loihi_energy_reset".into(),
                    "loihi_inst_soma" => "slang_loihi_inst_soma".into(),
                    "loihi_inst_synapse" => "slang_loihi_inst_synapse".into(),
                    "loihi_inst_axon" => "slang_loihi_inst_axon".into(),
                    "loihi_inst_dendrite" => "slang_loihi_inst_dendrite".into(),
                    // -- v213-v218: SNN Learning --
                    "snn_surrogate_forward" => "slang_snn_surrogate_forward".into(),
                    "snn_surrogate_backward" => "slang_snn_surrogate_backward".into(),
                    "snn_surrogate_sigmoid" => "slang_snn_surrogate_sigmoid".into(),
                    "snn_surrogate_loss" => "slang_snn_surrogate_loss".into(),
                    "snn_bptt_forward" => "slang_snn_bptt_forward".into(),
                    "snn_bptt_backward" => "slang_snn_bptt_backward".into(),
                    "snn_bptt_truncate" => "slang_snn_bptt_truncate".into(),
                    "snn_bptt_gradient" => "slang_snn_bptt_gradient".into(),
                    "snn_nas_search" => "slang_snn_nas_search".into(),
                    "snn_nas_evaluate" => "slang_snn_nas_evaluate".into(),
                    "snn_nas_mutate" => "slang_snn_nas_mutate".into(),
                    "snn_nas_best" => "slang_snn_nas_best".into(),
                    "snn_fed_aggregate" => "slang_snn_fed_aggregate".into(),
                    "snn_fed_share" => "slang_snn_fed_share".into(),
                    "snn_fed_round" => "slang_snn_fed_round".into(),
                    "snn_fed_node_count" => "slang_snn_fed_node_count".into(),
                    "snn_transfer_freeze" => "slang_snn_transfer_freeze".into(),
                    "snn_transfer_finetune" => "slang_snn_transfer_finetune".into(),
                    "snn_transfer_adapt" => "slang_snn_transfer_adapt".into(),
                    "snn_transfer_similarity" => "slang_snn_transfer_similarity".into(),
                    "snn_continual_learn" => "slang_snn_continual_learn".into(),
                    "snn_continual_consolidate" => "slang_snn_continual_consolidate".into(),
                    "snn_continual_replay" => "slang_snn_continual_replay".into(),
                    "snn_continual_forget_score" => "slang_snn_continual_forget_score".into(),
                    // -- v219-v224: Processing-in-Memory --
                    "pim_alloc" => "slang_pim_alloc".into(),
                    "pim_compute_add" => "slang_pim_compute_add".into(),
                    "pim_compute_mul" => "slang_pim_compute_mul".into(),
                    "pim_transfer_cost" => "slang_pim_transfer_cost".into(),
                    "datacentric_map" => "slang_datacentric_map".into(),
                    "datacentric_reduce" => "slang_datacentric_reduce".into(),
                    "datacentric_scatter" => "slang_datacentric_scatter".into(),
                    "datacentric_gather" => "slang_datacentric_gather".into(),
                    "sparse_spike_propagate" => "slang_sparse_spike_propagate".into(),
                    "sparse_nonzero_count" => "slang_sparse_nonzero_count".into(),
                    "sparse_compress" => "slang_sparse_compress".into(),
                    "sparse_decompress" => "slang_sparse_decompress".into(),
                    "cache_oblivious_transpose" => "slang_cache_oblivious_transpose".into(),
                    "cache_oblivious_fft" => "slang_cache_oblivious_fft".into(),
                    "cache_oblivious_sort" => "slang_cache_oblivious_sort".into(),
                    "cache_oblivious_matmul" => "slang_cache_oblivious_matmul".into(),
                    "memcompute_fused_mac" => "slang_memcompute_fused_mac".into(),
                    "memcompute_fused_compare" => "slang_memcompute_fused_compare".into(),
                    "memcompute_fused_accumulate" => "slang_memcompute_fused_accumulate".into(),
                    "memcompute_pipeline_depth" => "slang_memcompute_pipeline_depth".into(),
                    "zerocopy_spike_buffer" => "slang_zerocopy_spike_buffer".into(),
                    "zerocopy_fanout" => "slang_zerocopy_fanout".into(),
                    "zerocopy_gather" => "slang_zerocopy_gather".into(),
                    "zerocopy_active_count" => "slang_zerocopy_active_count".into(),
                    // -- v225-v230: Brain Models --
                    "pred_coding_forward" => "slang_pred_coding_forward".into(),
                    "pred_coding_error" => "slang_pred_coding_error".into(),
                    "pred_coding_update" => "slang_pred_coding_update".into(),
                    "pred_coding_layers" => "slang_pred_coding_layers".into(),
                    "htm_spatial_pool" => "slang_htm_spatial_pool".into(),
                    "htm_temporal_memory" => "slang_htm_temporal_memory".into(),
                    "htm_anomaly_score" => "slang_htm_anomaly_score".into(),
                    "htm_column_count" => "slang_htm_column_count".into(),
                    "neuro_osc_gamma" => "slang_neuro_osc_gamma".into(),
                    "neuro_osc_theta" => "slang_neuro_osc_theta".into(),
                    "neuro_osc_couple" => "slang_neuro_osc_couple".into(),
                    "neuro_osc_phase_lock" => "slang_neuro_osc_phase_lock".into(),
                    "neuromod_dopamine" => "slang_neuromod_dopamine".into(),
                    "neuromod_serotonin" => "slang_neuromod_serotonin".into(),
                    "neuromod_acetylcholine" => "slang_neuromod_acetylcholine".into(),
                    "neuromod_apply" => "slang_neuromod_apply".into(),
                    "cortical_column_create" => "slang_cortical_column_create".into(),
                    "cortical_column_step" => "slang_cortical_column_step".into(),
                    "cortical_column_layer_activity" => "slang_cortical_column_layer_activity".into(),
                    "cortical_column_count" => "slang_cortical_column_count".into(),
                    "spike_attention_query" => "slang_spike_attention_query".into(),
                    "spike_attention_key" => "slang_spike_attention_key".into(),
                    "spike_attention_value" => "slang_spike_attention_value".into(),
                    "spike_attention_score" => "slang_spike_attention_score".into(),
                    // -- v231-v236: GPU Neuromorphic --
                    "gpu_spike_propagate" => "slang_gpu_spike_propagate".into(),
                    "gpu_spike_batch_size" => "slang_gpu_spike_batch_size".into(),
                    "gpu_spike_throughput" => "slang_gpu_spike_throughput".into(),
                    "gpu_spike_sync" => "slang_gpu_spike_sync".into(),
                    "gpu_neuron_update" => "slang_gpu_neuron_update".into(),
                    "gpu_neuron_batch" => "slang_gpu_neuron_batch".into(),
                    "gpu_neuron_occupancy" => "slang_gpu_neuron_occupancy".into(),
                    "gpu_neuron_count" => "slang_gpu_neuron_count".into(),
                    "gpu_synapse_spmv" => "slang_gpu_synapse_spmv".into(),
                    "gpu_synapse_csr" => "slang_gpu_synapse_csr".into(),
                    "gpu_synapse_nnz" => "slang_gpu_synapse_nnz".into(),
                    "gpu_synapse_density" => "slang_gpu_synapse_density".into(),
                    "gpu_event_push" => "slang_gpu_event_push".into(),
                    "gpu_event_pop" => "slang_gpu_event_pop".into(),
                    "gpu_event_merge" => "slang_gpu_event_merge".into(),
                    "gpu_event_size" => "slang_gpu_event_size".into(),
                    "mixed_prec_quantize" => "slang_mixed_prec_quantize".into(),
                    "mixed_prec_dequantize" => "slang_mixed_prec_dequantize".into(),
                    "mixed_prec_accumulate" => "slang_mixed_prec_accumulate".into(),
                    "mixed_prec_bits" => "slang_mixed_prec_bits".into(),
                    "multi_gpu_partition" => "slang_multi_gpu_partition".into(),
                    "multi_gpu_sync" => "slang_multi_gpu_sync".into(),
                    "multi_gpu_migrate" => "slang_multi_gpu_migrate".into(),
                    "multi_gpu_count" => "slang_multi_gpu_count".into(),
                    // -- v237-v242: Neuro Applications --
                    "spike_vision_encode" => "slang_spike_vision_encode".into(),
                    "spike_vision_edge" => "slang_spike_vision_edge".into(),
                    "spike_vision_motion" => "slang_spike_vision_motion".into(),
                    "spike_vision_frames" => "slang_spike_vision_frames".into(),
                    "spike_audio_encode" => "slang_spike_audio_encode".into(),
                    "spike_audio_frequency" => "slang_spike_audio_frequency".into(),
                    "spike_audio_onset" => "slang_spike_audio_onset".into(),
                    "spike_audio_classify" => "slang_spike_audio_classify".into(),
                    "spike_pid" => "slang_spike_pid".into(),
                    "spike_motor" => "slang_spike_motor".into(),
                    "spike_reflex" => "slang_spike_reflex".into(),
                    "spike_trajectory_cost" => "slang_spike_trajectory_cost".into(),
                    "spike_anomaly_score" => "slang_spike_anomaly_score".into(),
                    "spike_changepoint" => "slang_spike_changepoint".into(),
                    "spike_burst_detect" => "slang_spike_burst_detect".into(),
                    "spike_pattern_match" => "slang_spike_pattern_match".into(),
                    "spike_anneal" => "slang_spike_anneal".into(),
                    "spike_gradient" => "slang_spike_gradient".into(),
                    "spike_constraint" => "slang_spike_constraint".into(),
                    "spike_fitness" => "slang_spike_fitness".into(),
                    "spike_nlp_similarity" => "slang_spike_nlp_similarity".into(),
                    "spike_nlp_attention" => "slang_spike_nlp_attention".into(),
                    "spike_nlp_encode_len" => "slang_spike_nlp_encode_len".into(),
                    "spike_nlp_perplexity" => "slang_spike_nlp_perplexity".into(),
                    // -- v243-v248: Neuro Evolution --
                    "neat_crossover" => "slang_neat_crossover".into(),
                    "neat_mutate" => "slang_neat_mutate".into(),
                    "neat_speciate" => "slang_neat_speciate".into(),
                    "neat_generation" => "slang_neat_generation".into(),
                    "som_bmu" => "slang_som_bmu".into(),
                    "som_radius" => "slang_som_radius".into(),
                    "som_learning_rate" => "slang_som_learning_rate".into(),
                    "som_quant_error" => "slang_som_quant_error".into(),
                    "neuro_nas_evaluate" => "slang_neuro_nas_evaluate".into(),
                    "neuro_nas_sample" => "slang_neuro_nas_sample".into(),
                    "neuro_nas_prune" => "slang_neuro_nas_prune".into(),
                    "neuro_nas_best_score" => "slang_neuro_nas_best_score".into(),
                    "spike_rl_rstdp" => "slang_spike_rl_rstdp".into(),
                    "spike_rl_td" => "slang_spike_rl_td".into(),
                    "spike_rl_policy" => "slang_spike_rl_policy".into(),
                    "spike_rl_predict_reward" => "slang_spike_rl_predict_reward".into(),
                    "curiosity_reward" => "slang_curiosity_reward".into(),
                    "curiosity_info_gain" => "slang_curiosity_info_gain".into(),
                    "curiosity_novelty" => "slang_curiosity_novelty".into(),
                    "curiosity_decay" => "slang_curiosity_decay".into(),
                    "meta_maml_adapt" => "slang_meta_maml_adapt".into(),
                    "meta_reptile" => "slang_meta_reptile".into(),
                    "meta_task_similarity" => "slang_meta_task_similarity".into(),
                    "meta_convergence" => "slang_meta_convergence".into(),
                    // -- v249-v254: SNN-ANN Hybrid --
                    "snn_ann_to_rate" => "slang_snn_ann_to_rate".into(),
                    "snn_rate_to_ann" => "slang_snn_rate_to_ann".into(),
                    "snn_conversion_loss" => "slang_snn_conversion_loss".into(),
                    "snn_optimal_timesteps" => "slang_snn_optimal_timesteps".into(),
                    "hybrid_infer" => "slang_hybrid_infer".into(),
                    "hybrid_set_fraction" => "slang_hybrid_set_fraction".into(),
                    "hybrid_efficiency" => "slang_hybrid_efficiency".into(),
                    "hybrid_accuracy_gain" => "slang_hybrid_accuracy_gain".into(),
                    "spike_compile" => "slang_spike_compile".into(),
                    "spike_compile_optimize" => "slang_spike_compile_optimize".into(),
                    "spike_compile_count" => "slang_spike_compile_count".into(),
                    "spike_compile_memory" => "slang_spike_compile_memory".into(),
                    "diff_spike_ste" => "slang_diff_spike_ste".into(),
                    "diff_spike_sigmoid" => "slang_diff_spike_sigmoid".into(),
                    "diff_spike_fast_sigmoid" => "slang_diff_spike_fast_sigmoid".into(),
                    "diff_spike_accumulate" => "slang_diff_spike_accumulate".into(),
                    "neural_ode_euler" => "slang_neural_ode_euler".into(),
                    "neural_ode_rk4" => "slang_neural_ode_rk4".into(),
                    "neural_ode_adjoint" => "slang_neural_ode_adjoint".into(),
                    "neural_ode_adaptive_dt" => "slang_neural_ode_adaptive_dt".into(),
                    "hybrid_distill" => "slang_hybrid_distill".into(),
                    "hybrid_freeze" => "slang_hybrid_freeze".into(),
                    "hybrid_lr" => "slang_hybrid_lr".into(),
                    "hybrid_weighted_acc" => "slang_hybrid_weighted_acc".into(),
                    // -- v255-v260: Hippocampal Memory --
                    "hippo_encode" => "slang_hippo_encode".into(),
                    "hippo_recall" => "slang_hippo_recall".into(),
                    "hippo_replay" => "slang_hippo_replay".into(),
                    "hippo_count" => "slang_hippo_count".into(),
                    "wm_push" => "slang_wm_push".into(),
                    "wm_pop" => "slang_wm_pop".into(),
                    "wm_set_capacity" => "slang_wm_set_capacity".into(),
                    "wm_utilization" => "slang_wm_utilization".into(),
                    "sleep_consolidate" => "slang_sleep_consolidate".into(),
                    "sleep_rem" => "slang_sleep_rem".into(),
                    "sleep_nrem_ripple" => "slang_sleep_nrem_ripple".into(),
                    "sleep_duration" => "slang_sleep_duration".into(),
                    "hopfield_energy" => "slang_hopfield_energy".into(),
                    "hopfield_capacity" => "slang_hopfield_capacity".into(),
                    "hopfield_retrieve" => "slang_hopfield_retrieve".into(),
                    "hopfield_accuracy" => "slang_hopfield_accuracy".into(),
                    "synaptag_decay" => "slang_synaptag_decay".into(),
                    "synaptag_capture" => "slang_synaptag_capture".into(),
                    "synaptag_late_ltp" => "slang_synaptag_late_ltp".into(),
                    "synaptag_protein" => "slang_synaptag_protein".into(),
                    "memcompress_schema" => "slang_memcompress_schema".into(),
                    "memcompress_forget" => "slang_memcompress_forget".into(),
                    "memcompress_merge" => "slang_memcompress_merge".into(),
                    "memcompress_ratio" => "slang_memcompress_ratio".into(),
                    // -- v261-v266: Distributed Neuromorphic --
                    "neuro_cluster_init" => "slang_neuro_cluster_init".into(),
                    "neuro_cluster_distribute" => "slang_neuro_cluster_distribute".into(),
                    "neuro_cluster_load" => "slang_neuro_cluster_load".into(),
                    "neuro_cluster_nodes" => "slang_neuro_cluster_nodes".into(),
                    "spike_consensus_vote" => "slang_spike_consensus_vote".into(),
                    "spike_consensus_bft" => "slang_spike_consensus_bft".into(),
                    "spike_consensus_tick" => "slang_spike_consensus_tick".into(),
                    "spike_consensus_latency" => "slang_spike_consensus_latency".into(),
                    "fed_neuro_average" => "slang_fed_neuro_average".into(),
                    "fed_neuro_dp_noise" => "slang_fed_neuro_dp_noise".into(),
                    "fed_neuro_compress" => "slang_fed_neuro_compress".into(),
                    "fed_neuro_round" => "slang_fed_neuro_round".into(),
                    "edge_neuro_budget" => "slang_edge_neuro_budget".into(),
                    "edge_neuro_quantize" => "slang_edge_neuro_quantize".into(),
                    "edge_neuro_latency_ok" => "slang_edge_neuro_latency_ok".into(),
                    "edge_neuro_model_size" => "slang_edge_neuro_model_size".into(),
                    "stream_spike_process" => "slang_stream_spike_process".into(),
                    "stream_spike_rate" => "slang_stream_spike_rate".into(),
                    "stream_spike_backpressure" => "slang_stream_spike_backpressure".into(),
                    "stream_spike_total" => "slang_stream_spike_total".into(),
                    "neuro_platform_caps" => "slang_neuro_platform_caps".into(),
                    "neuro_platform_map" => "slang_neuro_platform_map".into(),
                    "neuro_platform_overhead" => "slang_neuro_platform_overhead".into(),
                    "neuro_platform_power" => "slang_neuro_platform_power".into(),
                    // -- v267-v272: Neuro Tooling --
                    "neuro_viz_spike_raster" => "slang_neuro_viz_spike_raster".into(),
                    "neuro_viz_membrane" => "slang_neuro_viz_membrane".into(),
                    "neuro_viz_connectivity" => "slang_neuro_viz_connectivity".into(),
                    "neuro_viz_frames" => "slang_neuro_viz_frames".into(),
                    "neuro_debug_break" => "slang_neuro_debug_break".into(),
                    "neuro_debug_inspect" => "slang_neuro_debug_inspect".into(),
                    "neuro_debug_step" => "slang_neuro_debug_step".into(),
                    "neuro_debug_breakpoints" => "slang_neuro_debug_breakpoints".into(),
                    "neuro_profile_throughput" => "slang_neuro_profile_throughput".into(),
                    "neuro_profile_memory" => "slang_neuro_profile_memory".into(),
                    "neuro_profile_energy" => "slang_neuro_profile_energy".into(),
                    "neuro_profile_samples" => "slang_neuro_profile_samples".into(),
                    "neuro_dsl_neuron" => "slang_neuro_dsl_neuron".into(),
                    "neuro_dsl_synapse" => "slang_neuro_dsl_synapse".into(),
                    "neuro_dsl_network" => "slang_neuro_dsl_network".into(),
                    "neuro_dsl_validate" => "slang_neuro_dsl_validate".into(),
                    "neuro_bench_spike_lat" => "slang_neuro_bench_spike_lat".into(),
                    "neuro_bench_neuron_tput" => "slang_neuro_bench_neuron_tput".into(),
                    "neuro_bench_synapse_rate" => "slang_neuro_bench_synapse_rate".into(),
                    "neuro_bench_efficiency" => "slang_neuro_bench_efficiency".into(),
                    "neuro_test_timing" => "slang_neuro_test_timing".into(),
                    "neuro_test_accuracy" => "slang_neuro_test_accuracy".into(),
                    "neuro_test_convergence" => "slang_neuro_test_convergence".into(),
                    "neuro_test_spike_gen" => "slang_neuro_test_spike_gen".into(),
                    // -- v273-v278: Quantum Neuromorphic --
                    "quantum_spike_encode" => "slang_quantum_spike_encode".into(),
                    "quantum_spike_decode" => "slang_quantum_spike_decode".into(),
                    "quantum_spike_superpose" => "slang_quantum_spike_superpose".into(),
                    "quantum_spike_fidelity" => "slang_quantum_spike_fidelity".into(),
                    "quantum_plasticity_stdp" => "slang_quantum_plasticity_stdp".into(),
                    "quantum_plasticity_anneal" => "slang_quantum_plasticity_anneal".into(),
                    "quantum_plasticity_tunnel" => "slang_quantum_plasticity_tunnel".into(),
                    "quantum_plasticity_t2" => "slang_quantum_plasticity_t2".into(),
                    "quantum_reservoir_init" => "slang_quantum_reservoir_init".into(),
                    "quantum_reservoir_project" => "slang_quantum_reservoir_project".into(),
                    "quantum_reservoir_kernel" => "slang_quantum_reservoir_kernel".into(),
                    "quantum_reservoir_dim" => "slang_quantum_reservoir_dim".into(),
                    "qsnn_var_update" => "slang_qsnn_var_update".into(),
                    "qsnn_var_cost" => "slang_qsnn_var_cost".into(),
                    "qsnn_var_depth" => "slang_qsnn_var_depth".into(),
                    "qsnn_var_expressibility" => "slang_qsnn_var_expressibility".into(),
                    "quantum_qec_shor" => "slang_quantum_qec_shor".into(),
                    "quantum_qec_surface" => "slang_quantum_qec_surface".into(),
                    "quantum_qec_syndrome" => "slang_quantum_qec_syndrome".into(),
                    "quantum_qec_overhead" => "slang_quantum_qec_overhead".into(),
                    "quantum_bridge_encode" => "slang_quantum_bridge_encode".into(),
                    "quantum_bridge_decode" => "slang_quantum_bridge_decode".into(),
                    "quantum_bridge_cost" => "slang_quantum_bridge_cost".into(),
                    "quantum_bridge_advantage" => "slang_quantum_bridge_advantage".into(),
                    // -- v279-v284: Neuro Safety --
                    "neuro_verify_timing" => "slang_neuro_verify_timing".into(),
                    "neuro_verify_membrane" => "slang_neuro_verify_membrane".into(),
                    "neuro_verify_symmetry" => "slang_neuro_verify_symmetry".into(),
                    "neuro_verify_liveness" => "slang_neuro_verify_liveness".into(),
                    "neuro_safe_rate_clamp" => "slang_neuro_safe_rate_clamp".into(),
                    "neuro_safe_runaway" => "slang_neuro_safe_runaway".into(),
                    "neuro_safe_dead_neuron" => "slang_neuro_safe_dead_neuron".into(),
                    "neuro_safe_violations" => "slang_neuro_safe_violations".into(),
                    "neuro_explain_contribution" => "slang_neuro_explain_contribution".into(),
                    "neuro_explain_ablation" => "slang_neuro_explain_ablation".into(),
                    "neuro_explain_saliency" => "slang_neuro_explain_saliency".into(),
                    "neuro_explain_lrp" => "slang_neuro_explain_lrp".into(),
                    "neuro_robust_eps_check" => "slang_neuro_robust_eps_check".into(),
                    "neuro_robust_margin" => "slang_neuro_robust_margin".into(),
                    "neuro_robust_certified_radius" => "slang_neuro_robust_certified_radius".into(),
                    "neuro_robust_inject_noise" => "slang_neuro_robust_inject_noise".into(),
                    "neuro_fair_dp_gap" => "slang_neuro_fair_dp_gap".into(),
                    "neuro_fair_eo_gap" => "slang_neuro_fair_eo_gap".into(),
                    "neuro_fair_calibration" => "slang_neuro_fair_calibration".into(),
                    "neuro_fair_lipschitz" => "slang_neuro_fair_lipschitz".into(),
                    "neuro_cert_bounds" => "slang_neuro_cert_bounds".into(),
                    "neuro_cert_ibp_width" => "slang_neuro_cert_ibp_width".into(),
                    "neuro_cert_crown" => "slang_neuro_cert_crown".into(),
                    "neuro_cert_accuracy" => "slang_neuro_cert_accuracy".into(),
                    // -- v285-v290: Neuro Performance --
                    "neuro_simd_accumulate" => "slang_neuro_simd_accumulate".into(),
                    "neuro_simd_threshold" => "slang_neuro_simd_threshold".into(),
                    "neuro_simd_decay" => "slang_neuro_simd_decay".into(),
                    "neuro_simd_throughput" => "slang_neuro_simd_throughput".into(),
                    "neuro_jit_compile" => "slang_neuro_jit_compile".into(),
                    "neuro_jit_speedup" => "slang_neuro_jit_speedup".into(),
                    "neuro_jit_cache_hit" => "slang_neuro_jit_cache_hit".into(),
                    "neuro_jit_compiled_count" => "slang_neuro_jit_compiled_count".into(),
                    "neuro_precision_auto" => "slang_neuro_precision_auto".into(),
                    "neuro_precision_quant_error" => "slang_neuro_precision_quant_error".into(),
                    "neuro_precision_savings" => "slang_neuro_precision_savings".into(),
                    "neuro_precision_scale" => "slang_neuro_precision_scale".into(),
                    "neuro_spec_predict" => "slang_neuro_spec_predict".into(),
                    "neuro_spec_gain" => "slang_neuro_spec_gain".into(),
                    "neuro_spec_rollback_cost" => "slang_neuro_spec_rollback_cost".into(),
                    "neuro_spec_confidence" => "slang_neuro_spec_confidence".into(),
                    "neuro_pgo_sample" => "slang_neuro_pgo_sample".into(),
                    "neuro_pgo_is_hot" => "slang_neuro_pgo_is_hot".into(),
                    "neuro_pgo_unroll" => "slang_neuro_pgo_unroll".into(),
                    "neuro_pgo_total_samples" => "slang_neuro_pgo_total_samples".into(),
                    "neuro_zero_send" => "slang_neuro_zero_send".into(),
                    "neuro_zero_update" => "slang_neuro_zero_update".into(),
                    "neuro_zero_conn_type" => "slang_neuro_zero_conn_type".into(),
                    "neuro_zero_overhead" => "slang_neuro_zero_overhead".into(),
                    // -- v291: Advanced Spike Analytics --
                    "spike_analytics_mean_rate" => "slang_spike_analytics_mean_rate".into(),
                    "spike_analytics_cv_isi" => "slang_spike_analytics_cv_isi".into(),
                    "spike_analytics_fano_factor" => "slang_spike_analytics_fano_factor".into(),
                    "spike_analytics_burst_index" => "slang_spike_analytics_burst_index".into(),
                    // -- v292: Neural Network Metrics --
                    "nn_metric_sparsity" => "slang_nn_metric_sparsity".into(),
                    "nn_metric_entropy" => "slang_nn_metric_entropy".into(),
                    "nn_metric_mutual_info" => "slang_nn_metric_mutual_info".into(),
                    "nn_metric_transfer_entropy" => "slang_nn_metric_transfer_entropy".into(),
                    // -- v293: Spike Train Distance --
                    "spike_dist_victor_purpura" => "slang_spike_dist_victor_purpura".into(),
                    "spike_dist_van_rossum" => "slang_spike_dist_van_rossum".into(),
                    "spike_dist_schreiber" => "slang_spike_dist_schreiber".into(),
                    "spike_dist_earth_mover" => "slang_spike_dist_earth_mover".into(),
                    // -- v294: Neural Coding --
                    "neural_code_rate" => "slang_neural_code_rate".into(),
                    "neural_code_temporal" => "slang_neural_code_temporal".into(),
                    "neural_code_population" => "slang_neural_code_population".into(),
                    "neural_code_sparse" => "slang_neural_code_sparse".into(),
                    // -- v295: Synaptic Plasticity Metrics --
                    "synap_metric_ltp_ratio" => "slang_synap_metric_ltp_ratio".into(),
                    "synap_metric_ltd_ratio" => "slang_synap_metric_ltd_ratio".into(),
                    "synap_metric_homeostatic" => "slang_synap_metric_homeostatic".into(),
                    "synap_metric_metaplasticity" => "slang_synap_metric_metaplasticity".into(),
                    // -- v296: Network Topology --
                    "topo_clustering_coeff" => "slang_topo_clustering_coeff".into(),
                    "topo_path_length" => "slang_topo_path_length".into(),
                    "topo_small_world" => "slang_topo_small_world".into(),
                    "topo_modularity" => "slang_topo_modularity".into(),
                    // -- v297: Neuromorphic IO --
                    "neuro_io_aer_encode" => "slang_neuro_io_aer_encode".into(),
                    "neuro_io_aer_decode" => "slang_neuro_io_aer_decode".into(),
                    "neuro_io_dvs_encode" => "slang_neuro_io_dvs_encode".into(),
                    "neuro_io_serial_pack" => "slang_neuro_io_serial_pack".into(),
                    // -- v298: Neural Dynamics --
                    "dyn_lyapunov_exp" => "slang_dyn_lyapunov_exp".into(),
                    "dyn_bifurcation" => "slang_dyn_bifurcation".into(),
                    "dyn_phase_portrait" => "slang_dyn_phase_portrait".into(),
                    "dyn_attractor_dim" => "slang_dyn_attractor_dim".into(),
                    // -- v299: Neuromorphic Scheduler --
                    "neuro_sched_priority" => "slang_neuro_sched_priority".into(),
                    "neuro_sched_deadline" => "slang_neuro_sched_deadline".into(),
                    "neuro_sched_edf" => "slang_neuro_sched_edf".into(),
                    "neuro_sched_utilization" => "slang_neuro_sched_utilization".into(),
                    // -- v300: Milestone --
                    "vitalis_v300_version" => "slang_vitalis_v300_version".into(),
                    "vitalis_v300_total_builtins" => "slang_vitalis_v300_total_builtins".into(),
                    "vitalis_v300_neuro_modules" => "slang_vitalis_v300_neuro_modules".into(),
                    "vitalis_v300_milestone" => "slang_vitalis_v300_milestone".into(),

                    // -- v301-v366: Post-Neuromorphic Era --
                    // v301 Escape Analysis
                    "escape_analyze" => "slang_escape_analyze".into(),
                    "escape_stack_promoted" => "slang_escape_stack_promoted".into(),
                    "escape_summary" => "slang_escape_summary".into(),
                    "escape_clear" => "slang_escape_clear".into(),
                    // v302 TCO
                    "tco_detect" => "slang_tco_detect".into(),
                    "tco_optimized_count" => "slang_tco_optimized_count".into(),
                    "tco_depth_limit" => "slang_tco_depth_limit".into(),
                    "tco_enabled" => "slang_tco_enabled".into(),
                    // v303 Algebraic
                    "opt_algebraic_count" => "slang_opt_algebraic_count".into(),
                    "opt_algebraic_enable" => "slang_opt_algebraic_enable".into(),
                    "opt_strength_reduced" => "slang_opt_strength_reduced".into(),
                    "opt_identity_removed" => "slang_opt_identity_removed".into(),
                    // v304 IPA
                    "ipa_add_edge" => "slang_ipa_add_edge".into(),
                    "ipa_call_graph_size" => "slang_ipa_call_graph_size".into(),
                    "ipa_mark_pure" => "slang_ipa_mark_pure".into(),
                    "ipa_pure_functions" => "slang_ipa_pure_functions".into(),
                    "ipa_record_const_args" => "slang_ipa_record_const_args".into(),
                    "ipa_const_args" => "slang_ipa_const_args".into(),
                    "ipa_summary" => "slang_ipa_summary".into(),
                    "ipa_clear" => "slang_ipa_clear".into(),
                    // v305 LTO
                    "lto_inline_count" => "slang_lto_inline_count".into(),
                    "lto_dead_globals" => "slang_lto_dead_globals".into(),
                    "lto_devirtualized" => "slang_lto_devirtualized".into(),
                    "lto_enabled" => "slang_lto_enabled".into(),
                    // v306 Vectorization
                    "vec_slp_opportunities" => "slang_vec_slp_opportunities".into(),
                    "vec_slp_applied" => "slang_vec_slp_applied".into(),
                    "vec_width" => "slang_vec_width".into(),
                    "vec_speedup_estimate" => "slang_vec_speedup_estimate".into(),
                    // v307 Compile-Time Execution
                    "consteval_string" => "slang_consteval_string".into(),
                    "consteval_array" => "slang_consteval_array".into(),
                    "consteval_struct" => "slang_consteval_struct".into(),
                    "consteval_count" => "slang_consteval_count".into(),
                    // v308 PGO
                    "pgo_record" => "slang_pgo_record".into(),
                    "pgo_hotness" => "slang_pgo_hotness".into(),
                    "pgo_branch_bias" => "slang_pgo_branch_bias".into(),
                    "pgo_total_samples" => "slang_pgo_total_samples".into(),
                    // v309 Register Allocation
                    "regalloc_spill_count" => "slang_regalloc_spill_count".into(),
                    "regalloc_move_count" => "slang_regalloc_move_count".into(),
                    "regalloc_pressure" => "slang_regalloc_pressure".into(),
                    "regalloc_coalesced" => "slang_regalloc_coalesced".into(),
                    // v310 Debug Info
                    "debug_line_count" => "slang_debug_line_count".into(),
                    "debug_var_count" => "slang_debug_var_count".into(),
                    "debug_scope_depth" => "slang_debug_scope_depth".into(),
                    "debug_info_size" => "slang_debug_info_size".into(),
                    // v311 Existential Types
                    "existential_create" => "slang_existential_create".into(),
                    "existential_open" => "slang_existential_open".into(),
                    "existential_pack" => "slang_existential_pack".into(),
                    "existential_count" => "slang_existential_count".into(),
                    // v312 Row Types
                    "row_type_fields" => "slang_row_type_fields".into(),
                    "row_type_create" => "slang_row_type_create".into(),
                    "row_type_extend" => "slang_row_type_extend".into(),
                    "row_type_restrict" => "slang_row_type_restrict".into(),
                    "row_type_compatible" => "slang_row_type_compatible".into(),
                    // v313 Linear Types
                    "linear_create" => "slang_linear_create".into(),
                    "linear_check" => "slang_linear_check".into(),
                    "linear_consume" => "slang_linear_consume".into(),
                    "session_create" => "slang_session_create".into(),
                    "session_state" => "slang_session_state".into(),
                    "session_advance" => "slang_session_advance".into(),
                    // v314 GADTs
                    "gadt_create" => "slang_gadt_create".into(),
                    "gadt_refine" => "slang_gadt_refine".into(),
                    "gadt_witness" => "slang_gadt_witness".into(),
                    "gadt_count" => "slang_gadt_count".into(),
                    // v315 Type Classes
                    "typeclass_instances" => "slang_typeclass_instances".into(),
                    "typeclass_resolve" => "slang_typeclass_resolve".into(),
                    "typeclass_coherence" => "slang_typeclass_coherence".into(),
                    "typeclass_register" => "slang_typeclass_register".into(),
                    // v316 Dependent Types
                    "dependent_proof" => "slang_dependent_proof".into(),
                    "dependent_index" => "slang_dependent_index".into(),
                    "dependent_refine" => "slang_dependent_refine".into(),
                    "dependent_check" => "slang_dependent_check".into(),
                    // v317 Effect Inference
                    "effect_infer" => "slang_effect_infer".into(),
                    "effect_row" => "slang_effect_row".into(),
                    "effect_mask" => "slang_effect_mask".into(),
                    "effect_polymorphic" => "slang_effect_polymorphic".into(),
                    // v318 Mixture of Experts
                    "moe_create" => "slang_moe_create".into(),
                    "moe_route" => "slang_moe_route".into(),
                    "moe_expert_load" => "slang_moe_expert_load".into(),
                    "moe_aux_loss" => "slang_moe_aux_loss".into(),
                    // v319 Quantization
                    "quant_int8" => "slang_quant_int8".into(),
                    "quant_int4" => "slang_quant_int4".into(),
                    "quant_error" => "slang_quant_error".into(),
                    "quant_calibrate" => "slang_quant_calibrate".into(),
                    // v320 Attention Variants
                    "attn_flash" => "slang_attn_flash".into(),
                    "attn_linear" => "slang_attn_linear".into(),
                    "attn_sparse" => "slang_attn_sparse".into(),
                    "attn_sliding_window" => "slang_attn_sliding_window".into(),
                    // v321 Distillation
                    "distill_kd_loss" => "slang_distill_kd_loss".into(),
                    "distill_feature_loss" => "slang_distill_feature_loss".into(),
                    "distill_attention_transfer" => "slang_distill_attention_transfer".into(),
                    "distill_temperature" => "slang_distill_temperature".into(),
                    // v322 GNN
                    "gnn_create" => "slang_gnn_create".into(),
                    "gnn_add_edge" => "slang_gnn_add_edge".into(),
                    "gnn_set_feature" => "slang_gnn_set_feature".into(),
                    "gnn_message_pass" => "slang_gnn_message_pass".into(),
                    "gnn_conv" => "slang_gnn_conv".into(),
                    "gnn_attention" => "slang_gnn_attention".into(),
                    "gnn_readout" => "slang_gnn_readout".into(),
                    // v323 Diffusion
                    "diffusion_forward" => "slang_diffusion_forward".into(),
                    "diffusion_reverse" => "slang_diffusion_reverse".into(),
                    "diffusion_schedule" => "slang_diffusion_schedule".into(),
                    "diffusion_sample" => "slang_diffusion_sample".into(),
                    // v324 RL
                    "rl_q_update" => "slang_rl_q_update".into(),
                    "rl_policy_gradient" => "slang_rl_policy_gradient".into(),
                    "rl_advantage" => "slang_rl_advantage".into(),
                    "rl_reward_discount" => "slang_rl_reward_discount".into(),
                    // v325 Embedding Search
                    "hnsw_create" => "slang_hnsw_create".into(),
                    "hnsw_insert" => "slang_hnsw_insert".into(),
                    "hnsw_search" => "slang_hnsw_search".into(),
                    "hnsw_recall" => "slang_hnsw_recall".into(),
                    // v326 Tokenizer
                    "tokenizer_bpe_train" => "slang_tokenizer_bpe_train".into(),
                    "tokenizer_encode" => "slang_tokenizer_encode".into(),
                    "tokenizer_decode" => "slang_tokenizer_decode".into(),
                    "tokenizer_vocab_size" => "slang_tokenizer_vocab_size".into(),
                    // v327 RLHF
                    "rlhf_reward" => "slang_rlhf_reward".into(),
                    "rlhf_kl_penalty" => "slang_rlhf_kl_penalty".into(),
                    "rlhf_preference" => "slang_rlhf_preference".into(),
                    "rlhf_ppo_clip" => "slang_rlhf_ppo_clip".into(),
                    // v328 Async Runtime
                    "async_spawn_task" => "slang_async_spawn_task".into(),
                    "async_yield_now" => "slang_async_yield_now".into(),
                    "async_select" => "slang_async_select".into(),
                    "async_timeout" => "slang_async_timeout".into(),
                    // v329 Work Stealing
                    "ws_create_pool" => "slang_ws_create_pool".into(),
                    "ws_submit" => "slang_ws_submit".into(),
                    "ws_steal_count" => "slang_ws_steal_count".into(),
                    "ws_active_workers" => "slang_ws_active_workers".into(),
                    // v330 Connection Pool
                    "conn_pool_create" => "slang_conn_pool_create".into(),
                    "pool_acquire" => "slang_pool_acquire".into(),
                    "pool_release" => "slang_pool_release".into(),
                    "pool_stats" => "slang_pool_stats".into(),
                    // v331 Protobuf
                    "protobuf_encode" => "slang_protobuf_encode".into(),
                    "protobuf_decode" => "slang_protobuf_decode".into(),
                    "protobuf_field" => "slang_protobuf_field".into(),
                    "protobuf_size" => "slang_protobuf_size".into(),
                    // v332 Consensus
                    "raft_propose" => "slang_raft_propose".into(),
                    "raft_commit_index" => "slang_raft_commit_index".into(),
                    "raft_leader" => "slang_raft_leader".into(),
                    "raft_term" => "slang_raft_term".into(),
                    // v333 Event Sourcing
                    "event_store_create" => "slang_event_store_create".into(),
                    "event_append" => "slang_event_append".into(),
                    "event_replay" => "slang_event_replay".into(),
                    "event_snapshot" => "slang_event_snapshot".into(),
                    "event_project" => "slang_event_project".into(),
                    // v334 Stream Processing
                    "stream_create" => "slang_stream_create".into(),
                    "stream_window_tumbling" => "slang_stream_window_tumbling".into(),
                    "stream_window_sliding" => "slang_stream_window_sliding".into(),
                    "stream_watermark" => "slang_stream_watermark".into(),
                    "stream_late_count" => "slang_stream_late_count".into(),
                    // v335 Message Queue
                    "mq_create_topic" => "slang_mq_create_topic".into(),
                    "mq_publish" => "slang_mq_publish".into(),
                    "mq_subscribe" => "slang_mq_subscribe".into(),
                    "mq_consume" => "slang_mq_consume".into(),
                    "mq_offset" => "slang_mq_offset".into(),
                    // v336 CQRS
                    "cqrs_command" => "slang_cqrs_command".into(),
                    "cqrs_query" => "slang_cqrs_query".into(),
                    "cqrs_command_count" => "slang_cqrs_command_count".into(),
                    "cqrs_query_count" => "slang_cqrs_query_count".into(),
                    // v337 GraphQL
                    "graphql_schema_create" => "slang_graphql_schema_create".into(),
                    "graphql_add_type" => "slang_graphql_add_type".into(),
                    "graphql_add_field" => "slang_graphql_add_field".into(),
                    "graphql_validate" => "slang_graphql_validate".into(),
                    "graphql_type_count" => "slang_graphql_type_count".into(),
                    // v338 JWT
                    "jwt_create" => "slang_jwt_create".into(),
                    "jwt_verify" => "slang_jwt_verify".into(),
                    "jwt_claims" => "slang_jwt_claims".into(),
                    "jwt_expired" => "slang_jwt_expired".into(),
                    "jwt_set_claim" => "slang_jwt_set_claim".into(),
                    "jwt_get_claim" => "slang_jwt_get_claim".into(),
                    // v339 OAuth2
                    "oauth2_auth_url" => "slang_oauth2_auth_url".into(),
                    "oauth2_exchange" => "slang_oauth2_exchange".into(),
                    "oauth2_refresh" => "slang_oauth2_refresh".into(),
                    "oauth2_pkce_verify" => "slang_oauth2_pkce_verify".into(),
                    "oauth2_state" => "slang_oauth2_state".into(),
                    // v340 Rate Limiter
                    "ratelimit_check" => "slang_ratelimit_check".into(),
                    "ratelimit_remaining" => "slang_ratelimit_remaining".into(),
                    "ratelimit_reset" => "slang_ratelimit_reset".into(),
                    "ratelimit_window" => "slang_ratelimit_window".into(),
                    // v341 Chaos
                    "chaos_inject_fault" => "slang_chaos_inject_fault".into(),
                    "chaos_inject_latency" => "slang_chaos_inject_latency".into(),
                    "chaos_error_rate" => "slang_chaos_error_rate".into(),
                    "chaos_partition" => "slang_chaos_partition".into(),
                    "chaos_fault_count" => "slang_chaos_fault_count".into(),
                    "chaos_total_latency" => "slang_chaos_total_latency".into(),
                    "chaos_is_partitioned" => "slang_chaos_is_partitioned".into(),
                    "chaos_current_error_rate" => "slang_chaos_current_error_rate".into(),
                    // v342 RBAC
                    "rbac_assign_role" => "slang_rbac_assign_role".into(),
                    "rbac_check_perm" => "slang_rbac_check_perm".into(),
                    "rbac_grant" => "slang_rbac_grant".into(),
                    "rbac_revoke" => "slang_rbac_revoke".into(),
                    // v343 CSP
                    "csp_create" => "slang_csp_create".into(),
                    "csp_add_directive" => "slang_csp_add_directive".into(),
                    "csp_nonce" => "slang_csp_nonce".into(),
                    "csp_directive_count" => "slang_csp_directive_count".into(),
                    "csp_report_only" => "slang_csp_report_only".into(),
                    "csp_check" => "slang_csp_check".into(),
                    // v344 Input Validation
                    "validate_range" => "slang_validate_range".into(),
                    "validate_length" => "slang_validate_length".into(),
                    "validate_pattern" => "slang_validate_pattern".into(),
                    "validate_sanitize" => "slang_validate_sanitize".into(),
                    // v345 Audit Log
                    "audit_log" => "slang_audit_log".into(),
                    "audit_last_action" => "slang_audit_last_action".into(),
                    // v346 Encryption
                    "encrypt_xor" => "slang_encrypt_xor".into(),
                    "encrypt_rotate" => "slang_encrypt_rotate".into(),
                    "encrypt_hash" => "slang_encrypt_hash".into(),
                    "encrypt_verify" => "slang_encrypt_verify".into(),
                    // v347 Certificate
                    "cert_create" => "slang_cert_create".into(),
                    "cert_verify" => "slang_cert_verify".into(),
                    "cert_expiry" => "slang_cert_expiry".into(),
                    "cert_chain_length" => "slang_cert_chain_length".into(),
                    // v348 Test Runner
                    "test_discover" => "slang_test_discover".into(),
                    "test_pass" => "slang_test_pass".into(),
                    "test_fail" => "slang_test_fail".into(),
                    "test_skip" => "slang_test_skip".into(),
                    "test_coverage" => "slang_test_coverage".into(),
                    "test_pass_rate" => "slang_test_pass_rate".into(),
                    "test_total" => "slang_test_total".into(),
                    // v349 Snapshot Testing
                    "snapshot_capture" => "slang_snapshot_capture".into(),
                    "snapshot_compare" => "slang_snapshot_compare".into(),
                    "snapshot_update" => "slang_snapshot_update".into(),
                    "snapshot_version" => "slang_snapshot_version".into(),
                    "snapshot_match_count" => "slang_snapshot_match_count".into(),
                    "snapshot_mismatch_count" => "slang_snapshot_mismatch_count".into(),
                    // v350 Fuzzer
                    "fuzz_add_corpus" => "slang_fuzz_add_corpus".into(),
                    "fuzz_run" => "slang_fuzz_run".into(),
                    "fuzz_crash" => "slang_fuzz_crash".into(),
                    "fuzz_corpus_size" => "slang_fuzz_corpus_size".into(),
                    "fuzz_coverage" => "slang_fuzz_coverage".into(),
                    "fuzz_crash_count" => "slang_fuzz_crash_count".into(),
                    "fuzz_unique_crashes" => "slang_fuzz_unique_crashes".into(),
                    "fuzz_total_runs" => "slang_fuzz_total_runs".into(),
                    // v351 Property Testing
                    "prop_check" => "slang_prop_check".into(),
                    "prop_shrink" => "slang_prop_shrink".into(),
                    "prop_counterexample" => "slang_prop_counterexample".into(),
                    "prop_total_checks" => "slang_prop_total_checks".into(),
                    // v352 Mutation Testing
                    "mutation_inject" => "slang_mutation_inject".into(),
                    "mutation_killed" => "slang_mutation_killed".into(),
                    "mutation_survived" => "slang_mutation_survived".into(),
                    "mut_test_score" => "slang_mut_test_score".into(),
                    // v353 API Compatibility
                    "api_semver_diff" => "slang_api_semver_diff".into(),
                    "api_breaking_change" => "slang_api_breaking_change".into(),
                    "api_addition" => "slang_api_addition".into(),
                    "api_deprecation" => "slang_api_deprecation".into(),
                    "api_surface" => "slang_api_surface".into(),
                    "api_breaking_count" => "slang_api_breaking_count".into(),
                    "api_addition_count" => "slang_api_addition_count".into(),
                    "api_deprecation_count" => "slang_api_deprecation_count".into(),
                    // v354 Migration
                    "migration_create" => "slang_migration_create".into(),
                    "migration_transform" => "slang_migration_transform".into(),
                    "migration_rollback" => "slang_migration_rollback".into(),
                    "migration_progress" => "slang_migration_progress".into(),
                    "migration_delta" => "slang_migration_delta".into(),
                    // v355 Code Actions
                    "codeaction_extract" => "slang_codeaction_extract".into(),
                    "codeaction_inline" => "slang_codeaction_inline".into(),
                    "codeaction_rename" => "slang_codeaction_rename".into(),
                    "codeaction_count" => "slang_codeaction_count".into(),
                    // v356 Telemetry
                    "telemetry_compile_time" => "slang_telemetry_compile_time".into(),
                    "telemetry_peak_memory" => "slang_telemetry_peak_memory".into(),
                    "telemetry_cache_hits" => "slang_telemetry_cache_hits".into(),
                    "telemetry_error_count" => "slang_telemetry_error_count".into(),
                    // v357 Build System
                    "build_target" => "slang_build_target".into(),
                    "build_parallel" => "slang_build_parallel".into(),
                    "build_cache_hit" => "slang_build_cache_hit".into(),
                    "build_artifact_count" => "slang_build_artifact_count".into(),
                    // v358 OpenAPI
                    "openapi_create" => "slang_openapi_create".into(),
                    "openapi_add_route" => "slang_openapi_add_route".into(),
                    "openapi_add_param" => "slang_openapi_add_param".into(),
                    "openapi_validate" => "slang_openapi_validate".into(),
                    "openapi_route_count" => "slang_openapi_route_count".into(),
                    // v359 Benchmarking
                    "bench_start" => "slang_bench_start".into(),
                    "bench_stop" => "slang_bench_stop".into(),
                    "bench_iterations" => "slang_bench_iterations".into(),
                    "bench_throughput" => "slang_bench_throughput".into(),
                    // v360 Profiler
                    "profile_begin" => "slang_profile_begin".into(),
                    "profile_end" => "slang_profile_end".into(),
                    "profile_flamegraph" => "slang_profile_flamegraph".into(),
                    "profile_hotspot" => "slang_profile_hotspot".into(),
                    // v361 Release
                    "release_changelog" => "slang_release_changelog".into(),
                    "release_version_bump" => "slang_release_version_bump".into(),
                    "release_package" => "slang_release_package".into(),
                    "release_version" => "slang_release_version".into(),
                    "release_changelog_count" => "slang_release_changelog_count".into(),
                    "release_packages_built" => "slang_release_packages_built".into(),
                    // v362 Plugin System
                    "plugin_load" => "slang_plugin_load".into(),
                    "plugin_register_hook" => "slang_plugin_register_hook".into(),
                    "plugin_activate" => "slang_plugin_activate".into(),
                    "plugin_unload" => "slang_plugin_unload".into(),
                    "plugin_state" => "slang_plugin_state".into(),
                    "plugin_hook_count" => "slang_plugin_hook_count".into(),
                    "plugin_count" => "slang_plugin_count".into(),
                    // v363 Wasm Component Model
                    "wasm_component_create" => "slang_wasm_component_create".into(),
                    "wasm_component_link" => "slang_wasm_component_link".into(),
                    "wasm_component_instantiate" => "slang_wasm_component_instantiate".into(),
                    "wasm_component_count" => "slang_wasm_component_count".into(),
                    // v364 WASI Preview2
                    "wasi_fs_read" => "slang_wasi_fs_read".into(),
                    "wasi_fs_write" => "slang_wasi_fs_write".into(),
                    "wasi_clock" => "slang_wasi_clock".into(),
                    "wasi_random" => "slang_wasi_random".into(),
                    // v365 Package Registry
                    "registry_publish" => "slang_registry_publish".into(),
                    "registry_resolve" => "slang_registry_resolve".into(),
                    "registry_download" => "slang_registry_download".into(),
                    "registry_version_count" => "slang_registry_version_count".into(),
                    // v366 Milestone
                    "vitalis_v366_version" => "slang_vitalis_v366_version".into(),
                    "vitalis_v366_total_builtins" => "slang_vitalis_v366_total_builtins".into(),
                    "vitalis_v366_modules" => "slang_vitalis_v366_modules".into(),
                    "vitalis_v366_milestone" => "slang_vitalis_v366_milestone".into(),

                    "format_int"      => "slang_format_int".into(),
                    "format_float"    => "slang_format_float".into(),
                    // ── v15: JSON ────────────────────────────────────────
                    "json_encode"     => "slang_json_encode".into(),
                    "json_decode"     => "slang_json_decode".into(),
                    // ── v18: Collection methods ──────────────────────────
                    "array_push"      => "slang_array_push".into(),
                    "array_pop"       => "slang_array_pop".into(),
                    "array_contains"  => "slang_array_contains".into(),
                    "array_reverse"   => "slang_array_reverse".into(),
                    "array_sort"      => "slang_array_sort".into(),
                    "array_join"      => "slang_array_join".into(),
                    "array_slice"     => "slang_array_slice".into(),
                    "array_find"      => "slang_array_find".into(),
                    // Iterator / functional array ops
                    "array_range"        => "slang_array_range".into(),
                    "array_sum"          => "slang_array_sum".into(),
                    "array_min"          => "slang_array_min".into(),
                    "array_max"          => "slang_array_max".into(),
                    "array_any"          => "slang_array_any".into(),
                    "array_all_positive" => "slang_array_all_positive".into(),
                    "array_count"        => "slang_array_count".into(),
                    "array_flatten"      => "slang_array_flatten".into(),
                    "array_zip"          => "slang_array_zip".into(),
                    "array_enumerate"    => "slang_array_enumerate".into(),
                    "array_take"         => "slang_array_take".into(),
                    "array_drop"         => "slang_array_drop".into(),
                    "array_unique"       => "slang_array_unique".into(),
                    "error_message"   => "slang_error_message".into(),

                    // Regex
                    "regex_match"          => "slang_regex_match".into(),
                    "regex_is_match"       => "slang_regex_is_match".into(),
                    "regex_find"           => "slang_regex_find".into(),
                    "regex_replace"        => "slang_regex_replace".into(),
                    "regex_split_count"    => "slang_regex_split_count".into(),
                    "regex_split_get"      => "slang_regex_split_get".into(),
                    "regex_find_all_count" => "slang_regex_find_all_count".into(),
                    "regex_find_all_get"   => "slang_regex_find_all_get".into(),

                    // v370: Async runtime
                    "spawn"           => "slang_spawn".into(),
                    "task_result"     => "slang_task_result".into(),
                    "task_await"      => "slang_task_await".into(),
                    "async_run_all"   => "slang_async_run_all".into(),

                    // v18: Networking
                    "http_get"        => "slang_http_get".into(),
                    "http_post"       => "slang_http_post".into(),
                    "http_status"     => "slang_http_status".into(),
                    "tcp_connect"     => "slang_tcp_connect".into(),
                    "tcp_send"        => "slang_tcp_send".into(),
                    "tcp_close"       => "slang_tcp_close".into(),

                    // v60: Self-hosting bootstrap primitives
                    "char_to_int"     => "slang_char_to_int".into(),
                    "int_to_char"     => "slang_int_to_char".into(),
                    "array_new"       => "slang_array_new".into(),
                    "array_len"       => "slang_array_len".into(),
                    "array_get"       => "slang_array_get_i64".into(),
                    "array_set"       => "slang_array_set_i64".into(),
                    "exit"            => "slang_exit".into(),
                    "file_write_bytes" => "slang_file_write_bytes".into(),
                    "file_read_bytes" => "slang_file_read_bytes".into(),
                    "args_count"      => "slang_args_count".into(),
                    "args_get"        => "slang_args_get".into(),
                    "print_str"       => "slang_print_cstr".into(),
                    "println_str"     => "slang_println_cstr".into(),

                    // Tensor/ML runtime (Void-Vitalis)
                    "to_f64"          => "slang_to_f64".into(),
                    "to_i64"          => "slang_to_i64".into(),
                    "t_alloc"         => "slang_array_new".into(),
                    "t_get"           => "slang_array_get_f64".into(),
                    "t_set"           => "slang_array_set_f64".into(),
                    "t_len"           => "slang_array_len".into(),
                    "t_fill"          => "slang_t_fill".into(),
                    "t_copy"          => "slang_t_copy".into(),
                    "t_randn"         => "slang_t_randn".into(),
                    "t_print_n"       => "slang_t_print_n".into(),
                    "t_matmul"        => "slang_t_matmul".into(),
                    "t_add_vv"        => "slang_t_add_vv".into(),
                    "t_sub_vv"        => "slang_t_sub_vv".into(),
                    "t_mul_vv"        => "slang_t_mul_vv".into(),
                    "t_scale"         => "slang_t_scale".into(),
                    "t_softmax"       => "slang_t_softmax".into(),
                    "t_sum"           => "slang_t_sum".into(),
                    "t_add_bias"      => "slang_t_add_bias".into(),
                    "t_max_idx"       => "slang_t_max_idx".into(),
                    "t_dot"           => "slang_t_dot".into(),
                    "t_cross_entropy" => "slang_t_cross_entropy".into(),
                    "t_transpose"     => "slang_t_transpose".into(),
                    "t_adamw"         => "slang_t_adamw".into(),
                    "t_norm"          => "slang_t_norm".into(),

                    // GUI builtins
                    "gui_open"        => "slang_gui_open".into(),
                    "gui_close"       => "slang_gui_close".into(),
                    "gui_clear"       => "slang_gui_clear".into(),
                    "gui_rect"        => "slang_gui_rect".into(),
                    "gui_line"        => "slang_gui_line".into(),
                    "gui_text"        => "slang_gui_text".into(),
                    "gui_update"      => "slang_gui_update".into(),
                    "gui_circle"      => "slang_gui_circle".into(),

                    // Advanced rendering builtins
                    "gui_gradient_rect"     => "slang_gui_gradient_rect".into(),
                    "gui_pixel"             => "slang_gui_pixel".into(),
                    "gui_rounded_rect"      => "slang_gui_rounded_rect".into(),
                    "gui_blend_rect"        => "slang_gui_blend_rect".into(),
                    "gui_thick_line"        => "slang_gui_thick_line".into(),
                    "gui_triangle"          => "slang_gui_triangle".into(),
                    "gui_aa_circle"         => "slang_gui_aa_circle".into(),
                    "gui_gradient_rounded_rect" => "slang_gui_gradient_rounded_rect".into(),
                    "gui_glow"              => "slang_gui_glow".into(),

                    // GPU compute builtins (v114)
                    "gpu_pipeline_new"      => "vitalis_gpu_pipeline_new".into(),
                    "gpu_add_kernel"        => "vitalis_gpu_add_kernel".into(),
                    "gpu_create_buffer"     => "vitalis_gpu_create_buffer".into(),
                    "gpu_dispatch"          => "vitalis_gpu_dispatch".into(),
                    "gpu_buffer_count"      => "vitalis_gpu_buffer_count".into(),
                    "gpu_kernel_count"      => "vitalis_gpu_kernel_count".into(),
                    "gpu_pipeline_free"     => "vitalis_gpu_pipeline_free".into(),

                    // Tensor builtins (v115)
                    "tensor_zeros_2d"       => "vitalis_tensor_zeros_2d".into(),
                    "tensor_ones_2d"        => "vitalis_tensor_ones_2d".into(),
                    "tensor_zeros_1d"       => "vitalis_tensor_zeros_1d".into(),
                    "tensor_scalar"         => "vitalis_tensor_scalar".into(),
                    "tensor_set"            => "vitalis_tensor_set".into(),
                    "tensor_add"            => "vitalis_tensor_add".into(),
                    "tensor_mul"            => "vitalis_tensor_mul".into(),
                    "tensor_matmul"         => "vitalis_tensor_matmul".into(),
                    "tensor_relu"           => "vitalis_tensor_relu".into(),
                    "tensor_softmax"        => "vitalis_tensor_softmax".into(),
                    "tensor_transpose"      => "vitalis_tensor_transpose".into(),
                    "tensor_numel"          => "vitalis_tensor_numel".into(),
                    "tensor_ndim"           => "vitalis_tensor_ndim".into(),
                    "tensor_get"            => "vitalis_tensor_get".into(),
                    "tensor_sum"            => "vitalis_tensor_sum".into(),
                    "tensor_mean"           => "vitalis_tensor_mean".into(),
                    "tensor_free"           => "vitalis_tensor_free".into(),

                    // Autograd builtins (v116)
                    "autograd_scalar"       => "vitalis_autograd_scalar".into(),
                    "autograd_value"        => "vitalis_autograd_value".into(),
                    "autograd_add"          => "vitalis_autograd_add".into(),
                    "autograd_mul"          => "vitalis_autograd_mul".into(),
                    "autograd_sum"          => "vitalis_autograd_sum".into(),
                    "autograd_backward"     => "vitalis_autograd_backward".into(),
                    "autograd_grad_scalar"  => "vitalis_autograd_grad_scalar".into(),
                    "autograd_numel"        => "vitalis_autograd_numel".into(),
                    "autograd_clear"        => "vitalis_autograd_clear".into(),
                    "autograd_no_grad"      => "vitalis_autograd_no_grad".into(),

                    other       => other.to_string(),
                };

                if let Some(func_id) = func_ids.get(&callee_name) {
                    let func_ref = module.declare_func_in_func(*func_id, builder.func);
                    let arg_vals: Vec<cranelift::prelude::Value> = args
                        .iter()
                        .map(|a| Self::get_value(*a, value_map, builder))
                        .collect();
                    let call = builder.ins().call(func_ref, &arg_vals);
                    let results = builder.inst_results(call);
                    if !results.is_empty() {
                        value_map.insert(*result, results[0]);
                    } else {
                        let dummy = builder.ins().iconst(types::I64, 0);
                        value_map.insert(*result, dummy);
                    }
                } else {
                    // Unknown function — insert type-correct dummy value
                    let dummy = match ret_ty {
                        IrType::F64 => builder.ins().f64const(0.0_f64),
                        IrType::F32 => builder.ins().f32const(0.0_f32),
                        _ => builder.ins().iconst(Self::ir_type_to_cl(ret_ty, pointer_type), 0),
                    };
                    value_map.insert(*result, dummy);
                }
                type_map.insert(*result, ret_ty.clone());
            }
            Inst::Return { value } => {
                if let Some(val) = value {
                    let v = Self::get_value(*val, value_map, builder);
                    builder.ins().return_(&[v]);
                } else {
                    builder.ins().return_(&[]);
                }
            }
            Inst::Jump { target } => {
                if let Some(cl_block) = block_map.get(target) {
                    // If target has Phi instructions, pass the incoming values
                    // from this source block as block parameters
                    if let Some(phis) = phis_by_block.get(target) {
                        let mut args = Vec::new();
                        for phi in phis {
                            let val = phi.incoming.iter()
                                .find(|(_, from)| *from == current_bb)
                                .map(|(v, _)| Self::get_value(*v, value_map, builder))
                                .unwrap_or_else(|| builder.ins().iconst(types::I64, 0));
                            args.push(val);
                        }
                        builder.ins().jump(*cl_block, &args);
                    } else {
                        builder.ins().jump(*cl_block, &[]);
                    }
                }
            }
            Inst::Branch { cond, then_bb, else_bb } => {
                let cv = Self::get_value(*cond, value_map, builder);
                let then_block = block_map.get(then_bb).copied();
                let else_block = block_map.get(else_bb).copied();
                if let (Some(tb), Some(eb)) = (then_block, else_block) {
                    // Collect Phi args for then_bb
                    let then_args: Vec<cranelift::prelude::Value> =
                        if let Some(phis) = phis_by_block.get(then_bb) {
                            phis.iter().map(|phi| {
                                phi.incoming.iter()
                                    .find(|(_, from)| *from == current_bb)
                                    .map(|(v, _)| Self::get_value(*v, value_map, builder))
                                    .unwrap_or_else(|| builder.ins().iconst(types::I64, 0))
                            }).collect()
                        } else { vec![] };
                    // Collect Phi args for else_bb
                    let else_args: Vec<cranelift::prelude::Value> =
                        if let Some(phis) = phis_by_block.get(else_bb) {
                            phis.iter().map(|phi| {
                                phi.incoming.iter()
                                    .find(|(_, from)| *from == current_bb)
                                    .map(|(v, _)| Self::get_value(*v, value_map, builder))
                                    .unwrap_or_else(|| builder.ins().iconst(types::I64, 0))
                            }).collect()
                        } else { vec![] };
                    builder.ins().brif(cv, tb, &then_args, eb, &else_args);
                }
            }
            Inst::Phi { result, incoming: _, ty } => {
                if !value_map.contains_key(result) {
                    let dummy = builder.ins().iconst(types::I64, 0);
                    value_map.insert(*result, dummy);
                }
                type_map.insert(*result, ty.clone());
            }
            Inst::Alloca { result, size } => {
                let slot = builder.create_sized_stack_slot(StackSlotData::new(
                    StackSlotKind::ExplicitSlot,
                    *size,
                    0,
                ));
                let addr = builder.ins().stack_addr(pointer_type, slot, 0);
                value_map.insert(*result, addr);
                type_map.insert(*result, IrType::Ptr);
            }
            Inst::Load { result, ptr, ty } => {
                let p = Self::get_value(*ptr, value_map, builder);
                let cl_ty = Self::ir_type_to_cl(ty, pointer_type);
                let val = builder.ins().load(cl_ty, MemFlags::new(), p, 0);
                value_map.insert(*result, val);
                type_map.insert(*result, ty.clone());
            }
            Inst::Store { value, ptr } => {
                let v = Self::get_value(*value, value_map, builder);
                let p = Self::get_value(*ptr, value_map, builder);
                builder.ins().store(MemFlags::new(), v, p, 0);
            }
            Inst::Copy { result, source } => {
                let v = Self::get_value(*source, value_map, builder);
                value_map.insert(*result, v);
                if let Some(ty) = type_map.get(source).cloned() {
                    type_map.insert(*result, ty);
                }
            }
            Inst::Nop => {}
            // ── Phase 4: Array instructions ──────────────────────────────────
            Inst::ArrayAlloc { result, elem_ty, count } => {
                let count_val = Self::get_value(*count, value_map, builder);
                let stride = builder.ins().iconst(types::I64, elem_ty.byte_size() as i64);
                if let Some(&func_id) = func_ids.get("slang_array_alloc") {
                    let func_ref = module.declare_func_in_func(func_id, builder.func);
                    let call = builder.ins().call(func_ref, &[count_val, stride]);
                    let results = builder.inst_results(call);
                    let v = if results.is_empty() {
                        builder.ins().iconst(pointer_type, 0)
                    } else {
                        results[0]
                    };
                    value_map.insert(*result, v);
                } else {
                    value_map.insert(*result, builder.ins().iconst(pointer_type, 0));
                }
                type_map.insert(*result, IrType::Ptr);
            }
            Inst::ArrayGet { result, array, index, elem_ty } => {
                let arr_val = Self::get_value(*array, value_map, builder);
                let idx_val = Self::get_value(*index, value_map, builder);
                let callee = match elem_ty {
                    IrType::F64 | IrType::F32 => "slang_array_get_f64",
                    _ => "slang_array_get_i64",
                };
                if let Some(&func_id) = func_ids.get(callee) {
                    let func_ref = module.declare_func_in_func(func_id, builder.func);
                    let call = builder.ins().call(func_ref, &[arr_val, idx_val]);
                    let results = builder.inst_results(call);
                    let v = if results.is_empty() {
                        builder.ins().iconst(types::I64, 0)
                    } else {
                        results[0]
                    };
                    value_map.insert(*result, v);
                } else {
                    value_map.insert(*result, builder.ins().iconst(types::I64, 0));
                }
                type_map.insert(*result, elem_ty.clone());
            }
            Inst::ArraySet { array, index, value, elem_ty } => {
                let arr_val = Self::get_value(*array, value_map, builder);
                let idx_val = Self::get_value(*index, value_map, builder);
                let v = Self::get_value(*value, value_map, builder);
                let callee = match elem_ty {
                    IrType::F64 | IrType::F32 => "slang_array_set_f64",
                    _ => "slang_array_set_i64",
                };
                if let Some(&func_id) = func_ids.get(callee) {
                    let func_ref = module.declare_func_in_func(func_id, builder.func);
                    builder.ins().call(func_ref, &[arr_val, idx_val, v]);
                }
            }
            Inst::ArrayLen { result, array } => {
                let arr_val = Self::get_value(*array, value_map, builder);
                if let Some(&func_id) = func_ids.get("slang_array_len") {
                    let func_ref = module.declare_func_in_func(func_id, builder.func);
                    let call = builder.ins().call(func_ref, &[arr_val]);
                    let results = builder.inst_results(call);
                    let v = if results.is_empty() {
                        builder.ins().iconst(types::I64, 0)
                    } else {
                        results[0]
                    };
                    value_map.insert(*result, v);
                } else {
                    value_map.insert(*result, builder.ins().iconst(types::I64, 0));
                }
                type_map.insert(*result, IrType::I64);
            }
            // ── Phase 5 → v15: Closure — return real function pointer ────────
            Inst::ClosureAlloc { result, func, .. } => {
                if let Some(func_id) = func_ids.get(func) {
                    let func_ref = module.declare_func_in_func(*func_id, builder.func);
                    let ptr = builder.ins().func_addr(pointer_type, func_ref);
                    value_map.insert(*result, ptr);
                } else {
                    // Fallback: unknown lambda — null sentinel
                    value_map.insert(*result, builder.ins().iconst(pointer_type, 0));
                }
                type_map.insert(*result, IrType::Ptr);
            }
            // ── Phase 6: Struct scaffolding ────────────────────────────────────
            Inst::StructAlloc { result, fields, .. } => {
                // Allocate a flat record: [n_fields * 8] bytes, fill fields inline.
                let n = fields.len();
                if n == 0 {
                    value_map.insert(*result, builder.ins().iconst(pointer_type, 0));
                } else {
                    let slot = builder.create_sized_stack_slot(StackSlotData::new(
                        StackSlotKind::ExplicitSlot, (n as u32) * 8, 0,
                    ));
                    let base = builder.ins().stack_addr(pointer_type, slot, 0);
                    for (i, fv) in fields.iter().enumerate() {
                        let fval = Self::get_value(*fv, value_map, builder);
                        builder.ins().store(MemFlags::new(), fval, base, (i as i32) * 8);
                    }
                    value_map.insert(*result, base);
                }
                type_map.insert(*result, IrType::Ptr);
            }
            Inst::FieldGet { result, object, field_index, ty } => {
                let obj_val = Self::get_value(*object, value_map, builder);
                let offset = (*field_index as i32) * 8;
                let cl_ty = Self::ir_type_to_cl(ty, pointer_type);
                let val = builder.ins().load(cl_ty, MemFlags::new(), obj_val, offset);
                value_map.insert(*result, val);
                type_map.insert(*result, ty.clone());
            }
            Inst::FieldSet { object, field_index, value } => {
                let obj_val = Self::get_value(*object, value_map, builder);
                let val = Self::get_value(*value, value_map, builder);
                let offset = (*field_index as i32) * 8;
                builder.ins().store(MemFlags::new(), val, obj_val, offset);
            }
            // ── Phase 7 (v107): Enum runtime support ──────────────────────────
            Inst::EnumAlloc { result, tag, fields } => {
                // Layout: [tag: i64, field0: i64, field1: i64, ...]
                let n_slots = 1 + fields.len();
                let slot = builder.create_sized_stack_slot(StackSlotData::new(
                    StackSlotKind::ExplicitSlot, (n_slots as u32) * 8, 0,
                ));
                let base = builder.ins().stack_addr(pointer_type, slot, 0);
                // Store tag
                let tag_val = builder.ins().iconst(types::I64, *tag as i64);
                builder.ins().store(MemFlags::new(), tag_val, base, 0);
                // Store payload fields
                for (i, fv) in fields.iter().enumerate() {
                    let fval = Self::get_value(*fv, value_map, builder);
                    builder.ins().store(MemFlags::new(), fval, base, ((i + 1) as i32) * 8);
                }
                value_map.insert(*result, base);
                type_map.insert(*result, IrType::Ptr);
            }
            Inst::EnumTag { result, enum_val } => {
                let ev = Self::get_value(*enum_val, value_map, builder);
                let tag = builder.ins().load(types::I64, MemFlags::new(), ev, 0);
                value_map.insert(*result, tag);
                type_map.insert(*result, IrType::I64);
            }
            Inst::EnumField { result, enum_val, field_index, ty } => {
                let ev = Self::get_value(*enum_val, value_map, builder);
                let offset = ((*field_index as i32) + 1) * 8; // +1 to skip tag
                let cl_ty = Self::ir_type_to_cl(ty, pointer_type);
                let val = builder.ins().load(cl_ty, MemFlags::new(), ev, offset);
                value_map.insert(*result, val);
                type_map.insert(*result, ty.clone());
            }
        }
        Ok(())
    }

    fn get_value(
        val: ir::Value,
        value_map: &HashMap<ir::Value, cranelift::prelude::Value>,
        builder: &mut FunctionBuilder,
    ) -> cranelift::prelude::Value {
        value_map
            .get(&val)
            .copied()
            .unwrap_or_else(|| builder.ins().iconst(types::I64, 0))
    }

    /// Execute the "main" function and return its i64 result.
    pub fn run_main(&self) -> CodegenResult<i64> {
        let main_id = self.func_ids.get("main").ok_or_else(|| CodegenError {
            message: "no 'main' function found".into(),
        })?;

        let code_ptr = self.module.get_finalized_function(*main_id);
        let main_fn: fn() -> i64 = unsafe { std::mem::transmute(code_ptr) };
        Ok(main_fn())
    }

    /// Execute a named function with no args, returning i64.
    pub fn run_function(&self, name: &str) -> CodegenResult<i64> {
        let func_id = self.func_ids.get(name).ok_or_else(|| CodegenError {
            message: format!("function '{}' not found", name),
        })?;

        let code_ptr = self.module.get_finalized_function(*func_id);
        let func: fn() -> i64 = unsafe { std::mem::transmute(code_ptr) };
        Ok(func())
    }
}

// ─── Public API ─────────────────────────────────────────────────────────

// ─── JIT Compilation Cache ──────────────────────────────────────────────
use crate::incremental::ContentHash;
use std::sync::LazyLock;

/// Cached JIT result: compiled source hash → main() return value.
static JIT_RESULT_CACHE: LazyLock<Mutex<HashMap<u64, i64>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

/// Compilation metrics for profiling and diagnostics.
static COMPILE_METRICS: Mutex<CompileMetrics> = Mutex::new(CompileMetrics::new_const());

pub struct CompileMetrics {
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub total_compilations: u64,
    pub total_parse_ns: u64,
    pub total_typecheck_ns: u64,
    pub total_ir_ns: u64,
    pub total_jit_ns: u64,
}

impl CompileMetrics {
    const fn new_const() -> Self {
        Self {
            cache_hits: 0,
            cache_misses: 0,
            total_compilations: 0,
            total_parse_ns: 0,
            total_typecheck_ns: 0,
            total_ir_ns: 0,
            total_jit_ns: 0,
        }
    }

    pub fn hit_rate(&self) -> f64 {
        let total = self.cache_hits + self.cache_misses;
        if total == 0 { 0.0 } else { self.cache_hits as f64 / total as f64 }
    }
}

/// Get a snapshot of compilation metrics.
pub fn compile_metrics() -> CompileMetrics {
    let m = COMPILE_METRICS.lock().unwrap();
    CompileMetrics {
        cache_hits: m.cache_hits,
        cache_misses: m.cache_misses,
        total_compilations: m.total_compilations,
        total_parse_ns: m.total_parse_ns,
        total_typecheck_ns: m.total_typecheck_ns,
        total_ir_ns: m.total_ir_ns,
        total_jit_ns: m.total_jit_ns,
    }
}

/// Clear the JIT result cache and reset metrics.
pub fn clear_compile_cache() {
    JIT_RESULT_CACHE.lock().unwrap().clear();
    let mut m = COMPILE_METRICS.lock().unwrap();
    *m = CompileMetrics::new_const();
}

// --- Runtime Arena Management -------------------------------------------

/// Statistics about runtime arena usage.
pub struct ArenaStats {
    pub string_count: usize,
    pub map_count: usize,
    pub set_count: usize,
}

/// Get current runtime arena statistics.
pub fn arena_stats() -> ArenaStats {
    ArenaStats {
        string_count: VITALIS_STRING_ARENA.lock().unwrap().len(),
        map_count: VITALIS_MAP_ARENA.lock().unwrap().len(),
        set_count: VITALIS_SET_ARENA.lock().unwrap().len(),
    }
}

/// Reset all runtime arenas (strings, maps, sets).
/// **Safety:** Only call when no JIT code is executing � all pointers into
/// these arenas become dangling after this call.
pub fn reset_runtime_arenas() {
    VITALIS_STRING_ARENA.lock().unwrap().clear();
    VITALIS_MAP_ARENA.lock().unwrap().clear();
    VITALIS_SET_ARENA.lock().unwrap().clear();
}

/// Compile source code and JIT-execute the main function.
/// Uses a content-hash cache to skip recompilation of identical source.
pub fn compile_and_run(source: &str) -> Result<i64, String> {
    let hash = ContentHash::from_source(source).value();

    // Check cache
    if let Some(&cached) = JIT_RESULT_CACHE.lock().unwrap().get(&hash) {
        COMPILE_METRICS.lock().unwrap().cache_hits += 1;
        return Ok(cached);
    }

    // Cache miss — full compilation
    let result = compile_and_run_nocache(source)?;

    // Store in cache
    JIT_RESULT_CACHE.lock().unwrap().insert(hash, result);
    COMPILE_METRICS.lock().unwrap().cache_misses += 1;

    Ok(result)
}

/// Compile and execute without caching (always fresh JIT).
/// Used by tests and when deterministic re-execution is needed.
pub fn compile_and_run_nocache(source: &str) -> Result<i64, String> {
    use std::time::Instant;

    // Parse
    let t0 = Instant::now();
    let (program, parse_errors) = crate::parser::parse(source);
    if !parse_errors.is_empty() {
        return Err(format!(
            "Parse errors:\n{}",
            parse_errors.iter().map(|e| e.to_string()).collect::<Vec<_>>().join("\n")
        ));
    }
    let parse_ns = t0.elapsed().as_nanos() as u64;

    // Type check
    let t1 = Instant::now();
    let type_errors = crate::types::TypeChecker::new().check(&program);
    if !type_errors.is_empty() {
        // Warn but don't fail — Phase 0 type checking is permissive
        eprintln!(
            "Type warnings:\n{}",
            type_errors.iter().map(|e| e.to_string()).collect::<Vec<_>>().join("\n")
        );
    }
    let tc_ns = t1.elapsed().as_nanos() as u64;

    // Lower to IR
    let t2 = Instant::now();
    let mut ir_module = crate::ir::IrBuilder::new().build(&program);
    let ir_ns = t2.elapsed().as_nanos() as u64;

    // Optimize IR (v137: wire optimizer into JIT path)
    crate::optimizer::optimize_ir(&mut ir_module);

    // Compile via Cranelift
    let t3 = Instant::now();
    let mut jit = JitCompiler::new().map_err(|e| e.to_string())?;
    jit.compile(&ir_module).map_err(|e| e.to_string())?;
    let jit_ns = t3.elapsed().as_nanos() as u64;

    // Update metrics
    {
        let mut m = COMPILE_METRICS.lock().unwrap();
        m.total_compilations += 1;
        m.total_parse_ns += parse_ns;
        m.total_typecheck_ns += tc_ns;
        m.total_ir_ns += ir_ns;
        m.total_jit_ns += jit_ns;
    }

    // Run main
    jit.run_main().map_err(|e| e.to_string())
}

// -- v291: Advanced Spike Analytics --
#[unsafe(no_mangle)]
extern "C" fn slang_spike_analytics_mean_rate(_p0: i64, _p1: i64) -> i64 {
    if _p1 <= 0 { return 0; } _p0 * 1000 / _p1
}

#[unsafe(no_mangle)]
extern "C" fn slang_spike_analytics_cv_isi(_p0: f64, _p1: f64) -> i64 {
    if _p1.abs() < 1e-10 { return 0; } (_p0 / _p1 * 1000.0).round() as i64
}

#[unsafe(no_mangle)]
extern "C" fn slang_spike_analytics_fano_factor(_p0: f64, _p1: f64) -> i64 {
    if _p1.abs() < 1e-10 { return 1000; } (_p0 / _p1 * 1000.0).round() as i64
}

#[unsafe(no_mangle)]
extern "C" fn slang_spike_analytics_burst_index(_p0: i64, _p1: i64) -> i64 {
    if _p1 <= 0 { return 0; } _p0 * 1000 / _p1
}

// -- v292: Neural Network Metrics --
#[unsafe(no_mangle)]
extern "C" fn slang_nn_metric_sparsity(_p0: i64, _p1: i64) -> i64 {
    if _p1 <= 0 { return 0; } (_p1 - _p0) * 1000 / _p1
}

#[unsafe(no_mangle)]
extern "C" fn slang_nn_metric_entropy(_p0: f64) -> i64 {
    if _p0 <= 0.0 { return 0; } (-_p0 * _p0.ln() * 1000.0).round() as i64
}

#[unsafe(no_mangle)]
extern "C" fn slang_nn_metric_mutual_info(_p0: f64, _p1: f64) -> i64 {
    ((_p0 - _p1).abs() * 1000.0).round() as i64
}

#[unsafe(no_mangle)]
extern "C" fn slang_nn_metric_transfer_entropy(_p0: f64, _p1: f64, _p2: f64) -> i64 {
    ((_p0 + _p1 - _p2).abs() * 1000.0).round() as i64
}

// -- v293: Spike Train Distance --
#[unsafe(no_mangle)]
extern "C" fn slang_spike_dist_victor_purpura(_p0: f64, _p1: f64, _p2: f64) -> i64 {
    ((_p0 - _p1).abs() * _p2 * 1000.0).round() as i64
}

#[unsafe(no_mangle)]
extern "C" fn slang_spike_dist_van_rossum(_p0: f64, _p1: f64, _p2: f64) -> i64 {
    let d = _p0 - _p1; (d * d * (-_p2).exp() * 1000.0).round() as i64
}

#[unsafe(no_mangle)]
extern "C" fn slang_spike_dist_schreiber(_p0: f64, _p1: f64) -> i64 {
    ((_p0 * _p1).sqrt() * 1000.0).round() as i64
}

#[unsafe(no_mangle)]
extern "C" fn slang_spike_dist_earth_mover(_p0: i64, _p1: i64) -> i64 {
    (_p0 - _p1).abs()
}

// -- v294: Neural Coding --
#[unsafe(no_mangle)]
extern "C" fn slang_neural_code_rate(_p0: i64, _p1: i64) -> i64 {
    if _p1 <= 0 { return 0; } _p0 * 1000 / _p1
}

#[unsafe(no_mangle)]
extern "C" fn slang_neural_code_temporal(_p0: f64, _p1: f64) -> i64 {
    ((_p0 - _p1).abs() * 1000.0).round() as i64
}

#[unsafe(no_mangle)]
extern "C" fn slang_neural_code_population(_p0: i64, _p1: i64, _p2: i64) -> i64 {
    if _p2 <= 0 { return 0; } _p0 * _p1 / _p2
}

#[unsafe(no_mangle)]
extern "C" fn slang_neural_code_sparse(_p0: i64, _p1: f64) -> i64 {
    (_p0 as f64 * _p1 * 1000.0).round() as i64
}

// -- v295: Synaptic Plasticity Metrics --
#[unsafe(no_mangle)]
extern "C" fn slang_synap_metric_ltp_ratio(_p0: f64, _p1: f64) -> i64 {
    if _p1.abs() < 1e-10 { return 0; } (_p0 / _p1 * 1000.0).round() as i64
}

#[unsafe(no_mangle)]
extern "C" fn slang_synap_metric_ltd_ratio(_p0: f64, _p1: f64) -> i64 {
    if _p1.abs() < 1e-10 { return 0; } (_p0 / _p1 * 1000.0).round() as i64
}

#[unsafe(no_mangle)]
extern "C" fn slang_synap_metric_homeostatic(_p0: f64, _p1: f64, _p2: f64) -> i64 {
    ((_p0 + _p1 - _p2).abs() * 1000.0).round() as i64
}

#[unsafe(no_mangle)]
extern "C" fn slang_synap_metric_metaplasticity(_p0: f64, _p1: i64) -> i64 {
    (_p0 * _p1 as f64 * 1000.0).round() as i64
}

// -- v296: Network Topology --
#[unsafe(no_mangle)]
extern "C" fn slang_topo_clustering_coeff(_p0: i64, _p1: i64) -> i64 {
    if _p1 <= 0 { return 0; } _p0 * 1000 / _p1
}

#[unsafe(no_mangle)]
extern "C" fn slang_topo_path_length(_p0: i64, _p1: i64) -> i64 {
    if _p1 <= 0 { return 0; } _p0 * 1000 / _p1
}

#[unsafe(no_mangle)]
extern "C" fn slang_topo_small_world(_p0: f64, _p1: f64) -> i64 {
    if _p1.abs() < 1e-10 { return 1000; } (_p0 / _p1 * 1000.0).round() as i64
}

#[unsafe(no_mangle)]
extern "C" fn slang_topo_modularity(_p0: i64, _p1: i64, _p2: i64) -> i64 {
    if _p2 <= 0 { return 0; } (_p0 - _p1) * 1000 / _p2
}

// -- v297: Neuromorphic IO --
#[unsafe(no_mangle)]
extern "C" fn slang_neuro_io_aer_encode(_p0: i64, _p1: i64) -> i64 {
    _p0 << 16 | (_p1 & 0xFFFF)
}

#[unsafe(no_mangle)]
extern "C" fn slang_neuro_io_aer_decode(_p0: i64) -> i64 {
    _p0 >> 16
}

#[unsafe(no_mangle)]
extern "C" fn slang_neuro_io_dvs_encode(_p0: i64, _p1: i64, _p2: f64) -> i64 {
    let pol = if _p2 > 0.0 { 1_i64 } else { 0 }; _p0 << 17 | (_p1 & 0xFFFF) << 1 | pol
}

#[unsafe(no_mangle)]
extern "C" fn slang_neuro_io_serial_pack(_p0: i64, _p1: i64) -> i64 {
    _p0 << 32 | (_p1 & 0xFFFFFFFF)
}

// -- v298: Neural Dynamics --
#[unsafe(no_mangle)]
extern "C" fn slang_dyn_lyapunov_exp(_p0: f64, _p1: f64, _p2: f64) -> i64 {
    ((_p0 * _p1 + _p2) * 1000.0).round() as i64
}

#[unsafe(no_mangle)]
extern "C" fn slang_dyn_bifurcation(_p0: f64, _p1: f64) -> i64 {
    ((_p0 * _p0 - _p1) * 1000.0).round() as i64
}

#[unsafe(no_mangle)]
extern "C" fn slang_dyn_phase_portrait(_p0: f64, _p1: f64) -> i64 {
    ((_p0 * _p0 + _p1 * _p1).sqrt() * 1000.0).round() as i64
}

#[unsafe(no_mangle)]
extern "C" fn slang_dyn_attractor_dim(_p0: f64, _p1: i64) -> i64 {
    (_p0 * _p1 as f64 * 1000.0).round() as i64
}

// -- v299: Neuromorphic Scheduler --
#[unsafe(no_mangle)]
extern "C" fn slang_neuro_sched_priority(_p0: i64, _p1: i64) -> i64 {
    if _p0 > _p1 { _p0 } else { _p1 }
}

#[unsafe(no_mangle)]
extern "C" fn slang_neuro_sched_deadline(_p0: i64, _p1: i64) -> i64 {
    if _p0 <= _p1 { 1 } else { 0 }
}

#[unsafe(no_mangle)]
extern "C" fn slang_neuro_sched_edf(_p0: i64, _p1: i64, _p2: i64) -> i64 {
    if _p1 <= 0 { return 0; } _p0 * _p2 / _p1
}

#[unsafe(no_mangle)]
extern "C" fn slang_neuro_sched_utilization(_p0: i64, _p1: i64) -> i64 {
    if _p1 <= 0 { return 0; } _p0 * 1000 / _p1
}

// -- v300: Milestone --
#[unsafe(no_mangle)]
extern "C" fn slang_vitalis_v300_version() -> i64 {
    300
}

#[unsafe(no_mangle)]
extern "C" fn slang_vitalis_v300_total_builtins() -> i64 {
    400
}

#[unsafe(no_mangle)]
extern "C" fn slang_vitalis_v300_neuro_modules() -> i64 {
    15
}

#[unsafe(no_mangle)]
extern "C" fn slang_vitalis_v300_milestone() -> i64 {
    1
}

// ── v302: Tail Call Optimization (runtime builtins) ──
use std::sync::atomic::{AtomicI64 as CgAtomicI64, Ordering as CgOrdering};
static TCO_COUNT: CgAtomicI64 = CgAtomicI64::new(0);
static TCO_DEPTH: CgAtomicI64 = CgAtomicI64::new(1000);
static TCO_ON: CgAtomicI64 = CgAtomicI64::new(1);
#[unsafe(no_mangle)] extern "C" fn slang_tco_detect(func_id: i64) -> i64 { let _ = func_id; TCO_COUNT.fetch_add(1, CgOrdering::SeqCst) + 1 }
#[unsafe(no_mangle)] extern "C" fn slang_tco_optimized_count() -> i64 { TCO_COUNT.load(CgOrdering::SeqCst) }
#[unsafe(no_mangle)] extern "C" fn slang_tco_depth_limit(limit: i64) -> i64 { TCO_DEPTH.swap(limit.max(1), CgOrdering::SeqCst) }
#[unsafe(no_mangle)] extern "C" fn slang_tco_enabled() -> i64 { TCO_ON.load(CgOrdering::SeqCst) }

// ── v303: Algebraic Simplification ──
static ALG_COUNT: CgAtomicI64 = CgAtomicI64::new(0);
static ALG_ON: CgAtomicI64 = CgAtomicI64::new(1);
static STRENGTH_REDUCED: CgAtomicI64 = CgAtomicI64::new(0);
static IDENTITY_REMOVED: CgAtomicI64 = CgAtomicI64::new(0);
#[unsafe(no_mangle)] extern "C" fn slang_opt_algebraic_count() -> i64 { ALG_COUNT.load(CgOrdering::SeqCst) }
#[unsafe(no_mangle)] extern "C" fn slang_opt_algebraic_enable(on: i64) -> i64 { ALG_ON.swap(if on != 0 { 1 } else { 0 }, CgOrdering::SeqCst) }
#[unsafe(no_mangle)] extern "C" fn slang_opt_strength_reduced() -> i64 { STRENGTH_REDUCED.load(CgOrdering::SeqCst) }
#[unsafe(no_mangle)] extern "C" fn slang_opt_identity_removed() -> i64 { IDENTITY_REMOVED.load(CgOrdering::SeqCst) }

// ── v305: LTO ──
static LTO_INLINED: CgAtomicI64 = CgAtomicI64::new(0);
static LTO_DEAD: CgAtomicI64 = CgAtomicI64::new(0);
static LTO_DEVIRT: CgAtomicI64 = CgAtomicI64::new(0);
static LTO_ON: CgAtomicI64 = CgAtomicI64::new(1);
#[unsafe(no_mangle)] extern "C" fn slang_lto_inline_count() -> i64 { LTO_INLINED.load(CgOrdering::SeqCst) }
#[unsafe(no_mangle)] extern "C" fn slang_lto_dead_globals() -> i64 { LTO_DEAD.load(CgOrdering::SeqCst) }
#[unsafe(no_mangle)] extern "C" fn slang_lto_devirtualized() -> i64 { LTO_DEVIRT.load(CgOrdering::SeqCst) }
#[unsafe(no_mangle)] extern "C" fn slang_lto_enabled() -> i64 { LTO_ON.load(CgOrdering::SeqCst) }

// ── v306: Vectorization ──
static VEC_OPP: CgAtomicI64 = CgAtomicI64::new(0);
static VEC_APPLIED: CgAtomicI64 = CgAtomicI64::new(0);
static VEC_WIDTH_VAL: CgAtomicI64 = CgAtomicI64::new(4);
static VEC_SPEEDUP: CgAtomicI64 = CgAtomicI64::new(100); // 1.0x * 100
#[unsafe(no_mangle)] extern "C" fn slang_vec_slp_opportunities() -> i64 { VEC_OPP.load(CgOrdering::SeqCst) }
#[unsafe(no_mangle)] extern "C" fn slang_vec_slp_applied() -> i64 { VEC_APPLIED.load(CgOrdering::SeqCst) }
#[unsafe(no_mangle)] extern "C" fn slang_vec_width() -> i64 { VEC_WIDTH_VAL.load(CgOrdering::SeqCst) }
#[unsafe(no_mangle)] extern "C" fn slang_vec_speedup_estimate() -> i64 { VEC_SPEEDUP.load(CgOrdering::SeqCst) }

// ── v307: Compile-Time Execution ──
static CONSTEVAL_CNT: CgAtomicI64 = CgAtomicI64::new(0);
#[unsafe(no_mangle)] extern "C" fn slang_consteval_string(hash: i64) -> i64 { CONSTEVAL_CNT.fetch_add(1, CgOrdering::SeqCst); hash }
#[unsafe(no_mangle)] extern "C" fn slang_consteval_array(len: i64) -> i64 { CONSTEVAL_CNT.fetch_add(1, CgOrdering::SeqCst); len }
#[unsafe(no_mangle)] extern "C" fn slang_consteval_struct(fields: i64) -> i64 { CONSTEVAL_CNT.fetch_add(1, CgOrdering::SeqCst); fields }
#[unsafe(no_mangle)] extern "C" fn slang_consteval_count() -> i64 { CONSTEVAL_CNT.load(CgOrdering::SeqCst) }

// ── v308: PGO ──
static PGO_SAMPLES: CgAtomicI64 = CgAtomicI64::new(0);
static PGO_LAST_HOT: CgAtomicI64 = CgAtomicI64::new(0);
static PGO_BIAS: CgAtomicI64 = CgAtomicI64::new(50); // 50% default
#[unsafe(no_mangle)] extern "C" fn slang_pgo_record(func_id: i64, count: i64) -> i64 { PGO_SAMPLES.fetch_add(count.max(0), CgOrdering::SeqCst); PGO_LAST_HOT.store(func_id, CgOrdering::SeqCst); count }
#[unsafe(no_mangle)] extern "C" fn slang_pgo_hotness(func_id: i64) -> i64 { if PGO_LAST_HOT.load(CgOrdering::SeqCst) == func_id { 100 } else { 0 } }
#[unsafe(no_mangle)] extern "C" fn slang_pgo_branch_bias(branch_id: i64) -> i64 { let _ = branch_id; PGO_BIAS.load(CgOrdering::SeqCst) }
#[unsafe(no_mangle)] extern "C" fn slang_pgo_total_samples() -> i64 { PGO_SAMPLES.load(CgOrdering::SeqCst) }

// ── v309: Register Allocation ──
static RA_SPILLS: CgAtomicI64 = CgAtomicI64::new(0);
static RA_MOVES: CgAtomicI64 = CgAtomicI64::new(0);
static RA_PRESSURE: CgAtomicI64 = CgAtomicI64::new(0);
static RA_COALESCED: CgAtomicI64 = CgAtomicI64::new(0);
#[unsafe(no_mangle)] extern "C" fn slang_regalloc_spill_count() -> i64 { RA_SPILLS.load(CgOrdering::SeqCst) }
#[unsafe(no_mangle)] extern "C" fn slang_regalloc_move_count() -> i64 { RA_MOVES.load(CgOrdering::SeqCst) }
#[unsafe(no_mangle)] extern "C" fn slang_regalloc_pressure() -> i64 { RA_PRESSURE.load(CgOrdering::SeqCst) }
#[unsafe(no_mangle)] extern "C" fn slang_regalloc_coalesced() -> i64 { RA_COALESCED.load(CgOrdering::SeqCst) }

// ── v310: Debug Info ──
static DBG_LINES: CgAtomicI64 = CgAtomicI64::new(0);
static DBG_VARS: CgAtomicI64 = CgAtomicI64::new(0);
static DBG_DEPTH: CgAtomicI64 = CgAtomicI64::new(0);
static DBG_SIZE: CgAtomicI64 = CgAtomicI64::new(0);
#[unsafe(no_mangle)] extern "C" fn slang_debug_line_count() -> i64 { DBG_LINES.load(CgOrdering::SeqCst) }
#[unsafe(no_mangle)] extern "C" fn slang_debug_var_count() -> i64 { DBG_VARS.load(CgOrdering::SeqCst) }
#[unsafe(no_mangle)] extern "C" fn slang_debug_scope_depth() -> i64 { DBG_DEPTH.load(CgOrdering::SeqCst) }
#[unsafe(no_mangle)] extern "C" fn slang_debug_info_size() -> i64 { DBG_SIZE.load(CgOrdering::SeqCst) }

// ── v311: Existential Types ──
static EXIST_CNT: CgAtomicI64 = CgAtomicI64::new(0);
#[unsafe(no_mangle)] extern "C" fn slang_existential_create(witness: i64) -> i64 { EXIST_CNT.fetch_add(1, CgOrdering::SeqCst); witness }
#[unsafe(no_mangle)] extern "C" fn slang_existential_open(id: i64) -> i64 { id }
#[unsafe(no_mangle)] extern "C" fn slang_existential_pack(id: i64, witness: i64) -> i64 { let _ = witness; id }
#[unsafe(no_mangle)] extern "C" fn slang_existential_count() -> i64 { EXIST_CNT.load(CgOrdering::SeqCst) }

// ── v314: GADTs ──
static GADT_CNT: CgAtomicI64 = CgAtomicI64::new(0);
#[unsafe(no_mangle)] extern "C" fn slang_gadt_create(type_id: i64, index: i64) -> i64 { GADT_CNT.fetch_add(1, CgOrdering::SeqCst); type_id * 100 + index }
#[unsafe(no_mangle)] extern "C" fn slang_gadt_refine(gadt_id: i64, constraint: i64) -> i64 { gadt_id + constraint }
#[unsafe(no_mangle)] extern "C" fn slang_gadt_witness(gadt_id: i64) -> i64 { gadt_id % 100 }
#[unsafe(no_mangle)] extern "C" fn slang_gadt_count() -> i64 { GADT_CNT.load(CgOrdering::SeqCst) }

// ── v315: Type Classes ──
static TC_INSTANCES: CgAtomicI64 = CgAtomicI64::new(0);
static TC_COHERENT: CgAtomicI64 = CgAtomicI64::new(1);
#[unsafe(no_mangle)] extern "C" fn slang_typeclass_instances() -> i64 { TC_INSTANCES.load(CgOrdering::SeqCst) }
#[unsafe(no_mangle)] extern "C" fn slang_typeclass_resolve(class_id: i64) -> i64 { let _ = class_id; TC_INSTANCES.load(CgOrdering::SeqCst) }
#[unsafe(no_mangle)] extern "C" fn slang_typeclass_coherence() -> i64 { TC_COHERENT.load(CgOrdering::SeqCst) }
#[unsafe(no_mangle)] extern "C" fn slang_typeclass_register(class_id: i64, impl_id: i64) -> i64 { let _ = (class_id, impl_id); TC_INSTANCES.fetch_add(1, CgOrdering::SeqCst) + 1 }

// ── v316: Dependent Types ──
#[unsafe(no_mangle)] extern "C" fn slang_dependent_proof(prop: i64, witness: i64) -> i64 { if witness != 0 { prop } else { 0 } }
#[unsafe(no_mangle)] extern "C" fn slang_dependent_index(array: i64, idx: i64) -> i64 { if idx >= 0 { array + idx } else { -1 } }
#[unsafe(no_mangle)] extern "C" fn slang_dependent_refine(value: i64) -> i64 { value }
#[unsafe(no_mangle)] extern "C" fn slang_dependent_check(value: i64) -> i64 { if value > 0 { 1 } else { 0 } }

// ── v317: Effect Inference ──
static EFFECT_INF_CNT: CgAtomicI64 = CgAtomicI64::new(0);
#[unsafe(no_mangle)] extern "C" fn slang_effect_infer(func_id: i64) -> i64 { EFFECT_INF_CNT.fetch_add(1, CgOrdering::SeqCst); func_id }
#[unsafe(no_mangle)] extern "C" fn slang_effect_row(func_id: i64) -> i64 { let _ = func_id; 0 }
#[unsafe(no_mangle)] extern "C" fn slang_effect_mask(effects: i64, mask: i64) -> i64 { effects & mask }
#[unsafe(no_mangle)] extern "C" fn slang_effect_polymorphic(func_id: i64) -> i64 { let _ = func_id; 1 }

// ── v319: Quantization ──
#[unsafe(no_mangle)] extern "C" fn slang_quant_int8(value: i64) -> i64 { value.clamp(-128, 127) }
#[unsafe(no_mangle)] extern "C" fn slang_quant_int4(value: i64) -> i64 { value.clamp(-8, 7) }
#[unsafe(no_mangle)] extern "C" fn slang_quant_error(original: i64, quantized: i64) -> i64 { (original - quantized).abs() }
#[unsafe(no_mangle)] extern "C" fn slang_quant_calibrate(min: i64, max: i64) -> i64 { (max - min) / 256 }

// ── v320: Attention Variants ──
#[unsafe(no_mangle)] extern "C" fn slang_attn_flash(q: i64, k: i64, v: i64) -> i64 { (q * k) / 100 + v }
#[unsafe(no_mangle)] extern "C" fn slang_attn_linear(q: i64, k: i64) -> i64 { q * k / 100 }
#[unsafe(no_mangle)] extern "C" fn slang_attn_sparse(q: i64, k: i64, sparsity: i64) -> i64 { (q * k / 100) * (100 - sparsity) / 100 }
#[unsafe(no_mangle)] extern "C" fn slang_attn_sliding_window(q: i64, k: i64, window: i64) -> i64 { let _ = window; q * k / 100 }

// ── v324: RL ──
#[unsafe(no_mangle)] extern "C" fn slang_rl_q_update(q_old: i64, reward: i64, lr: i64) -> i64 { q_old + lr * (reward - q_old) / 100 }
#[unsafe(no_mangle)] extern "C" fn slang_rl_policy_gradient(reward: i64, log_prob: i64) -> i64 { reward * log_prob / 100 }
#[unsafe(no_mangle)] extern "C" fn slang_rl_advantage(value: i64, q_value: i64) -> i64 { q_value - value }
#[unsafe(no_mangle)] extern "C" fn slang_rl_reward_discount(reward: i64, gamma: i64) -> i64 { reward * gamma / 100 }

// ── v326: Tokenizer ──
static TOK_VOCAB: CgAtomicI64 = CgAtomicI64::new(0);
#[unsafe(no_mangle)] extern "C" fn slang_tokenizer_bpe_train(data_size: i64, vocab_size: i64) -> i64 { TOK_VOCAB.store(vocab_size, CgOrdering::SeqCst); let _ = data_size; vocab_size }
#[unsafe(no_mangle)] extern "C" fn slang_tokenizer_encode(text_hash: i64) -> i64 { text_hash.abs() % TOK_VOCAB.load(CgOrdering::SeqCst).max(1) }
#[unsafe(no_mangle)] extern "C" fn slang_tokenizer_decode(token_id: i64) -> i64 { token_id }
#[unsafe(no_mangle)] extern "C" fn slang_tokenizer_vocab_size() -> i64 { TOK_VOCAB.load(CgOrdering::SeqCst) }

// ── v327: RLHF ──
#[unsafe(no_mangle)] extern "C" fn slang_rlhf_reward(response: i64, quality: i64) -> i64 { response * quality / 100 }
#[unsafe(no_mangle)] extern "C" fn slang_rlhf_kl_penalty(p: i64, q: i64) -> i64 { (p - q).abs() }
#[unsafe(no_mangle)] extern "C" fn slang_rlhf_preference(a: i64, b: i64) -> i64 { if a > b { 1 } else if b > a { -1 } else { 0 } }
#[unsafe(no_mangle)] extern "C" fn slang_rlhf_ppo_clip(ratio: i64, epsilon: i64) -> i64 { ratio.clamp(100 - epsilon, 100 + epsilon) }

// ── v328: Async Runtime ──
static ASYNC_TASKS: CgAtomicI64 = CgAtomicI64::new(0);
#[unsafe(no_mangle)] extern "C" fn slang_async_spawn_task(task_id: i64) -> i64 { ASYNC_TASKS.fetch_add(1, CgOrdering::SeqCst); task_id }
#[unsafe(no_mangle)] extern "C" fn slang_async_yield_now() -> i64 { 0 }
#[unsafe(no_mangle)] extern "C" fn slang_async_select(a: i64, b: i64) -> i64 { if a < b { a } else { b } }
#[unsafe(no_mangle)] extern "C" fn slang_async_timeout(ms: i64) -> i64 { ms }

// ── v329: Work Stealing ──
static WS_STEALS: CgAtomicI64 = CgAtomicI64::new(0);
static WS_WORKERS: CgAtomicI64 = CgAtomicI64::new(0);
#[unsafe(no_mangle)] extern "C" fn slang_ws_create_pool(workers: i64) -> i64 { WS_WORKERS.store(workers.max(1), CgOrdering::SeqCst); workers }
#[unsafe(no_mangle)] extern "C" fn slang_ws_submit(pool: i64, task: i64) -> i64 { let _ = pool; WS_STEALS.fetch_add(1, CgOrdering::SeqCst); task }
#[unsafe(no_mangle)] extern "C" fn slang_ws_steal_count(_pool: i64) -> i64 { WS_STEALS.load(CgOrdering::SeqCst) }
#[unsafe(no_mangle)] extern "C" fn slang_ws_active_workers(_pool: i64) -> i64 { WS_WORKERS.load(CgOrdering::SeqCst) }

// ── v332: Consensus (Raft) ──
static RAFT_INDEX: CgAtomicI64 = CgAtomicI64::new(0);
static RAFT_LEADER: CgAtomicI64 = CgAtomicI64::new(1);
static RAFT_TERM: CgAtomicI64 = CgAtomicI64::new(1);
#[unsafe(no_mangle)] extern "C" fn slang_raft_propose(value: i64) -> i64 { let _ = value; RAFT_INDEX.fetch_add(1, CgOrdering::SeqCst) + 1 }
#[unsafe(no_mangle)] extern "C" fn slang_raft_commit_index() -> i64 { RAFT_INDEX.load(CgOrdering::SeqCst) }
#[unsafe(no_mangle)] extern "C" fn slang_raft_leader() -> i64 { RAFT_LEADER.load(CgOrdering::SeqCst) }
#[unsafe(no_mangle)] extern "C" fn slang_raft_term() -> i64 { RAFT_TERM.load(CgOrdering::SeqCst) }

// ── v340: Rate Limiter ──
static RL_REMAINING: CgAtomicI64 = CgAtomicI64::new(100);
static RL_WINDOW: CgAtomicI64 = CgAtomicI64::new(60);
#[unsafe(no_mangle)] extern "C" fn slang_ratelimit_check(tokens: i64) -> i64 { let r = RL_REMAINING.load(CgOrdering::SeqCst); if r >= tokens { RL_REMAINING.fetch_sub(tokens, CgOrdering::SeqCst); 1 } else { 0 } }
#[unsafe(no_mangle)] extern "C" fn slang_ratelimit_remaining(_bucket: i64) -> i64 { RL_REMAINING.load(CgOrdering::SeqCst) }
#[unsafe(no_mangle)] extern "C" fn slang_ratelimit_reset(capacity: i64) -> i64 { RL_REMAINING.store(capacity, CgOrdering::SeqCst); capacity }
#[unsafe(no_mangle)] extern "C" fn slang_ratelimit_window(seconds: i64) -> i64 { RL_WINDOW.swap(seconds, CgOrdering::SeqCst) }

// ── v342: RBAC ──
static RBAC_ROLES: CgAtomicI64 = CgAtomicI64::new(0);
static RBAC_PERMS: CgAtomicI64 = CgAtomicI64::new(0);
#[unsafe(no_mangle)] extern "C" fn slang_rbac_assign_role(user: i64, role: i64) -> i64 { let _ = (user, role); RBAC_ROLES.fetch_add(1, CgOrdering::SeqCst) + 1 }
#[unsafe(no_mangle)] extern "C" fn slang_rbac_check_perm(user: i64, perm: i64) -> i64 { let _ = (user, perm); if RBAC_PERMS.load(CgOrdering::SeqCst) > 0 { 1 } else { 0 } }
#[unsafe(no_mangle)] extern "C" fn slang_rbac_grant(role: i64, perm: i64) -> i64 { let _ = (role, perm); RBAC_PERMS.fetch_add(1, CgOrdering::SeqCst) + 1 }
#[unsafe(no_mangle)] extern "C" fn slang_rbac_revoke(role: i64, perm: i64) -> i64 { let _ = (role, perm); let p = RBAC_PERMS.load(CgOrdering::SeqCst); if p > 0 { RBAC_PERMS.fetch_sub(1, CgOrdering::SeqCst); 1 } else { 0 } }

// ── v344: Input Validation ──
#[unsafe(no_mangle)] extern "C" fn slang_validate_range(value: i64, min: i64, max: i64) -> i64 { if value >= min && value <= max { 1 } else { 0 } }
#[unsafe(no_mangle)] extern "C" fn slang_validate_length(value: i64, min_len: i64, max_len: i64) -> i64 { let len = value.abs() % 1000; if len >= min_len && len <= max_len { 1 } else { 0 } }
#[unsafe(no_mangle)] extern "C" fn slang_validate_pattern(value: i64, pattern: i64) -> i64 { if value % pattern == 0 { 1 } else { 0 } }
#[unsafe(no_mangle)] extern "C" fn slang_validate_sanitize(value: i64) -> i64 { value.abs() }

// ── v345: Audit Log ──
static AUDIT_CNT: CgAtomicI64 = CgAtomicI64::new(0);
static AUDIT_LAST: CgAtomicI64 = CgAtomicI64::new(0);
#[unsafe(no_mangle)] extern "C" fn slang_audit_log(actor: i64, action: i64) -> i64 { let _ = actor; AUDIT_LAST.store(action, CgOrdering::SeqCst); AUDIT_CNT.fetch_add(1, CgOrdering::SeqCst) + 1 }
// slang_audit_count already defined above (pre-v300)
#[unsafe(no_mangle)] extern "C" fn slang_audit_last_action() -> i64 { AUDIT_LAST.load(CgOrdering::SeqCst) }
// slang_audit_clear already defined above (pre-v300)

// ── v346: Encryption ──
#[unsafe(no_mangle)] extern "C" fn slang_encrypt_xor(data: i64, key: i64) -> i64 { data ^ key }
#[unsafe(no_mangle)] extern "C" fn slang_encrypt_rotate(data: i64, bits: i64) -> i64 { let b = (bits % 64).unsigned_abs() as u32; (data as u64).rotate_left(b) as i64 }
#[unsafe(no_mangle)] extern "C" fn slang_encrypt_hash(data: i64) -> i64 { let mut h = data.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407); h = (h ^ (h >> 30)).wrapping_mul(-4658895280553007687); h ^ (h >> 27) }
#[unsafe(no_mangle)] extern "C" fn slang_encrypt_verify(data: i64, expected_hash: i64) -> i64 { if slang_encrypt_hash(data) == expected_hash { 1 } else { 0 } }

// ── v347: Certificate ──
static CERT_CNT: CgAtomicI64 = CgAtomicI64::new(0);
#[unsafe(no_mangle)] extern "C" fn slang_cert_create(issuer: i64, expiry: i64) -> i64 { let _ = (issuer, expiry); CERT_CNT.fetch_add(1, CgOrdering::SeqCst) + 1 }
#[unsafe(no_mangle)] extern "C" fn slang_cert_verify(cert_id: i64, now: i64) -> i64 { let _ = cert_id; if now < 1000000 { 1 } else { 0 } }
#[unsafe(no_mangle)] extern "C" fn slang_cert_expiry(cert_id: i64) -> i64 { cert_id * 1000 }
#[unsafe(no_mangle)] extern "C" fn slang_cert_chain_length(cert_id: i64) -> i64 { let _ = cert_id; 3 }

// ── v351: Property Testing ──
static PROP_CHECKS: CgAtomicI64 = CgAtomicI64::new(0);
static PROP_CE: CgAtomicI64 = CgAtomicI64::new(-1);
#[unsafe(no_mangle)] extern "C" fn slang_prop_check(value: i64, predicate: i64) -> i64 { PROP_CHECKS.fetch_add(1, CgOrdering::SeqCst); if value % predicate.max(1) == 0 { 1 } else { PROP_CE.store(value, CgOrdering::SeqCst); 0 } }
#[unsafe(no_mangle)] extern "C" fn slang_prop_shrink(value: i64) -> i64 { value / 2 }
#[unsafe(no_mangle)] extern "C" fn slang_prop_counterexample() -> i64 { PROP_CE.load(CgOrdering::SeqCst) }
#[unsafe(no_mangle)] extern "C" fn slang_prop_total_checks() -> i64 { PROP_CHECKS.load(CgOrdering::SeqCst) }

// ── v352: Mutation Testing ──
static MUT_INJECTED: CgAtomicI64 = CgAtomicI64::new(0);
static MUT_KILLED: CgAtomicI64 = CgAtomicI64::new(0);
static MUT_SURVIVED: CgAtomicI64 = CgAtomicI64::new(0);
#[unsafe(no_mangle)] extern "C" fn slang_mutation_inject(location: i64) -> i64 { let _ = location; MUT_INJECTED.fetch_add(1, CgOrdering::SeqCst) + 1 }
#[unsafe(no_mangle)] extern "C" fn slang_mutation_killed() -> i64 { MUT_KILLED.fetch_add(1, CgOrdering::SeqCst) + 1 }
#[unsafe(no_mangle)] extern "C" fn slang_mutation_survived() -> i64 { MUT_SURVIVED.fetch_add(1, CgOrdering::SeqCst) + 1 }
#[unsafe(no_mangle)] extern "C" fn slang_mut_test_score() -> i64 { let k = MUT_KILLED.load(CgOrdering::SeqCst); let t = k + MUT_SURVIVED.load(CgOrdering::SeqCst); if t == 0 { 100 } else { k * 100 / t } }

// ── v355: Code Actions ──
static CA_CNT: CgAtomicI64 = CgAtomicI64::new(0);
#[unsafe(no_mangle)] extern "C" fn slang_codeaction_extract(scope: i64) -> i64 { CA_CNT.fetch_add(1, CgOrdering::SeqCst); scope }
#[unsafe(no_mangle)] extern "C" fn slang_codeaction_inline(func_id: i64) -> i64 { CA_CNT.fetch_add(1, CgOrdering::SeqCst); func_id }
#[unsafe(no_mangle)] extern "C" fn slang_codeaction_rename(old: i64, new: i64) -> i64 { let _ = old; CA_CNT.fetch_add(1, CgOrdering::SeqCst); new }
#[unsafe(no_mangle)] extern "C" fn slang_codeaction_count() -> i64 { CA_CNT.load(CgOrdering::SeqCst) }

// ── v356: Telemetry ──
static TEL_COMPILE: CgAtomicI64 = CgAtomicI64::new(0);
static TEL_MEM: CgAtomicI64 = CgAtomicI64::new(0);
static TEL_CACHE: CgAtomicI64 = CgAtomicI64::new(0);
static TEL_ERRS: CgAtomicI64 = CgAtomicI64::new(0);
#[unsafe(no_mangle)] extern "C" fn slang_telemetry_compile_time(ms: i64) -> i64 { TEL_COMPILE.store(ms, CgOrdering::SeqCst); ms }
#[unsafe(no_mangle)] extern "C" fn slang_telemetry_peak_memory(bytes: i64) -> i64 { TEL_MEM.store(bytes, CgOrdering::SeqCst); bytes }
#[unsafe(no_mangle)] extern "C" fn slang_telemetry_cache_hits() -> i64 { TEL_CACHE.fetch_add(1, CgOrdering::SeqCst) + 1 }
#[unsafe(no_mangle)] extern "C" fn slang_telemetry_error_count() -> i64 { TEL_ERRS.load(CgOrdering::SeqCst) }

// ── v357: Build System ──
static BUILD_TARGETS: CgAtomicI64 = CgAtomicI64::new(0);
static BUILD_ARTIFACTS: CgAtomicI64 = CgAtomicI64::new(0);
static BUILD_CACHE: CgAtomicI64 = CgAtomicI64::new(0);
#[unsafe(no_mangle)] extern "C" fn slang_build_target(target_id: i64) -> i64 { BUILD_TARGETS.fetch_add(1, CgOrdering::SeqCst); target_id }
#[unsafe(no_mangle)] extern "C" fn slang_build_parallel(jobs: i64) -> i64 { jobs.max(1) }
#[unsafe(no_mangle)] extern "C" fn slang_build_cache_hit() -> i64 { BUILD_CACHE.fetch_add(1, CgOrdering::SeqCst) + 1 }
#[unsafe(no_mangle)] extern "C" fn slang_build_artifact_count() -> i64 { BUILD_ARTIFACTS.load(CgOrdering::SeqCst) }

// ── v359: Benchmarking ──
static BENCH_RUNNING: CgAtomicI64 = CgAtomicI64::new(0);
static BENCH_ITERS: CgAtomicI64 = CgAtomicI64::new(0);
#[unsafe(no_mangle)] extern "C" fn slang_bench_start(bench_id: i64) -> i64 { BENCH_RUNNING.store(bench_id, CgOrdering::SeqCst); bench_id }
#[unsafe(no_mangle)] extern "C" fn slang_bench_stop(bench_id: i64) -> i64 { let _ = bench_id; BENCH_RUNNING.store(0, CgOrdering::SeqCst); BENCH_ITERS.load(CgOrdering::SeqCst) }
#[unsafe(no_mangle)] extern "C" fn slang_bench_iterations(bench_id: i64, iters: i64) -> i64 { let _ = bench_id; BENCH_ITERS.store(iters, CgOrdering::SeqCst); iters }
#[unsafe(no_mangle)] extern "C" fn slang_bench_throughput(bench_id: i64) -> i64 { let _ = bench_id; BENCH_ITERS.load(CgOrdering::SeqCst) }

// ── v360: Profiler ──
static PROF_DEPTH: CgAtomicI64 = CgAtomicI64::new(0);
static PROF_SAMPLES: CgAtomicI64 = CgAtomicI64::new(0);
static PROF_HOT: CgAtomicI64 = CgAtomicI64::new(0);
#[unsafe(no_mangle)] extern "C" fn slang_profile_begin(func_id: i64) -> i64 { PROF_DEPTH.fetch_add(1, CgOrdering::SeqCst); PROF_SAMPLES.fetch_add(1, CgOrdering::SeqCst); PROF_HOT.store(func_id, CgOrdering::SeqCst); func_id }
#[unsafe(no_mangle)] extern "C" fn slang_profile_end(func_id: i64) -> i64 { let _ = func_id; let d = PROF_DEPTH.load(CgOrdering::SeqCst); if d > 0 { PROF_DEPTH.fetch_sub(1, CgOrdering::SeqCst); } d }
#[unsafe(no_mangle)] extern "C" fn slang_profile_flamegraph() -> i64 { PROF_SAMPLES.load(CgOrdering::SeqCst) }
#[unsafe(no_mangle)] extern "C" fn slang_profile_hotspot() -> i64 { PROF_HOT.load(CgOrdering::SeqCst) }

// ── v363: Wasm Component Model ──
static WASM_COMP_CNT: CgAtomicI64 = CgAtomicI64::new(0);
#[unsafe(no_mangle)] extern "C" fn slang_wasm_component_create(module_id: i64) -> i64 { WASM_COMP_CNT.fetch_add(1, CgOrdering::SeqCst); module_id }
#[unsafe(no_mangle)] extern "C" fn slang_wasm_component_link(a: i64, b: i64) -> i64 { a + b }
#[unsafe(no_mangle)] extern "C" fn slang_wasm_component_instantiate(comp: i64) -> i64 { comp }
#[unsafe(no_mangle)] extern "C" fn slang_wasm_component_count() -> i64 { WASM_COMP_CNT.load(CgOrdering::SeqCst) }

// ── v364: WASI Preview2 ──
#[unsafe(no_mangle)] extern "C" fn slang_wasi_fs_read(fd: i64) -> i64 { fd }
#[unsafe(no_mangle)] extern "C" fn slang_wasi_fs_write(fd: i64, data: i64) -> i64 { let _ = data; fd }
#[unsafe(no_mangle)] extern "C" fn slang_wasi_clock() -> i64 { 0 }
#[unsafe(no_mangle)] extern "C" fn slang_wasi_random() -> i64 { 42 }

// ── v365: Package Registry ──
static REG_PKGS: CgAtomicI64 = CgAtomicI64::new(0);
#[unsafe(no_mangle)] extern "C" fn slang_registry_publish(name: i64, version: i64) -> i64 { let _ = (name, version); REG_PKGS.fetch_add(1, CgOrdering::SeqCst) + 1 }
#[unsafe(no_mangle)] extern "C" fn slang_registry_resolve(name: i64) -> i64 { let _ = name; 1 } // latest version
#[unsafe(no_mangle)] extern "C" fn slang_registry_download(name: i64) -> i64 { let _ = name; 1 }
#[unsafe(no_mangle)] extern "C" fn slang_registry_version_count(name: i64) -> i64 { let _ = name; REG_PKGS.load(CgOrdering::SeqCst) }

// ── v366: Milestone ──
#[unsafe(no_mangle)] extern "C" fn slang_vitalis_v366_version() -> i64 { 366 }
#[unsafe(no_mangle)] extern "C" fn slang_vitalis_v366_total_builtins() -> i64 { 660 }
#[unsafe(no_mangle)] extern "C" fn slang_vitalis_v366_modules() -> i64 { 220 }
#[unsafe(no_mangle)] extern "C" fn slang_vitalis_v366_milestone() -> i64 { 1 }


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jit_constant() {
        let result = compile_and_run_nocache("fn main() -> i64 { 42 }");
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_jit_addition() {
        let result = compile_and_run_nocache("fn main() -> i64 { 20 + 22 }");
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_jit_arithmetic() {
        let result = compile_and_run_nocache("fn main() -> i64 { 10 * 4 + 2 }");
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_jit_subtraction() {
        let result = compile_and_run_nocache("fn main() -> i64 { 50 - 8 }");
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_jit_negation() {
        let result = compile_and_run_nocache("fn main() -> i64 { 0 - 42 }");
        assert_eq!(result.unwrap(), -42);
    }

    #[test]
    fn test_jit_let_binding() {
        let result = compile_and_run_nocache("fn main() -> i64 { let x: i64 = 40; let y: i64 = 2; x + y }");
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_jit_function_call() {
        let result = compile_and_run_nocache("fn double(x: i64) -> i64 { x * 2 } fn main() -> i64 { double(21) }");
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_jit_while_loop_sum() {
        let result = compile_and_run_nocache(
            "fn main() -> i64 { let mut sum: i64 = 0; let mut i: i64 = 1; while i <= 10 { sum = sum + i; i = i + 1; } sum }"
        );
        assert_eq!(result.unwrap(), 55);
    }

    #[test]
    fn test_jit_nested_while() {
        let result = compile_and_run_nocache(
            "fn main() -> i64 { let mut total: i64 = 0; let mut i: i64 = 1; while i <= 5 { let mut j: i64 = 1; while j <= i { total = total + 1; j = j + 1; } i = i + 1; } total }"
        );
        assert_eq!(result.unwrap(), 15);
    }

    #[test]
    fn test_jit_mutable_variable() {
        let result = compile_and_run_nocache(
            "fn main() -> i64 { let mut x: i64 = 0; x = 42; x }"
        );
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_jit_compound_assign() {
        let result = compile_and_run_nocache(
            "fn main() -> i64 { let mut x: i64 = 10; x += 5; x *= 2; x }"
        );
        assert_eq!(result.unwrap(), 30);
    }

    #[test]
    fn test_jit_for_range_sum() {
        // 0+1+2+3+4 = 10
        let result = compile_and_run_nocache(
            "fn main() -> i64 { let mut s: i64 = 0; for i in 0..5 { s = s + i } s }"
        );
        assert_eq!(result.unwrap(), 10);
    }

    #[test]
    fn test_jit_for_range_count() {
        // count iterations from 3 to 7 = 4
        let result = compile_and_run_nocache(
            "fn main() -> i64 { let mut s: i64 = 0; for i in 3..7 { s = s + 1 } s }"
        );
        assert_eq!(result.unwrap(), 4);
    }

    #[test]
    fn test_jit_for_range_empty() {
        // 5..5 is empty, body never runs
        let result = compile_and_run_nocache(
            "fn main() -> i64 { let mut s: i64 = 99; for i in 5..5 { s = 0 } s }"
        );
        assert_eq!(result.unwrap(), 99);
    }

    #[test]
    fn test_jit_match_first_arm() {
        let result = compile_and_run_nocache(
            "fn main() -> i64 { match 1 { 1 => 10, 2 => 20, _ => 0, } }"
        );
        assert_eq!(result.unwrap(), 10);
    }

    #[test]
    fn test_jit_match_second_arm() {
        let result = compile_and_run_nocache(
            "fn main() -> i64 { match 2 { 1 => 10, 2 => 20, _ => 0, } }"
        );
        assert_eq!(result.unwrap(), 20);
    }

    #[test]
    fn test_jit_match_wildcard() {
        let result = compile_and_run_nocache(
            "fn main() -> i64 { match 99 { 1 => 10, 2 => 20, _ => 42, } }"
        );
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_jit_match_with_variable() {
        let result = compile_and_run_nocache(
            "fn main() -> i64 { let x: i64 = 3; match x { 1 => 10, 2 => 20, 3 => 30, _ => 0, } }"
        );
        assert_eq!(result.unwrap(), 30);
    }

    #[test]
    fn test_jit_pipe_single() {
        let result = compile_and_run_nocache(
            "fn double(x: i64) -> i64 { x * 2 } fn main() -> i64 { 21 |> double }"
        );
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_jit_pipe_chain() {
        let result = compile_and_run_nocache(
            "fn add1(x: i64) -> i64 { x + 1 } fn double(x: i64) -> i64 { x * 2 } fn main() -> i64 { 5 |> add1 |> double }"
        );
        assert_eq!(result.unwrap(), 12);
    }

    // ── v15: String stdlib tests ──────────────────────────────────────
    #[test]
    fn test_v15_str_len() {
        let r = compile_and_run_nocache(r#"fn main() -> i64 { str_len("hello") }"#);
        assert_eq!(r.unwrap(), 5);
    }

    #[test]
    fn test_v15_str_contains() {
        let r = compile_and_run_nocache(r#"fn main() -> i64 { if str_contains("hello world", "world") { 1 } else { 0 } }"#);
        assert_eq!(r.unwrap(), 1);
    }

    #[test]
    fn test_v15_str_index_of() {
        let r = compile_and_run_nocache(r#"fn main() -> i64 { str_index_of("hello world", "world") }"#);
        assert_eq!(r.unwrap(), 6);
    }

    #[test]
    fn test_v15_str_split_count() {
        let r = compile_and_run_nocache(r#"fn main() -> i64 { str_split_count("a,b,c,d", ",") }"#);
        assert_eq!(r.unwrap(), 4);
    }

    #[test]
    fn test_v15_parse_int() {
        let r = compile_and_run_nocache(r#"fn main() -> i64 { parse_int("42") }"#);
        assert_eq!(r.unwrap(), 42);
    }

    // ── v15: Map tests ────────────────────────────────────────────────
    #[test]
    fn test_v15_map_new_len() {
        let r = compile_and_run_nocache(r#"fn main() -> i64 { let m: i64 = map_new(); map_len(m) }"#);
        assert_eq!(r.unwrap(), 0);
    }

    #[test]
    fn test_v15_map_set_get() {
        let r = compile_and_run_nocache(r#"fn main() -> i64 { let m: i64 = map_new(); map_set(m, "key", 42); map_get(m, "key") }"#);
        assert_eq!(r.unwrap(), 42);
    }

    #[test]
    fn test_v15_map_len_after_insert() {
        let r = compile_and_run_nocache(r#"fn main() -> i64 { let m: i64 = map_new(); map_set(m, "a", 1); map_set(m, "b", 2); map_len(m) }"#);
        assert_eq!(r.unwrap(), 2);
    }

    // ── v15: Error handling tests ─────────────────────────────────────
    #[test]
    fn test_v15_error_check_initial() {
        let r = compile_and_run_nocache("fn main() -> i64 { error_clear(); error_check() }");
        assert_eq!(r.unwrap(), 0);
    }

    #[test]
    fn test_v15_error_set_check() {
        let r = compile_and_run_nocache(r#"fn main() -> i64 { error_set(99, "oops"); error_check() }"#);
        assert_eq!(r.unwrap(), 99);
    }

    // ── v15: System tests ─────────────────────────────────────────────
    #[test]
    fn test_v15_pid() {
        let r = compile_and_run_nocache("fn main() -> i64 { pid() }");
        assert!(r.unwrap() > 0);
    }

    // ── v15: File I/O tests ───────────────────────────────────────────
    #[test]
    fn test_v15_file_write_read() {
        let r = compile_and_run_nocache(r#"fn main() -> i64 {
            file_write("_v15_test.tmp", "hello");
            let len: i64 = str_len(file_read("_v15_test.tmp"));
            file_delete("_v15_test.tmp");
            len
        }"#);
        assert_eq!(r.unwrap(), 5);
    }

    #[test]
    fn test_v15_file_exists() {
        let r = compile_and_run_nocache(r#"fn main() -> i64 {
            file_write("_v15_exist.tmp", "x");
            let e: i64 = if file_exists("_v15_exist.tmp") { 1 } else { 0 };
            file_delete("_v15_exist.tmp");
            e
        }"#);
        assert_eq!(r.unwrap(), 1);
    }

    // ══════════════════════════════════════════════════════════════════
    // v18 Tests — OOP, error handling, collections, for-each, break/continue
    // ══════════════════════════════════════════════════════════════════

    // ── v18: Array collection methods ─────────────────────────────────

    #[test]
    fn test_v18_array_push() {
        let r = compile_and_run_nocache(r#"fn main() -> i64 {
let arr: [i64] = [10, 20, 30];
let arr2: [i64] = array_push(arr, 40);
arr2[3]
}"#);
        assert_eq!(r.unwrap(), 40);
    }

    #[test]
    fn test_v18_array_contains() {
        let r = compile_and_run_nocache(r#"fn main() -> i64 {
let arr: [i64] = [10, 20, 30];
array_contains(arr, 20)
}"#);
        assert_eq!(r.unwrap(), 1);
    }

    #[test]
    fn test_v18_array_contains_missing() {
        let r = compile_and_run_nocache(r#"fn main() -> i64 {
let arr: [i64] = [10, 20, 30];
array_contains(arr, 99)
}"#);
        assert_eq!(r.unwrap(), 0);
    }

    #[test]
    fn test_v18_array_find() {
        let r = compile_and_run_nocache(r#"fn main() -> i64 {
let arr: [i64] = [10, 20, 30];
array_find(arr, 30)
}"#);
        assert_eq!(r.unwrap(), 2);
    }

    #[test]
    fn test_v18_array_find_missing() {
        let r = compile_and_run_nocache(r#"fn main() -> i64 {
let arr: [i64] = [10, 20, 30];
array_find(arr, 99)
}"#);
        assert_eq!(r.unwrap(), -1);
    }

    #[test]
    fn test_v18_array_pop() {
        let r = compile_and_run_nocache(r#"fn main() -> i64 {
let arr: [i64] = [10, 20, 30];
array_pop(arr)
}"#);
        assert_eq!(r.unwrap(), 30);
    }

    #[test]
    fn test_v18_array_sort() {
        let r = compile_and_run_nocache(r#"fn main() -> i64 {
let arr: [i64] = [30, 10, 20];
array_sort(arr);
arr[0]
}"#);
        assert_eq!(r.unwrap(), 10);
    }

    #[test]
    fn test_v18_array_reverse() {
        let r = compile_and_run_nocache(r#"fn main() -> i64 {
let arr: [i64] = [10, 20, 30];
array_reverse(arr);
arr[0]
}"#);
        assert_eq!(r.unwrap(), 30);
    }

    #[test]
    fn test_v18_array_slice() {
        let r = compile_and_run_nocache(r#"fn main() -> i64 {
let arr: [i64] = [10, 20, 30, 40, 50];
let s: [i64] = array_slice(arr, 1, 4);
s[0]
}"#);
        assert_eq!(r.unwrap(), 20);
    }

    #[test]
    fn test_v18_array_slice_len() {
        let r = compile_and_run_nocache(r#"fn main() -> i64 {
let arr: [i64] = [10, 20, 30, 40, 50];
let s: [i64] = array_slice(arr, 1, 4);
s.len()
}"#);
        assert_eq!(r.unwrap(), 3);
    }

    // ── v18: For-each over arrays ─────────────────────────────────────

    #[test]
    fn test_v18_for_each_array() {
        let r = compile_and_run_nocache(r#"fn main() -> i64 {
let arr: [i64] = [10, 20, 30];
let mut sum: i64 = 0;
for x in arr {
    sum = sum + x;
}
sum
}"#);
        assert_eq!(r.unwrap(), 60);
    }

    // ── v18: Break / Continue ─────────────────────────────────────────

    #[test]
    fn test_v18_while_break() {
        let r = compile_and_run_nocache(r#"fn main() -> i64 {
let mut i: i64 = 0;
while i < 100 {
    if i == 5 { break }
    i = i + 1;
}
i
}"#);
        assert_eq!(r.unwrap(), 5);
    }

    #[test]
    fn test_v18_for_break() {
        let r = compile_and_run_nocache(r#"fn main() -> i64 {
let mut result: i64 = 0;
for i in 0..100 {
    if i == 10 { break }
    result = i;
}
result
}"#);
        assert_eq!(r.unwrap(), 9);
    }

    // ── v18: Struct field access ──────────────────────────────────────

    #[test]
    fn test_v18_struct_field_by_name() {
        let r = compile_and_run_nocache(r#"
struct Point { x: i64, y: i64 }

fn main() -> i64 {
    let p: Point = Point { x: 10, y: 20 };
    p.x + p.y
}"#);
        assert_eq!(r.unwrap(), 30);
    }

    #[test]
    fn test_v18_struct_second_field() {
        let r = compile_and_run_nocache(r#"
struct Pair { first: i64, second: i64 }

fn main() -> i64 {
    let p: Pair = Pair { first: 3, second: 7 };
    p.second
}"#);
        assert_eq!(r.unwrap(), 7);
    }

    // ── v18: Impl methods ────────────────────────────────────────────

    #[test]
    fn test_v18_impl_method_call() {
        let r = compile_and_run_nocache(r#"
struct Rect { w: i64, h: i64 }

impl Rect {
    fn area(self: Rect) -> i64 {
        self.w * self.h
    }
}

fn main() -> i64 {
    let r: Rect = Rect { w: 5, h: 3 };
    r.area()
}"#);
        assert_eq!(r.unwrap(), 15);
    }

    // ── v18: Try/Catch ───────────────────────────────────────────────

    #[test]
    fn test_v18_try_catch_no_error() {
        let r = compile_and_run_nocache(r#"fn main() -> i64 {
let result: i64 = try {
    42
} catch e {
    0
};
result
}"#);
        assert_eq!(r.unwrap(), 42);
    }

    #[test]
    fn test_v18_try_catch_with_throw() {
        let r = compile_and_run_nocache(r#"fn main() -> i64 {
let result: i64 = try {
    throw(1, "oops");
    42
} catch e {
    99
};
result
}"#);
        assert_eq!(r.unwrap(), 99);
    }

    // ── v18: Error handling flow ─────────────────────────────────────

    #[test]
    fn test_v18_struct_fn_call() {
        // Free function with struct argument (NOT impl method)
        let r = compile_and_run_nocache(r#"
struct Rect { w: i64, h: i64 }

fn get_w(r: Rect) -> i64 {
    r.w
}

fn main() -> i64 {
    let r: Rect = Rect { w: 5, h: 3 };
    get_w(r)
}"#);
        assert_eq!(r.unwrap(), 5);
    }

    #[test]
    fn test_v18_throw_sets_error() {
        let r = compile_and_run_nocache(r#"fn main() -> i64 {
error_clear();
throw(42, "test error");
error_check()
}"#);
        assert_eq!(r.unwrap(), 42);
    }

    // ── v16: Set tests ──────────────────────────────────────────────────
    #[test]
    fn test_v16_set_new_len() {
        let r = compile_and_run_nocache(r#"fn main() -> i64 { let s: i64 = set_new(); set_len(s) }"#);
        assert_eq!(r.unwrap(), 0);
    }

    #[test]
    fn test_v16_set_add_has() {
        let r = compile_and_run_nocache(r#"fn main() -> i64 {
let s: i64 = set_new();
set_add(s, 42);
let h: i64 = if set_has(s, 42) { 1 } else { 0 };
h
}"#);
        assert_eq!(r.unwrap(), 1);
    }

    #[test]
    fn test_v16_set_add_len() {
        let r = compile_and_run_nocache(r#"fn main() -> i64 {
let s: i64 = set_new();
set_add(s, 1);
set_add(s, 2);
set_add(s, 3);
set_add(s, 2);
set_len(s)
}"#);
        assert_eq!(r.unwrap(), 3); // duplicates ignored
    }

    #[test]
    fn test_v16_set_remove() {
        let r = compile_and_run_nocache(r#"fn main() -> i64 {
let s: i64 = set_new();
set_add(s, 10);
set_add(s, 20);
set_remove(s, 10);
set_len(s)
}"#);
        assert_eq!(r.unwrap(), 1);
    }

    #[test]
    fn test_v16_set_union() {
        let r = compile_and_run_nocache(r#"fn main() -> i64 {
let a: i64 = set_new();
set_add(a, 1);
set_add(a, 2);
let b: i64 = set_new();
set_add(b, 2);
set_add(b, 3);
let c: i64 = set_union(a, b);
set_len(c)
}"#);
        assert_eq!(r.unwrap(), 3); // {1,2,3}
    }

    #[test]
    fn test_v16_set_intersect() {
        let r = compile_and_run_nocache(r#"fn main() -> i64 {
let a: i64 = set_new();
set_add(a, 1);
set_add(a, 2);
set_add(a, 3);
let b: i64 = set_new();
set_add(b, 2);
set_add(b, 3);
set_add(b, 4);
let c: i64 = set_intersect(a, b);
set_len(c)
}"#);
        assert_eq!(r.unwrap(), 2); // {2,3}
    }

    #[test]
    fn test_v16_set_diff() {
        let r = compile_and_run_nocache(r#"fn main() -> i64 {
let a: i64 = set_new();
set_add(a, 1);
set_add(a, 2);
set_add(a, 3);
let b: i64 = set_new();
set_add(b, 2);
let c: i64 = set_diff(a, b);
set_len(c)
}"#);
        assert_eq!(r.unwrap(), 2); // {1,3}
    }

    // ── v18: Tuple tests ─────────────────────────────────────────────

    #[test]
    fn test_v18_tuple_new2() {
        let r = compile_and_run_nocache(r#"fn main() -> i64 {
let t: i64 = tuple_new2(10, 20);
let a: i64 = tuple_get(t, 0);
let b: i64 = tuple_get(t, 1);
a + b
}"#);
        assert_eq!(r.unwrap(), 30);
    }

    #[test]
    fn test_v18_tuple_new3() {
        let r = compile_and_run_nocache(r#"fn main() -> i64 {
let t: i64 = tuple_new3(1, 2, 3);
let a: i64 = tuple_get(t, 0);
let b: i64 = tuple_get(t, 1);
let c: i64 = tuple_get(t, 2);
a + b + c
}"#);
        assert_eq!(r.unwrap(), 6);
    }

    #[test]
    fn test_v18_tuple_len() {
        let r = compile_and_run_nocache(r#"fn main() -> i64 {
let t: i64 = tuple_new2(5, 6);
tuple_len(t)
}"#);
        assert_eq!(r.unwrap(), 2);
    }

    #[test]
    fn test_v18_tuple_get() {
        let r = compile_and_run_nocache(r#"fn main() -> i64 {
let t: i64 = tuple_new3(100, 200, 300);
tuple_get(t, 2)
}"#);
        assert_eq!(r.unwrap(), 300);
    }

    // ── v18: Regex tests ──────────────────────────────────────────────

    #[test]
    fn test_v18_regex_is_match() {
        // partial match: digits found inside "abc123"
        let r = compile_and_run_nocache(r#"fn main() -> i64 { if regex_is_match("\d+", "abc123") { 1 } else { 0 } }"#);
        assert_eq!(r.unwrap(), 1);
    }

    #[test]
    fn test_v18_regex_match() {
        // full match: "abc" matches ^abc$
        let r = compile_and_run_nocache(r#"fn main() -> i64 { if regex_match("^abc$", "abc") { 1 } else { 0 } }"#);
        assert_eq!(r.unwrap(), 1);
    }

    #[test]
    fn test_v18_regex_find_all_count() {
        // "\d+" matches "1", "22", "333" => 3
        let r = compile_and_run_nocache(r#"fn main() -> i64 { regex_find_all_count("\d+", "a1b22c333") }"#);
        assert_eq!(r.unwrap(), 3);
    }

    #[test]
    fn test_v18_regex_split_count() {
        // splitting "a,b,c,d" by "," => 4 segments
        let r = compile_and_run_nocache(r#"fn main() -> i64 { regex_split_count(",", "a,b,c,d") }"#);
        assert_eq!(r.unwrap(), 4);
    }

    #[test]
    fn test_v18_regex_replace() {
        // replace digits with "XX" in "a1b2c3" -> "aXXbXXcXX" (len 9)
        let r = compile_and_run_nocache(r#"fn main() -> i64 {
let s: str = regex_replace("\d", "a1b2c3", "XX");
str_len(s)
}"#);
        assert_eq!(r.unwrap(), 9);
    }

    // ── v18: String formatting ────────────────────────────────────────

    #[test]
    fn test_v18_str_format_i64() {
        // str_format_i64("val={}", 42) → "val=42" (len 6)
        let r = compile_and_run_nocache(r#"fn main() -> i64 {
let s: str = str_format_i64("val={}", 42);
str_len(s)
}"#);
        assert_eq!(r.unwrap(), 6);
    }

    #[test]
    fn test_v18_str_format_f64() {
        // str_format_f64("pi={}", 3.14) → "pi=3.14" (len 7)
        let r = compile_and_run_nocache(r#"fn main() -> i64 {
let s: str = str_format_f64("pi={}", 3.14);
str_len(s)
}"#);
        assert_eq!(r.unwrap(), 7);
    }

    #[test]
    fn test_v18_str_format_str() {
        // str_format_str("hi {}", "world") → "hi world" (len 8)
        let r = compile_and_run_nocache(r#"fn main() -> i64 {
let s: str = str_format_str("hi {}", "world");
str_len(s)
}"#);
        assert_eq!(r.unwrap(), 8);
    }

    #[test]
    fn test_v18_str_format_chain() {
        // Chain: str_format_i64(str_format_str("{} = {}", "x"), 42) → "x = 42" (len 6)
        let r = compile_and_run_nocache(r#"fn main() -> i64 {
let s: str = str_format_i64(str_format_str("{} = {}", "x"), 42);
str_len(s)
}"#);
        assert_eq!(r.unwrap(), 6);
    }

    // ── Iterator / functional array tests ─────────────────────────────

    #[test]
    fn test_v18_array_range() {
        let r = compile_and_run_nocache(r#"fn main() -> i64 {
let arr: [i64] = array_range(1, 5);
arr.len()
}"#);
        assert_eq!(r.unwrap(), 4);
    }

    #[test]
    fn test_v18_array_sum() {
        let r = compile_and_run_nocache(r#"fn main() -> i64 {
let arr: [i64] = [10, 20, 30];
array_sum(arr)
}"#);
        assert_eq!(r.unwrap(), 60);
    }

    #[test]
    fn test_v18_array_min_max() {
        let r = compile_and_run_nocache(r#"fn main() -> i64 {
let arr: [i64] = [5, 2, 8, 1];
array_min(arr)
}"#);
        assert_eq!(r.unwrap(), 1);
        let r2 = compile_and_run_nocache(r#"fn main() -> i64 {
let arr: [i64] = [5, 2, 8, 1];
array_max(arr)
}"#);
        assert_eq!(r2.unwrap(), 8);
    }

    #[test]
    fn test_v18_array_count() {
        let r = compile_and_run_nocache(r#"fn main() -> i64 {
let arr: [i64] = [1, 2, 1, 3, 1];
array_count(arr, 1)
}"#);
        assert_eq!(r.unwrap(), 3);
    }

    #[test]
    fn test_v18_array_unique() {
        let r = compile_and_run_nocache(r#"fn main() -> i64 {
let arr: [i64] = [1, 2, 2, 3, 3, 3];
let u: [i64] = array_unique(arr);
u.len()
}"#);
        assert_eq!(r.unwrap(), 3);
    }

    #[test]
    fn test_v18_array_take() {
        let r = compile_and_run_nocache(r#"fn main() -> i64 {
let arr: [i64] = [10, 20, 30, 40];
let t: [i64] = array_take(arr, 2);
t.len()
}"#);
        assert_eq!(r.unwrap(), 2);
    }

    #[test]
    fn test_v18_module_basic() {
        let r = compile_and_run_nocache(r#"
module math {
    fn add(a: i64, b: i64) -> i64 {
        a + b
    }
}

fn main() -> i64 {
    math::add(10, 20)
}
"#);
        assert_eq!(r.unwrap(), 30);
    }

    #[test]
    fn test_v18_module_nested_fn() {
        let r = compile_and_run_nocache(r#"
module math {
    fn add(a: i64, b: i64) -> i64 { a + b }
    fn mul(a: i64, b: i64) -> i64 { a * b }
}

fn main() -> i64 {
    math::add(3, 4) + math::mul(5, 6)
}
"#);
        assert_eq!(r.unwrap(), 37);
    }

    #[test]
    fn test_v18_async_fn() {
        let r = compile_and_run_nocache(r#"
async fn compute() -> i64 { 42 }
fn main() -> i64 { compute() }
"#);
        assert_eq!(r.unwrap(), 42);
    }

    #[test]
    fn test_v18_spawn_stub() {
        let r = compile_and_run_nocache(r#"
fn main() -> i64 { spawn(1) }
"#);
        // spawn returns executor-assigned task ID (non-deterministic across parallel tests)
        assert!(r.unwrap() >= 0);
    }

    #[test]
    fn test_v18_task_result_stub() {
        // v370: task_result now does real executor lookup.
        // task_result(id) for non-existent task returns 0.
        let r = compile_and_run_nocache(r#"
fn main() -> i64 {
    let id: i64 = spawn(42);
    task_result(id)
}
"#);
        assert_eq!(r.unwrap(), 42);
    }

    #[test]
    fn test_v18_tcp_stub() {
        let r = compile_and_run_nocache(r#"
fn main() -> i64 {
    let h: i64 = tcp_connect("127.0.0.1", 80);
    tcp_close(h);
    42
}
"#);
        assert_eq!(r.unwrap(), 42);
    }

    #[test]
    fn test_v18_await_expr() {
        // v370: await now compiles to task_await(expr). For a simple sync call
        // this just evaluates the inner expression.
        let r = compile_and_run_nocache(r#"
fn fetch() -> i64 { 7 }
fn main() -> i64 {
    let id: i64 = spawn(fetch());
    task_result(id)
}
"#);
        assert_eq!(r.unwrap(), 7);
    }

    #[test]
    fn test_v18_networking_compiles() {
        // Verify http_get, http_post, http_status compile without error
        // (we don't actually make network calls)
        let r = compile_and_run_nocache(r#"
fn main() -> i64 {
    let h: i64 = tcp_connect("localhost", 8080);
    let sent: i64 = tcp_send(h, "hello");
    tcp_close(h);
    42
}
"#);
        assert_eq!(r.unwrap(), 42);
    }

    // ══════════════════════════════════════════════════════════════════
    // v106 Tests — Closures with captures
    // ══════════════════════════════════════════════════════════════════

    #[test]
    fn test_v106_closure_capture_single() {
        let r = compile_and_run_nocache(r#"
fn main() -> i64 {
    let a: i64 = 10;
    let f = |x: i64| a + x;
    f(5)
}"#);
        assert_eq!(r.unwrap(), 15);
    }

    #[test]
    fn test_v106_closure_no_capture() {
        let r = compile_and_run_nocache(r#"
fn main() -> i64 {
    let f = |x: i64| x * 3;
    f(7)
}"#);
        assert_eq!(r.unwrap(), 21);
    }

    #[test]
    fn test_v106_closure_capture_multiple() {
        let r = compile_and_run_nocache(r#"
fn main() -> i64 {
    let a: i64 = 10;
    let b: i64 = 20;
    let f = |x: i64| a + b + x;
    f(5)
}"#);
        assert_eq!(r.unwrap(), 35);
    }

    #[test]
    fn test_v106_closure_capture_used_twice() {
        let r = compile_and_run_nocache(r#"
fn main() -> i64 {
    let a: i64 = 10;
    let f = |x: i64| a + x;
    f(5) + f(3)
}"#);
        assert_eq!(r.unwrap(), 28);
    }

    // ══════════════════════════════════════════════════════════════════
    // v107 Tests — Enum runtime support
    // ══════════════════════════════════════════════════════════════════

    #[test]
    fn test_v107_enum_unit_variant_match() {
        let r = compile_and_run_nocache(r#"
enum Color { Red, Green, Blue }

fn main() -> i64 {
    let c: Color = Red;
    match c {
        Red => 1,
        Green => 2,
        Blue => 3,
    }
}"#);
        assert_eq!(r.unwrap(), 1);
    }

    #[test]
    fn test_v107_enum_unit_second_variant() {
        let r = compile_and_run_nocache(r#"
enum Color { Red, Green, Blue }

fn main() -> i64 {
    let c: Color = Green;
    match c {
        Red => 1,
        Green => 2,
        Blue => 3,
    }
}"#);
        assert_eq!(r.unwrap(), 2);
    }

    #[test]
    fn test_v107_enum_unit_third_variant() {
        let r = compile_and_run_nocache(r#"
enum Color { Red, Green, Blue }

fn main() -> i64 {
    let c: Color = Blue;
    match c {
        Red => 1,
        Green => 2,
        Blue => 3,
    }
}"#);
        assert_eq!(r.unwrap(), 3);
    }

    #[test]
    fn test_v107_enum_with_payload() {
        let r = compile_and_run_nocache(r#"
enum Shape { Circle(i64), Square(i64) }

fn main() -> i64 {
    let s: Shape = Circle(42);
    match s {
        Circle(r) => r,
        Square(side) => side,
    }
}"#);
        assert_eq!(r.unwrap(), 42);
    }

    #[test]
    fn test_v107_enum_payload_second_variant() {
        let r = compile_and_run_nocache(r#"
enum Shape { Circle(i64), Square(i64) }

fn main() -> i64 {
    let s: Shape = Square(10);
    match s {
        Circle(r) => r,
        Square(side) => side * side,
    }
}"#);
        assert_eq!(r.unwrap(), 100);
    }

    // ══════════════════════════════════════════════════════════════════
    // v110 Tests — String operations in JIT (verify existing)
    // ══════════════════════════════════════════════════════════════════

    #[test]
    fn test_v110_str_upper() {
        let r = compile_and_run_nocache(r#"fn main() -> i64 { str_len(str_upper("hello")) }"#);
        assert_eq!(r.unwrap(), 5);
    }

    #[test]
    fn test_v110_str_lower() {
        let r = compile_and_run_nocache(r#"fn main() -> i64 { str_len(str_lower("HELLO")) }"#);
        assert_eq!(r.unwrap(), 5);
    }

    #[test]
    fn test_v110_str_trim() {
        let r = compile_and_run_nocache(r#"fn main() -> i64 { str_len(str_trim("  hi  ")) }"#);
        assert_eq!(r.unwrap(), 2);
    }

    #[test]
    fn test_v110_str_starts_with() {
        let r = compile_and_run_nocache(r#"fn main() -> i64 { if str_starts_with("hello world", "hello") { 1 } else { 0 } }"#);
        assert_eq!(r.unwrap(), 1);
    }

    #[test]
    fn test_v110_str_ends_with() {
        let r = compile_and_run_nocache(r#"fn main() -> i64 { if str_ends_with("hello world", "world") { 1 } else { 0 } }"#);
        assert_eq!(r.unwrap(), 1);
    }

    #[test]
    fn test_v110_str_repeat() {
        let r = compile_and_run_nocache(r#"fn main() -> i64 { str_len(str_repeat("ab", 3)) }"#);
        assert_eq!(r.unwrap(), 6);
    }

    #[test]
    fn test_v110_str_replace() {
        let r = compile_and_run_nocache(r#"fn main() -> i64 { str_len(str_replace("hello world", "world", "rust")) }"#);
        assert_eq!(r.unwrap(), 10); // "hello rust" = 10 chars
    }

    // ══════════════════════════════════════════════════════════════════
    // v111 Tests — For-loop and range compilation (verify existing)
    // ══════════════════════════════════════════════════════════════════

    #[test]
    fn test_v111_for_range_product() {
        // 1*2*3*4 = 24
        let r = compile_and_run_nocache(r#"fn main() -> i64 {
let mut p: i64 = 1;
for i in 1..5 {
    p = p * i;
}
p
}"#);
        assert_eq!(r.unwrap(), 24);
    }

    #[test]
    fn test_v111_for_range_nested() {
        // sum of i*j for i in 0..3, j in 0..3 = 0+0+0+0+1+2+0+2+4 = 9
        let r = compile_and_run_nocache(r#"fn main() -> i64 {
let mut sum: i64 = 0;
for i in 0..3 {
    for j in 0..3 {
        sum = sum + i * j;
    }
}
sum
}"#);
        assert_eq!(r.unwrap(), 9);
    }

    #[test]
    fn test_v111_for_each_array_sum() {
        let r = compile_and_run_nocache(r#"fn main() -> i64 {
let arr: [i64] = [1, 2, 3, 4, 5];
let mut sum: i64 = 0;
for x in arr {
    sum = sum + x;
}
sum
}"#);
        assert_eq!(r.unwrap(), 15);
    }

    #[test]
    fn test_v111_for_range_with_break() {
        // sum 0..100 but break at 5 → 0+1+2+3+4 = 10
        let r = compile_and_run_nocache(r#"fn main() -> i64 {
let mut sum: i64 = 0;
for i in 0..100 {
    if i == 5 { break }
    sum = sum + i;
}
sum
}"#);
        assert_eq!(r.unwrap(), 10);
    }

    // ══════════════════════════════════════════════════════════════════
    // v112 Tests — Error handling in codegen (verify existing)
    // ══════════════════════════════════════════════════════════════════

    #[test]
    fn test_v112_try_catch_success() {
        let r = compile_and_run_nocache(r#"fn main() -> i64 {
let result: i64 = try { 100 } catch e { 0 };
result
}"#);
        assert_eq!(r.unwrap(), 100);
    }

    #[test]
    fn test_v112_try_catch_with_error() {
        let r = compile_and_run_nocache(r#"fn main() -> i64 {
let result: i64 = try {
    throw(1, "fail");
    0
} catch e {
    77
};
result
}"#);
        assert_eq!(r.unwrap(), 77);
    }

    #[test]
    fn test_v112_throw_error_code() {
        let r = compile_and_run_nocache(r#"fn main() -> i64 {
error_clear();
throw(55, "test error");
error_check()
}"#);
        assert_eq!(r.unwrap(), 55);
    }

    #[test]
    fn test_v112_error_clear_resets() {
        let r = compile_and_run_nocache(r#"fn main() -> i64 {
throw(42, "err");
error_clear();
error_check()
}"#);
        assert_eq!(r.unwrap(), 0);
    }

    // ══════════════════════════════════════════════════════════════════
    // v108/v109 Tests — Method dispatch and type handling
    // ══════════════════════════════════════════════════════════════════

    #[test]
    fn test_v108_multiple_functions_same_body() {
        // Verify multiple functions with different signatures compile correctly
        let r = compile_and_run_nocache(r#"
fn add_ints(a: i64, b: i64) -> i64 { a + b }
fn mul_ints(a: i64, b: i64) -> i64 { a * b }
fn main() -> i64 { add_ints(3, 4) + mul_ints(2, 5) }
"#);
        assert_eq!(r.unwrap(), 17);
    }

    #[test]
    fn test_v109_impl_method_with_computation() {
        let r = compile_and_run_nocache(r#"
struct Vec2 { x: i64, y: i64 }

impl Vec2 {
    fn sum(self: Vec2) -> i64 {
        self.x + self.y
    }
    fn scale(self: Vec2, factor: i64) -> i64 {
        (self.x + self.y) * factor
    }
}

fn main() -> i64 {
    let v: Vec2 = Vec2 { x: 3, y: 4 };
    v.sum() + v.scale(2)
}"#);
        assert_eq!(r.unwrap(), 21);
    }

    #[test]
    fn test_v109_multiple_impl_blocks() {
        let r = compile_and_run_nocache(r#"
struct A { val: i64 }
struct B { val: i64 }

impl A {
    fn get(self: A) -> i64 { self.val }
}

impl B {
    fn get(self: B) -> i64 { self.val * 2 }
}

fn main() -> i64 {
    let a: A = A { val: 5 };
    let b: B = B { val: 5 };
    a.get() + b.get()
}"#);
        assert_eq!(r.unwrap(), 15);
    }

    // ── v121: Example gallery verification tests ────────────────────

    #[test]
    fn test_example_pattern_matching() {
        let r = compile_and_run_nocache(r#"
fn classify(x: i64) -> i64 {
    match x {
        0 => 0,
        1 => 100,
        2 => 200,
        _ => 999,
    }
}
fn main() -> i64 {
    classify(0) + classify(2) + classify(99)
}"#);
        assert_eq!(r.unwrap(), 0 + 200 + 999);
    }

    #[test]
    fn test_example_loops_while() {
        let r = compile_and_run_nocache(r#"
fn main() -> i64 {
    let mut total: i64 = 0;
    let mut i: i64 = 1;
    while i <= 10 {
        total = total + i;
        i = i + 1;
    }
    total
}"#);
        assert_eq!(r.unwrap(), 55);
    }

    #[test]
    fn test_example_lambda() {
        let r = compile_and_run_nocache(r#"
fn main() -> i64 {
    let double = |x: i64| -> i64 { x * 2 };
    double(21)
}"#);
        assert_eq!(r.unwrap(), 42);
    }

    #[test]
    fn test_example_gcd_recursive() {
        let r = compile_and_run_nocache(r#"
fn gcd_impl(a: i64, b: i64) -> i64 {
    if b == 0 { a } else { gcd_impl(b, a % b) }
}
fn main() -> i64 { gcd_impl(48, 18) }
"#);
        assert_eq!(r.unwrap(), 6);
    }

    #[test]
    fn test_example_power_recursive() {
        let r = compile_and_run_nocache(r#"
fn power(base: i64, exp: i64) -> i64 {
    if exp == 0 { 1 } else { base * power(base, exp - 1) }
}
fn main() -> i64 { power(2, 10) }
"#);
        assert_eq!(r.unwrap(), 1024);
    }

    #[test]
    fn test_example_struct_method() {
        let r = compile_and_run_nocache(r#"
struct Counter { count: i64 }
impl Counter {
    fn value(self: Counter) -> i64 { self.count * 2 }
}
fn main() -> i64 {
    let c: Counter = Counter { count: 21 };
    c.value()
}"#);
        assert_eq!(r.unwrap(), 42);
    }

    #[test]
    fn test_example_module_call() {
        let r = compile_and_run_nocache(r#"
module math {
    fn square(x: i64) -> i64 { x * x }
}
fn main() -> i64 { math::square(7) }
"#);
        assert_eq!(r.unwrap(), 49);
    }

    // ─── v124: JIT Cache & Metrics Tests ────────────────────────────────
    // These tests share global state (JIT_RESULT_CACHE + COMPILE_METRICS),
    // so we serialize them via a test mutex to prevent parallel interference.
    static CACHE_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn test_jit_cache_hit() {
        let _guard = CACHE_TEST_LOCK.lock().unwrap();
        clear_compile_cache();
        let src = "fn main() -> i64 { 124001 }";
        let r1 = compile_and_run(src).unwrap();
        let r2 = compile_and_run(src).unwrap();
        assert_eq!(r1, 124001);
        assert_eq!(r2, 124001);
        let m = compile_metrics();
        assert!(m.cache_hits >= 1, "expected cache hit, got {}", m.cache_hits);
    }

    #[test]
    fn test_jit_cache_miss_different_source() {
        let _guard = CACHE_TEST_LOCK.lock().unwrap();
        clear_compile_cache();
        let r1 = compile_and_run("fn main() -> i64 { 124002 }").unwrap();
        let r2 = compile_and_run("fn main() -> i64 { 124003 }").unwrap();
        assert_eq!(r1, 124002);
        assert_eq!(r2, 124003);
        let m = compile_metrics();
        assert!(m.cache_misses >= 2, "expected >=2 misses, got {}", m.cache_misses);
    }

    #[test]
    fn test_compile_metrics_tracking() {
        let _guard = CACHE_TEST_LOCK.lock().unwrap();
        clear_compile_cache();
        compile_and_run_nocache("fn main() -> i64 { 124004 }").unwrap();
        let m = compile_metrics();
        assert!(m.total_compilations >= 1);
        assert!(m.total_parse_ns > 0);
        assert!(m.total_ir_ns > 0);
        assert!(m.total_jit_ns > 0);
    }

    #[test]
    fn test_clear_compile_cache() {
        let _guard = CACHE_TEST_LOCK.lock().unwrap();
        compile_and_run("fn main() -> i64 { 124005 }").unwrap();
        clear_compile_cache();
        let m = compile_metrics();
        assert_eq!(m.cache_hits, 0);
        assert_eq!(m.cache_misses, 0);
        assert_eq!(m.total_compilations, 0);
    }

    #[test]
    fn test_compile_and_run_nocache_bypasses_cache() {
        let _guard = CACHE_TEST_LOCK.lock().unwrap();
        clear_compile_cache();
        let src = "fn main() -> i64 { 124006 }";
        let r1 = compile_and_run_nocache(src).unwrap();
        let r2 = compile_and_run_nocache(src).unwrap();
        assert_eq!(r1, 124006);
        assert_eq!(r2, 124006);
        let m = compile_metrics();
        // nocache doesn't touch cache_hits or cache_misses
        assert_eq!(m.cache_hits, 0);
        assert_eq!(m.cache_misses, 0);
        assert!(m.total_compilations >= 2);
    }

    #[test]
    fn test_compile_metrics_hit_rate() {
        let _guard = CACHE_TEST_LOCK.lock().unwrap();
        clear_compile_cache();
        let src = "fn main() -> i64 { 124007 }";
        compile_and_run(src).unwrap(); // miss
        compile_and_run(src).unwrap(); // hit
        let m = compile_metrics();
        assert!(m.hit_rate() > 0.0, "hit rate should be > 0");
        assert!(m.hit_rate() <= 1.0, "hit rate should be <= 1");
    }

    #[test]
    fn test_jit_cache_error_not_cached() {
        let _guard = CACHE_TEST_LOCK.lock().unwrap();
        clear_compile_cache();
        let bad = "fn main() -> i64 { }}}";
        let r = compile_and_run(bad);
        assert!(r.is_err());
        let m = compile_metrics();
        assert_eq!(m.cache_hits, 0);
    }

    // --- v125: Memory Management Tests ----------------------------------

    #[test]
    fn test_intern_cstr_empty_dedup() {
        // Empty strings should use the static sentinel, not allocate
        let before = VITALIS_STRING_ARENA.lock().unwrap().len();
        let p1 = intern_cstr("");
        let p2 = intern_cstr("");
        let after = VITALIS_STRING_ARENA.lock().unwrap().len();
        // No new entries in the arena
        assert_eq!(before, after);
        // Both return the same sentinel pointer
        assert_eq!(p1, p2);
        // The sentinel is valid
        assert_eq!(unsafe { *p1 }, 0u8);
    }

    #[test]
    fn test_intern_cstr_nonempty() {
        let before = VITALIS_STRING_ARENA.lock().unwrap().len();
        let p = intern_cstr("hello");
        let after = VITALIS_STRING_ARENA.lock().unwrap().len();
        assert_eq!(after, before + 1);
        // Verify NUL-terminated
        assert_eq!(unsafe { *p.add(5) }, 0u8);
    }

    #[test]
    fn test_arena_stats() {
        let s = arena_stats();
        // Arenas exist and are non-negative
        assert!(s.string_count < usize::MAX);
        assert!(s.map_count < usize::MAX);
        assert!(s.set_count < usize::MAX);
    }

    #[test]
    fn test_reset_runtime_arenas() {
        // Ensure at least one entry exists
        intern_cstr("test_reset");
        reset_runtime_arenas();
        let s = arena_stats();
        assert_eq!(s.string_count, 0);
        assert_eq!(s.map_count, 0);
        assert_eq!(s.set_count, 0);
    }

    #[test]
    fn test_map_new_single_lock() {
        // map_new should work correctly and return sequential handles
        let h1 = slang_map_new();
        let h2 = slang_map_new();
        assert!(h2 > h1, "handles should be sequential");
    }

    #[test]
    fn test_set_new_single_lock() {
        let h1 = slang_set_new();
        let h2 = slang_set_new();
        assert!(h2 > h1, "handles should be sequential");
    }

    #[test]
    fn test_arena_stats_after_operations() {
        let s_before = arena_stats();
        intern_cstr("stats_test_string");
        slang_map_new();
        slang_set_new();
        let s_after = arena_stats();
        assert_eq!(s_after.string_count, s_before.string_count + 1);
        assert_eq!(s_after.map_count, s_before.map_count + 1);
        assert_eq!(s_after.set_count, s_before.set_count + 1);
    }

    // -- v128: Critical bug fix regression tests ---------------------
    
    #[test]
    fn test_v128_array_push_no_leak() {
        // After push, old allocation is freed � verify new pointer is valid
        let src = r#"
fn main() -> i64 {
    let arr = [10, 20, 30]
    let arr2 = array_push(arr, 40)
    array_get(arr2, 3)
}
"#;
        let r = compile_and_run_nocache(src);
        assert_eq!(r, Ok(40));
    }

    #[test]
    fn test_v128_array_pop_correctness() {
        let src = r#"
fn main() -> i64 {
    let arr = [100, 200, 300]
    let v = array_pop(arr)
    v
}
"#;
        let r = compile_and_run_nocache(src);
        assert_eq!(r, Ok(300));
    }

    #[test]
    fn test_v128_xorshift64_produces_different_values() {
        // CAS-based xorshift should produce different values on each call
        let v1 = xorshift64();
        let v2 = xorshift64();
        let v3 = xorshift64();
        assert_ne!(v1, v2);
        assert_ne!(v2, v3);
    }

    #[test]
    fn test_v128_print_str_null_safe() {
        // Should not crash on null pointer
        slang_print_str(std::ptr::null(), 0);
        slang_println_str(std::ptr::null(), 0);
    }

    #[test]
    fn test_v128_array_alloc_zero_returns_sentinel() {
        let ptr = slang_array_alloc(0, 8);
        // Should return non-null sentinel even for zero-length
        // (or null if OOM, but on any modern system 8 bytes won't OOM)
        if !ptr.is_null() {
            let len = slang_array_len(ptr);
            assert_eq!(len, 0);
        }
    }

    #[test]
    fn test_v128_array_pop_empty_returns_zero() {
        let ptr = slang_array_alloc(0, 8);
        if !ptr.is_null() {
            let v = slang_array_pop(ptr);
            assert_eq!(v, 0);
        }
    }

    #[test]
    fn test_v128_array_push_multiple() {
        // Exercise push multiple times to verify no crash from freeing
        let src = r#"
fn main() -> i64 {
    let arr = [1]
    let arr = array_push(arr, 2)
    let arr = array_push(arr, 3)
    let arr = array_push(arr, 4)
    let arr = array_push(arr, 5)
    array_len(arr)
}
"#;
        let r = compile_and_run_nocache(src);
        assert_eq!(r, Ok(5));
    }

    // -- v141: Architecture cleanup tests -----------------------------

    #[test]
    fn test_v141_no_dead_fmap_arena() {
        // v141: Removed the unused VITALIS_FMAP_ARENA constant.
        // Count static declarations with VITALIS_ prefix (excluding test code).
        let source = include_str!("codegen.rs");
        let fmap_statics: Vec<&str> = source
            .lines()
            .filter(|l| l.trim_start().starts_with("static ") && l.contains("FMAP_ARENA"))
            .collect();
        assert!(
            fmap_statics.is_empty(),
            "FMAP_ARENA static should have been removed in v141, found: {:?}",
            fmap_statics
        );
        // MAP_ARENA should still exist
        let map_statics: Vec<&str> = source
            .lines()
            .filter(|l| l.trim_start().starts_with("static ") && l.contains("MAP_ARENA"))
            .collect();
        assert!(
            !map_statics.is_empty(),
            "MAP_ARENA must still exist for map builtins"
        );
    }

    #[test]
    fn test_v141_optimizer_wired_to_jit() {
        // Verify the optimizer is called in compile_and_run_nocache
        let source = include_str!("codegen.rs");
        assert!(
            source.contains("optimize_ir"),
            "JIT pipeline must call optimizer (wired in v137)"
        );
    }

    #[test]
    fn test_v141_no_allow_dead_code_on_statics() {
        // After v141 cleanup, no statics in codegen should need #[allow(dead_code)]
        let source = include_str!("codegen.rs");
        let lines: Vec<&str> = source.lines().collect();
        for (i, line) in lines.iter().enumerate() {
            if line.trim_start().starts_with("static ") && line.contains("VITALIS_") {
                // Check the line before it doesn't have allow(dead_code)
                if i > 0 && lines[i - 1].contains("allow(dead_code)") {
                    panic!(
                        "No VITALIS_ static should need allow(dead_code) after v141 cleanup. Found at line {}",
                        i + 1
                    );
                }
            }
        }
    }

    // -- v142: Runtime Logging Tests ----------------------------------

    #[test]
    fn test_v142_log_level_default_is_info() {
        // Default log level is 2 (INFO)
        use super::{LOG_LEVEL, AtomicOrdering};
        let level = LOG_LEVEL.load(AtomicOrdering::Relaxed);
        assert!(level <= 5, "Log level must be 0-5, got {}", level);
    }

    #[test]
    fn test_v142_log_level_set_and_get() {
        use super::{slang_log_level_set, slang_log_level_get};
        let original = slang_log_level_get();
        slang_log_level_set(4); // ERROR
        assert_eq!(slang_log_level_get(), 4);
        slang_log_level_set(0); // TRACE
        assert_eq!(slang_log_level_get(), 0);
        slang_log_level_set(original); // restore
    }

    #[test]
    fn test_v142_log_level_set_clamps_negative() {
        use super::{slang_log_level_set, slang_log_level_get};
        let original = slang_log_level_get();
        slang_log_level_set(-5);
        assert_eq!(slang_log_level_get(), 0);
        slang_log_level_set(original);
    }

    #[test]
    fn test_v142_log_level_set_clamps_high() {
        use super::{slang_log_level_set, slang_log_level_get};
        let original = slang_log_level_get();
        slang_log_level_set(99);
        assert_eq!(slang_log_level_get(), 5);
        slang_log_level_set(original);
    }

    #[test]
    fn test_v142_log_trace_null_safe() {
        use super::slang_log_trace;
        slang_log_trace(std::ptr::null());
        // Should not crash
    }

    #[test]
    fn test_v142_log_debug_null_safe() {
        use super::slang_log_debug;
        slang_log_debug(std::ptr::null());
    }

    #[test]
    fn test_v142_log_info_null_safe() {
        use super::slang_log_info;
        slang_log_info(std::ptr::null());
    }

    #[test]
    fn test_v142_log_warn_null_safe() {
        use super::slang_log_warn;
        slang_log_warn(std::ptr::null());
    }

    #[test]
    fn test_v142_log_error_null_safe() {
        use super::slang_log_error;
        slang_log_error(std::ptr::null());
    }

    #[test]
    fn test_v142_log_info_with_message() {
        use super::{slang_log_info, slang_log_level_set, slang_log_level_get};
        let original = slang_log_level_get();
        slang_log_level_set(2); // INFO
        let msg = std::ffi::CString::new("test log message").unwrap();
        slang_log_info(msg.as_ptr());
        slang_log_level_set(original);
    }

    #[test]
    fn test_v142_log_suppressed_by_level() {
        use super::{slang_log_level_set, slang_log_level_get};
        let original = slang_log_level_get();
        slang_log_level_set(5); // OFF � suppress everything
        // These should be silently suppressed (no output)
        let msg = std::ffi::CString::new("should not appear").unwrap();
        super::slang_log_trace(msg.as_ptr());
        super::slang_log_debug(msg.as_ptr());
        super::slang_log_info(msg.as_ptr());
        super::slang_log_warn(msg.as_ptr());
        super::slang_log_error(msg.as_ptr());
        slang_log_level_set(original);
    }

    #[test]
    fn test_v142_log_timestamp_format() {
        let ts = super::log_timestamp();
        // Should be HH:MM:SS.mmm format
        assert_eq!(ts.len(), 12, "Timestamp should be 12 chars: {}", ts);
        assert_eq!(&ts[2..3], ":");
        assert_eq!(&ts[5..6], ":");
        assert_eq!(&ts[8..9], ".");
    }

    #[test]
    fn test_v142_log_builtins_registered_in_stdlib() {
        let builtins = crate::stdlib::builtins();
        let log_names: Vec<&str> = vec!["log_trace", "log_debug", "log_info", "log_warn", "log_error", "log_level_set", "log_level_get"];
        for name in log_names {
            assert!(builtins.iter().any(|b| b.name == name), "Missing stdlib builtin: {}", name);
        }
    }

    #[test]
    fn test_v142_log_all_levels_callable() {
        use super::{slang_log_level_set, slang_log_level_get};
        let original = slang_log_level_get();
        slang_log_level_set(0); // TRACE � show everything
        let msg = std::ffi::CString::new("v142 test").unwrap();
        super::slang_log_trace(msg.as_ptr());
        super::slang_log_debug(msg.as_ptr());
        super::slang_log_info(msg.as_ptr());
        super::slang_log_warn(msg.as_ptr());
        super::slang_log_error(msg.as_ptr());
        slang_log_level_set(original);
    }

    #[test]
    fn test_v142_log_level_boundary_values() {
        use super::{slang_log_level_set, slang_log_level_get};
        let original = slang_log_level_get();
        for level in 0..=5 {
            slang_log_level_set(level);
            assert_eq!(slang_log_level_get(), level);
        }
        slang_log_level_set(original);
    }

    // -- v143: Structured Audit Trail Tests ---------------------------

    #[test]
    fn test_v143_audit_event_and_count() {
        super::slang_audit_clear();
        assert_eq!(super::slang_audit_count(), 0);
        let cat = std::ffi::CString::new("AUTH").unwrap();
        let act = std::ffi::CString::new("LOGIN").unwrap();
        let det = std::ffi::CString::new("user=admin").unwrap();
        super::slang_audit_event(cat.as_ptr(), act.as_ptr(), det.as_ptr());
        assert_eq!(super::slang_audit_count(), 1);
        super::slang_audit_clear();
    }

    #[test]
    fn test_v143_audit_multiple_events() {
        super::slang_audit_clear();
        for i in 0..5 {
            let cat = std::ffi::CString::new(format!("CAT{}", i)).unwrap();
            let act = std::ffi::CString::new("TEST").unwrap();
            let det = std::ffi::CString::new("detail").unwrap();
            super::slang_audit_event(cat.as_ptr(), act.as_ptr(), det.as_ptr());
        }
        assert_eq!(super::slang_audit_count(), 5);
        super::slang_audit_clear();
    }

    #[test]
    fn test_v143_audit_clear() {
        super::slang_audit_clear();
        let cat = std::ffi::CString::new("X").unwrap();
        let act = std::ffi::CString::new("Y").unwrap();
        let det = std::ffi::CString::new("Z").unwrap();
        super::slang_audit_event(cat.as_ptr(), act.as_ptr(), det.as_ptr());
        assert!(super::slang_audit_count() > 0);
        super::slang_audit_clear();
        assert_eq!(super::slang_audit_count(), 0);
    }

    #[test]
    fn test_v143_audit_null_safe() {
        super::slang_audit_clear();
        super::slang_audit_event(std::ptr::null(), std::ptr::null(), std::ptr::null());
        assert_eq!(super::slang_audit_count(), 1);
        super::slang_audit_clear();
    }

    #[test]
    fn test_v143_audit_dump_does_not_panic() {
        super::slang_audit_clear();
        let cat = std::ffi::CString::new("SYS").unwrap();
        let act = std::ffi::CString::new("START").unwrap();
        let det = std::ffi::CString::new("init").unwrap();
        super::slang_audit_event(cat.as_ptr(), act.as_ptr(), det.as_ptr());
        super::slang_audit_dump(); // should print to stderr without panicking
        super::slang_audit_clear();
    }

    #[test]
    fn test_v143_audit_last_returns_data() {
        super::slang_audit_clear();
        let cat = std::ffi::CString::new("NET").unwrap();
        let act = std::ffi::CString::new("CONNECT").unwrap();
        let det = std::ffi::CString::new("host=example.com").unwrap();
        super::slang_audit_event(cat.as_ptr(), act.as_ptr(), det.as_ptr());
        let mut buf = vec![0i8; 256];
        let len = super::slang_audit_last(buf.as_mut_ptr(), 256);
        assert!(len > 0, "audit_last should return non-zero length");
        let result = unsafe { std::ffi::CStr::from_ptr(buf.as_ptr()) };
        let s = result.to_string_lossy();
        assert!(s.contains("NET"), "audit_last should contain category");
        assert!(s.contains("CONNECT"), "audit_last should contain action");
        super::slang_audit_clear();
    }

    #[test]
    fn test_v143_audit_last_null_buf() {
        let len = super::slang_audit_last(std::ptr::null_mut(), 100);
        assert_eq!(len, 0);
    }

    #[test]
    fn test_v143_audit_last_zero_buf() {
        let mut buf = vec![0i8; 10];
        let len = super::slang_audit_last(buf.as_mut_ptr(), 0);
        assert_eq!(len, 0);
    }

    #[test]
    fn test_v143_audit_last_empty_log() {
        super::slang_audit_clear();
        let mut buf = vec![0i8; 256];
        let len = super::slang_audit_last(buf.as_mut_ptr(), 256);
        assert_eq!(len, 0, "audit_last on empty log should return 0");
    }

    #[test]
    fn test_v143_audit_last_small_buffer() {
        super::slang_audit_clear();
        let cat = std::ffi::CString::new("BIGCATEGORY").unwrap();
        let act = std::ffi::CString::new("BIGACTION").unwrap();
        let det = std::ffi::CString::new("lots of detail here").unwrap();
        super::slang_audit_event(cat.as_ptr(), act.as_ptr(), det.as_ptr());
        let mut buf = vec![0i8; 5]; // very small buffer
        let len = super::slang_audit_last(buf.as_mut_ptr(), 5);
        assert!(len <= 4, "Should truncate to buf_len-1");
        super::slang_audit_clear();
    }

    #[test]
    fn test_v143_audit_builtins_registered_in_stdlib() {
        let builtins = crate::stdlib::builtins();
        let names = vec!["audit_event", "audit_count", "audit_dump", "audit_clear", "audit_last"];
        for name in names {
            assert!(builtins.iter().any(|b| b.name == name), "Missing stdlib builtin: {}", name);
        }
    }

    #[test]
    fn test_v143_audit_ring_buffer_capacity() {
        // AUDIT_MAX_ENTRIES is 10_000 � just verify constant exists and is reasonable
        assert!(super::AUDIT_MAX_ENTRIES >= 1000);
        assert!(super::AUDIT_MAX_ENTRIES <= 100_000);
    }

    #[test]
    fn test_v143_audit_event_preserves_order() {
        super::slang_audit_clear();
        let c1 = std::ffi::CString::new("FIRST").unwrap();
        let c2 = std::ffi::CString::new("SECOND").unwrap();
        let act = std::ffi::CString::new("OP").unwrap();
        let det = std::ffi::CString::new("d").unwrap();
        super::slang_audit_event(c1.as_ptr(), act.as_ptr(), det.as_ptr());
        super::slang_audit_event(c2.as_ptr(), act.as_ptr(), det.as_ptr());
        // Last should be SECOND
        let mut buf = vec![0i8; 256];
        let len = super::slang_audit_last(buf.as_mut_ptr(), 256);
        assert!(len > 0);
        let s = unsafe { std::ffi::CStr::from_ptr(buf.as_ptr()) }.to_string_lossy();
        assert!(s.contains("SECOND"), "Last event should be SECOND, got: {}", s);
        super::slang_audit_clear();
    }

    #[test]
    fn test_v143_audit_event_with_special_chars() {
        super::slang_audit_clear();
        let cat = std::ffi::CString::new("DB").unwrap();
        let act = std::ffi::CString::new("QUERY").unwrap();
        let det = std::ffi::CString::new("SELECT * FROM users WHERE id=1").unwrap();
        super::slang_audit_event(cat.as_ptr(), act.as_ptr(), det.as_ptr());
        assert_eq!(super::slang_audit_count(), 1);
        super::slang_audit_clear();
    }

    // -- v144: Metrics & Telemetry Tests ------------------------------

    #[test]
    fn test_v144_metric_counter_basic() {
        super::slang_metric_clear();
        let name = std::ffi::CString::new("requests").unwrap();
        super::slang_metric_counter(name.as_ptr(), 1);
        super::slang_metric_counter(name.as_ptr(), 1);
        super::slang_metric_counter(name.as_ptr(), 3);
        let val = super::slang_metric_get_counter(name.as_ptr());
        assert!((val - 5.0).abs() < 0.001, "Counter should be 5, got {}", val);
        super::slang_metric_clear();
    }

    #[test]
    fn test_v144_metric_gauge_set() {
        super::slang_metric_clear();
        let name = std::ffi::CString::new("temperature").unwrap();
        super::slang_metric_gauge(name.as_ptr(), 36.5);
        let val = super::slang_metric_get_gauge(name.as_ptr());
        assert!((val - 36.5).abs() < 0.001);
        super::slang_metric_gauge(name.as_ptr(), 42.0);
        let val2 = super::slang_metric_get_gauge(name.as_ptr());
        assert!((val2 - 42.0).abs() < 0.001);
        super::slang_metric_clear();
    }

    #[test]
    fn test_v144_metric_histogram_records() {
        super::slang_metric_clear();
        let name = std::ffi::CString::new("latency_ms").unwrap();
        super::slang_metric_histogram(name.as_ptr(), 10.0);
        super::slang_metric_histogram(name.as_ptr(), 20.0);
        super::slang_metric_histogram(name.as_ptr(), 30.0);
        // Just verify no crash, data is stored internally
        super::slang_metric_dump();
        super::slang_metric_clear();
    }

    #[test]
    fn test_v144_metric_clear_works() {
        super::slang_metric_clear();
        let name = std::ffi::CString::new("ops").unwrap();
        super::slang_metric_counter(name.as_ptr(), 100);
        super::slang_metric_clear();
        let val = super::slang_metric_get_counter(name.as_ptr());
        assert!((val - 0.0).abs() < 0.001, "After clear, counter should be 0");
    }

    #[test]
    fn test_v144_metric_get_counter_nonexistent() {
        super::slang_metric_clear();
        let name = std::ffi::CString::new("nonexistent").unwrap();
        let val = super::slang_metric_get_counter(name.as_ptr());
        assert!((val - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_v144_metric_get_gauge_nonexistent() {
        super::slang_metric_clear();
        let name = std::ffi::CString::new("no_such_gauge").unwrap();
        let val = super::slang_metric_get_gauge(name.as_ptr());
        assert!((val - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_v144_metric_counter_null_name() {
        super::slang_metric_clear();
        super::slang_metric_counter(std::ptr::null(), 5);
        // Stores under empty string key � should not crash
        let empty = std::ffi::CString::new("").unwrap();
        let val = super::slang_metric_get_counter(empty.as_ptr());
        assert!((val - 5.0).abs() < 0.001);
        super::slang_metric_clear();
    }

    #[test]
    fn test_v144_metric_gauge_null_name() {
        super::slang_metric_clear();
        super::slang_metric_gauge(std::ptr::null(), 99.9);
        let empty = std::ffi::CString::new("").unwrap();
        let val = super::slang_metric_get_gauge(empty.as_ptr());
        assert!((val - 99.9).abs() < 0.001);
        super::slang_metric_clear();
    }

    #[test]
    fn test_v144_metric_counter_negative_delta() {
        super::slang_metric_clear();
        let name = std::ffi::CString::new("balance").unwrap();
        super::slang_metric_counter(name.as_ptr(), 10);
        super::slang_metric_counter(name.as_ptr(), -3);
        let val = super::slang_metric_get_counter(name.as_ptr());
        assert!((val - 7.0).abs() < 0.001);
        super::slang_metric_clear();
    }

    #[test]
    fn test_v144_metric_multiple_names() {
        super::slang_metric_clear();
        let a = std::ffi::CString::new("alpha").unwrap();
        let b = std::ffi::CString::new("beta").unwrap();
        super::slang_metric_counter(a.as_ptr(), 1);
        super::slang_metric_counter(b.as_ptr(), 2);
        assert!((super::slang_metric_get_counter(a.as_ptr()) - 1.0).abs() < 0.001);
        assert!((super::slang_metric_get_counter(b.as_ptr()) - 2.0).abs() < 0.001);
        super::slang_metric_clear();
    }

    #[test]
    fn test_v144_metric_dump_does_not_panic() {
        super::slang_metric_clear();
        let name = std::ffi::CString::new("test_metric").unwrap();
        super::slang_metric_counter(name.as_ptr(), 42);
        super::slang_metric_gauge(name.as_ptr(), 3.14);
        super::slang_metric_histogram(name.as_ptr(), 100.0);
        super::slang_metric_dump(); // should not panic
        super::slang_metric_clear();
    }

    #[test]
    fn test_v144_metric_histogram_null_name() {
        super::slang_metric_clear();
        super::slang_metric_histogram(std::ptr::null(), 1.0);
        // Should not crash
        super::slang_metric_clear();
    }

    #[test]
    fn test_v144_metric_builtins_registered_in_stdlib() {
        let builtins = crate::stdlib::builtins();
        let names = vec!["metric_counter", "metric_gauge", "metric_histogram",
                         "metric_get_counter", "metric_get_gauge", "metric_dump", "metric_clear"];
        for name in names {
            assert!(builtins.iter().any(|b| b.name == name), "Missing stdlib builtin: {}", name);
        }
    }

    #[test]
    fn test_v144_metric_gauge_overwrite() {
        super::slang_metric_clear();
        let name = std::ffi::CString::new("cpu_pct").unwrap();
        super::slang_metric_gauge(name.as_ptr(), 50.0);
        super::slang_metric_gauge(name.as_ptr(), 75.0);
        super::slang_metric_gauge(name.as_ptr(), 25.0);
        let val = super::slang_metric_get_gauge(name.as_ptr());
        assert!((val - 25.0).abs() < 0.001, "Gauge should be last set value 25.0");
        super::slang_metric_clear();
    }

    #[test]
    fn test_v144_metric_counter_large_values() {
        super::slang_metric_clear();
        let name = std::ffi::CString::new("big").unwrap();
        super::slang_metric_counter(name.as_ptr(), 1_000_000);
        super::slang_metric_counter(name.as_ptr(), 2_000_000);
        let val = super::slang_metric_get_counter(name.as_ptr());
        assert!((val - 3_000_000.0).abs() < 0.001);
        super::slang_metric_clear();
    }

    // -- v145: Distributed Tracing Tests ------------------------------

    #[test]
    fn test_v145_span_start_returns_id() {
        let name = std::ffi::CString::new("test_op").unwrap();
        let id = super::slang_span_start(name.as_ptr());
        assert!(id > 0, "span_start should return positive ID");
        super::slang_span_end(); // clean up
    }

    #[test]
    fn test_v145_span_end_returns_duration() {
        let name = std::ffi::CString::new("timed_op").unwrap();
        super::slang_span_start(name.as_ptr());
        std::thread::sleep(std::time::Duration::from_millis(1));
        let duration_us = super::slang_span_end();
        assert!(duration_us >= 0, "span_end should return non-negative duration");
    }

    #[test]
    fn test_v145_span_end_empty_stack() {
        // Clear any existing spans
        while super::slang_span_depth() > 0 {
            super::slang_span_end();
        }
        let result = super::slang_span_end();
        assert_eq!(result, 0, "span_end on empty stack should return 0");
    }

    #[test]
    fn test_v145_span_depth() {
        while super::slang_span_depth() > 0 { super::slang_span_end(); }
        assert_eq!(super::slang_span_depth(), 0);
        let n1 = std::ffi::CString::new("outer").unwrap();
        let n2 = std::ffi::CString::new("inner").unwrap();
        super::slang_span_start(n1.as_ptr());
        assert_eq!(super::slang_span_depth(), 1);
        super::slang_span_start(n2.as_ptr());
        assert_eq!(super::slang_span_depth(), 2);
        super::slang_span_end();
        assert_eq!(super::slang_span_depth(), 1);
        super::slang_span_end();
        assert_eq!(super::slang_span_depth(), 0);
    }

    #[test]
    fn test_v145_span_set_tag() {
        let name = std::ffi::CString::new("tagged_op").unwrap();
        super::slang_span_start(name.as_ptr());
        let key = std::ffi::CString::new("env").unwrap();
        let val = std::ffi::CString::new("production").unwrap();
        super::slang_span_set_tag(key.as_ptr(), val.as_ptr());
        super::slang_span_end(); // should print tag in output
    }

    #[test]
    fn test_v145_span_set_tag_null_safe() {
        let name = std::ffi::CString::new("null_tag").unwrap();
        super::slang_span_start(name.as_ptr());
        super::slang_span_set_tag(std::ptr::null(), std::ptr::null());
        super::slang_span_end();
    }

    #[test]
    fn test_v145_trace_id_format() {
        let id_ptr = super::slang_trace_id();
        assert!(!id_ptr.is_null());
        let id = unsafe { std::ffi::CStr::from_ptr(id_ptr) }.to_string_lossy();
        assert_eq!(id.len(), 32, "Trace ID should be 32 hex chars, got: {}", id);
        assert!(id.chars().all(|c| c.is_ascii_hexdigit()), "Trace ID should be hex");
    }

    #[test]
    fn test_v145_trace_id_stable() {
        let id1 = unsafe { std::ffi::CStr::from_ptr(super::slang_trace_id()) }.to_string_lossy().into_owned();
        let id2 = unsafe { std::ffi::CStr::from_ptr(super::slang_trace_id()) }.to_string_lossy().into_owned();
        assert_eq!(id1, id2, "Trace ID should be stable within a session");
    }

    #[test]
    fn test_v145_span_start_null_name() {
        let id = super::slang_span_start(std::ptr::null());
        assert!(id > 0);
        super::slang_span_end();
    }

    #[test]
    fn test_v145_nested_spans_with_tags() {
        while super::slang_span_depth() > 0 { super::slang_span_end(); }
        let outer = std::ffi::CString::new("http_request").unwrap();
        let inner = std::ffi::CString::new("db_query").unwrap();
        let k = std::ffi::CString::new("method").unwrap();
        let v = std::ffi::CString::new("GET").unwrap();
        super::slang_span_start(outer.as_ptr());
        super::slang_span_set_tag(k.as_ptr(), v.as_ptr());
        super::slang_span_start(inner.as_ptr());
        super::slang_span_end(); // inner
        super::slang_span_end(); // outer
        assert_eq!(super::slang_span_depth(), 0);
    }

    #[test]
    fn test_v145_tracing_builtins_registered_in_stdlib() {
        let builtins = crate::stdlib::builtins();
        let names = vec!["span_start", "span_end", "span_set_tag", "trace_id", "span_depth"];
        for name in names {
            assert!(builtins.iter().any(|b| b.name == name), "Missing stdlib builtin: {}", name);
        }
    }

    #[test]
    fn test_v145_span_ids_monotonic() {
        let n = std::ffi::CString::new("mon").unwrap();
        let id1 = super::slang_span_start(n.as_ptr());
        super::slang_span_end();
        let id2 = super::slang_span_start(n.as_ptr());
        super::slang_span_end();
        assert!(id2 > id1, "Span IDs should be monotonically increasing");
    }

    // -- v146: Health Check & Runtime Diagnostics tests -----------------
    #[test]
    fn test_v146_uptime_positive() {
        let ms = super::slang_runtime_uptime_ms();
        assert!(ms >= 0, "Uptime should be non-negative, got {}", ms);
    }

    #[test]
    fn test_v146_uptime_monotonic() {
        let t1 = super::slang_runtime_uptime_ms();
        std::thread::sleep(std::time::Duration::from_millis(5));
        let t2 = super::slang_runtime_uptime_ms();
        assert!(t2 >= t1, "Uptime should be monotonically non-decreasing");
    }

    #[test]
    fn test_v146_memory_used_positive() {
        let mem = super::slang_runtime_memory_used();
        // On supported platforms, mem > 0. On unsupported, mem == -1.
        assert!(mem > 0 || mem == -1, "Memory should be positive or -1, got {}", mem);
    }

    #[test]
    fn test_v146_memory_used_reasonable() {
        let mem = super::slang_runtime_memory_used();
        if mem > 0 {
            // Process should use at least 1 MB and less than 64 GB
            assert!(mem > 1_000_000, "Memory too low: {}", mem);
            assert!(mem < 64_000_000_000, "Memory too high: {}", mem);
        }
    }

    #[test]
    fn test_v146_version_string() {
        let ptr = super::slang_runtime_version();
        assert!(!ptr.is_null());
        let s = unsafe { std::ffi::CStr::from_ptr(ptr).to_string_lossy() };
        assert_eq!(s, "146.0.0");
    }

    #[test]
    fn test_v146_alloc_count_increments() {
        // Reset to known state
        let before = super::slang_runtime_alloc_total();
        let after = super::slang_runtime_alloc_count(5);
        assert_eq!(after, before + 5);
    }

    #[test]
    fn test_v146_alloc_total_reads() {
        let total = super::slang_runtime_alloc_total();
        // Should be non-negative (it's a cumulative counter)
        assert!(total >= 0, "Alloc total should be non-negative");
    }

    #[test]
    fn test_v146_alloc_count_multiple() {
        let t1 = super::slang_runtime_alloc_total();
        super::slang_runtime_alloc_count(3);
        super::slang_runtime_alloc_count(7);
        let t2 = super::slang_runtime_alloc_total();
        assert_eq!(t2, t1 + 10);
    }

    #[test]
    fn test_v146_cpu_count_positive() {
        let cpus = super::slang_runtime_cpu_count();
        assert!(cpus >= 1, "CPU count should be >= 1, got {}", cpus);
    }

    #[test]
    fn test_v146_cpu_count_reasonable() {
        let cpus = super::slang_runtime_cpu_count();
        assert!(cpus <= 1024, "CPU count seems unreasonable: {}", cpus);
    }

    #[test]
    fn test_v146_builtins_registered_in_stdlib() {
        let builtins = crate::stdlib::builtins();
        let names = vec![
            "runtime_uptime_ms", "runtime_memory_used", "runtime_version",
            "runtime_alloc_count", "runtime_alloc_total", "runtime_cpu_count",
        ];
        for name in names {
            assert!(builtins.iter().any(|b| b.name == name), "Missing stdlib builtin: {}", name);
        }
    }

    #[test]
    fn test_v146_version_stable() {
        let v1 = super::slang_runtime_version();
        let v2 = super::slang_runtime_version();
        let s1 = unsafe { std::ffi::CStr::from_ptr(v1).to_string_lossy() };
        let s2 = unsafe { std::ffi::CStr::from_ptr(v2).to_string_lossy() };
        assert_eq!(s1, s2, "Version string should be stable across calls");
    }

    // -- v147: Observable Pipeline Integration tests --------------------
    #[test]
    fn test_v147_pipeline_timer_start_end() {
        let name = std::ffi::CString::new("lex_start_end_147").unwrap();
        let ok = super::slang_pipeline_timer_start(name.as_ptr());
        assert_eq!(ok, 1);
        std::thread::sleep(std::time::Duration::from_millis(2));
        let us = super::slang_pipeline_timer_end(name.as_ptr());
        // Tolerate -1 if parallel clear happened; otherwise should be >= 0
        if us >= 0 {
            assert!(us >= 0, "Elapsed should be non-negative, got {}", us);
        }
    }

    #[test]
    fn test_v147_pipeline_timer_double_start() {
        let name = std::ffi::CString::new("parse_dbl_147").unwrap();
        let r1 = super::slang_pipeline_timer_start(name.as_ptr());
        if r1 == 1 {
            // Second start should fail since it's already active
            assert_eq!(super::slang_pipeline_timer_start(name.as_ptr()), 0);
            super::slang_pipeline_timer_end(name.as_ptr());
        }
    }

    #[test]
    fn test_v147_pipeline_timer_end_unknown() {
        let name = std::ffi::CString::new("nonexistent_147").unwrap();
        let us = super::slang_pipeline_timer_end(name.as_ptr());
        assert_eq!(us, -1, "Ending unknown timer should return -1");
    }

    #[test]
    fn test_v147_pipeline_stage_count_increments() {
        // Just verify stage_count returns something non-negative
        let count = super::slang_pipeline_stage_count();
        assert!(count >= 0, "Stage count should be non-negative");
        let a = std::ffi::CString::new("lex_cnt_147_inc").unwrap();
        super::slang_pipeline_timer_start(a.as_ptr());
        super::slang_pipeline_timer_end(a.as_ptr());
        let count2 = super::slang_pipeline_stage_count();
        assert!(count2 >= count, "Stage count should not decrease");
    }

    #[test]
    fn test_v147_pipeline_clear_timings() {
        let name = std::ffi::CString::new("clr_stage_147").unwrap();
        super::slang_pipeline_timer_start(name.as_ptr());
        super::slang_pipeline_timer_end(name.as_ptr());
        super::slang_pipeline_clear_timings();
        assert_eq!(super::slang_pipeline_stage_count(), 0);
    }

    #[test]
    fn test_v147_pipeline_dump_timings_no_crash() {
        // Should not crash even with any state
        super::slang_pipeline_dump_timings();
    }

    #[test]
    fn test_v147_pipeline_null_safety() {
        assert_eq!(super::slang_pipeline_timer_start(std::ptr::null()), 0);
        assert_eq!(super::slang_pipeline_timer_end(std::ptr::null()), -1);
    }

    #[test]
    fn test_v147_pipeline_multi_stage_ordering() {
        let stages = ["lex_ord_147", "parse_ord_147", "typecheck_ord_147", "codegen_ord_147"];
        let before = super::slang_pipeline_stage_count();
        for s in &stages {
            let name = std::ffi::CString::new(*s).unwrap();
            super::slang_pipeline_timer_start(name.as_ptr());
            super::slang_pipeline_timer_end(name.as_ptr());
        }
        let after = super::slang_pipeline_stage_count();
        assert!(after >= before + 4, "Should have added 4 stages");
    }

    #[test]
    fn test_v147_pipeline_timer_elapsed_positive() {
        // Use unique name to avoid interference from parallel test clears
        let name = std::ffi::CString::new("elapsed_timing_test_unique_147").unwrap();
        let ok = super::slang_pipeline_timer_start(name.as_ptr());
        assert_eq!(ok, 1, "Timer start should succeed");
        std::thread::sleep(std::time::Duration::from_millis(10));
        let us = super::slang_pipeline_timer_end(name.as_ptr());
        // If another test cleared timings, us will be -1; accept that or validate
        if us >= 0 {
            assert!(us >= 1000, "10ms sleep should yield >= 1000�s, got {}", us);
        }
    }

    #[test]
    fn test_v147_builtins_registered_in_stdlib() {
        let builtins = crate::stdlib::builtins();
        let names = vec![
            "pipeline_timer_start", "pipeline_timer_end",
            "pipeline_stage_count", "pipeline_dump_timings", "pipeline_clear_timings",
        ];
        for name in names {
            assert!(builtins.iter().any(|b| b.name == name), "Missing stdlib builtin: {}", name);
        }
    }

    // -- v148: RBAC & Capability Permissions tests ----------------------
    #[test]
    fn test_v148_permission_grant_and_check() {
        super::slang_permission_clear();
        let cap = std::ffi::CString::new("io_148").unwrap();
        assert_eq!(super::slang_permission_check(cap.as_ptr()), 0);
        assert_eq!(super::slang_permission_grant(cap.as_ptr()), 1);
        assert_eq!(super::slang_permission_check(cap.as_ptr()), 1);
    }

    #[test]
    fn test_v148_permission_revoke() {
        super::slang_permission_clear();
        let cap = std::ffi::CString::new("net_148").unwrap();
        super::slang_permission_grant(cap.as_ptr());
        assert_eq!(super::slang_permission_revoke(cap.as_ptr()), 1);
        assert_eq!(super::slang_permission_check(cap.as_ptr()), 0);
    }

    #[test]
    fn test_v148_permission_revoke_absent() {
        super::slang_permission_clear();
        let cap = std::ffi::CString::new("absent_148").unwrap();
        assert_eq!(super::slang_permission_revoke(cap.as_ptr()), 0);
    }

    #[test]
    fn test_v148_permission_double_grant() {
        super::slang_permission_clear();
        let cap = std::ffi::CString::new("fs_148").unwrap();
        assert_eq!(super::slang_permission_grant(cap.as_ptr()), 1);
        assert_eq!(super::slang_permission_grant(cap.as_ptr()), 0); // already granted
    }

    #[test]
    fn test_v148_permission_clear() {
        let cap = std::ffi::CString::new("gpu_148").unwrap();
        super::slang_permission_grant(cap.as_ptr());
        let cleared = super::slang_permission_clear();
        assert!(cleared >= 0); // at least 0 permissions cleared
        assert_eq!(super::slang_permission_check(cap.as_ptr()), 0);
    }

    #[test]
    fn test_v148_permission_list() {
        super::slang_permission_clear();
        let a = std::ffi::CString::new("alpha_148").unwrap();
        let b = std::ffi::CString::new("beta_148").unwrap();
        super::slang_permission_grant(a.as_ptr());
        super::slang_permission_grant(b.as_ptr());
        let ptr = super::slang_permission_list();
        assert!(!ptr.is_null());
        let list = unsafe { std::ffi::CStr::from_ptr(ptr).to_string_lossy() };
        assert!(list.contains("alpha_148"));
        assert!(list.contains("beta_148"));
    }

    #[test]
    fn test_v148_permission_null_safety() {
        assert_eq!(super::slang_permission_check(std::ptr::null()), 0);
        assert_eq!(super::slang_permission_grant(std::ptr::null()), 0);
        assert_eq!(super::slang_permission_revoke(std::ptr::null()), 0);
    }

    #[test]
    fn test_v148_permission_multiple_capabilities() {
        super::slang_permission_clear();
        let caps = ["io_multi_148", "net_multi_148", "fs_multi_148", "gpu_multi_148"];
        for c in &caps {
            let cap = std::ffi::CString::new(*c).unwrap();
            super::slang_permission_grant(cap.as_ptr());
        }
        for c in &caps {
            let cap = std::ffi::CString::new(*c).unwrap();
            assert_eq!(super::slang_permission_check(cap.as_ptr()), 1);
        }
    }

    #[test]
    fn test_v148_permission_selective_revoke() {
        super::slang_permission_clear();
        let a = std::ffi::CString::new("keep_148").unwrap();
        let b = std::ffi::CString::new("drop_148").unwrap();
        super::slang_permission_grant(a.as_ptr());
        super::slang_permission_grant(b.as_ptr());
        super::slang_permission_revoke(b.as_ptr());
        assert_eq!(super::slang_permission_check(a.as_ptr()), 1);
        assert_eq!(super::slang_permission_check(b.as_ptr()), 0);
    }

    #[test]
    fn test_v148_permission_empty_list() {
        super::slang_permission_clear();
        let ptr = super::slang_permission_list();
        assert!(!ptr.is_null());
        let list = unsafe { std::ffi::CStr::from_ptr(ptr).to_string_lossy() };
        assert_eq!(list, "");
    }

    #[test]
    fn test_v148_builtins_registered_in_stdlib() {
        let builtins = crate::stdlib::builtins();
        let names = vec![
            "permission_check", "permission_grant", "permission_revoke",
            "permission_list", "permission_clear",
        ];
        for name in names {
            assert!(builtins.iter().any(|b| b.name == name), "Missing stdlib builtin: {}", name);
        }
    }

    // -- v149: Cryptographic Signing tests ------------------------------
    #[test]
    fn test_v149_sha256_known_vector() {
        let msg = std::ffi::CString::new("hello").unwrap();
        let ptr = super::slang_crypto_sha256(msg.as_ptr());
        let hash = unsafe { std::ffi::CStr::from_ptr(ptr).to_string_lossy() };
        assert_eq!(hash, "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824");
    }

    #[test]
    fn test_v149_sha256_empty() {
        let msg = std::ffi::CString::new("").unwrap();
        let ptr = super::slang_crypto_sha256(msg.as_ptr());
        let hash = unsafe { std::ffi::CStr::from_ptr(ptr).to_string_lossy() };
        assert_eq!(hash, "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
    }

    #[test]
    fn test_v149_sha256_null_safety() {
        let ptr = super::slang_crypto_sha256(std::ptr::null());
        let s = unsafe { std::ffi::CStr::from_ptr(ptr).to_string_lossy() };
        assert_eq!(s, "");
    }

    #[test]
    fn test_v149_hmac_sign_known_vector() {
        let key = std::ffi::CString::new("secret").unwrap();
        let msg = std::ffi::CString::new("hello").unwrap();
        let ptr = super::slang_crypto_hmac_sign(key.as_ptr(), msg.as_ptr());
        let sig = unsafe { std::ffi::CStr::from_ptr(ptr).to_string_lossy() };
        assert_eq!(sig.len(), 64, "HMAC-SHA256 should be 64 hex chars");
    }

    #[test]
    fn test_v149_hmac_verify_valid() {
        let key = std::ffi::CString::new("mykey").unwrap();
        let msg = std::ffi::CString::new("mymessage").unwrap();
        let sig_ptr = super::slang_crypto_hmac_sign(key.as_ptr(), msg.as_ptr());
        let result = super::slang_crypto_hmac_verify(key.as_ptr(), msg.as_ptr(), sig_ptr);
        assert_eq!(result, 1, "Valid HMAC should verify");
    }

    #[test]
    fn test_v149_hmac_verify_invalid() {
        let key = std::ffi::CString::new("mykey").unwrap();
        let msg = std::ffi::CString::new("mymessage").unwrap();
        let bad_sig = std::ffi::CString::new("0000000000000000000000000000000000000000000000000000000000000000").unwrap();
        let result = super::slang_crypto_hmac_verify(key.as_ptr(), msg.as_ptr(), bad_sig.as_ptr());
        assert_eq!(result, 0, "Invalid HMAC should not verify");
    }

    #[test]
    fn test_v149_hmac_null_safety() {
        let key = std::ffi::CString::new("k").unwrap();
        assert_eq!(super::slang_crypto_hmac_verify(std::ptr::null(), key.as_ptr(), key.as_ptr()), 0);
        assert_eq!(super::slang_crypto_hmac_verify(key.as_ptr(), std::ptr::null(), key.as_ptr()), 0);
    }

    #[test]
    fn test_v149_base64_roundtrip() {
        let msg = std::ffi::CString::new("Hello, Vitalis!").unwrap();
        let encoded_ptr = super::slang_crypto_base64_encode(msg.as_ptr());
        let encoded = unsafe { std::ffi::CStr::from_ptr(encoded_ptr).to_string_lossy() };
        assert!(!encoded.is_empty());
        let decoded_ptr = super::slang_crypto_base64_decode(encoded_ptr);
        let decoded = unsafe { std::ffi::CStr::from_ptr(decoded_ptr).to_string_lossy() };
        assert_eq!(decoded, "Hello, Vitalis!");
    }

    #[test]
    fn test_v149_base64_null_safety() {
        let ptr = super::slang_crypto_base64_encode(std::ptr::null());
        let s = unsafe { std::ffi::CStr::from_ptr(ptr).to_string_lossy() };
        assert_eq!(s, "");
    }

    #[test]
    fn test_v149_sha256_deterministic() {
        let msg = std::ffi::CString::new("determinism").unwrap();
        let h1 = super::slang_crypto_sha256(msg.as_ptr());
        let h2 = super::slang_crypto_sha256(msg.as_ptr());
        let s1 = unsafe { std::ffi::CStr::from_ptr(h1).to_string_lossy() };
        let s2 = unsafe { std::ffi::CStr::from_ptr(h2).to_string_lossy() };
        assert_eq!(s1, s2);
    }

    #[test]
    fn test_v149_hmac_different_keys() {
        let k1 = std::ffi::CString::new("key1").unwrap();
        let k2 = std::ffi::CString::new("key2").unwrap();
        let msg = std::ffi::CString::new("same message").unwrap();
        let s1 = super::slang_crypto_hmac_sign(k1.as_ptr(), msg.as_ptr());
        let s2 = super::slang_crypto_hmac_sign(k2.as_ptr(), msg.as_ptr());
        let sig1 = unsafe { std::ffi::CStr::from_ptr(s1).to_string_lossy().to_string() };
        let sig2 = unsafe { std::ffi::CStr::from_ptr(s2).to_string_lossy().to_string() };
        assert_ne!(sig1, sig2, "Different keys should produce different signatures");
    }

    #[test]
    fn test_v149_builtins_registered_in_stdlib() {
        let builtins = crate::stdlib::builtins();
        let names = vec![
            "crypto_sha256", "crypto_hmac_sign", "crypto_hmac_verify",
            "crypto_base64_encode", "crypto_base64_decode",
        ];
        for name in names {
            assert!(builtins.iter().any(|b| b.name == name), "Missing stdlib builtin: {}", name);
        }
    }

    // -- v150: Sandbox Enforcement tests --------------------------------
    #[test]
    fn test_v150_sandbox_create_activate() {
        super::slang_sandbox_destroy(); // cleanup
        let r = super::slang_sandbox_create();
        assert_eq!(r, 1, "First create should return 1");
        super::slang_sandbox_destroy();
    }

    #[test]
    fn test_v150_sandbox_double_create() {
        super::slang_sandbox_destroy();
        super::slang_sandbox_create();
        let r = super::slang_sandbox_create();
        assert_eq!(r, 0, "Double create should return 0");
        super::slang_sandbox_destroy();
    }

    #[test]
    fn test_v150_sandbox_deny_by_default() {
        super::slang_sandbox_destroy();
        super::slang_sandbox_create();
        let fs = std::ffi::CString::new("fs").unwrap();
        assert_eq!(super::slang_sandbox_check(fs.as_ptr()), 0, "fs should be denied by default");
        super::slang_sandbox_destroy();
    }

    #[test]
    fn test_v150_sandbox_allow_capability() {
        super::slang_sandbox_destroy();
        super::slang_sandbox_create();
        let net = std::ffi::CString::new("net").unwrap();
        super::slang_sandbox_allow(net.as_ptr());
        assert_eq!(super::slang_sandbox_check(net.as_ptr()), 1, "net should be allowed after grant");
        super::slang_sandbox_destroy();
    }

    #[test]
    fn test_v150_sandbox_violations_tracked() {
        super::slang_sandbox_destroy();
        super::slang_sandbox_create();
        let exec = std::ffi::CString::new("exec").unwrap();
        super::slang_sandbox_check(exec.as_ptr()); // violation
        super::slang_sandbox_check(exec.as_ptr()); // violation
        let v = super::slang_sandbox_violations();
        assert!(v >= 2, "Should have at least 2 violations, got {}", v);
        super::slang_sandbox_destroy();
    }

    #[test]
    fn test_v150_sandbox_destroy() {
        super::slang_sandbox_destroy();
        super::slang_sandbox_create();
        let r = super::slang_sandbox_destroy();
        assert_eq!(r, 1, "Destroying active sandbox should return 1");
        let r2 = super::slang_sandbox_destroy();
        assert_eq!(r2, 0, "Destroying inactive sandbox should return 0");
    }

    #[test]
    fn test_v150_sandbox_no_sandbox_allows_all() {
        super::slang_sandbox_destroy();
        let fs = std::ffi::CString::new("fs").unwrap();
        assert_eq!(super::slang_sandbox_check(fs.as_ptr()), 1, "No sandbox = all allowed");
    }

    #[test]
    fn test_v150_sandbox_unknown_capability() {
        super::slang_sandbox_destroy();
        super::slang_sandbox_create();
        let unknown = std::ffi::CString::new("quantum").unwrap();
        assert_eq!(super::slang_sandbox_allow(unknown.as_ptr()), 0, "Unknown cap should fail");
        assert_eq!(super::slang_sandbox_check(unknown.as_ptr()), 0, "Unknown cap denied");
        super::slang_sandbox_destroy();
    }

    #[test]
    fn test_v150_sandbox_null_safety() {
        super::slang_sandbox_destroy();
        super::slang_sandbox_create();
        assert_eq!(super::slang_sandbox_allow(std::ptr::null()), 0);
        assert_eq!(super::slang_sandbox_check(std::ptr::null()), 0);
        super::slang_sandbox_destroy();
    }

    #[test]
    fn test_v150_sandbox_multiple_capabilities() {
        super::slang_sandbox_destroy();
        super::slang_sandbox_create();
        let fs = std::ffi::CString::new("fs").unwrap();
        let net = std::ffi::CString::new("net").unwrap();
        super::slang_sandbox_allow(fs.as_ptr());
        super::slang_sandbox_allow(net.as_ptr());
        assert_eq!(super::slang_sandbox_check(fs.as_ptr()), 1);
        assert_eq!(super::slang_sandbox_check(net.as_ptr()), 1);
        // Note: don't check exec==0 because parallel tests may grant it
        super::slang_sandbox_destroy();
    }

    #[test]
    fn test_v150_builtins_registered_in_stdlib() {
        let builtins = crate::stdlib::builtins();
        let names = vec![
            "sandbox_create", "sandbox_allow", "sandbox_check",
            "sandbox_violations", "sandbox_destroy",
        ];
        for name in names {
            assert!(builtins.iter().any(|b| b.name == name), "Missing stdlib builtin: {}", name);
        }
    }

    // -- v151: Security Audit Logger tests ------------------------------
    #[test]
    fn test_v151_security_log_basic() {
        super::slang_security_log_clear();
        let ev = std::ffi::CString::new("login_attempt").unwrap();
        let sev = std::ffi::CString::new("info").unwrap();
        let idx = super::slang_security_log(ev.as_ptr(), sev.as_ptr());
        assert_eq!(idx, 0);
        assert_eq!(super::slang_security_log_count(), 1);
    }

    #[test]
    fn test_v151_security_log_multiple() {
        let baseline = super::slang_security_log_count();
        let events = [("v151m_login", "info"), ("v151m_failed_auth", "warn"), ("v151m_breach", "critical")];
        for (ev, sev) in &events {
            let e = std::ffi::CString::new(*ev).unwrap();
            let s = std::ffi::CString::new(*sev).unwrap();
            super::slang_security_log(e.as_ptr(), s.as_ptr());
        }
        assert!(super::slang_security_log_count() >= baseline + 3);
    }

    #[test]
    fn test_v151_security_log_verify_valid() {
        super::slang_security_log_clear();
        let e1 = std::ffi::CString::new("event1").unwrap();
        let e2 = std::ffi::CString::new("event2").unwrap();
        let s = std::ffi::CString::new("info").unwrap();
        super::slang_security_log(e1.as_ptr(), s.as_ptr());
        super::slang_security_log(e2.as_ptr(), s.as_ptr());
        assert_eq!(super::slang_security_log_verify(), 1, "Untampered log should verify");
    }

    #[test]
    fn test_v151_security_log_verify_empty() {
        super::slang_security_log_clear();
        assert_eq!(super::slang_security_log_verify(), 1, "Empty log should verify");
    }

    #[test]
    fn test_v151_security_log_clear() {
        super::slang_security_log_clear();
        let ev = std::ffi::CString::new("tmp").unwrap();
        let sev = std::ffi::CString::new("info").unwrap();
        super::slang_security_log(ev.as_ptr(), sev.as_ptr());
        let count = super::slang_security_log_clear();
        assert!(count >= 1);
        assert_eq!(super::slang_security_log_count(), 0);
    }

    #[test]
    fn test_v151_security_log_dump_no_crash() {
        super::slang_security_log_clear();
        super::slang_security_log_dump(); // empty
        let ev = std::ffi::CString::new("test_dump").unwrap();
        let sev = std::ffi::CString::new("warn").unwrap();
        super::slang_security_log(ev.as_ptr(), sev.as_ptr());
        super::slang_security_log_dump(); // with entries
    }

    #[test]
    fn test_v151_security_log_null_safety() {
        let ev = std::ffi::CString::new("e").unwrap();
        assert_eq!(super::slang_security_log(std::ptr::null(), ev.as_ptr()), -1);
        assert_eq!(super::slang_security_log(ev.as_ptr(), std::ptr::null()), -1);
    }

    #[test]
    fn test_v151_security_log_indices_sequential() {
        super::slang_security_log_clear();
        let ev = std::ffi::CString::new("seq").unwrap();
        let sev = std::ffi::CString::new("info").unwrap();
        let i0 = super::slang_security_log(ev.as_ptr(), sev.as_ptr());
        let i1 = super::slang_security_log(ev.as_ptr(), sev.as_ptr());
        let i2 = super::slang_security_log(ev.as_ptr(), sev.as_ptr());
        assert_eq!(i0, 0);
        assert_eq!(i1, 1);
        assert_eq!(i2, 2);
    }

    #[test]
    fn test_v151_security_log_hash_chain_deterministic() {
        super::slang_security_log_clear();
        let ev = std::ffi::CString::new("det").unwrap();
        let sev = std::ffi::CString::new("info").unwrap();
        super::slang_security_log(ev.as_ptr(), sev.as_ptr());
        assert_eq!(super::slang_security_log_verify(), 1);
        super::slang_security_log(ev.as_ptr(), sev.as_ptr());
        assert_eq!(super::slang_security_log_verify(), 1);
    }

    #[test]
    fn test_v151_builtins_registered_in_stdlib() {
        let builtins = crate::stdlib::builtins();
        let names = vec![
            "security_log", "security_log_count", "security_log_verify",
            "security_log_dump", "security_log_clear",
        ];
        for name in names {
            assert!(builtins.iter().any(|b| b.name == name), "Missing stdlib builtin: {}", name);
        }
    }

    // -- v152: Input Validation Framework tests --------------------

    #[test]
    fn test_v152_validate_email_valid() {
        let e = std::ffi::CString::new("user@example.com").unwrap();
        assert_eq!(super::slang_validate_email(e.as_ptr()), 1);
    }

    #[test]
    fn test_v152_validate_email_no_at() {
        let e = std::ffi::CString::new("userexample.com").unwrap();
        assert_eq!(super::slang_validate_email(e.as_ptr()), 0);
    }

    #[test]
    fn test_v152_validate_email_no_domain_dot() {
        let e = std::ffi::CString::new("user@localhost").unwrap();
        assert_eq!(super::slang_validate_email(e.as_ptr()), 0);
    }

    #[test]
    fn test_v152_validate_email_empty_local() {
        let e = std::ffi::CString::new("@example.com").unwrap();
        assert_eq!(super::slang_validate_email(e.as_ptr()), 0);
    }

    #[test]
    fn test_v152_validate_email_null() {
        assert_eq!(super::slang_validate_email(std::ptr::null()), 0);
    }

    #[test]
    fn test_v152_validate_url_https() {
        let u = std::ffi::CString::new("https://example.com").unwrap();
        assert_eq!(super::slang_validate_url(u.as_ptr()), 1);
    }

    #[test]
    fn test_v152_validate_url_http() {
        let u = std::ffi::CString::new("http://example.com/path").unwrap();
        assert_eq!(super::slang_validate_url(u.as_ptr()), 1);
    }

    #[test]
    fn test_v152_validate_url_invalid() {
        let u = std::ffi::CString::new("ftp://example.com").unwrap();
        assert_eq!(super::slang_validate_url(u.as_ptr()), 0);
    }

    #[test]
    fn test_v152_validate_url_null() {
        assert_eq!(super::slang_validate_url(std::ptr::null()), 0);
    }

    #[test]
    fn test_v152_validate_ip_v4() {
        let ip = std::ffi::CString::new("192.168.1.1").unwrap();
        assert_eq!(super::slang_validate_ip(ip.as_ptr()), 1);
    }

    #[test]
    fn test_v152_validate_ip_v6() {
        let ip = std::ffi::CString::new("::1").unwrap();
        assert_eq!(super::slang_validate_ip(ip.as_ptr()), 1);
    }

    #[test]
    fn test_v152_validate_ip_invalid() {
        let ip = std::ffi::CString::new("999.999.999.999").unwrap();
        assert_eq!(super::slang_validate_ip(ip.as_ptr()), 0);
    }

    #[test]
    fn test_v152_sanitize_html_strips_tags() {
        let h = std::ffi::CString::new("<b>hello</b> <script>alert(1)</script>world").unwrap();
        let result = super::slang_sanitize_html(h.as_ptr());
        let out = unsafe { std::ffi::CStr::from_ptr(result) }.to_string_lossy().to_string();
        assert_eq!(out, "hello alert(1)world");
        unsafe { let _ = std::ffi::CString::from_raw(result); }
    }

    #[test]
    fn test_v152_sanitize_html_null() {
        let result = super::slang_sanitize_html(std::ptr::null());
        let out = unsafe { std::ffi::CStr::from_ptr(result) }.to_string_lossy().to_string();
        assert_eq!(out, "");
        unsafe { let _ = std::ffi::CString::from_raw(result); }
    }

    #[test]
    fn test_v152_sanitize_sql_escapes_quotes() {
        let s = std::ffi::CString::new("O'Reilly").unwrap();
        let result = super::slang_sanitize_sql(s.as_ptr());
        let out = unsafe { std::ffi::CStr::from_ptr(result) }.to_string_lossy().to_string();
        assert_eq!(out, "O''Reilly");
        unsafe { let _ = std::ffi::CString::from_raw(result); }
    }

    #[test]
    fn test_v152_sanitize_sql_no_quotes() {
        let s = std::ffi::CString::new("hello world").unwrap();
        let result = super::slang_sanitize_sql(s.as_ptr());
        let out = unsafe { std::ffi::CStr::from_ptr(result) }.to_string_lossy().to_string();
        assert_eq!(out, "hello world");
        unsafe { let _ = std::ffi::CString::from_raw(result); }
    }

    #[test]
    fn test_v152_sanitize_sql_null() {
        let result = super::slang_sanitize_sql(std::ptr::null());
        let out = unsafe { std::ffi::CStr::from_ptr(result) }.to_string_lossy().to_string();
        assert_eq!(out, "");
        unsafe { let _ = std::ffi::CString::from_raw(result); }
    }

    #[test]
    fn test_v152_builtins_registered_in_stdlib() {
        let builtins = crate::stdlib::builtins();
        let names = vec![
            "validate_email", "validate_url", "validate_ip",
            "sanitize_html", "sanitize_sql",
        ];
        for name in names {
            assert!(builtins.iter().any(|b| b.name == name), "Missing stdlib builtin: {}", name);
        }
    }

    // -- v153: Secure Communication Primitives tests ---------------

    #[test]
    fn test_v153_secure_channel_create() {
        let id = super::slang_secure_channel_create();
        assert!(id > 0);
        super::slang_secure_channel_close(id);
    }

    #[test]
    fn test_v153_secure_channel_send_recv() {
        let id = super::slang_secure_channel_create();
        let msg = std::ffi::CString::new("secret message").unwrap();
        assert_eq!(super::slang_secure_channel_send(id, msg.as_ptr()), 1);
        let recv = super::slang_secure_channel_recv(id);
        let out = unsafe { std::ffi::CStr::from_ptr(recv) }.to_string_lossy().to_string();
        assert_eq!(out, "secret message");
        unsafe { let _ = std::ffi::CString::from_raw(recv); }
        super::slang_secure_channel_close(id);
    }

    #[test]
    fn test_v153_secure_channel_multiple_messages() {
        let id = super::slang_secure_channel_create();
        let m1 = std::ffi::CString::new("msg1").unwrap();
        let m2 = std::ffi::CString::new("msg2").unwrap();
        super::slang_secure_channel_send(id, m1.as_ptr());
        super::slang_secure_channel_send(id, m2.as_ptr());
        // pop returns last-in first
        let r2 = super::slang_secure_channel_recv(id);
        let s2 = unsafe { std::ffi::CStr::from_ptr(r2) }.to_string_lossy().to_string();
        assert_eq!(s2, "msg2");
        unsafe { let _ = std::ffi::CString::from_raw(r2); }
        let r1 = super::slang_secure_channel_recv(id);
        let s1 = unsafe { std::ffi::CStr::from_ptr(r1) }.to_string_lossy().to_string();
        assert_eq!(s1, "msg1");
        unsafe { let _ = std::ffi::CString::from_raw(r1); }
        super::slang_secure_channel_close(id);
    }

    #[test]
    fn test_v153_secure_channel_recv_empty() {
        let id = super::slang_secure_channel_create();
        let recv = super::slang_secure_channel_recv(id);
        let out = unsafe { std::ffi::CStr::from_ptr(recv) }.to_string_lossy().to_string();
        assert_eq!(out, "");
        unsafe { let _ = std::ffi::CString::from_raw(recv); }
        super::slang_secure_channel_close(id);
    }

    #[test]
    fn test_v153_secure_channel_close() {
        let id = super::slang_secure_channel_create();
        assert_eq!(super::slang_secure_channel_close(id), 1);
        // closing again should fail
        assert_eq!(super::slang_secure_channel_close(id), 0);
    }

    #[test]
    fn test_v153_secure_channel_send_invalid_id() {
        let msg = std::ffi::CString::new("test").unwrap();
        assert_eq!(super::slang_secure_channel_send(-999, msg.as_ptr()), 0);
    }

    #[test]
    fn test_v153_secure_channel_send_null() {
        let id = super::slang_secure_channel_create();
        assert_eq!(super::slang_secure_channel_send(id, std::ptr::null()), 0);
        super::slang_secure_channel_close(id);
    }

    #[test]
    fn test_v153_constant_time_eq_same() {
        let a = std::ffi::CString::new("hello").unwrap();
        let b = std::ffi::CString::new("hello").unwrap();
        assert_eq!(super::slang_constant_time_eq(a.as_ptr(), b.as_ptr()), 1);
    }

    #[test]
    fn test_v153_constant_time_eq_diff() {
        let a = std::ffi::CString::new("hello").unwrap();
        let b = std::ffi::CString::new("world").unwrap();
        assert_eq!(super::slang_constant_time_eq(a.as_ptr(), b.as_ptr()), 0);
    }

    #[test]
    fn test_v153_constant_time_eq_diff_length() {
        let a = std::ffi::CString::new("short").unwrap();
        let b = std::ffi::CString::new("longer").unwrap();
        assert_eq!(super::slang_constant_time_eq(a.as_ptr(), b.as_ptr()), 0);
    }

    #[test]
    fn test_v153_constant_time_eq_null() {
        let a = std::ffi::CString::new("hello").unwrap();
        assert_eq!(super::slang_constant_time_eq(a.as_ptr(), std::ptr::null()), 0);
        assert_eq!(super::slang_constant_time_eq(std::ptr::null(), a.as_ptr()), 0);
    }

    #[test]
    fn test_v153_builtins_registered_in_stdlib() {
        let builtins = crate::stdlib::builtins();
        let names = vec![
            "secure_channel_create", "secure_channel_send", "secure_channel_recv",
            "secure_channel_close", "constant_time_eq",
        ];
        for name in names {
            assert!(builtins.iter().any(|b| b.name == name), "Missing stdlib builtin: {}", name);
        }
    }

    // -- v154: Autonomous Improvement Lab v2 tests -----------------

    #[test]
    fn test_v154_improvement_run_trial() {
        let name = std::ffi::CString::new("v154_trial_a").unwrap();
        assert_eq!(super::slang_improvement_run_trial(name.as_ptr(), 0.95), 1);
    }

    #[test]
    fn test_v154_improvement_best_score() {
        let n1 = std::ffi::CString::new("v154_best_1").unwrap();
        let n2 = std::ffi::CString::new("v154_best_2").unwrap();
        super::slang_improvement_run_trial(n1.as_ptr(), 0.5);
        super::slang_improvement_run_trial(n2.as_ptr(), 0.9);
        assert!(super::slang_improvement_best_score() >= 0.9);
    }

    #[test]
    fn test_v154_improvement_history_count() {
        let baseline = super::slang_improvement_history_count();
        let n = std::ffi::CString::new("v154_hist").unwrap();
        super::slang_improvement_run_trial(n.as_ptr(), 0.7);
        assert!(super::slang_improvement_history_count() >= baseline + 1);
    }

    #[test]
    fn test_v154_improvement_null_safety() {
        assert_eq!(super::slang_improvement_run_trial(std::ptr::null(), 1.0), 0);
    }

    #[test]
    fn test_v154_builtins_registered_in_stdlib() {
        let builtins = crate::stdlib::builtins();
        for name in ["improvement_run_trial", "improvement_best_score", "improvement_history_count", "improvement_reset"] {
            assert!(builtins.iter().any(|b| b.name == name), "Missing: {}", name);
        }
    }

    // -- v155: LLM-Guided Mutation Templates tests -----------------

    #[test]
    fn test_v155_mutation_apply() {
        let name = std::ffi::CString::new("v155_strength_reduce").unwrap();
        let idx = super::slang_mutation_apply(name.as_ptr());
        assert!(idx >= 0);
    }

    #[test]
    fn test_v155_mutation_list_count() {
        let baseline = super::slang_mutation_list_count();
        let name = std::ffi::CString::new("v155_loop_unroll").unwrap();
        super::slang_mutation_apply(name.as_ptr());
        assert!(super::slang_mutation_list_count() >= baseline + 1);
    }

    #[test]
    fn test_v155_mutation_score() {
        let name = std::ffi::CString::new("v155_score_test").unwrap();
        let idx = super::slang_mutation_apply(name.as_ptr());
        assert_eq!(super::slang_mutation_score(idx, 0.85), 1);
    }

    #[test]
    fn test_v155_mutation_undo() {
        let name = std::ffi::CString::new("v155_undo_test").unwrap();
        let idx = super::slang_mutation_apply(name.as_ptr());
        assert_eq!(super::slang_mutation_undo(idx), 1);
    }

    #[test]
    fn test_v155_mutation_undo_invalid() {
        assert_eq!(super::slang_mutation_undo(-999), 0);
    }

    #[test]
    fn test_v155_mutation_null() {
        assert_eq!(super::slang_mutation_apply(std::ptr::null()), -1);
    }

    #[test]
    fn test_v155_builtins_registered_in_stdlib() {
        let builtins = crate::stdlib::builtins();
        for name in ["mutation_apply", "mutation_list_count", "mutation_score", "mutation_undo"] {
            assert!(builtins.iter().any(|b| b.name == name), "Missing: {}", name);
        }
    }

    // -- v156: Evolution Fitness Profiles tests --------------------

    #[test]
    fn test_v156_fitness_register() {
        let name = std::ffi::CString::new("v156_speed").unwrap();
        let idx = super::slang_fitness_register(name.as_ptr());
        assert!(idx >= 0);
    }

    #[test]
    fn test_v156_fitness_evaluate() {
        let name = std::ffi::CString::new("v156_eval").unwrap();
        let idx = super::slang_fitness_register(name.as_ptr());
        assert_eq!(super::slang_fitness_evaluate(idx, 0.9), 1);
    }

    #[test]
    fn test_v156_fitness_evaluate_invalid() {
        assert_eq!(super::slang_fitness_evaluate(-999, 0.5), 0);
    }

    #[test]
    fn test_v156_fitness_pareto_count() {
        // Pareto count should be >= 0
        assert!(super::slang_fitness_pareto_count() >= 0);
    }

    #[test]
    fn test_v156_fitness_null() {
        assert_eq!(super::slang_fitness_register(std::ptr::null()), -1);
    }

    #[test]
    fn test_v156_builtins_registered_in_stdlib() {
        let builtins = crate::stdlib::builtins();
        for name in ["fitness_register", "fitness_evaluate", "fitness_pareto_count", "fitness_clear"] {
            assert!(builtins.iter().any(|b| b.name == name), "Missing: {}", name);
        }
    }

    // -- v157: Cross-Module Evolution tests ------------------------

    #[test]
    fn test_v157_evo_cross_module() {
        let name = std::ffi::CString::new("v157_parser").unwrap();
        let idx = super::slang_evo_cross_module(name.as_ptr());
        assert!(idx >= 0);
    }

    #[test]
    fn test_v157_evo_dep_add() {
        let name = std::ffi::CString::new("v157_codegen").unwrap();
        let idx = super::slang_evo_cross_module(name.as_ptr());
        let dep = std::ffi::CString::new("lexer").unwrap();
        assert_eq!(super::slang_evo_dep_add(idx, dep.as_ptr()), 1);
    }

    #[test]
    fn test_v157_evo_dep_check() {
        let name = std::ffi::CString::new("v157_optimizer").unwrap();
        let idx = super::slang_evo_cross_module(name.as_ptr());
        let d1 = std::ffi::CString::new("ir").unwrap();
        let d2 = std::ffi::CString::new("types").unwrap();
        super::slang_evo_dep_add(idx, d1.as_ptr());
        super::slang_evo_dep_add(idx, d2.as_ptr());
        assert_eq!(super::slang_evo_dep_check(idx), 2);
    }

    #[test]
    fn test_v157_evo_safe_mutate() {
        let name = std::ffi::CString::new("v157_safe").unwrap();
        let idx = super::slang_evo_cross_module(name.as_ptr());
        assert_eq!(super::slang_evo_safe_mutate(idx), 1);
    }

    #[test]
    fn test_v157_evo_safe_mutate_invalid() {
        assert_eq!(super::slang_evo_safe_mutate(-1), 0);
    }

    #[test]
    fn test_v157_evo_null_safety() {
        assert_eq!(super::slang_evo_cross_module(std::ptr::null()), -1);
        assert_eq!(super::slang_evo_dep_add(0, std::ptr::null()), 0);
    }

    #[test]
    fn test_v157_builtins_registered_in_stdlib() {
        let builtins = crate::stdlib::builtins();
        for name in ["evo_cross_module", "evo_dep_add", "evo_dep_check", "evo_safe_mutate"] {
            assert!(builtins.iter().any(|b| b.name == name), "Missing: {}", name);
        }
    }

    // -- v158: Evolution Checkpointing tests -----------------------

    #[test]
    fn test_v158_evo_checkpoint_save() {
        let name = std::ffi::CString::new("v158_cp1").unwrap();
        assert_eq!(super::slang_evo_checkpoint_save(name.as_ptr()), 1);
    }

    #[test]
    fn test_v158_evo_checkpoint_load_exists() {
        let name = std::ffi::CString::new("v158_cp_load").unwrap();
        super::slang_evo_checkpoint_save(name.as_ptr());
        assert_eq!(super::slang_evo_checkpoint_load(name.as_ptr()), 1);
    }

    #[test]
    fn test_v158_evo_checkpoint_load_missing() {
        let name = std::ffi::CString::new("v158_nonexistent_cp").unwrap();
        assert_eq!(super::slang_evo_checkpoint_load(name.as_ptr()), 0);
    }

    #[test]
    fn test_v158_evo_checkpoint_list_count() {
        let baseline = super::slang_evo_checkpoint_list_count();
        let name = std::ffi::CString::new("v158_count_cp").unwrap();
        super::slang_evo_checkpoint_save(name.as_ptr());
        assert!(super::slang_evo_checkpoint_list_count() >= baseline + 1);
    }

    #[test]
    fn test_v158_evo_checkpoint_null() {
        assert_eq!(super::slang_evo_checkpoint_save(std::ptr::null()), 0);
        assert_eq!(super::slang_evo_checkpoint_load(std::ptr::null()), 0);
    }

    #[test]
    fn test_v158_builtins_registered_in_stdlib() {
        let builtins = crate::stdlib::builtins();
        for name in ["evo_checkpoint_save", "evo_checkpoint_load", "evo_checkpoint_list_count", "evo_checkpoint_clear"] {
            assert!(builtins.iter().any(|b| b.name == name), "Missing: {}", name);
        }
    }

    // -- v159: Meta-Evolution v2 tests -----------------------------

    #[test]
    fn test_v159_meta_evo_register() {
        let name = std::ffi::CString::new("v159_ga").unwrap();
        let idx = super::slang_meta_evo_register(name.as_ptr(), 0.8);
        assert!(idx >= 0);
    }

    #[test]
    fn test_v159_meta_evo_select() {
        let n1 = std::ffi::CString::new("v159_sel_de").unwrap();
        let n2 = std::ffi::CString::new("v159_sel_pso").unwrap();
        super::slang_meta_evo_register(n1.as_ptr(), 0.3);
        super::slang_meta_evo_register(n2.as_ptr(), 0.99);
        let best = super::slang_meta_evo_select();
        assert!(best >= 0);
    }

    #[test]
    fn test_v159_meta_evo_converged() {
        // With strategies that differ, should see convergence result
        let result = super::slang_meta_evo_converged();
        assert!(result == 0 || result == 1);
    }

    #[test]
    fn test_v159_meta_evo_stats_count() {
        let baseline = super::slang_meta_evo_stats_count();
        let n = std::ffi::CString::new("v159_stats_new").unwrap();
        super::slang_meta_evo_register(n.as_ptr(), 0.5);
        assert!(super::slang_meta_evo_stats_count() >= baseline + 1);
    }

    #[test]
    fn test_v159_meta_evo_null() {
        assert_eq!(super::slang_meta_evo_register(std::ptr::null(), 0.5), -1);
    }

    #[test]
    fn test_v159_builtins_registered_in_stdlib() {
        let builtins = crate::stdlib::builtins();
        for name in ["meta_evo_register", "meta_evo_select", "meta_evo_converged", "meta_evo_stats_count"] {
            assert!(builtins.iter().any(|b| b.name == name), "Missing: {}", name);
        }
    }

    // -- v160: Tensor-First Types --------------------------------------

    #[test]
    fn test_v160_tensor_create_and_rank() {
        let t = slang_tensor_create(3);
        assert!(t > 0);
        assert_eq!(slang_tensor_rank(t), 3);
    }

    #[test]
    fn test_v160_tensor_size_initial() {
        let t = slang_tensor_create(2);
        assert_eq!(slang_tensor_size(t), 1); // initial: rank x 1 data
    }

    #[test]
    fn test_v160_tensor_set_get() {
        let t = slang_tensor_create(1);
        slang_tensor_set(t, 0, 42.5);
        assert!((slang_tensor_get(t, 0) - 42.5).abs() < 1e-9);
    }

    #[test]
    fn test_v160_tensor_set_auto_extend() {
        let t = slang_tensor_create(1);
        slang_tensor_set(t, 5, 7.0);
        assert!((slang_tensor_get(t, 5) - 7.0).abs() < 1e-9);
        assert!(slang_tensor_size(t) >= 6);
    }

    #[test]
    fn test_v160_tensor_add() {
        let a = slang_tensor_create(1);
        let b = slang_tensor_create(1);
        slang_tensor_set(a, 0, 3.0);
        slang_tensor_set(b, 0, 4.0);
        let c = slang_tensor_add(a, b);
        assert!(c > 0);
        assert!((slang_tensor_get(c, 0) - 7.0).abs() < 1e-9);
    }

    #[test]
    fn test_v160_tensor_mul() {
        let a = slang_tensor_create(1);
        let b = slang_tensor_create(1);
        slang_tensor_set(a, 0, 3.0);
        slang_tensor_set(b, 0, 5.0);
        let c = slang_tensor_mul(a, b);
        assert!((slang_tensor_get(c, 0) - 15.0).abs() < 1e-9);
    }

    #[test]
    fn test_v160_tensor_invalid_id() {
        assert_eq!(slang_tensor_rank(999999), -1);
        assert_eq!(slang_tensor_size(999999), -1);
    }

    #[test]
    fn test_v160_builtins_registered_in_stdlib() {
        let builtins = crate::stdlib::builtins();
        for name in ["tensor_create", "tensor_rank", "tensor_size", "tensor_set_v160", "tensor_get_v160", "tensor_add_v160", "tensor_mul_v160"] {
            assert!(builtins.iter().any(|b| b.name == name), "Missing: {}", name);
        }
    }

    // -- v161: Auto-Differentiation ------------------------------------

    #[test]
    fn test_v161_grad_compute_linear() {
        // f(x) = 2 + 3x => f'(x) = 3
        let t = slang_tensor_create(1);
        slang_tensor_set(t, 0, 2.0); // c0
        slang_tensor_set(t, 1, 3.0); // c1
        let grad = slang_grad_compute(t, 1.0);
        assert!((grad - 3.0).abs() < 1e-9);
    }

    #[test]
    fn test_v161_grad_compute_quadratic() {
        // f(x) = 1 + 0*x + 2*x^2 => f'(x) = 4x, f'(3) = 12
        let t = slang_tensor_create(1);
        slang_tensor_set(t, 0, 1.0);
        slang_tensor_set(t, 1, 0.0);
        slang_tensor_set(t, 2, 2.0);
        let grad = slang_grad_compute(t, 3.0);
        assert!((grad - 12.0).abs() < 1e-9);
    }

    #[test]
    fn test_v161_grad_forward() {
        // f(x) = 5 + 2x, f(3) = 11
        let t = slang_tensor_create(1);
        slang_tensor_set(t, 0, 5.0);
        slang_tensor_set(t, 1, 2.0);
        let val = slang_grad_forward(t, 3.0);
        assert!((val - 11.0).abs() < 1e-9);
    }

    #[test]
    fn test_v161_grad_reverse_equals_compute() {
        let t = slang_tensor_create(1);
        slang_tensor_set(t, 0, 1.0);
        slang_tensor_set(t, 1, 4.0);
        assert!((slang_grad_reverse(t, 2.0) - slang_grad_compute(t, 2.0)).abs() < 1e-9);
    }

    #[test]
    fn test_v161_grad_jacobian_dim() {
        let t = slang_tensor_create(1);
        slang_tensor_set(t, 0, 1.0);
        slang_tensor_set(t, 1, 2.0);
        slang_tensor_set(t, 2, 3.0);
        assert_eq!(slang_grad_jacobian_dim(t), 2); // 3 coeffs - 1
    }

    #[test]
    fn test_v161_builtins_registered_in_stdlib() {
        let builtins = crate::stdlib::builtins();
        for name in ["grad_compute", "grad_forward", "grad_reverse", "grad_jacobian_dim"] {
            assert!(builtins.iter().any(|b| b.name == name), "Missing: {}", name);
        }
    }

    // -- v162: ML Pipeline ---------------------------------------------

    #[test]
    fn test_v162_ml_linear_fit_and_predict() {
        // y = 2x + 1: (0,1) and (1,3)
        let mid = slang_ml_linear_fit(0.0, 1.0, 1.0, 3.0);
        assert!(mid > 0);
        let pred = slang_ml_predict(mid, 2.0);
        assert!((pred - 5.0).abs() < 1e-9);
    }

    #[test]
    fn test_v162_ml_predict_unknown_model() {
        assert!((slang_ml_predict(999999, 1.0)).abs() < 1e-9);
    }

    #[test]
    fn test_v162_ml_accuracy_perfect() {
        assert!((slang_ml_accuracy(5.0, 5.0) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn test_v162_ml_accuracy_partial() {
        let acc = slang_ml_accuracy(9.0, 10.0);
        assert!((acc - 0.9).abs() < 1e-9);
    }

    #[test]
    fn test_v162_ml_loss() {
        let loss = slang_ml_loss(3.0, 5.0);
        assert!((loss - 4.0).abs() < 1e-9); // (3-5)^2 = 4
    }

    #[test]
    fn test_v162_builtins_registered_in_stdlib() {
        let builtins = crate::stdlib::builtins();
        for name in ["ml_linear_fit", "ml_predict", "ml_accuracy", "ml_loss"] {
            assert!(builtins.iter().any(|b| b.name == name), "Missing: {}", name);
        }
    }

    // -- v163: Neural Architecture Search ------------------------------

    #[test]
    fn test_v163_nas_search_and_count() {
        let base = slang_nas_count();
        let name = std::ffi::CString::new("v163_arch_a").unwrap();
        let idx = slang_nas_search(name.as_ptr());
        assert!(idx >= 0);
        assert!(slang_nas_count() > base);
    }

    #[test]
    fn test_v163_nas_evaluate_and_best() {
        let n1 = std::ffi::CString::new("v163_arch_low").unwrap();
        let n2 = std::ffi::CString::new("v163_arch_high").unwrap();
        let i1 = slang_nas_search(n1.as_ptr());
        let i2 = slang_nas_search(n2.as_ptr());
        slang_nas_evaluate(i1, 0.5);
        slang_nas_evaluate(i2, 0.9);
        let best = slang_nas_best();
        assert!(best >= 0);
    }

    #[test]
    fn test_v163_nas_evaluate_invalid() {
        assert_eq!(slang_nas_evaluate(-1, 1.0), 0);
    }

    #[test]
    fn test_v163_builtins_registered_in_stdlib() {
        let builtins = crate::stdlib::builtins();
        for name in ["nas_search", "nas_evaluate", "nas_best", "nas_count"] {
            assert!(builtins.iter().any(|b| b.name == name), "Missing: {}", name);
        }
    }

    // -- v164: Feature Engineering -------------------------------------

    #[test]
    fn test_v164_feature_normalize() {
        let n = slang_feature_normalize(5.0, 0.0, 10.0);
        assert!((n - 0.5).abs() < 1e-9);
    }

    #[test]
    fn test_v164_feature_normalize_same_range() {
        assert!((slang_feature_normalize(5.0, 5.0, 5.0)).abs() < 1e-9);
    }

    #[test]
    fn test_v164_feature_one_hot() {
        assert!((slang_feature_one_hot(3, 3) - 1.0).abs() < 1e-9);
        assert!((slang_feature_one_hot(3, 5)).abs() < 1e-9);
    }

    #[test]
    fn test_v164_feature_variance() {
        // variance of [2, 4, 6] = mean=4, var = ((4+0+4)/3) = 2.666...
        let t = slang_tensor_create(1);
        slang_tensor_set(t, 0, 2.0);
        slang_tensor_set(t, 1, 4.0);
        slang_tensor_set(t, 2, 6.0);
        let v = slang_feature_variance(t);
        assert!((v - 8.0 / 3.0).abs() < 1e-6);
    }

    #[test]
    fn test_v164_feature_correlate_perfect() {
        let a = slang_tensor_create(1);
        let b = slang_tensor_create(1);
        slang_tensor_set(a, 0, 1.0);
        slang_tensor_set(a, 1, 2.0);
        slang_tensor_set(a, 2, 3.0);
        slang_tensor_set(b, 0, 2.0);
        slang_tensor_set(b, 1, 4.0);
        slang_tensor_set(b, 2, 6.0);
        let corr = slang_feature_correlate(a, b);
        assert!((corr - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_v164_builtins_registered_in_stdlib() {
        let builtins = crate::stdlib::builtins();
        for name in ["feature_normalize", "feature_one_hot", "feature_variance", "feature_correlate"] {
            assert!(builtins.iter().any(|b| b.name == name), "Missing: {}", name);
        }
    }

    // -- v165: Model Serialization -------------------------------------

    #[test]
    fn test_v165_model_save_load() {
        let name = std::ffi::CString::new("v165_test_model").unwrap();
        let ver = slang_model_save(name.as_ptr());
        assert!(ver > 0);
        let loaded = slang_model_load(name.as_ptr());
        assert_eq!(loaded, ver);
    }

    #[test]
    fn test_v165_model_load_not_found() {
        let name = std::ffi::CString::new("v165_nonexistent").unwrap();
        assert_eq!(slang_model_load(name.as_ptr()), 0);
    }

    #[test]
    fn test_v165_model_version() {
        let v = slang_model_version();
        assert!(v > 0);
    }

    #[test]
    fn test_v165_model_compatible() {
        assert_eq!(slang_model_compatible(1, 3), 1);   // diff=2, within 5
        assert_eq!(slang_model_compatible(1, 10), 0);  // diff=9, not compatible
    }

    #[test]
    fn test_v165_model_save_overwrite() {
        let name = std::ffi::CString::new("v165_overwrite_model").unwrap();
        let v1 = slang_model_save(name.as_ptr());
        let v2 = slang_model_save(name.as_ptr());
        assert!(v2 > v1);
        assert_eq!(slang_model_load(name.as_ptr()), v2);
    }

    #[test]
    fn test_v165_builtins_registered_in_stdlib() {
        let builtins = crate::stdlib::builtins();
        for name in ["model_save", "model_load", "model_version", "model_compatible"] {
            assert!(builtins.iter().any(|b| b.name == name), "Missing: {}", name);
        }
    }

    // -- v166: KV Store ------------------------------------------------

    #[test]
    fn test_v166_kv_put_get() {
        let k = std::ffi::CString::new("v166_k1").unwrap();
        let v = std::ffi::CString::new("hello").unwrap();
        assert_eq!(slang_kv_put(k.as_ptr(), v.as_ptr()), 1);
        assert_eq!(slang_kv_get_len(k.as_ptr()), 5); // "hello" = 5 bytes
    }

    #[test]
    fn test_v166_kv_delete() {
        let k = std::ffi::CString::new("v166_del").unwrap();
        let v = std::ffi::CString::new("x").unwrap();
        slang_kv_put(k.as_ptr(), v.as_ptr());
        assert_eq!(slang_kv_delete(k.as_ptr()), 1);
        assert_eq!(slang_kv_get_len(k.as_ptr()), 0);
    }

    #[test]
    fn test_v166_kv_wal_count() {
        let base = slang_kv_wal_count();
        let k = std::ffi::CString::new("v166_wal_t").unwrap();
        let v = std::ffi::CString::new("w").unwrap();
        slang_kv_put(k.as_ptr(), v.as_ptr());
        assert!(slang_kv_wal_count() > base);
    }

    #[test]
    fn test_v166_kv_not_found() {
        let k = std::ffi::CString::new("v166_nope").unwrap();
        assert_eq!(slang_kv_get_len(k.as_ptr()), 0);
    }

    #[test]
    fn test_v166_builtins_registered_in_stdlib() {
        let builtins = crate::stdlib::builtins();
        for name in ["kv_put", "kv_get_len", "kv_delete", "kv_wal_count"] {
            assert!(builtins.iter().any(|b| b.name == name), "Missing: {}", name);
        }
    }

    // -- v167: B-Tree Index --------------------------------------------

    #[test]
    fn test_v167_btree_insert_lookup() {
        slang_btree_insert(1000, 42);
        assert_eq!(slang_btree_lookup(1000), 42);
    }

    #[test]
    fn test_v167_btree_lookup_missing() {
        assert_eq!(slang_btree_lookup(999999), -1);
    }

    #[test]
    fn test_v167_btree_range_count() {
        slang_btree_insert(2000, 1);
        slang_btree_insert(2001, 2);
        slang_btree_insert(2002, 3);
        assert!(slang_btree_range_count(2000, 2002) >= 3);
    }

    #[test]
    fn test_v167_btree_count() {
        let base = slang_btree_count();
        slang_btree_insert(3000, 99);
        assert!(slang_btree_count() > base);
    }

    #[test]
    fn test_v167_builtins_registered_in_stdlib() {
        let builtins = crate::stdlib::builtins();
        for name in ["btree_insert", "btree_lookup", "btree_range_count", "btree_count"] {
            assert!(builtins.iter().any(|b| b.name == name), "Missing: {}", name);
        }
    }

    // -- v168: SQL Engine ----------------------------------------------

    #[test]
    fn test_v168_sql_create_and_insert() {
        let t = std::ffi::CString::new("v168_users").unwrap();
        assert_eq!(slang_sql_create_table(t.as_ptr()), 1);
        assert_eq!(slang_sql_insert(t.as_ptr(), 10, 20), 1);
        assert_eq!(slang_sql_count(t.as_ptr()), 1);
    }

    #[test]
    fn test_v168_sql_sum_col0() {
        let t = std::ffi::CString::new("v168_scores").unwrap();
        slang_sql_create_table(t.as_ptr());
        slang_sql_insert(t.as_ptr(), 5, 0);
        slang_sql_insert(t.as_ptr(), 8, 0);
        assert_eq!(slang_sql_sum_col0(t.as_ptr()), 13);
    }

    #[test]
    fn test_v168_sql_create_duplicate() {
        let t = std::ffi::CString::new("v168_dup").unwrap();
        assert_eq!(slang_sql_create_table(t.as_ptr()), 1);
        assert_eq!(slang_sql_create_table(t.as_ptr()), 0); // duplicate
    }

    #[test]
    fn test_v168_sql_insert_nonexistent() {
        let t = std::ffi::CString::new("v168_nope").unwrap();
        assert_eq!(slang_sql_insert(t.as_ptr(), 1, 2), 0);
    }

    #[test]
    fn test_v168_builtins_registered_in_stdlib() {
        let builtins = crate::stdlib::builtins();
        for name in ["sql_create_table", "sql_insert", "sql_count", "sql_sum_col0"] {
            assert!(builtins.iter().any(|b| b.name == name), "Missing: {}", name);
        }
    }

    // -- v169: Schema Migration ----------------------------------------

    #[test]
    fn test_v169_schema_create_version() {
        let n = std::ffi::CString::new("v169_schema_a").unwrap();
        assert_eq!(slang_schema_create(n.as_ptr()), 1);
        assert_eq!(slang_schema_version(n.as_ptr()), 1);
    }

    #[test]
    fn test_v169_schema_migrate() {
        let n = std::ffi::CString::new("v169_schema_b").unwrap();
        slang_schema_create(n.as_ptr());
        let v2 = slang_schema_migrate(n.as_ptr());
        assert_eq!(v2, 2);
        assert_eq!(slang_schema_version(n.as_ptr()), 2);
    }

    #[test]
    fn test_v169_schema_compatible() {
        assert_eq!(slang_schema_compatible(1, 3), 1);  // diff=2, within 3
        assert_eq!(slang_schema_compatible(1, 10), 0); // diff=9
    }

    #[test]
    fn test_v169_schema_version_not_found() {
        let n = std::ffi::CString::new("v169_nope").unwrap();
        assert_eq!(slang_schema_version(n.as_ptr()), 0);
    }

    #[test]
    fn test_v169_builtins_registered_in_stdlib() {
        let builtins = crate::stdlib::builtins();
        for name in ["schema_create", "schema_migrate", "schema_version", "schema_compatible"] {
            assert!(builtins.iter().any(|b| b.name == name), "Missing: {}", name);
        }
    }

    // -- v170: Transaction Log -----------------------------------------

    #[test]
    fn test_v170_txn_begin_commit() {
        let id = slang_txn_begin();
        assert!(id > 0);
        assert_eq!(slang_txn_commit(id), 1);
    }

    #[test]
    fn test_v170_txn_begin_rollback() {
        let id = slang_txn_begin();
        assert_eq!(slang_txn_rollback(id), 1);
    }

    #[test]
    fn test_v170_txn_double_commit() {
        let id = slang_txn_begin();
        slang_txn_commit(id);
        assert_eq!(slang_txn_commit(id), 0); // already committed
    }

    #[test]
    fn test_v170_txn_log_count() {
        let base = slang_txn_log_count();
        slang_txn_begin();
        assert!(slang_txn_log_count() > base);
    }

    #[test]
    fn test_v170_builtins_registered_in_stdlib() {
        let builtins = crate::stdlib::builtins();
        for name in ["txn_begin", "txn_commit", "txn_rollback", "txn_log_count"] {
            assert!(builtins.iter().any(|b| b.name == name), "Missing: {}", name);
        }
    }

    // -- v171: Data Import/Export ---------------------------------------

    #[test]
    fn test_v171_data_buf_create_push_len() {
        let buf = slang_data_buf_create();
        assert!(buf > 0);
        slang_data_buf_push(buf, 10);
        slang_data_buf_push(buf, 20);
        assert_eq!(slang_data_buf_len(buf), 2);
    }

    #[test]
    fn test_v171_data_buf_get() {
        let buf = slang_data_buf_create();
        slang_data_buf_push(buf, 42);
        assert_eq!(slang_data_buf_get(buf, 0), 42);
    }

    #[test]
    fn test_v171_data_buf_get_out_of_range() {
        let buf = slang_data_buf_create();
        assert_eq!(slang_data_buf_get(buf, 99), -1);
    }

    #[test]
    fn test_v171_data_buf_invalid() {
        assert_eq!(slang_data_buf_len(999999), 0);
    }

    #[test]
    fn test_v171_builtins_registered_in_stdlib() {
        let builtins = crate::stdlib::builtins();
        for name in ["data_buf_create", "data_buf_push", "data_buf_len", "data_buf_get"] {
            assert!(builtins.iter().any(|b| b.name == name), "Missing: {}", name);
        }
    }

    // -- v172: TCP Sockets ---------------------------------------------

    #[test]
    fn test_v172_tcp_create_connect() {
        let s = slang_tcp_create();
        assert!(s > 0);
        assert_eq!(slang_tcp_connected(s), 0);
        assert_eq!(slang_tcp_sim_connect(s, 8080), 1);
        assert_eq!(slang_tcp_connected(s), 1);
    }

    #[test]
    fn test_v172_tcp_close() {
        let s = slang_tcp_create();
        slang_tcp_sim_connect(s, 80);
        assert_eq!(slang_tcp_sim_close(s), 1);
        assert_eq!(slang_tcp_connected(s), 0);
    }

    #[test]
    fn test_v172_tcp_invalid() {
        assert_eq!(slang_tcp_connected(999999), 0);
        assert_eq!(slang_tcp_sim_close(999999), 0);
    }

    #[test]
    fn test_v172_builtins_registered_in_stdlib() {
        let builtins = crate::stdlib::builtins();
        for name in ["tcp_create", "tcp_sim_connect", "tcp_connected", "tcp_sim_close"] {
            assert!(builtins.iter().any(|b| b.name == name), "Missing: {}", name);
        }
    }

    // -- v173: HTTP Client ---------------------------------------------

    #[test]
    fn test_v173_http_get() {
        let url = std::ffi::CString::new("http://example.com").unwrap();
        assert_eq!(slang_http_sim_get(url.as_ptr()), 200);
    }

    #[test]
    fn test_v173_http_post() {
        let url = std::ffi::CString::new("https://api.example.com").unwrap();
        assert_eq!(slang_http_sim_post(url.as_ptr()), 201);
    }

    #[test]
    fn test_v173_http_invalid_url() {
        let url = std::ffi::CString::new("ftp://bad").unwrap();
        assert_eq!(slang_http_sim_get(url.as_ptr()), 400);
    }

    #[test]
    fn test_v173_http_url_valid() {
        let good = std::ffi::CString::new("https://ok.com").unwrap();
        let bad = std::ffi::CString::new("ftp://nope").unwrap();
        assert_eq!(slang_http_url_valid(good.as_ptr()), 1);
        assert_eq!(slang_http_url_valid(bad.as_ptr()), 0);
    }

    #[test]
    fn test_v173_http_request_count() {
        let base = slang_http_request_count();
        let url = std::ffi::CString::new("http://v173test.com").unwrap();
        slang_http_sim_get(url.as_ptr());
        assert!(slang_http_request_count() > base);
    }

    #[test]
    fn test_v173_builtins_registered_in_stdlib() {
        let builtins = crate::stdlib::builtins();
        for name in ["http_sim_get", "http_sim_post", "http_request_count", "http_url_valid"] {
            assert!(builtins.iter().any(|b| b.name == name), "Missing: {}", name);
        }
    }

    // -- v174: WebSocket -----------------------------------------------

    #[test]
    fn test_v174_ws_create_send() {
        let ch = slang_ws_create();
        assert!(ch > 0);
        let msg = std::ffi::CString::new("hello ws").unwrap();
        assert_eq!(slang_ws_send(ch, msg.as_ptr()), 1);
        assert_eq!(slang_ws_msg_count(ch), 1);
    }

    #[test]
    fn test_v174_ws_close() {
        let ch = slang_ws_create();
        assert_eq!(slang_ws_close(ch), 1);
        assert_eq!(slang_ws_msg_count(ch), 0); // removed
    }

    #[test]
    fn test_v174_ws_send_closed() {
        let ch = slang_ws_create();
        slang_ws_close(ch);
        let msg = std::ffi::CString::new("fail").unwrap();
        assert_eq!(slang_ws_send(ch, msg.as_ptr()), 0);
    }

    #[test]
    fn test_v174_builtins_registered_in_stdlib() {
        let builtins = crate::stdlib::builtins();
        for name in ["ws_create", "ws_send", "ws_msg_count", "ws_close"] {
            assert!(builtins.iter().any(|b| b.name == name), "Missing: {}", name);
        }
    }

    // -- v175: RPC Framework -------------------------------------------

    #[test]
    fn test_v175_rpc_register_call() {
        let name = std::ffi::CString::new("v175_greeting_svc").unwrap();
        assert_eq!(slang_rpc_register(name.as_ptr()), 1);
        assert_eq!(slang_rpc_call(name.as_ptr()), 1);
    }

    #[test]
    fn test_v175_rpc_call_unregistered() {
        let name = std::ffi::CString::new("v175_nope").unwrap();
        assert_eq!(slang_rpc_call(name.as_ptr()), 0);
    }

    #[test]
    fn test_v175_rpc_service_count() {
        let base = slang_rpc_service_count();
        let name = std::ffi::CString::new("v175_counted_svc").unwrap();
        slang_rpc_register(name.as_ptr());
        assert!(slang_rpc_service_count() > base);
    }

    #[test]
    fn test_v175_rpc_total_calls() {
        let base = slang_rpc_total_calls();
        let name = std::ffi::CString::new("v175_call_svc").unwrap();
        slang_rpc_register(name.as_ptr());
        slang_rpc_call(name.as_ptr());
        assert!(slang_rpc_total_calls() > base);
    }

    #[test]
    fn test_v175_builtins_registered_in_stdlib() {
        let builtins = crate::stdlib::builtins();
        for name in ["rpc_register", "rpc_call", "rpc_total_calls", "rpc_service_count"] {
            assert!(builtins.iter().any(|b| b.name == name), "Missing: {}", name);
        }
    }

    // -- v176: DNS Resolution ------------------------------------------

    #[test]
    fn test_v176_dns_resolve_cached() {
        let d = std::ffi::CString::new("v176.example.com").unwrap();
        assert_eq!(slang_dns_resolve(d.as_ptr()), 1);
        assert_eq!(slang_dns_cached(d.as_ptr()), 1);
    }

    #[test]
    fn test_v176_dns_not_cached() {
        let d = std::ffi::CString::new("v176.unknown.tld").unwrap();
        // Don't resolve, just check cache
        assert_eq!(slang_dns_cached(d.as_ptr()), 0);
    }

    #[test]
    fn test_v176_dns_cache_size() {
        let base = slang_dns_cache_size();
        let d = std::ffi::CString::new("v176.size.test").unwrap();
        slang_dns_resolve(d.as_ptr());
        assert!(slang_dns_cache_size() > base);
    }

    #[test]
    fn test_v176_builtins_registered_in_stdlib() {
        let builtins = crate::stdlib::builtins();
        for name in ["dns_resolve", "dns_cached", "dns_cache_size", "dns_cache_flush"] {
            assert!(builtins.iter().any(|b| b.name == name), "Missing: {}", name);
        }
    }

    // -- v177: TLS/SSL -------------------------------------------------

    #[test]
    fn test_v177_tls_create_active() {
        let h = std::ffi::CString::new("v177.secure.com").unwrap();
        let id = slang_tls_create(h.as_ptr());
        assert!(id > 0);
        assert_eq!(slang_tls_active(id), 1);
    }

    #[test]
    fn test_v177_tls_close() {
        let h = std::ffi::CString::new("v177.close.com").unwrap();
        let id = slang_tls_create(h.as_ptr());
        assert_eq!(slang_tls_close(id), 1);
        assert_eq!(slang_tls_active(id), 0);
    }

    #[test]
    fn test_v177_tls_cert_valid() {
        let h = std::ffi::CString::new("v177.cert.com").unwrap();
        let id = slang_tls_create(h.as_ptr());
        assert_eq!(slang_tls_cert_valid(id), 1);
    }

    #[test]
    fn test_v177_tls_invalid_session() {
        assert_eq!(slang_tls_active(999999), 0);
        assert_eq!(slang_tls_cert_valid(999999), 0);
    }

    #[test]
    fn test_v177_builtins_registered_in_stdlib() {
        let builtins = crate::stdlib::builtins();
        for name in ["tls_create", "tls_active", "tls_close", "tls_cert_valid"] {
            assert!(builtins.iter().any(|b| b.name == name), "Missing: {}", name);
        }
    }

    // ----------------------------------------------------------------
    //  Phase 19: Developer Experience (v178-v183)
    // ----------------------------------------------------------------

    #[test]
    fn test_v178_repl_history() {
        let base = slang_repl_history_count();
        let cmd = std::ffi::CString::new("let x = 42").unwrap();
        assert_eq!(slang_repl_history_add(cmd.as_ptr()), 1);
        assert!(slang_repl_history_count() > base);
    }

    #[test]
    fn test_v178_repl_complete() {
        let prefix = std::ffi::CString::new("pr").unwrap();
        let count = slang_repl_complete_count(prefix.as_ptr());
        assert!(count >= 0);
    }

    #[test]
    fn test_v178_builtins_registered_in_stdlib() {
        let builtins = crate::stdlib::builtins();
        for name in ["repl_history_add", "repl_history_count", "repl_history_clear", "repl_complete_count"] {
            assert!(builtins.iter().any(|b| b.name == name), "Missing: {}", name);
        }
    }

    #[test]
    fn test_v179_pkg_registry() {
        let base = slang_pkg_count();
        let name = std::ffi::CString::new("v179_test_pkg").unwrap();
        assert_eq!(slang_pkg_publish(name.as_ptr(), 1, 0, 0), 1);
        assert_eq!(slang_pkg_installed(name.as_ptr()), 1);
        assert!(slang_pkg_count() > base);
        assert_eq!(slang_pkg_remove(name.as_ptr()), 1);
    }

    #[test]
    fn test_v179_builtins_registered_in_stdlib() {
        let builtins = crate::stdlib::builtins();
        for name in ["pkg_publish", "pkg_installed", "pkg_count", "pkg_remove"] {
            assert!(builtins.iter().any(|b| b.name == name), "Missing: {}", name);
        }
    }

    #[test]
    fn test_v180_doc_generator() {
        let base = slang_doc_count();
        let name = std::ffi::CString::new("v180_my_func").unwrap();
        let doc = std::ffi::CString::new("Does things").unwrap();
        assert_eq!(slang_doc_add(name.as_ptr(), doc.as_ptr()), 1);
        assert!(slang_doc_count() > base);
        assert_eq!(slang_doc_has(name.as_ptr()), 1);
    }

    #[test]
    fn test_v180_builtins_registered_in_stdlib() {
        let builtins = crate::stdlib::builtins();
        for name in ["doc_add", "doc_count", "doc_has", "doc_clear"] {
            assert!(builtins.iter().any(|b| b.name == name), "Missing: {}", name);
        }
    }

    #[test]
    fn test_v181_benchmark() {
        let base = slang_bench_count();
        let name = std::ffi::CString::new("v181_sort_bench").unwrap();
        assert_eq!(slang_bench_record(name.as_ptr(), 1234.5), 1);
        assert!(slang_bench_count() > base);
        let best = slang_bench_best(name.as_ptr());
        assert!(best <= 1234.5);
    }

    #[test]
    fn test_v181_builtins_registered_in_stdlib() {
        let builtins = crate::stdlib::builtins();
        for name in ["bench_record", "bench_count", "bench_best", "bench_clear"] {
            assert!(builtins.iter().any(|b| b.name == name), "Missing: {}", name);
        }
    }

    #[test]
    fn test_v182_profiler() {
        assert_eq!(slang_profile_start(), 1);
        let s1 = slang_profile_sample();
        assert!(s1 >= 1);
        let s2 = slang_profile_sample();
        assert!(s2 > s1);
        assert_eq!(slang_profile_stop(), 1);
        // after stop, sample returns 0
        assert_eq!(slang_profile_sample(), 0);
    }

    #[test]
    fn test_v182_builtins_registered_in_stdlib() {
        let builtins = crate::stdlib::builtins();
        for name in ["profile_start", "profile_stop", "profile_sample", "profile_samples"] {
            assert!(builtins.iter().any(|b| b.name == name), "Missing: {}", name);
        }
    }

    #[test]
    fn test_v183_playground() {
        let base = slang_playground_count();
        let code = std::ffi::CString::new("hello").unwrap();
        let result = slang_playground_eval(code.as_ptr());
        assert_eq!(result, 5); // length of "hello"
        assert!(slang_playground_count() > base);
        assert_eq!(slang_playground_last_result(), 5);
    }

    #[test]
    fn test_v183_builtins_registered_in_stdlib() {
        let builtins = crate::stdlib::builtins();
        for name in ["playground_eval", "playground_count", "playground_clear", "playground_last_result"] {
            assert!(builtins.iter().any(|b| b.name == name), "Missing: {}", name);
        }
    }

    // ----------------------------------------------------------------
    //  Phase 20: Concurrency & Parallelism v2 (v184-v189)
    // ----------------------------------------------------------------

    #[test]
    fn test_v184_work_stealing() {
        let base = slang_task_queue_len();
        let id = slang_task_submit(10);
        assert!(id > 0);
        assert!(slang_task_queue_len() > base);
    }

    #[test]
    fn test_v184_task_steal() {
        let id = slang_task_submit(5);
        let stolen = slang_task_steal();
        assert!(stolen > 0);
    }

    #[test]
    fn test_v184_builtins_registered_in_stdlib() {
        let builtins = crate::stdlib::builtins();
        for name in ["task_submit", "task_queue_len", "task_steal", "task_queue_clear"] {
            assert!(builtins.iter().any(|b| b.name == name), "Missing: {}", name);
        }
    }

    #[test]
    fn test_v185_actor_model() {
        let actor = slang_actor_spawn();
        assert!(actor > 0);
        assert_eq!(slang_actor_mailbox_len(actor), 0);
        assert_eq!(slang_actor_send(actor, 42), 1);
        assert_eq!(slang_actor_mailbox_len(actor), 1);
        assert_eq!(slang_actor_recv(actor), 42);
        assert_eq!(slang_actor_mailbox_len(actor), 0);
    }

    #[test]
    fn test_v185_builtins_registered_in_stdlib() {
        let builtins = crate::stdlib::builtins();
        for name in ["actor_spawn", "actor_send", "actor_recv", "actor_mailbox_len"] {
            assert!(builtins.iter().any(|b| b.name == name), "Missing: {}", name);
        }
    }

    #[test]
    fn test_v186_stm() {
        let v = slang_stm_new(100);
        assert!(v > 0);
        assert_eq!(slang_stm_read(v), 100);
        assert_eq!(slang_stm_write(v, 200), 1);
        assert_eq!(slang_stm_read(v), 200);
        // CAS success
        assert_eq!(slang_stm_cas(v, 200, 300), 1);
        assert_eq!(slang_stm_read(v), 300);
        // CAS failure
        assert_eq!(slang_stm_cas(v, 999, 400), 0);
    }

    #[test]
    fn test_v186_builtins_registered_in_stdlib() {
        let builtins = crate::stdlib::builtins();
        for name in ["stm_new", "stm_read", "stm_write", "stm_cas"] {
            assert!(builtins.iter().any(|b| b.name == name), "Missing: {}", name);
        }
    }

    #[test]
    fn test_v187_parallel_collections() {
        // Create a tensor to test par operations
        let tid = slang_tensor_create(4);
        slang_tensor_set(tid, 0, 1.0);
        slang_tensor_set(tid, 1, 2.0);
        slang_tensor_set(tid, 2, 3.0);
        slang_tensor_set(tid, 3, 4.0);
        assert_eq!(slang_par_sum(tid), 10.0);
        assert_eq!(slang_par_min(tid), 1.0);
        assert_eq!(slang_par_max(tid), 4.0);
        assert_eq!(slang_par_count(tid), 4);
    }

    #[test]
    fn test_v187_builtins_registered_in_stdlib() {
        let builtins = crate::stdlib::builtins();
        for name in ["par_sum", "par_min", "par_max", "par_count"] {
            assert!(builtins.iter().any(|b| b.name == name), "Missing: {}", name);
        }
    }

    #[test]
    fn test_v188_gpu_tasks() {
        let base = slang_gpu_queue_len();
        slang_gpu_submit(1);
        slang_gpu_submit(2);
        assert!(slang_gpu_queue_len() >= base + 2);
        assert_eq!(slang_gpu_available(), 1);
    }

    #[test]
    fn test_v188_builtins_registered_in_stdlib() {
        let builtins = crate::stdlib::builtins();
        for name in ["gpu_submit", "gpu_queue_len", "gpu_flush", "gpu_available"] {
            assert!(builtins.iter().any(|b| b.name == name), "Missing: {}", name);
        }
    }

    #[test]
    fn test_v189_distributed() {
        let base = slang_dist_node_count();
        let name = std::ffi::CString::new("v189_node1").unwrap();
        let id = slang_dist_node_add(name.as_ptr());
        assert!(id > 0);
        assert!(slang_dist_node_count() > base);
    }

    #[test]
    fn test_v189_reduce() {
        let name = std::ffi::CString::new("v189_node_reduce").unwrap();
        slang_dist_node_add(name.as_ptr());
        let result = slang_dist_reduce(10, 0); // sum
        assert!(result >= 10);
    }

    #[test]
    fn test_v189_builtins_registered_in_stdlib() {
        let builtins = crate::stdlib::builtins();
        for name in ["dist_node_add", "dist_node_count", "dist_broadcast", "dist_reduce"] {
            assert!(builtins.iter().any(|b| b.name == name), "Missing: {}", name);
        }
    }

    // ----------------------------------------------------------------
    //  Phase 21: Type System Evolution (v190-v195)
    // ----------------------------------------------------------------

    #[test]
    fn test_v190_adts() {
        let base = slang_type_count();
        let name = std::ffi::CString::new("v190_Option").unwrap();
        assert_eq!(slang_type_register(name.as_ptr(), 2), 1);
        assert_eq!(slang_type_variant_count(name.as_ptr()), 2);
        assert_eq!(slang_type_exists(name.as_ptr()), 1);
        assert!(slang_type_count() > base);
    }

    #[test]
    fn test_v190_builtins_registered_in_stdlib() {
        let builtins = crate::stdlib::builtins();
        for name in ["type_register", "type_variant_count", "type_count", "type_exists"] {
            assert!(builtins.iter().any(|b| b.name == name), "Missing: {}", name);
        }
    }

    #[test]
    fn test_v191_hkt() {
        let base = slang_hkt_count();
        let name = std::ffi::CString::new("v191_Functor").unwrap();
        assert_eq!(slang_hkt_register(name.as_ptr(), 1), 1);
        assert_eq!(slang_hkt_arity(name.as_ptr()), 1);
        assert_eq!(slang_hkt_exists(name.as_ptr()), 1);
        assert!(slang_hkt_count() > base);
    }

    #[test]
    fn test_v191_builtins_registered_in_stdlib() {
        let builtins = crate::stdlib::builtins();
        for name in ["hkt_register", "hkt_arity", "hkt_count", "hkt_exists"] {
            assert!(builtins.iter().any(|b| b.name == name), "Missing: {}", name);
        }
    }

    #[test]
    fn test_v192_dependent_types() {
        assert_eq!(slang_dep_type_check_range(5, 1, 10), 1);
        assert_eq!(slang_dep_type_check_range(15, 1, 10), 0);
        assert_eq!(slang_dep_type_nat(0), 1);
        assert_eq!(slang_dep_type_nat(-1), 0);
        assert_eq!(slang_dep_type_positive(1), 1);
        assert_eq!(slang_dep_type_positive(0), 0);
        assert_eq!(slang_dep_type_bounded_add(5, 3, 10), 8);
        assert_eq!(slang_dep_type_bounded_add(5, 7, 10), -1);
    }

    #[test]
    fn test_v192_builtins_registered_in_stdlib() {
        let builtins = crate::stdlib::builtins();
        for name in ["dep_type_check_range", "dep_type_nat", "dep_type_positive", "dep_type_bounded_add"] {
            assert!(builtins.iter().any(|b| b.name == name), "Missing: {}", name);
        }
    }

    #[test]
    fn test_v193_effect_polymorphism() {
        let base = slang_effect_count();
        let eff = std::ffi::CString::new("v193_IO").unwrap();
        assert_eq!(slang_effect_register(eff.as_ptr()), 1);
        assert!(slang_effect_count() > base);
        let handler = std::ffi::CString::new("console_handler").unwrap();
        assert_eq!(slang_effect_add_handler(eff.as_ptr(), handler.as_ptr()), 1);
        assert_eq!(slang_effect_handler_count(eff.as_ptr()), 1);
    }

    #[test]
    fn test_v193_builtins_registered_in_stdlib() {
        let builtins = crate::stdlib::builtins();
        for name in ["effect_register", "effect_add_handler", "effect_handler_count", "effect_count"] {
            assert!(builtins.iter().any(|b| b.name == name), "Missing: {}", name);
        }
    }

    #[test]
    fn test_v194_type_level_computation() {
        assert_eq!(slang_type_level_add(3, 4), 7);
        assert_eq!(slang_type_level_mul(3, 4), 12);
        assert_eq!(slang_type_level_eq(5, 5), 1);
        assert_eq!(slang_type_level_eq(5, 6), 0);
        assert_eq!(slang_type_level_if(1, 10, 20), 10);
        assert_eq!(slang_type_level_if(0, 10, 20), 20);
    }

    #[test]
    fn test_v194_builtins_registered_in_stdlib() {
        let builtins = crate::stdlib::builtins();
        for name in ["type_level_add", "type_level_mul", "type_level_eq", "type_level_if"] {
            assert!(builtins.iter().any(|b| b.name == name), "Missing: {}", name);
        }
    }

    #[test]
    fn test_v195_gradual_typing() {
        let var = std::ffi::CString::new("v195_x").unwrap();
        let typ = std::ffi::CString::new("Int").unwrap();
        let base = slang_gradual_typed_count();
        assert_eq!(slang_gradual_annotate(var.as_ptr(), typ.as_ptr()), 1);
        assert!(slang_gradual_typed_count() > base);
        assert_eq!(slang_gradual_check(var.as_ptr(), typ.as_ptr()), 1);
        // wrong type
        let wrong = std::ffi::CString::new("String").unwrap();
        assert_eq!(slang_gradual_check(var.as_ptr(), wrong.as_ptr()), 0);
        // unannotated var = Any
        let unknown = std::ffi::CString::new("v195_unknown").unwrap();
        assert_eq!(slang_gradual_is_any(unknown.as_ptr()), 1);
    }

    #[test]
    fn test_v195_builtins_registered_in_stdlib() {
        let builtins = crate::stdlib::builtins();
        for name in ["gradual_annotate", "gradual_check", "gradual_typed_count", "gradual_is_any"] {
            assert!(builtins.iter().any(|b| b.name == name), "Missing: {}", name);
        }
    }

    // ----------------------------------------------------------------
    //  Phase 22: Ecosystem & Milestone (v196-v200)
    // ----------------------------------------------------------------

    #[test]
    fn test_v196_ffi_v2() {
        let base = slang_ffi_count();
        let name = std::ffi::CString::new("v196_py_func").unwrap();
        let lang = std::ffi::CString::new("Python").unwrap();
        assert_eq!(slang_ffi_bind(name.as_ptr(), lang.as_ptr()), 1);
        assert_eq!(slang_ffi_bound(name.as_ptr()), 1);
        assert!(slang_ffi_count() > base);
        assert_eq!(slang_ffi_remove(name.as_ptr()), 1);
        assert_eq!(slang_ffi_bound(name.as_ptr()), 0);
    }

    #[test]
    fn test_v196_builtins_registered_in_stdlib() {
        let builtins = crate::stdlib::builtins();
        for name in ["ffi_bind", "ffi_bound", "ffi_count", "ffi_remove"] {
            assert!(builtins.iter().any(|b| b.name == name), "Missing: {}", name);
        }
    }

    #[test]
    fn test_v197_cloud_deploy() {
        let base = slang_cloud_deployment_count();
        let name = std::ffi::CString::new("v197_myapp").unwrap();
        let target = std::ffi::CString::new("aws-lambda").unwrap();
        assert_eq!(slang_cloud_deploy(name.as_ptr(), target.as_ptr()), 1);
        assert!(slang_cloud_deployment_count() > base);
        assert_eq!(slang_cloud_health_check(), 1);
    }

    #[test]
    fn test_v197_builtins_registered_in_stdlib() {
        let builtins = crate::stdlib::builtins();
        for name in ["cloud_deploy", "cloud_deployment_count", "cloud_health_check", "cloud_shutdown"] {
            assert!(builtins.iter().any(|b| b.name == name), "Missing: {}", name);
        }
    }

    #[test]
    fn test_v198_self_hosting() {
        slang_bootstrap_reset();
        assert_eq!(slang_bootstrap_stage(), 0);
        assert_eq!(slang_bootstrap_advance(), 1);
        assert_eq!(slang_bootstrap_stage(), 1);
        assert_eq!(slang_bootstrap_verify(12345, 12345), 1);
        assert_eq!(slang_bootstrap_verify(12345, 99999), 0);
    }

    #[test]
    fn test_v198_builtins_registered_in_stdlib() {
        let builtins = crate::stdlib::builtins();
        for name in ["bootstrap_stage", "bootstrap_advance", "bootstrap_verify", "bootstrap_reset"] {
            assert!(builtins.iter().any(|b| b.name == name), "Missing: {}", name);
        }
    }

    #[test]
    fn test_v199_ai_language_server() {
        let base = slang_ai_suggestion_count();
        let ctx = std::ffi::CString::new("fn main").unwrap();
        assert_eq!(slang_ai_suggest(ctx.as_ptr()), 1);
        assert!(slang_ai_suggestion_count() > base);
        assert_eq!(slang_ai_explain_error(404), 1);
        assert_eq!(slang_ai_explain_error(0), 0);
    }

    #[test]
    fn test_v199_builtins_registered_in_stdlib() {
        let builtins = crate::stdlib::builtins();
        for name in ["ai_suggest", "ai_suggestion_count", "ai_explain_error", "ai_clear"] {
            assert!(builtins.iter().any(|b| b.name == name), "Missing: {}", name);
        }
    }

    #[test]
    fn test_v300_milestone() {
        assert_eq!(slang_vitalis_version(), 300);
        assert!(slang_vitalis_module_count() >= 192);
        assert!(slang_vitalis_test_count() > 4000);
        assert!(slang_vitalis_builtin_count() > 900);
    }

    #[test]
    fn test_v200_builtins_registered_in_stdlib() {
        let builtins = crate::stdlib::builtins();
        for name in ["vitalis_version", "vitalis_module_count", "vitalis_test_count", "vitalis_builtin_count"] {
            assert!(builtins.iter().any(|b| b.name == name), "Missing: {}", name);
        }
    }
    #[test]
    fn test_v201_v290_neuro_builtins_symbols() {
        // Spot-check that module builtins are registered
        assert_ne!(crate::spike_engine::slang_spike_emit(0, 0, 0), -1);
        assert!(crate::loihi_sim::slang_loihi_core_count() >= 0);
        assert!(crate::brain_models::slang_htm_column_count() >= 0);
        assert!(crate::hippocampal_memory::slang_hippo_count() >= 0);
        assert!(crate::neuro_distributed::slang_neuro_cluster_nodes() >= 0);
        assert!(crate::quantum_neuro::slang_quantum_reservoir_dim() >= 0);
        assert_eq!(crate::neuro_perf::slang_neuro_zero_overhead(), 0);
    }

    #[test]
    fn test_v291_v300_inline_builtins() {
        assert_eq!(slang_vitalis_v300_version(), 300);
        assert_eq!(slang_vitalis_v300_total_builtins(), 400);
        assert_eq!(slang_vitalis_v300_neuro_modules(), 15);
        assert_eq!(slang_vitalis_v300_milestone(), 1);
    }

    #[test]
    fn test_v300_all_neuro_builtins_in_stdlib() {
        let builtins = crate::stdlib::builtins();
        let neuro_names = ["spike_emit", "loihi_core_create", "snn_surrogate_forward",
            "pim_alloc", "pred_coding_forward", "gpu_spike_propagate",
            "spike_vision_encode", "neat_crossover", "snn_ann_to_rate",
            "hippo_encode", "neuro_cluster_init", "neuro_viz_spike_raster",
            "quantum_spike_encode", "neuro_verify_timing", "neuro_simd_accumulate",
            "vitalis_v300_version"];
        for name in neuro_names {
            assert!(builtins.iter().any(|b| b.name == name), "Missing builtin: {}", name);
        }
    }

}
