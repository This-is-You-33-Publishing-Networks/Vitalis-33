//! OAuth2 — Authorization Framework (v339)
//!
//! Auth URL generation, token exchange, refresh, PKCE support.

use std::sync::{LazyLock, Mutex};
use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, Ordering};

struct AuthSession {
    client_id: i64,
    state: i64,           // 0=pending, 1=authorized, 2=refreshed, 3=revoked
    code_verifier: i64,   // PKCE code verifier hash
    access_token: i64,
    refresh_count: i64,
}

static SESSIONS: LazyLock<Mutex<HashMap<i64, AuthSession>>> = LazyLock::new(|| Mutex::new(HashMap::new()));
static SESSION_COUNTER: AtomicI64 = AtomicI64::new(0);

/// Generate an authorization URL (returns session ID).
#[unsafe(no_mangle)]
pub extern "C" fn slang_oauth2_auth_url(client_id: i64, code_verifier: i64) -> i64 {
    let id = SESSION_COUNTER.fetch_add(1, Ordering::SeqCst);
    SESSIONS.lock().unwrap().insert(id, AuthSession {
        client_id, state: 0, code_verifier, access_token: 0, refresh_count: 0,
    });
    id
}

/// Exchange authorization code for token. Returns access token or -1.
#[unsafe(no_mangle)]
pub extern "C" fn slang_oauth2_exchange(session_id: i64, code_verifier: i64) -> i64 {
    let mut sessions = SESSIONS.lock().unwrap();
    if let Some(s) = sessions.get_mut(&session_id) {
        if s.state == 0 && s.code_verifier == code_verifier {
            s.state = 1;
            s.access_token = session_id * 1000 + 1; // deterministic token
            s.access_token
        } else { -1 }
    } else { -1 }
}

/// Refresh token. Returns new access token or -1.
#[unsafe(no_mangle)]
pub extern "C" fn slang_oauth2_refresh(session_id: i64) -> i64 {
    let mut sessions = SESSIONS.lock().unwrap();
    if let Some(s) = sessions.get_mut(&session_id) {
        if s.state == 1 || s.state == 2 {
            s.refresh_count += 1;
            s.access_token = session_id * 1000 + s.refresh_count + 1;
            s.state = 2;
            s.access_token
        } else { -1 }
    } else { -1 }
}

/// PKCE verification. Returns 1 if verifier matches session.
#[unsafe(no_mangle)]
pub extern "C" fn slang_oauth2_pkce_verify(session_id: i64, code_verifier: i64) -> i64 {
    let sessions = SESSIONS.lock().unwrap();
    match sessions.get(&session_id) {
        Some(s) if s.code_verifier == code_verifier => 1,
        _ => 0,
    }
}

/// Get session state (0=pending, 1=auth'd, 2=refreshed, 3=revoked).
#[unsafe(no_mangle)]
pub extern "C" fn slang_oauth2_state(session_id: i64) -> i64 {
    SESSIONS.lock().unwrap().get(&session_id).map(|s| s.state).unwrap_or(-1)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_auth_url() { let id = slang_oauth2_auth_url(1, 42); assert!(id >= 0); }
    #[test] fn test_state_pending() { let id = slang_oauth2_auth_url(1, 42); assert_eq!(slang_oauth2_state(id), 0); }
    #[test] fn test_exchange() {
        let id = slang_oauth2_auth_url(1, 42);
        let token = slang_oauth2_exchange(id, 42);
        assert!(token > 0);
    }
    #[test] fn test_exchange_wrong_verifier() {
        let id = slang_oauth2_auth_url(1, 42);
        assert_eq!(slang_oauth2_exchange(id, 99), -1);
    }
    #[test] fn test_state_after_exchange() {
        let id = slang_oauth2_auth_url(1, 42);
        slang_oauth2_exchange(id, 42);
        assert_eq!(slang_oauth2_state(id), 1);
    }
    #[test] fn test_refresh() {
        let id = slang_oauth2_auth_url(1, 42);
        slang_oauth2_exchange(id, 42);
        let new_token = slang_oauth2_refresh(id);
        assert!(new_token > 0);
    }
    #[test] fn test_refresh_without_exchange() {
        let id = slang_oauth2_auth_url(1, 42);
        assert_eq!(slang_oauth2_refresh(id), -1);
    }
    #[test] fn test_state_after_refresh() {
        let id = slang_oauth2_auth_url(1, 42);
        slang_oauth2_exchange(id, 42);
        slang_oauth2_refresh(id);
        assert_eq!(slang_oauth2_state(id), 2);
    }
    #[test] fn test_pkce_verify() {
        let id = slang_oauth2_auth_url(1, 42);
        assert_eq!(slang_oauth2_pkce_verify(id, 42), 1);
    }
    #[test] fn test_pkce_verify_wrong() {
        let id = slang_oauth2_auth_url(1, 42);
        assert_eq!(slang_oauth2_pkce_verify(id, 99), 0);
    }
    #[test] fn test_invalid_session() { assert_eq!(slang_oauth2_state(999), -1); }
    #[test] fn test_exchange_invalid() { assert_eq!(slang_oauth2_exchange(999, 0), -1); }
    #[test] fn test_refresh_invalid() { assert_eq!(slang_oauth2_refresh(999), -1); }
    #[test] fn test_double_exchange() {
        let id = slang_oauth2_auth_url(1, 42);
        slang_oauth2_exchange(id, 42);
        assert_eq!(slang_oauth2_exchange(id, 42), -1); // can't re-exchange
    }
    #[test] fn test_multiple_refreshes() {
        let id = slang_oauth2_auth_url(1, 42);
        slang_oauth2_exchange(id, 42);
        let a = slang_oauth2_refresh(id);
        let b = slang_oauth2_refresh(id);
        assert_ne!(a, b); // different tokens
    }
    #[test] fn test_multiple_sessions() {
        let a = slang_oauth2_auth_url(1, 10);
        let b = slang_oauth2_auth_url(2, 20);
        assert_ne!(a, b);
    }
    #[test] fn test_pkce_verify_invalid_session() { assert_eq!(slang_oauth2_pkce_verify(999, 0), 0); }
    #[test] fn test_zero_verifier() {
        let id = slang_oauth2_auth_url(1, 0);
        assert_eq!(slang_oauth2_pkce_verify(id, 0), 1);
    }
    #[test] fn test_client_id_preserved() {
        let id = slang_oauth2_auth_url(42, 10);
        let sessions = SESSIONS.lock().unwrap();
        assert_eq!(sessions.get(&id).unwrap().client_id, 42);
    }
    #[test] fn test_refresh_new_token_each_time() {
        let id = slang_oauth2_auth_url(1, 42);
        slang_oauth2_exchange(id, 42);
        let mut tokens = Vec::new();
        for _ in 0..5 { tokens.push(slang_oauth2_refresh(id)); }
        // All should be unique
        tokens.sort();
        tokens.dedup();
        assert_eq!(tokens.len(), 5);
    }
}
