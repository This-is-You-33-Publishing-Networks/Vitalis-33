//! Telepathic Link — Vitalis v1003
//!
//! Zero-latency inter-compiler communication protocol for shared
//! understanding and thought-transfer between compiler instances.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<TlState>> = LazyLock::new(|| Mutex::new(TlState::default()));

#[derive(Default)]
struct TlState {
    channels: Vec<(String, String)>,
    messages: Vec<(String, String, String)>,
    bandwidth: f64,
}

pub struct TelepathicLink;

impl TelepathicLink {
    pub fn open_channel(from: &str, to: &str) -> usize {
        let mut s = STATE.lock().unwrap();
        s.channels.push((from.to_string(), to.to_string()));
        s.bandwidth += 1.0;
        s.channels.len()
    }

    pub fn transmit(from: &str, to: &str, thought: &str) -> bool {
        let mut s = STATE.lock().unwrap();
        let linked = s.channels.iter().any(|(f, t)| f == from && t == to);
        if linked {
            s.messages.push((from.to_string(), to.to_string(), thought.to_string()));
        }
        linked
    }

    pub fn receive(to: &str) -> Vec<String> {
        let s = STATE.lock().unwrap();
        s.messages.iter().filter(|(_, t, _)| t == to).map(|(_, _, m)| m.clone()).collect()
    }

    pub fn bandwidth() -> f64 { STATE.lock().unwrap().bandwidth }
    pub fn channel_count() -> usize { STATE.lock().unwrap().channels.len() }
    pub fn reset() { *STATE.lock().unwrap() = TlState::default(); }
}

#[unsafe(no_mangle)]
pub extern "C" fn tl_open(id: i64) -> i64 { TelepathicLink::open_channel(&format!("n{}", id), &format!("n{}", id + 1)) as i64 }
#[unsafe(no_mangle)]
pub extern "C" fn tl_transmit(from: i64, to: i64) -> i64 { if TelepathicLink::transmit(&format!("n{}", from), &format!("n{}", to), "thought") { 1 } else { 0 } }
#[unsafe(no_mangle)]
pub extern "C" fn tl_bandwidth() -> f64 { TelepathicLink::bandwidth() }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_open() { TelepathicLink::reset(); assert_eq!(TelepathicLink::open_channel("a", "b"), 1); }
    #[test] fn test_transmit() { TelepathicLink::reset(); TelepathicLink::open_channel("a", "b"); assert!(TelepathicLink::transmit("a", "b", "hello")); }
    #[test] fn test_transmit_no_channel() { TelepathicLink::reset(); assert!(!TelepathicLink::transmit("x", "y", "hi")); }
    #[test] fn test_receive() { TelepathicLink::reset(); TelepathicLink::open_channel("a", "b"); TelepathicLink::transmit("a", "b", "msg"); let msgs = TelepathicLink::receive("b"); assert_eq!(msgs.len(), 1); }
    #[test] fn test_bandwidth() { TelepathicLink::reset(); TelepathicLink::open_channel("a", "b"); assert!((TelepathicLink::bandwidth() - 1.0).abs() < 0.01); }
    #[test] fn test_ffi() { TelepathicLink::reset(); assert_eq!(tl_open(1), 1); }
}
