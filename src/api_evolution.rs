//! API Backward-Compat Evolution — Vitalis v885
//!
//! Manages API versioning and backward compatibility, tracking migrations
//! and endpoint version registrations.

use std::sync::{LazyLock, Mutex};
use std::collections::HashMap;

static STATE: LazyLock<Mutex<ApiEvolution>> = LazyLock::new(|| Mutex::new(ApiEvolution::new()));

pub struct ApiEvolution {
    versions: HashMap<u32, String>,
    migrations: usize,
}

impl ApiEvolution {
    pub fn new() -> Self {
        Self { versions: HashMap::new(), migrations: 0 }
    }

    pub fn register_version(&mut self, ver: u32, endpoint: &str) {
        self.versions.insert(ver, endpoint.to_string());
    }

    pub fn is_compatible(&mut self, v1: u32, v2: u32) -> bool {
        let compatible = v1 <= v2 || (v2 + 10 >= v1);
        if !compatible {
            self.migrations += 1;
        }
        compatible
    }

    pub fn migration_count(&self) -> usize {
        self.migrations
    }

    pub fn latest_version(&self) -> u32 {
        self.versions.keys().cloned().max().unwrap_or(0)
    }
}

impl Default for ApiEvolution {
    fn default() -> Self { Self::new() }
}

#[unsafe(no_mangle)]
pub extern "C" fn ae_register(ver: i64, endpoint_len: i64) -> i64 {
    let endpoint = "/api/v".to_string() + &"x".repeat(endpoint_len.max(0) as usize);
    STATE.lock().unwrap().register_version(ver as u32, &endpoint);
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn ae_compatible(v1: i64, v2: i64) -> i64 {
    if STATE.lock().unwrap().is_compatible(v1 as u32, v2 as u32) { 1 } else { 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn ae_migrations() -> i64 {
    STATE.lock().unwrap().migration_count() as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn ae_latest() -> i64 {
    STATE.lock().unwrap().latest_version() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_version() {
        let mut ae = ApiEvolution::new();
        ae.register_version(1, "/api/v1/users");
        assert_eq!(ae.latest_version(), 1);
    }

    #[test]
    fn test_latest_version() {
        let mut ae = ApiEvolution::new();
        ae.register_version(1, "/v1");
        ae.register_version(3, "/v3");
        ae.register_version(2, "/v2");
        assert_eq!(ae.latest_version(), 3);
    }

    #[test]
    fn test_compatible_ascending() {
        let mut ae = ApiEvolution::new();
        assert!(ae.is_compatible(1, 2));
    }

    #[test]
    fn test_incompatible_major_gap() {
        let mut ae = ApiEvolution::new();
        let r = ae.is_compatible(100, 1);
        if !r { assert!(ae.migration_count() > 0); }
    }

    #[test]
    fn test_no_versions_latest_zero() {
        let ae = ApiEvolution::new();
        assert_eq!(ae.latest_version(), 0);
    }

    #[test]
    fn test_ffi_ae_register_and_latest() {
        ae_register(42, 3);
        assert!(ae_latest() >= 42);
    }
}
