//! Spike-Based OS Primitives — Vitalis v735
//!
//! Implements neuromorphic operating system primitives using spike-based
//! inter-process communication and process management.

use std::sync::{LazyLock, Mutex};
use std::collections::HashMap;

static STATE: LazyLock<Mutex<NeuromorphicOS>> = LazyLock::new(|| Mutex::new(NeuromorphicOS::new()));

pub struct NeuromorphicOS {
    processes: HashMap<u64, String>,
    next_pid: u64,
}

impl NeuromorphicOS {
    pub fn new() -> Self {
        Self { processes: HashMap::new(), next_pid: 1 }
    }

    pub fn spawn_process(&mut self, name: &str) -> u64 {
        let pid = self.next_pid;
        self.next_pid += 1;
        self.processes.insert(pid, name.to_string());
        pid
    }

    pub fn send_ipc_spike(&self, from: u64, to: u64) -> bool {
        self.processes.contains_key(&from) && self.processes.contains_key(&to)
    }

    pub fn process_count(&self) -> usize {
        self.processes.len()
    }

    pub fn terminate(&mut self, pid: u64) -> bool {
        self.processes.remove(&pid).is_some()
    }
}

impl Default for NeuromorphicOS {
    fn default() -> Self { Self::new() }
}

#[unsafe(no_mangle)]
pub extern "C" fn nos_spawn(name_len: i64) -> i64 {
    let name = "proc_".to_string() + &"x".repeat(name_len.max(0) as usize);
    STATE.lock().unwrap().spawn_process(&name) as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn nos_spike(from: i64, to: i64) -> i64 {
    if STATE.lock().unwrap().send_ipc_spike(from as u64, to as u64) { 1 } else { 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn nos_count() -> i64 {
    STATE.lock().unwrap().process_count() as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn nos_terminate(pid: i64) -> i64 {
    if STATE.lock().unwrap().terminate(pid as u64) { 1 } else { 0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spawn_returns_pid() {
        let mut os = NeuromorphicOS::new();
        let pid = os.spawn_process("worker");
        assert!(pid > 0);
    }

    #[test]
    fn test_process_count() {
        let mut os = NeuromorphicOS::new();
        os.spawn_process("a");
        os.spawn_process("b");
        assert_eq!(os.process_count(), 2);
    }

    #[test]
    fn test_ipc_spike_valid() {
        let mut os = NeuromorphicOS::new();
        let p1 = os.spawn_process("sender");
        let p2 = os.spawn_process("receiver");
        assert!(os.send_ipc_spike(p1, p2));
    }

    #[test]
    fn test_ipc_spike_invalid() {
        let os = NeuromorphicOS::new();
        assert!(!os.send_ipc_spike(99, 100));
    }

    #[test]
    fn test_terminate() {
        let mut os = NeuromorphicOS::new();
        let pid = os.spawn_process("temp");
        assert!(os.terminate(pid));
        assert_eq!(os.process_count(), 0);
    }

    #[test]
    fn test_ffi_nos_spawn_and_count() {
        let pid = nos_spawn(3);
        assert!(pid > 0);
        assert!(nos_count() >= 1);
    }
}
