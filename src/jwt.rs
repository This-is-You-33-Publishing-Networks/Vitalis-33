//! JWT — JSON Web Token (v338)
//!
//! Token creation, verification, claims extraction, expiry checking.

use std::sync::{LazyLock, Mutex};
use std::collections::HashMap;

struct Token {
    subject: i64,
    issued_at: i64,
    expires_at: i64,
    claims: HashMap<i64, i64>,
}

static TOKENS: LazyLock<Mutex<HashMap<i64, Token>>> = LazyLock::new(|| Mutex::new(HashMap::new()));
static TOKEN_COUNTER: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(0);

/// Create a JWT for a subject with an expiry time. Returns token ID.
#[unsafe(no_mangle)]
pub extern "C" fn slang_jwt_create(subject: i64, issued_at: i64, expires_at: i64) -> i64 {
    let id = TOKEN_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    TOKENS.lock().unwrap().insert(id, Token {
        subject, issued_at, expires_at, claims: HashMap::new(),
    });
    id
}

/// Verify a token. Returns 1 if valid and not expired at `now`, 0 otherwise.
#[unsafe(no_mangle)]
pub extern "C" fn slang_jwt_verify(token_id: i64, now: i64) -> i64 {
    let tokens = TOKENS.lock().unwrap();
    match tokens.get(&token_id) {
        Some(t) if now >= t.issued_at && now < t.expires_at => 1,
        _ => 0,
    }
}

/// Get the subject from a token. Returns -1 if invalid.
#[unsafe(no_mangle)]
pub extern "C" fn slang_jwt_claims(token_id: i64) -> i64 {
    TOKENS.lock().unwrap().get(&token_id).map(|t| t.subject).unwrap_or(-1)
}

/// Check if token is expired at `now`. Returns 1 if expired.
#[unsafe(no_mangle)]
pub extern "C" fn slang_jwt_expired(token_id: i64, now: i64) -> i64 {
    let tokens = TOKENS.lock().unwrap();
    match tokens.get(&token_id) {
        Some(t) if now >= t.expires_at => 1,
        Some(_) => 0,
        None => -1,
    }
}

/// Set a custom claim on a token.
#[unsafe(no_mangle)]
pub extern "C" fn slang_jwt_set_claim(token_id: i64, key: i64, value: i64) -> i64 {
    let mut tokens = TOKENS.lock().unwrap();
    if let Some(t) = tokens.get_mut(&token_id) {
        t.claims.insert(key, value);
        1
    } else { -1 }
}

/// Get a custom claim from a token.
#[unsafe(no_mangle)]
pub extern "C" fn slang_jwt_get_claim(token_id: i64, key: i64) -> i64 {
    TOKENS.lock().unwrap().get(&token_id)
        .and_then(|t| t.claims.get(&key).copied())
        .unwrap_or(-1)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_create() { let id = slang_jwt_create(1, 0, 100); assert!(id >= 0); }
    #[test] fn test_verify_valid() { let id = slang_jwt_create(1, 0, 100); assert_eq!(slang_jwt_verify(id, 50), 1); }
    #[test] fn test_verify_expired() { let id = slang_jwt_create(1, 0, 100); assert_eq!(slang_jwt_verify(id, 100), 0); }
    #[test] fn test_verify_before_issue() { let id = slang_jwt_create(1, 10, 100); assert_eq!(slang_jwt_verify(id, 5), 0); }
    #[test] fn test_claims() { let id = slang_jwt_create(42, 0, 100); assert_eq!(slang_jwt_claims(id), 42); }
    #[test] fn test_claims_invalid() { assert_eq!(slang_jwt_claims(999), -1); }
    #[test] fn test_expired() { let id = slang_jwt_create(1, 0, 50); assert_eq!(slang_jwt_expired(id, 50), 1); }
    #[test] fn test_not_expired() { let id = slang_jwt_create(1, 0, 50); assert_eq!(slang_jwt_expired(id, 49), 0); }
    #[test] fn test_expired_invalid() { assert_eq!(slang_jwt_expired(999, 0), -1); }
    #[test] fn test_verify_at_issue() { let id = slang_jwt_create(1, 10, 100); assert_eq!(slang_jwt_verify(id, 10), 1); }
    #[test] fn test_zero_lifetime() { let id = slang_jwt_create(1, 0, 0); assert_eq!(slang_jwt_verify(id, 0), 0); }
    #[test] fn test_set_claim() { let id = slang_jwt_create(1, 0, 100); assert_eq!(slang_jwt_set_claim(id, 1, 42), 1); }
    #[test] fn test_get_claim() {
        let id = slang_jwt_create(1, 0, 100);
        slang_jwt_set_claim(id, 1, 42);
        assert_eq!(slang_jwt_get_claim(id, 1), 42);
    }
    #[test] fn test_get_missing_claim() { let id = slang_jwt_create(1, 0, 100); assert_eq!(slang_jwt_get_claim(id, 99), -1); }
    #[test] fn test_multiple_claims() {
        let id = slang_jwt_create(1, 0, 100);
        for i in 0..5 { slang_jwt_set_claim(id, i, i * 10); }
        for i in 0..5 { assert_eq!(slang_jwt_get_claim(id, i), i * 10); }
    }
    #[test] fn test_overwrite_claim() {
        let id = slang_jwt_create(1, 0, 100);
        slang_jwt_set_claim(id, 1, 10);
        slang_jwt_set_claim(id, 1, 20);
        assert_eq!(slang_jwt_get_claim(id, 1), 20);
    }
    #[test] fn test_verify_invalid_token() { assert_eq!(slang_jwt_verify(999, 0), 0); }
    #[test] fn test_negative_subject() {
        let id = slang_jwt_create(-1, 0, 100);
        assert_eq!(slang_jwt_claims(id), -1);
    }
    #[test] fn test_large_expiry() {
        let id = slang_jwt_create(1, 0, i64::MAX);
        assert_eq!(slang_jwt_verify(id, 1000000), 1);
    }
    #[test] fn test_multiple_tokens() {
        let a = slang_jwt_create(10, 0, 100);
        let b = slang_jwt_create(20, 0, 100);
        assert_eq!(slang_jwt_claims(a), 10);
        assert_eq!(slang_jwt_claims(b), 20);
    }
}
