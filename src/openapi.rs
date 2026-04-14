//! OpenAPI — Spec Generation & Validation (v358)
//!
//! API spec building, route registration, parameter validation.

use std::sync::{LazyLock, Mutex};
use std::collections::HashMap;

struct ApiSpec {
    routes: HashMap<i64, Vec<i64>>, // route_id → param_ids
    route_count: i64,
    param_count: i64,
}

static SPECS: LazyLock<Mutex<HashMap<i64, ApiSpec>>> = LazyLock::new(|| Mutex::new(HashMap::new()));
static SPEC_COUNTER: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(0);

/// Create a new API spec. Returns spec ID.
#[unsafe(no_mangle)]
pub extern "C" fn slang_openapi_create() -> i64 {
    let id = SPEC_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    SPECS.lock().unwrap().insert(id, ApiSpec {
        routes: HashMap::new(), route_count: 0, param_count: 0,
    });
    id
}

/// Add a route to the spec. Returns route count.
#[unsafe(no_mangle)]
pub extern "C" fn slang_openapi_add_route(spec_id: i64, route_id: i64) -> i64 {
    let mut specs = SPECS.lock().unwrap();
    if let Some(s) = specs.get_mut(&spec_id) {
        s.routes.entry(route_id).or_insert_with(Vec::new);
        s.route_count = s.routes.len() as i64;
        s.route_count
    } else { -1 }
}

/// Add a parameter to a route. Returns param count.
#[unsafe(no_mangle)]
pub extern "C" fn slang_openapi_add_param(spec_id: i64, route_id: i64, param_id: i64) -> i64 {
    let mut specs = SPECS.lock().unwrap();
    if let Some(s) = specs.get_mut(&spec_id) {
        if let Some(params) = s.routes.get_mut(&route_id) {
            params.push(param_id);
            s.param_count += 1;
            s.param_count
        } else { -1 }
    } else { -1 }
}

/// Validate a parameter exists for a route. Returns 1 if valid.
#[unsafe(no_mangle)]
pub extern "C" fn slang_openapi_validate(spec_id: i64, route_id: i64, param_id: i64) -> i64 {
    let specs = SPECS.lock().unwrap();
    if let Some(s) = specs.get(&spec_id) {
        if let Some(params) = s.routes.get(&route_id) {
            if params.contains(&param_id) { 1 } else { 0 }
        } else { 0 }
    } else { -1 }
}

/// Get route count.
#[unsafe(no_mangle)]
pub extern "C" fn slang_openapi_route_count(spec_id: i64) -> i64 {
    SPECS.lock().unwrap().get(&spec_id).map(|s| s.route_count).unwrap_or(-1)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_create() { assert!(slang_openapi_create() >= 0); }
    #[test] fn test_add_route() {
        let s = slang_openapi_create();
        assert_eq!(slang_openapi_add_route(s, 1), 1);
    }
    #[test] fn test_add_param() {
        let s = slang_openapi_create();
        slang_openapi_add_route(s, 1);
        assert!(slang_openapi_add_param(s, 1, 10) >= 1);
    }
    #[test] fn test_validate_exists() {
        let s = slang_openapi_create();
        slang_openapi_add_route(s, 1);
        slang_openapi_add_param(s, 1, 10);
        assert_eq!(slang_openapi_validate(s, 1, 10), 1);
    }
    #[test] fn test_validate_missing() {
        let s = slang_openapi_create();
        slang_openapi_add_route(s, 1);
        assert_eq!(slang_openapi_validate(s, 1, 99), 0);
    }
    #[test] fn test_route_count() {
        let s = slang_openapi_create();
        slang_openapi_add_route(s, 1);
        slang_openapi_add_route(s, 2);
        assert_eq!(slang_openapi_route_count(s), 2);
    }
    #[test] fn test_invalid_spec() { assert_eq!(slang_openapi_add_route(999, 1), -1); }
    #[test] fn test_param_invalid_route() {
        let s = slang_openapi_create();
        assert_eq!(slang_openapi_add_param(s, 99, 1), -1);
    }
    #[test] fn test_validate_invalid_spec() { assert_eq!(slang_openapi_validate(999, 1, 1), -1); }
    #[test] fn test_validate_missing_route() {
        let s = slang_openapi_create();
        assert_eq!(slang_openapi_validate(s, 99, 1), 0);
    }
    #[test] fn test_multiple_params() {
        let s = slang_openapi_create();
        slang_openapi_add_route(s, 1);
        for i in 0..5 { slang_openapi_add_param(s, 1, i); }
        for i in 0..5 { assert_eq!(slang_openapi_validate(s, 1, i), 1); }
    }
    #[test] fn test_multiple_routes() {
        let s = slang_openapi_create();
        for i in 0..10 { slang_openapi_add_route(s, i); }
        assert_eq!(slang_openapi_route_count(s), 10);
    }
    #[test] fn test_route_count_empty() {
        let s = slang_openapi_create();
        assert_eq!(slang_openapi_route_count(s), 0);
    }
    #[test] fn test_route_count_invalid() { assert_eq!(slang_openapi_route_count(999), -1); }
    #[test] fn test_route_idempotent() {
        let s = slang_openapi_create();
        slang_openapi_add_route(s, 1);
        slang_openapi_add_route(s, 1);
        assert_eq!(slang_openapi_route_count(s), 1);
    }
    #[test] fn test_negative_ids() {
        let s = slang_openapi_create();
        slang_openapi_add_route(s, -1);
        slang_openapi_add_param(s, -1, -10);
        assert_eq!(slang_openapi_validate(s, -1, -10), 1);
    }
    #[test] fn test_separate_specs() {
        let a = slang_openapi_create();
        let b = slang_openapi_create();
        slang_openapi_add_route(a, 1);
        assert_eq!(slang_openapi_route_count(b), 0);
    }
    #[test] fn test_zero_ids() {
        let s = slang_openapi_create();
        slang_openapi_add_route(s, 0);
        slang_openapi_add_param(s, 0, 0);
        assert_eq!(slang_openapi_validate(s, 0, 0), 1);
    }
    #[test] fn test_param_count_increments() {
        let s = slang_openapi_create();
        slang_openapi_add_route(s, 1);
        let a = slang_openapi_add_param(s, 1, 1);
        let b = slang_openapi_add_param(s, 1, 2);
        assert_eq!(b, a + 1);
    }
    #[test] fn test_param_across_routes() {
        let s = slang_openapi_create();
        slang_openapi_add_route(s, 1);
        slang_openapi_add_route(s, 2);
        slang_openapi_add_param(s, 1, 10);
        assert_eq!(slang_openapi_validate(s, 2, 10), 0);
    }
}
