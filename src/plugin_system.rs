//! Plugin System — Extension Framework (v362)
//!
//! Plugin loading, hook registration, lifecycle management.

use std::sync::{LazyLock, Mutex};
use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, Ordering};

struct Plugin {
    state: i64,          // 0=loaded, 1=active, 2=unloaded
    hooks: Vec<i64>,     // hook IDs
    priority: i64,
}

static PLUGINS: LazyLock<Mutex<HashMap<i64, Plugin>>> = LazyLock::new(|| Mutex::new(HashMap::new()));
static PLUGIN_COUNTER: AtomicI64 = AtomicI64::new(0);
static HOOK_COUNTER: AtomicI64 = AtomicI64::new(0);

/// Load a plugin. Returns plugin ID.
#[unsafe(no_mangle)]
pub extern "C" fn slang_plugin_load(priority: i64) -> i64 {
    let id = PLUGIN_COUNTER.fetch_add(1, Ordering::SeqCst);
    PLUGINS.lock().unwrap().insert(id, Plugin { state: 0, hooks: Vec::new(), priority });
    id
}

/// Register a hook on a plugin. Returns hook ID.
#[unsafe(no_mangle)]
pub extern "C" fn slang_plugin_register_hook(plugin_id: i64) -> i64 {
    let mut plugins = PLUGINS.lock().unwrap();
    if let Some(p) = plugins.get_mut(&plugin_id) {
        if p.state == 2 { return -1; } // can't hook unloaded
        let hook_id = HOOK_COUNTER.fetch_add(1, Ordering::SeqCst);
        p.hooks.push(hook_id);
        hook_id
    } else { -1 }
}

/// Activate a plugin. Returns 1 on success.
#[unsafe(no_mangle)]
pub extern "C" fn slang_plugin_activate(plugin_id: i64) -> i64 {
    let mut plugins = PLUGINS.lock().unwrap();
    if let Some(p) = plugins.get_mut(&plugin_id) {
        if p.state == 2 { return -1; }
        p.state = 1;
        1
    } else { -1 }
}

/// Unload a plugin. Returns 1 on success.
#[unsafe(no_mangle)]
pub extern "C" fn slang_plugin_unload(plugin_id: i64) -> i64 {
    let mut plugins = PLUGINS.lock().unwrap();
    if let Some(p) = plugins.get_mut(&plugin_id) {
        p.state = 2;
        p.hooks.clear();
        1
    } else { -1 }
}

/// Get plugin state (0=loaded, 1=active, 2=unloaded).
#[unsafe(no_mangle)]
pub extern "C" fn slang_plugin_state(plugin_id: i64) -> i64 {
    PLUGINS.lock().unwrap().get(&plugin_id).map(|p| p.state).unwrap_or(-1)
}

/// Get hook count for a plugin.
#[unsafe(no_mangle)]
pub extern "C" fn slang_plugin_hook_count(plugin_id: i64) -> i64 {
    PLUGINS.lock().unwrap().get(&plugin_id).map(|p| p.hooks.len() as i64).unwrap_or(-1)
}

/// Get total loaded plugin count.
#[unsafe(no_mangle)]
pub extern "C" fn slang_plugin_count() -> i64 {
    PLUGINS.lock().unwrap().len() as i64
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_load() { let id = slang_plugin_load(0); assert!(id >= 0); }
    #[test] fn test_state_loaded() { let id = slang_plugin_load(0); assert_eq!(slang_plugin_state(id), 0); }
    #[test] fn test_activate() {
        let id = slang_plugin_load(0);
        assert_eq!(slang_plugin_activate(id), 1);
        assert_eq!(slang_plugin_state(id), 1);
    }
    #[test] fn test_unload() {
        let id = slang_plugin_load(0);
        assert_eq!(slang_plugin_unload(id), 1);
        assert_eq!(slang_plugin_state(id), 2);
    }
    #[test] fn test_register_hook() {
        let id = slang_plugin_load(0);
        assert!(slang_plugin_register_hook(id) >= 0);
    }
    #[test] fn test_hook_count() {
        let id = slang_plugin_load(0);
        slang_plugin_register_hook(id);
        slang_plugin_register_hook(id);
        assert_eq!(slang_plugin_hook_count(id), 2);
    }
    #[test] fn test_hook_after_unload() {
        let id = slang_plugin_load(0);
        slang_plugin_unload(id);
        assert_eq!(slang_plugin_register_hook(id), -1);
    }
    #[test] fn test_activate_after_unload() {
        let id = slang_plugin_load(0);
        slang_plugin_unload(id);
        assert_eq!(slang_plugin_activate(id), -1);
    }
    #[test] fn test_hooks_cleared_on_unload() {
        let id = slang_plugin_load(0);
        slang_plugin_register_hook(id);
        slang_plugin_unload(id);
        assert_eq!(slang_plugin_hook_count(id), 0);
    }
    #[test] fn test_count() { assert!(slang_plugin_count() >= 1); }
    #[test] fn test_invalid_state() { assert_eq!(slang_plugin_state(999), -1); }
    #[test] fn test_invalid_activate() { assert_eq!(slang_plugin_activate(999), -1); }
    #[test] fn test_invalid_unload() { assert_eq!(slang_plugin_unload(999), -1); }
    #[test] fn test_invalid_hook() { assert_eq!(slang_plugin_register_hook(999), -1); }
    #[test] fn test_multiple_plugins() {
        let a = slang_plugin_load(0);
        let b = slang_plugin_load(1);
        slang_plugin_activate(a);
        assert_eq!(slang_plugin_state(a), 1);
        assert_eq!(slang_plugin_state(b), 0);
    }
    #[test] fn test_priority_stored() {
        let id = slang_plugin_load(42);
        let plugins = PLUGINS.lock().unwrap();
        assert_eq!(plugins.get(&id).unwrap().priority, 42);
    }
    #[test] fn test_hook_count_empty() {
        let id = slang_plugin_load(0);
        assert_eq!(slang_plugin_hook_count(id), 0);
    }
    #[test] fn test_hook_count_invalid() { assert_eq!(slang_plugin_hook_count(999), -1); }
    #[test] fn test_unique_hook_ids() {
        let id = slang_plugin_load(0);
        let h1 = slang_plugin_register_hook(id);
        let h2 = slang_plugin_register_hook(id);
        assert_ne!(h1, h2);
    }
    #[test] fn test_many_hooks() {
        let id = slang_plugin_load(0);
        for _ in 0..10 { slang_plugin_register_hook(id); }
        assert_eq!(slang_plugin_hook_count(id), 10);
    }
}
