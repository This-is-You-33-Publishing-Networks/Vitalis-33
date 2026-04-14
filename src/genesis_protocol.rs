//! Genesis Protocol — Vitalis v1307
//!
//! Protocol to spawn entirely new languages from Vitalis.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<GenesisProtocolState>> = LazyLock::new(|| Mutex::new(GenesisProtocolState::default()));

#[derive(Default)]
struct GenesisProtocolState {
    spawned: Vec<String>,
    protocols: Vec<(String, Vec<String>)>,
    generation: u32,
}

pub struct GenesisProtocol;

impl GenesisProtocol {
    pub fn spawn(name: &str) {
        let mut s = STATE.lock().unwrap();
        s.spawned.push(name.to_string());
        s.generation += 1;
    }
    pub fn add_protocol(name: &str, steps: Vec<String>) {
        let mut s = STATE.lock().unwrap();
        s.protocols.push((name.to_string(), steps));
    }
    pub fn spawned_count() -> usize { STATE.lock().unwrap().spawned.len() }
    pub fn protocol_count() -> usize { STATE.lock().unwrap().protocols.len() }
    pub fn generation() -> u32 { STATE.lock().unwrap().generation }
    pub fn reset() { *STATE.lock().unwrap() = GenesisProtocolState::default(); }
}

#[unsafe(no_mangle)]
pub extern "C" fn gen_spawn() -> usize { GenesisProtocol::spawn("lang"); GenesisProtocol::spawned_count() }
#[unsafe(no_mangle)]
pub extern "C" fn gen_spawned_count() -> usize { GenesisProtocol::spawned_count() }
#[unsafe(no_mangle)]
pub extern "C" fn gen_generation() -> u32 { GenesisProtocol::generation() }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_spawn() { GenesisProtocol::reset(); GenesisProtocol::spawn("nova"); assert_eq!(GenesisProtocol::spawned_count(), 1); }
    #[test] fn test_add_protocol() { GenesisProtocol::reset(); GenesisProtocol::add_protocol("bootstrap", vec!["lex".into(), "parse".into()]); assert_eq!(GenesisProtocol::protocol_count(), 1); }
    #[test] fn test_generation_increments() { GenesisProtocol::reset(); GenesisProtocol::spawn("a"); GenesisProtocol::spawn("b"); assert_eq!(GenesisProtocol::generation(), 2); }
    #[test] fn test_multiple_spawns() { GenesisProtocol::reset(); for i in 0..3 { GenesisProtocol::spawn(&format!("lang{}", i)); } assert_eq!(GenesisProtocol::spawned_count(), 3); }
    #[test] fn test_protocol_steps() { GenesisProtocol::reset(); GenesisProtocol::add_protocol("p", vec!["a".into()]); assert_eq!(STATE.lock().unwrap().protocols[0].1.len(), 1); }
    #[test] fn test_initial_generation() { GenesisProtocol::reset(); assert_eq!(GenesisProtocol::generation(), 0); }
    #[test] fn test_reset() { GenesisProtocol::reset(); GenesisProtocol::spawn("x"); GenesisProtocol::reset(); assert_eq!(GenesisProtocol::spawned_count(), 0); }
}
