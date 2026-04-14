//! Session Types — v384
//! Binary session type protocol verification for typed communication channels.

use std::sync::{LazyLock, Mutex};
use std::collections::HashMap;

/// Protocol actions in a session type.
#[derive(Debug, Clone, PartialEq)]
pub enum SessionType {
    Send(String, Box<SessionType>),
    Recv(String, Box<SessionType>),
    Choose(Vec<(String, SessionType)>),
    Offer(Vec<(String, SessionType)>),
    Rec(String, Box<SessionType>),
    Var(String),
    End,
}

impl SessionType {
    /// Compute the dual (mirror) of a session type.
    pub fn dual(&self) -> SessionType {
        match self {
            SessionType::Send(ty, cont) => SessionType::Recv(ty.clone(), Box::new(cont.dual())),
            SessionType::Recv(ty, cont) => SessionType::Send(ty.clone(), Box::new(cont.dual())),
            SessionType::Choose(branches) => {
                SessionType::Offer(branches.iter().map(|(l, s)| (l.clone(), s.dual())).collect())
            }
            SessionType::Offer(branches) => {
                SessionType::Choose(branches.iter().map(|(l, s)| (l.clone(), s.dual())).collect())
            }
            SessionType::Rec(name, body) => SessionType::Rec(name.clone(), Box::new(body.dual())),
            SessionType::Var(name) => SessionType::Var(name.clone()),
            SessionType::End => SessionType::End,
        }
    }

    /// Check if two session types are duals of each other.
    pub fn is_dual(&self, other: &SessionType) -> bool {
        self.dual() == *other
    }

    /// Validate a protocol trace against this session type.
    pub fn validate_trace(&self, trace: &[ProtocolAction]) -> bool {
        self.validate_trace_inner(trace, 0).0
    }

    fn validate_trace_inner(&self, trace: &[ProtocolAction], pos: usize) -> (bool, usize) {
        if pos >= trace.len() {
            return (matches!(self, SessionType::End), pos);
        }
        match self {
            SessionType::Send(expected_ty, cont) => {
                if let ProtocolAction::Send(ty) = &trace[pos] {
                    if ty == expected_ty {
                        return cont.validate_trace_inner(trace, pos + 1);
                    }
                }
                (false, pos)
            }
            SessionType::Recv(expected_ty, cont) => {
                if let ProtocolAction::Recv(ty) = &trace[pos] {
                    if ty == expected_ty {
                        return cont.validate_trace_inner(trace, pos + 1);
                    }
                }
                (false, pos)
            }
            SessionType::Choose(branches) => {
                if let ProtocolAction::Choose(label) = &trace[pos] {
                    for (l, s) in branches {
                        if l == label {
                            return s.validate_trace_inner(trace, pos + 1);
                        }
                    }
                }
                (false, pos)
            }
            SessionType::Offer(branches) => {
                if let ProtocolAction::Offer(label) = &trace[pos] {
                    for (l, s) in branches {
                        if l == label {
                            return s.validate_trace_inner(trace, pos + 1);
                        }
                    }
                }
                (false, pos)
            }
            SessionType::End => (pos >= trace.len(), pos),
            SessionType::Rec(_, body) => body.validate_trace_inner(trace, pos),
            SessionType::Var(_) => (true, pos), // recursion point
        }
    }

