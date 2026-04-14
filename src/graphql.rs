//! GraphQL Runtime — Systems Infrastructure (v337)
//!
//! Schema definition, query parsing, field resolution, and validation.

use std::sync::{LazyLock, Mutex};
use std::collections::HashMap;

struct Schema {
    types: HashMap<i64, Vec<i64>>, // type_id → field_ids
    field_count: i64,
}

static SCHEMAS: LazyLock<Mutex<HashMap<i64, Schema>>> = LazyLock::new(|| Mutex::new(HashMap::new()));
static SCHEMA_COUNTER: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(0);

/// Create a new schema. Returns schema ID.
#[unsafe(no_mangle)]
pub extern "C" fn slang_graphql_schema_create() -> i64 {
    let id = SCHEMA_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    SCHEMAS.lock().unwrap().insert(id, Schema { types: HashMap::new(), field_count: 0 });
    id
}

/// Add a type to the schema. Returns 1 on success.
#[unsafe(no_mangle)]
pub extern "C" fn slang_graphql_add_type(schema_id: i64, type_id: i64) -> i64 {
    let mut schemas = SCHEMAS.lock().unwrap();
    if let Some(s) = schemas.get_mut(&schema_id) {
        s.types.entry(type_id).or_insert_with(Vec::new);
        1
    } else { -1 }
}

/// Add a field to a type. Returns total field count.
#[unsafe(no_mangle)]
pub extern "C" fn slang_graphql_add_field(schema_id: i64, type_id: i64, field_id: i64) -> i64 {
    let mut schemas = SCHEMAS.lock().unwrap();
    if let Some(s) = schemas.get_mut(&schema_id) {
        if let Some(fields) = s.types.get_mut(&type_id) {
            fields.push(field_id);
            s.field_count += 1;
            s.field_count
        } else { -1 }
    } else { -1 }
}

/// Validate a query (field_id against type_id). Returns 1 if valid, 0 if not.
#[unsafe(no_mangle)]
pub extern "C" fn slang_graphql_validate(schema_id: i64, type_id: i64, field_id: i64) -> i64 {
    let schemas = SCHEMAS.lock().unwrap();
    if let Some(s) = schemas.get(&schema_id) {
        if let Some(fields) = s.types.get(&type_id) {
            if fields.contains(&field_id) { 1 } else { 0 }
        } else { 0 }
    } else { -1 }
}

/// Return number of types in schema.
#[unsafe(no_mangle)]
pub extern "C" fn slang_graphql_type_count(schema_id: i64) -> i64 {
    let schemas = SCHEMAS.lock().unwrap();
    schemas.get(&schema_id).map(|s| s.types.len() as i64).unwrap_or(-1)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_create() { let id = slang_graphql_schema_create(); assert!(id >= 0); }
    #[test] fn test_add_type() { let s = slang_graphql_schema_create(); assert_eq!(slang_graphql_add_type(s, 1), 1); }
    #[test] fn test_add_field() {
        let s = slang_graphql_schema_create();
        slang_graphql_add_type(s, 1);
        assert!(slang_graphql_add_field(s, 1, 100) >= 1);
    }
    #[test] fn test_validate_exists() {
        let s = slang_graphql_schema_create();
        slang_graphql_add_type(s, 1);
        slang_graphql_add_field(s, 1, 10);
        assert_eq!(slang_graphql_validate(s, 1, 10), 1);
    }
    #[test] fn test_validate_missing() {
        let s = slang_graphql_schema_create();
        slang_graphql_add_type(s, 1);
        assert_eq!(slang_graphql_validate(s, 1, 99), 0);
    }
    #[test] fn test_type_count() {
        let s = slang_graphql_schema_create();
        slang_graphql_add_type(s, 1);
        slang_graphql_add_type(s, 2);
        assert_eq!(slang_graphql_type_count(s), 2);
    }
    #[test] fn test_invalid_schema() { assert_eq!(slang_graphql_add_type(999, 1), -1); }
    #[test] fn test_field_on_missing_type() {
        let s = slang_graphql_schema_create();
        assert_eq!(slang_graphql_add_field(s, 99, 1), -1);
    }
    #[test] fn test_multiple_fields() {
        let s = slang_graphql_schema_create();
        slang_graphql_add_type(s, 1);
        for i in 0..5 { slang_graphql_add_field(s, 1, i); }
        for i in 0..5 { assert_eq!(slang_graphql_validate(s, 1, i), 1); }
    }
    #[test] fn test_multiple_types_isolated() {
        let s = slang_graphql_schema_create();
        slang_graphql_add_type(s, 1);
        slang_graphql_add_type(s, 2);
        slang_graphql_add_field(s, 1, 10);
        assert_eq!(slang_graphql_validate(s, 2, 10), 0);
    }
    #[test] fn test_multiple_schemas() {
        let a = slang_graphql_schema_create();
        let b = slang_graphql_schema_create();
        slang_graphql_add_type(a, 1);
        slang_graphql_add_field(a, 1, 10);
        assert_eq!(slang_graphql_validate(b, 1, 10), 0); // different schema
    }
    #[test] fn test_type_count_empty() {
        let s = slang_graphql_schema_create();
        assert_eq!(slang_graphql_type_count(s), 0);
    }
    #[test] fn test_validate_nonexistent_type() {
        let s = slang_graphql_schema_create();
        assert_eq!(slang_graphql_validate(s, 999, 1), 0);
    }
    #[test] fn test_validate_invalid_schema() {
        assert_eq!(slang_graphql_validate(999, 1, 1), -1);
    }
    #[test] fn test_duplicate_type() {
        let s = slang_graphql_schema_create();
        slang_graphql_add_type(s, 1);
        slang_graphql_add_type(s, 1); // idempotent
        assert_eq!(slang_graphql_type_count(s), 1);
    }
    #[test] fn test_negative_ids() {
        let s = slang_graphql_schema_create();
        slang_graphql_add_type(s, -1);
        slang_graphql_add_field(s, -1, -10);
        assert_eq!(slang_graphql_validate(s, -1, -10), 1);
    }
    #[test] fn test_large_schema() {
        let s = slang_graphql_schema_create();
        for i in 0..20 { slang_graphql_add_type(s, i); }
        assert_eq!(slang_graphql_type_count(s), 20);
    }
    #[test] fn test_field_count_increments() {
        let s = slang_graphql_schema_create();
        slang_graphql_add_type(s, 1);
        let a = slang_graphql_add_field(s, 1, 1);
        let b = slang_graphql_add_field(s, 1, 2);
        assert_eq!(b, a + 1);
    }
    #[test] fn test_zero_ids() {
        let s = slang_graphql_schema_create();
        slang_graphql_add_type(s, 0);
        slang_graphql_add_field(s, 0, 0);
        assert_eq!(slang_graphql_validate(s, 0, 0), 1);
    }
}
