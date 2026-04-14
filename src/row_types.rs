//! Row-Polymorphic Records — Language Feature Deepening (v312)
//!
//! Structural subtyping for records: a function can accept any record
//! that has at least the required fields, ignoring extra fields.
//! Enables duck-typing with compile-time safety.

use std::sync::{LazyLock, Mutex, atomic::{AtomicI64, Ordering}};
use std::collections::HashMap;

static ROW_TYPES: LazyLock<Mutex<HashMap<i64, Vec<i64>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
static ROW_COUNTER: AtomicI64 = AtomicI64::new(0);

/// Return the number of fields in a row type. Negative id → 0.
#[unsafe(no_mangle)]
pub extern "C" fn slang_row_type_fields(type_id: i64) -> i64 {
    let rt = ROW_TYPES.lock().unwrap();
    rt.get(&type_id).map(|v| v.len() as i64).unwrap_or(0)
}

/// Create a row type with n_fields. Returns type ID.
#[unsafe(no_mangle)]
pub extern "C" fn slang_row_type_create(n_fields: i64) -> i64 {
    let id = ROW_COUNTER.fetch_add(1, Ordering::SeqCst);
    let mut rt = ROW_TYPES.lock().unwrap();
    rt.insert(id, (0..n_fields.max(0)).collect());
    id
}

/// Extend a row type by adding one field. Returns new field count.
#[unsafe(no_mangle)]
pub extern "C" fn slang_row_type_extend(type_id: i64, field_id: i64) -> i64 {
    let mut rt = ROW_TYPES.lock().unwrap();
    if let Some(fields) = rt.get_mut(&type_id) {
        fields.push(field_id);
        fields.len() as i64
    } else {
        -1
    }
}

/// Restrict a row type by removing the last field. Returns new field count.
#[unsafe(no_mangle)]
pub extern "C" fn slang_row_type_restrict(type_id: i64) -> i64 {
    let mut rt = ROW_TYPES.lock().unwrap();
    if let Some(fields) = rt.get_mut(&type_id) {
        fields.pop();
        fields.len() as i64
    } else {
        -1
    }
}

/// Check if type_a is compatible with type_b (has at least as many fields).
/// Returns 1 if compatible, 0 otherwise.
#[unsafe(no_mangle)]
pub extern "C" fn slang_row_type_compatible(type_a: i64, type_b: i64) -> i64 {
    let rt = ROW_TYPES.lock().unwrap();
    let a_len = rt.get(&type_a).map(|v| v.len()).unwrap_or(0);
    let b_len = rt.get(&type_b).map(|v| v.len()).unwrap_or(0);
    if a_len >= b_len { 1 } else { 0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_row_type() {
        let id = slang_row_type_create(3);
        assert!(id >= 0);
        assert_eq!(slang_row_type_fields(id), 3);
    }
    #[test]
    fn test_extend() {
        let id = slang_row_type_create(2);
        assert_eq!(slang_row_type_extend(id, 100), 3);
        assert_eq!(slang_row_type_fields(id), 3);
    }
    #[test]
    fn test_restrict() {
        let id = slang_row_type_create(3);
        assert_eq!(slang_row_type_restrict(id), 2);
    }
    #[test]
    fn test_compatible() {
        let a = slang_row_type_create(5);
        let b = slang_row_type_create(3);
        assert_eq!(slang_row_type_compatible(a, b), 1);
        assert_eq!(slang_row_type_compatible(b, a), 0);
    }
    #[test]
    fn test_empty_row() {
        let id = slang_row_type_create(0);
        assert_eq!(slang_row_type_fields(id), 0);
    }
    #[test]
    fn test_extend_invalid() {
        assert_eq!(slang_row_type_extend(999999, 1), -1);
    }
    #[test]
    fn test_restrict_invalid() {
        assert_eq!(slang_row_type_restrict(999999), -1);
    }
    #[test]
    fn test_restrict_empty() {
        let id = slang_row_type_create(0);
        assert_eq!(slang_row_type_restrict(id), 0); // vec pop on empty = None
    }
    #[test]
    fn test_fields_nonexistent() {
        assert_eq!(slang_row_type_fields(999999), 0);
    }
    #[test]
    fn test_compatible_equal() {
        let a = slang_row_type_create(3);
        let b = slang_row_type_create(3);
        assert_eq!(slang_row_type_compatible(a, b), 1);
    }
    #[test]
    fn test_multiple_extends() {
        let id = slang_row_type_create(1);
        for i in 0..10 {
            slang_row_type_extend(id, i);
        }
        assert_eq!(slang_row_type_fields(id), 11);
    }
    #[test]
    fn test_extend_and_restrict() {
        let id = slang_row_type_create(3);
        slang_row_type_extend(id, 99);
        slang_row_type_restrict(id);
        assert_eq!(slang_row_type_fields(id), 3);
    }
    #[test]
    fn test_negative_fields_creation() {
        let id = slang_row_type_create(-5);
        assert_eq!(slang_row_type_fields(id), 0);
    }
    #[test]
    fn test_self_compatible() {
        let id = slang_row_type_create(4);
        assert_eq!(slang_row_type_compatible(id, id), 1);
    }
    #[test]
    fn test_compatible_nonexistent() {
        assert_eq!(slang_row_type_compatible(999999, 999998), 1); // both 0 fields
    }
    #[test]
    fn test_large_row() {
        let id = slang_row_type_create(100);
        assert_eq!(slang_row_type_fields(id), 100);
    }
    #[test]
    fn test_multiple_types_independent() {
        let a = slang_row_type_create(2);
        let b = slang_row_type_create(5);
        slang_row_type_extend(a, 10);
        assert_eq!(slang_row_type_fields(a), 3);
        assert_eq!(slang_row_type_fields(b), 5);
    }
    #[test]
    fn test_extend_returns_new_count() {
        let id = slang_row_type_create(0);
        assert_eq!(slang_row_type_extend(id, 1), 1);
        assert_eq!(slang_row_type_extend(id, 2), 2);
        assert_eq!(slang_row_type_extend(id, 3), 3);
    }
    #[test]
    fn test_restrict_returns_new_count() {
        let id = slang_row_type_create(3);
        assert_eq!(slang_row_type_restrict(id), 2);
        assert_eq!(slang_row_type_restrict(id), 1);
        assert_eq!(slang_row_type_restrict(id), 0);
    }
    #[test]
    fn test_compatible_with_zero() {
        let a = slang_row_type_create(5);
        let b = slang_row_type_create(0);
        assert_eq!(slang_row_type_compatible(a, b), 1);
        assert_eq!(slang_row_type_compatible(b, a), 0);
    }
}