    /// Count the depth of the protocol.
    pub fn depth(&self) -> usize {
        match self {
            SessionType::Send(_, cont) | SessionType::Recv(_, cont) => 1 + cont.depth(),
            SessionType::Choose(branches) | SessionType::Offer(branches) => {
                1 + branches.iter().map(|(_, s)| s.depth()).max().unwrap_or(0)
            }
            SessionType::Rec(_, body) => 1 + body.depth(),
            SessionType::Var(_) | SessionType::End => 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ProtocolAction {
    Send(String),
    Recv(String),
    Choose(String),
    Offer(String),
}

/// A typed channel that tracks protocol state.
#[derive(Debug)]
pub struct SessionChannel {
    pub id: i64,
    pub protocol: SessionType,
    pub trace: Vec<ProtocolAction>,
    pub closed: bool,
}

impl SessionChannel {
    pub fn new(id: i64, protocol: SessionType) -> Self {
        Self { id, protocol, trace: Vec::new(), closed: false }
    }

    pub fn send(&mut self, ty: &str) -> bool {
        if self.closed { return false; }
        self.trace.push(ProtocolAction::Send(ty.to_string()));
        true
    }

    pub fn recv(&mut self, ty: &str) -> bool {
        if self.closed { return false; }
        self.trace.push(ProtocolAction::Recv(ty.to_string()));
        true
    }

    pub fn choose(&mut self, label: &str) -> bool {
        if self.closed { return false; }
        self.trace.push(ProtocolAction::Choose(label.to_string()));
        true
    }

    pub fn offer(&mut self, label: &str) -> bool {
        if self.closed { return false; }
        self.trace.push(ProtocolAction::Offer(label.to_string()));
        true
    }

    pub fn close(&mut self) -> bool {
        if self.closed { return false; }
        self.closed = true;
        self.protocol.validate_trace(&self.trace)
    }

    pub fn is_valid(&self) -> bool {
        self.protocol.validate_trace(&self.trace)
    }
}

static SESSION_STORE: LazyLock<Mutex<HashMap<i64, SessionChannel>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
static SESSION_NEXT_ID: LazyLock<Mutex<i64>> = LazyLock::new(|| Mutex::new(1));

fn session_alloc() -> i64 {
    let mut next = SESSION_NEXT_ID.lock().unwrap();
    let id = *next;
    *next += 1;
    id
}

pub extern "C" fn slang_session_create(protocol_depth: i64) -> i64 {
    let id = session_alloc();
    // Build a simple Send-Recv chain of given depth
    let mut proto = SessionType::End;
    for i in (0..protocol_depth).rev() {
        if i % 2 == 0 {
            proto = SessionType::Send("int".into(), Box::new(proto));
        } else {
            proto = SessionType::Recv("int".into(), Box::new(proto));
        }
    }
    let channel = SessionChannel::new(id, proto);
    SESSION_STORE.lock().unwrap().insert(id, channel);
    id
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_session_send(id: i64) -> i64 {
    let mut store = SESSION_STORE.lock().unwrap();
    if let Some(ch) = store.get_mut(&id) {
        if ch.send("int") { 1 } else { 0 }
    } else {
        -1
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_session_recv(id: i64) -> i64 {
    let mut store = SESSION_STORE.lock().unwrap();
    if let Some(ch) = store.get_mut(&id) {
        if ch.recv("int") { 1 } else { 0 }
    } else {
        -1
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_session_choose(id: i64, label: i64) -> i64 {
    let mut store = SESSION_STORE.lock().unwrap();
    if let Some(ch) = store.get_mut(&id) {
        let label_str = format!("branch_{}", label);
        if ch.choose(&label_str) { 1 } else { 0 }
    } else {
        -1
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_session_offer(id: i64, label: i64) -> i64 {
    let mut store = SESSION_STORE.lock().unwrap();
    if let Some(ch) = store.get_mut(&id) {
        let label_str = format!("branch_{}", label);
        if ch.offer(&label_str) { 1 } else { 0 }
    } else {
        -1
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_session_close(id: i64) -> i64 {
    let mut store = SESSION_STORE.lock().unwrap();
    if let Some(ch) = store.get_mut(&id) {
        if ch.close() { 1 } else { 0 }
    } else {
        -1
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_session_is_dual(a: i64, b: i64) -> i64 {
    let store = SESSION_STORE.lock().unwrap();
    if let (Some(ca), Some(cb)) = (store.get(&a), store.get(&b)) {
        if ca.protocol.is_dual(&cb.protocol) { 1 } else { 0 }
    } else {
        -1
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_session_validate(id: i64) -> i64 {
    let store = SESSION_STORE.lock().unwrap();
    if let Some(ch) = store.get(&id) {
        if ch.is_valid() { 1 } else { 0 }
    } else {
        -1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_send_recv_dual() {
        let s = SessionType::Send("int".into(), Box::new(SessionType::End));
        let r = SessionType::Recv("int".into(), Box::new(SessionType::End));
        assert!(s.is_dual(&r));
    }

    #[test]
    fn test_dual_symmetric() {
        let s = SessionType::Send("int".into(), Box::new(SessionType::End));
        let d = s.dual();
        assert_eq!(d.dual(), s);
    }

    #[test]
    fn test_end_dual() {
        assert!(SessionType::End.is_dual(&SessionType::End));
    }

    #[test]
    fn test_choose_offer_dual() {
        let c = SessionType::Choose(vec![
            ("a".into(), SessionType::End),
            ("b".into(), SessionType::End),
        ]);
        let o = SessionType::Offer(vec![
            ("a".into(), SessionType::End),
            ("b".into(), SessionType::End),
        ]);
        assert!(c.is_dual(&o));
    }

    #[test]
    fn test_not_dual() {
        let s1 = SessionType::Send("int".into(), Box::new(SessionType::End));
        let s2 = SessionType::Send("int".into(), Box::new(SessionType::End));
        assert!(!s1.is_dual(&s2));
    }

    #[test]
    fn test_validate_trace_send() {
        let proto = SessionType::Send("int".into(), Box::new(SessionType::End));
        let trace = vec![ProtocolAction::Send("int".into())];
        assert!(proto.validate_trace(&trace));
    }

    #[test]
    fn test_validate_trace_wrong_type() {
        let proto = SessionType::Send("int".into(), Box::new(SessionType::End));
        let trace = vec![ProtocolAction::Send("float".into())];
        assert!(!proto.validate_trace(&trace));
    }

    #[test]
    fn test_validate_trace_wrong_action() {
        let proto = SessionType::Send("int".into(), Box::new(SessionType::End));
        let trace = vec![ProtocolAction::Recv("int".into())];
        assert!(!proto.validate_trace(&trace));
    }

    #[test]
    fn test_validate_multi_step() {
        let proto = SessionType::Send("int".into(),
            Box::new(SessionType::Recv("bool".into(),
                Box::new(SessionType::End))));
        let trace = vec![
            ProtocolAction::Send("int".into()),
            ProtocolAction::Recv("bool".into()),
        ];
        assert!(proto.validate_trace(&trace));
    }

    #[test]
    fn test_validate_incomplete_trace() {
        let proto = SessionType::Send("int".into(),
            Box::new(SessionType::Recv("bool".into(),
                Box::new(SessionType::End))));
        let trace = vec![ProtocolAction::Send("int".into())];
        assert!(!proto.validate_trace(&trace));
    }

    #[test]
    fn test_channel_send() {
        let proto = SessionType::Send("int".into(), Box::new(SessionType::End));
        let mut ch = SessionChannel::new(1, proto);
        assert!(ch.send("int"));
    }

    #[test]
    fn test_channel_close_valid() {
        let proto = SessionType::Send("int".into(), Box::new(SessionType::End));
        let mut ch = SessionChannel::new(1, proto);
        ch.send("int");
        assert!(ch.close());
    }

    #[test]
    fn test_channel_close_invalid() {
        let proto = SessionType::Send("int".into(), Box::new(SessionType::End));
        let mut ch = SessionChannel::new(1, proto);
        ch.recv("int"); // wrong action
        assert!(!ch.close());
    }

    #[test]
    fn test_channel_double_close() {
        let proto = SessionType::End;
        let mut ch = SessionChannel::new(1, proto);
        ch.close();
        assert!(!ch.close()); // already closed
    }

    #[test]
    fn test_send_after_close() {
        let proto = SessionType::Send("int".into(), Box::new(SessionType::End));
        let mut ch = SessionChannel::new(1, proto);
        ch.close();
        assert!(!ch.send("int"));
    }

    #[test]
    fn test_depth_end() {
        assert_eq!(SessionType::End.depth(), 0);
    }

    #[test]
    fn test_depth_chain() {
        let proto = SessionType::Send("int".into(),
            Box::new(SessionType::Recv("bool".into(),
                Box::new(SessionType::End))));
        assert_eq!(proto.depth(), 2);
    }

    #[test]
    fn test_depth_choose() {
        let proto = SessionType::Choose(vec![
            ("a".into(), SessionType::Send("int".into(), Box::new(SessionType::End))),
            ("b".into(), SessionType::End),
        ]);
        assert_eq!(proto.depth(), 2); // 1 for Choose + 1 for Send
    }

    #[test]
    fn test_complex_protocol() {
        // Send int, choose branch, send bool, end
        let proto = SessionType::Send("int".into(),
            Box::new(SessionType::Choose(vec![
                ("ok".into(), SessionType::Send("bool".into(), Box::new(SessionType::End))),
                ("err".into(), SessionType::End),
            ])));
        let trace = vec![
            ProtocolAction::Send("int".into()),
            ProtocolAction::Choose("ok".into()),
            ProtocolAction::Send("bool".into()),
        ];
        assert!(proto.validate_trace(&trace));
    }
}
