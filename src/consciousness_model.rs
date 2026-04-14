//! Global Workspace Theory Compiler — Vitalis v930
//!
//! Implements a Global Workspace Theory model for compiler consciousness,
//! broadcasting information across compiler modules and tracking attention.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<ConsciousnessModel>> = LazyLock::new(|| Mutex::new(ConsciousnessModel::new()));

pub struct ConsciousnessModel {
    workspace: Vec<String>,
    focus: String,
    cycles: u64,
}

impl ConsciousnessModel {
    pub fn new() -> Self {
        Self {
            workspace: Vec::new(),
            focus: String::new(),
            cycles: 0,
        }
    }

    pub fn broadcast(&mut self, info: &str) {
        self.workspace.push(info.to_string());
        if self.workspace.len() > 16 {
            self.workspace.remove(0);
        }
        self.focus = info.to_string();
        self.cycles += 1;
    }

    pub fn attention_focus(&self) -> String {
        self.focus.clone()
    }

    pub fn workspace_size(&self) -> usize {
        self.workspace.len()
    }

    pub fn cycles(&self) -> u64 {
        self.cycles
    }
}

impl Default for ConsciousnessModel {
    fn default() -> Self { Self::new() }
}

#[unsafe(no_mangle)]
pub extern "C" fn gwt_broadcast(info_len: i64) -> i64 {
    let info = "broadcast_".to_string() + &"i".repeat(info_len.max(0) as usize);
    STATE.lock().unwrap().broadcast(&info);
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn gwt_focus() -> i64 {
    STATE.lock().unwrap().attention_focus().len() as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn gwt_size() -> i64 {
    STATE.lock().unwrap().workspace_size() as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn gwt_cycles() -> i64 {
    STATE.lock().unwrap().cycles() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_broadcast_updates_focus() {
        let mut cm = ConsciousnessModel::new();
        cm.broadcast("type_check");
        assert_eq!(cm.attention_focus(), "type_check");
    }

    #[test]
    fn test_workspace_grows() {
        let mut cm = ConsciousnessModel::new();
        cm.broadcast("a");
        cm.broadcast("b");
        assert_eq!(cm.workspace_size(), 2);
    }

    #[test]
    fn test_workspace_bounded() {
        let mut cm = ConsciousnessModel::new();
        for i in 0..20 {
            cm.broadcast(&format!("item_{i}"));
        }
        assert!(cm.workspace_size() <= 16);
    }

    #[test]
    fn test_cycles_increment() {
        let mut cm = ConsciousnessModel::new();
        cm.broadcast("x");
        cm.broadcast("y");
        assert_eq!(cm.cycles(), 2);
    }

    #[test]
    fn test_empty_focus() {
        let cm = ConsciousnessModel::new();
        assert!(cm.attention_focus().is_empty());
    }

    #[test]
    fn test_ffi_gwt_broadcast_cycles() {
        gwt_broadcast(5);
        assert!(gwt_cycles() >= 1);
    }
}
