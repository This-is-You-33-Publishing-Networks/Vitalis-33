//! Cortical Column Program Layout — Vitalis v750
//!
//! Organizes program structure into cortical columns inspired by
//! neocortical architecture for hierarchical program composition.

use std::sync::{LazyLock, Mutex};
use std::collections::HashMap;

static STATE: LazyLock<Mutex<CorticalLayout>> = LazyLock::new(|| Mutex::new(CorticalLayout::new()));

pub struct CorticalLayout {
    columns: HashMap<u32, (String, usize)>,
    connections: Vec<(u32, u32)>,
    next_id: u32,
}

impl CorticalLayout {
    pub fn new() -> Self {
        Self { columns: HashMap::new(), connections: Vec::new(), next_id: 0 }
    }

    pub fn add_column(&mut self, name: &str, depth: usize) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        self.columns.insert(id, (name.to_string(), depth));
        id
    }

    pub fn link_columns(&mut self, a: u32, b: u32) {
        if self.columns.contains_key(&a) && self.columns.contains_key(&b) {
            self.connections.push((a, b));
        }
    }

    pub fn column_count(&self) -> usize {
        self.columns.len()
    }

    pub fn connection_count(&self) -> usize {
        self.connections.len()
    }
}

impl Default for CorticalLayout {
    fn default() -> Self { Self::new() }
}

#[unsafe(no_mangle)]
pub extern "C" fn cpl_add_column(name_len: i64, depth: i64) -> i64 {
    let name = "col_".to_string() + &"x".repeat(name_len.max(0) as usize);
    STATE.lock().unwrap().add_column(&name, depth as usize) as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn cpl_link(a: i64, b: i64) -> i64 {
    STATE.lock().unwrap().link_columns(a as u32, b as u32);
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn cpl_col_count() -> i64 {
    STATE.lock().unwrap().column_count() as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn cpl_conn_count() -> i64 {
    STATE.lock().unwrap().connection_count() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_column() {
        let mut cl = CorticalLayout::new();
        let id = cl.add_column("sensory", 6);
        assert_eq!(id, 0);
        assert_eq!(cl.column_count(), 1);
    }

    #[test]
    fn test_link_columns() {
        let mut cl = CorticalLayout::new();
        let a = cl.add_column("A", 4);
        let b = cl.add_column("B", 4);
        cl.link_columns(a, b);
        assert_eq!(cl.connection_count(), 1);
    }

    #[test]
    fn test_link_invalid_columns() {
        let mut cl = CorticalLayout::new();
        cl.link_columns(99, 100);
        assert_eq!(cl.connection_count(), 0);
    }

    #[test]
    fn test_multiple_columns() {
        let mut cl = CorticalLayout::new();
        for i in 0..5 {
            cl.add_column(&format!("col{i}"), i);
        }
        assert_eq!(cl.column_count(), 5);
    }

    #[test]
    fn test_ffi_add_and_count() {
        let id = cpl_add_column(3, 4);
        assert!(id >= 0);
        assert!(cpl_col_count() >= 1);
    }

    #[test]
    fn test_ffi_link_and_conn_count() {
        let a = cpl_add_column(2, 3) as i64;
        let b = cpl_add_column(2, 3) as i64;
        cpl_link(a, b);
        assert!(cpl_conn_count() >= 1);
    }
}
