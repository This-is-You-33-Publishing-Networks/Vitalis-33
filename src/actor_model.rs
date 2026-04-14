//! Actor Model — v367
//! Lightweight actor system with mailboxes, supervision trees, and message passing.

use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, Ordering};

static NEXT_ACTOR_ID: AtomicI64 = AtomicI64::new(1);

#[derive(Debug, Clone, PartialEq)]
pub enum ActorState {
    Running,
    Stopped,
    Failed,
}

#[derive(Debug, Clone)]
pub struct Message {
    pub sender: i64,
    pub payload: i64,
    pub timestamp: i64,
}

#[derive(Debug)]
pub struct Actor {
    pub id: i64,
    pub state: ActorState,
    pub mailbox: Vec<Message>,
    pub supervisor: Option<i64>,
    pub children: Vec<i64>,
    pub processed_count: i64,
}

impl Actor {
    pub fn new(id: i64) -> Self {
        Self {
            id,
            state: ActorState::Running,
            mailbox: Vec::new(),
            supervisor: None,
            children: Vec::new(),
            processed_count: 0,
        }
    }

    pub fn send(&mut self, msg: Message) {
        if self.state == ActorState::Running {
            self.mailbox.push(msg);
        }
    }

    pub fn receive(&mut self) -> Option<Message> {
        if self.state == ActorState::Running && !self.mailbox.is_empty() {
            self.processed_count += 1;
            Some(self.mailbox.remove(0))
        } else {
            None
        }
    }

    pub fn stop(&mut self) {
        self.state = ActorState::Stopped;
    }

    pub fn is_alive(&self) -> bool {
        self.state == ActorState::Running
    }
}

#[derive(Debug, Default)]
pub struct ActorSystem {
    pub actors: HashMap<i64, Actor>,
}

impl ActorSystem {
    pub fn new() -> Self {
        Self { actors: HashMap::new() }
    }

    pub fn spawn(&mut self) -> i64 {
        let id = NEXT_ACTOR_ID.fetch_add(1, Ordering::Relaxed);
        self.actors.insert(id, Actor::new(id));
        id
    }

    pub fn send(&mut self, target: i64, sender: i64, payload: i64) -> bool {
        if let Some(actor) = self.actors.get_mut(&target) {
            actor.send(Message { sender, payload, timestamp: 0 });
            true
        } else {
            false
        }
    }

    pub fn receive(&mut self, actor_id: i64) -> Option<Message> {
        self.actors.get_mut(&actor_id).and_then(|a| a.receive())
    }

    pub fn stop(&mut self, actor_id: i64) -> bool {
        if let Some(actor) = self.actors.get_mut(&actor_id) {
            actor.stop();
            true
        } else {
            false
        }
    }

    pub fn supervise(&mut self, supervisor_id: i64, child_id: i64) -> bool {
        if self.actors.contains_key(&supervisor_id) && self.actors.contains_key(&child_id) {
            if let Some(child) = self.actors.get_mut(&child_id) {
                child.supervisor = Some(supervisor_id);
            }
            if let Some(sup) = self.actors.get_mut(&supervisor_id) {
                sup.children.push(child_id);
            }
            true
        } else {
            false
        }
    }

    pub fn mailbox_size(&self, actor_id: i64) -> i64 {
        self.actors.get(&actor_id).map(|a| a.mailbox.len() as i64).unwrap_or(-1)
    }

    pub fn actor_count(&self) -> i64 {
        self.actors.values().filter(|a| a.is_alive()).count() as i64
    }

    pub fn is_alive(&self, actor_id: i64) -> bool {
        self.actors.get(&actor_id).map(|a| a.is_alive()).unwrap_or(false)
    }
}

use std::sync::Mutex;
use std::sync::LazyLock;
static ACTOR_SYS: LazyLock<Mutex<ActorSystem>> = LazyLock::new(|| Mutex::new(ActorSystem::new()));

pub extern "C" fn slang_actor_spawn() -> i64 {
    ACTOR_SYS.lock().unwrap().spawn()
}

pub extern "C" fn slang_actor_send(target: i64, sender: i64, payload: i64) -> i64 {
    if ACTOR_SYS.lock().unwrap().send(target, sender, payload) { 1 } else { 0 }
}

pub extern "C" fn slang_actor_recv(actor_id: i64) -> i64 {
    ACTOR_SYS.lock().unwrap().receive(actor_id).map(|m| m.payload).unwrap_or(-1)
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_actor_stop(actor_id: i64) -> i64 {
    if ACTOR_SYS.lock().unwrap().stop(actor_id) { 1 } else { 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_actor_count() -> i64 {
    ACTOR_SYS.lock().unwrap().actor_count()
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_actor_mailbox_size(actor_id: i64) -> i64 {
    ACTOR_SYS.lock().unwrap().mailbox_size(actor_id)
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_actor_is_alive(actor_id: i64) -> i64 {
    if ACTOR_SYS.lock().unwrap().is_alive(actor_id) { 1 } else { 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_actor_supervise(supervisor: i64, child: i64) -> i64 {
    if ACTOR_SYS.lock().unwrap().supervise(supervisor, child) { 1 } else { 0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_actor_new() {
        let actor = Actor::new(1);
        assert_eq!(actor.id, 1);
        assert_eq!(actor.state, ActorState::Running);
        assert!(actor.mailbox.is_empty());
    }

    #[test]
    fn test_actor_send_receive() {
        let mut actor = Actor::new(1);
        actor.send(Message { sender: 0, payload: 42, timestamp: 0 });
        let msg = actor.receive().unwrap();
        assert_eq!(msg.payload, 42);
    }

    #[test]
    fn test_actor_stop() {
        let mut actor = Actor::new(1);
        actor.stop();
        assert_eq!(actor.state, ActorState::Stopped);
        assert!(!actor.is_alive());
    }

    #[test]
    fn test_stopped_actor_rejects_messages() {
        let mut actor = Actor::new(1);
        actor.stop();
        actor.send(Message { sender: 0, payload: 1, timestamp: 0 });
        assert!(actor.mailbox.is_empty());
    }

    #[test]
    fn test_empty_receive() {
        let mut actor = Actor::new(1);
        assert!(actor.receive().is_none());
    }

    #[test]
    fn test_system_spawn() {
        let mut sys = ActorSystem::new();
        let id1 = sys.spawn();
        let id2 = sys.spawn();
        assert_ne!(id1, id2);
        assert!(sys.is_alive(id1));
        assert!(sys.is_alive(id2));
    }

    #[test]
    fn test_system_send_receive() {
        let mut sys = ActorSystem::new();
        let a1 = sys.spawn();
        let a2 = sys.spawn();
        assert!(sys.send(a2, a1, 99));
        let msg = sys.receive(a2).unwrap();
        assert_eq!(msg.payload, 99);
        assert_eq!(msg.sender, a1);
    }

    #[test]
    fn test_system_stop() {
        let mut sys = ActorSystem::new();
        let id = sys.spawn();
        assert!(sys.stop(id));
        assert!(!sys.is_alive(id));
    }

    #[test]
    fn test_system_actor_count() {
        let mut sys = ActorSystem::new();
        sys.spawn();
        sys.spawn();
        let id3 = sys.spawn();
        assert_eq!(sys.actor_count(), 3);
        sys.stop(id3);
        assert_eq!(sys.actor_count(), 2);
    }

    #[test]
    fn test_system_mailbox_size() {
        let mut sys = ActorSystem::new();
        let id = sys.spawn();
        assert_eq!(sys.mailbox_size(id), 0);
        sys.send(id, 0, 10);
        sys.send(id, 0, 20);
        assert_eq!(sys.mailbox_size(id), 2);
    }

    #[test]
    fn test_send_to_nonexistent() {
        let mut sys = ActorSystem::new();
        assert!(!sys.send(999, 0, 1));
    }

    #[test]
    fn test_supervision() {
        let mut sys = ActorSystem::new();
        let sup = sys.spawn();
        let child = sys.spawn();
        assert!(sys.supervise(sup, child));
        assert_eq!(sys.actors[&child].supervisor, Some(sup));
        assert!(sys.actors[&sup].children.contains(&child));
    }

    #[test]
    fn test_supervise_nonexistent() {
        let mut sys = ActorSystem::new();
        let id = sys.spawn();
        assert!(!sys.supervise(id, 999));
    }

    #[test]
    fn test_fifo_ordering() {
        let mut actor = Actor::new(1);
        for i in 0..5 {
            actor.send(Message { sender: 0, payload: i, timestamp: 0 });
        }
        for i in 0..5 {
            assert_eq!(actor.receive().unwrap().payload, i);
        }
    }

    #[test]
    fn test_processed_count() {
        let mut actor = Actor::new(1);
        actor.send(Message { sender: 0, payload: 1, timestamp: 0 });
        actor.send(Message { sender: 0, payload: 2, timestamp: 0 });
        actor.receive();
        actor.receive();
        assert_eq!(actor.processed_count, 2);
    }

    #[test]
    fn test_receive_nonexistent_actor() {
        let mut sys = ActorSystem::new();
        assert!(sys.receive(999).is_none());
    }

    #[test]
    fn test_multiple_messages() {
        let mut sys = ActorSystem::new();
        let target = sys.spawn();
        for i in 0..10 {
            sys.send(target, 0, i);
        }
        assert_eq!(sys.mailbox_size(target), 10);
    }

    #[test]
    fn test_mailbox_unknown_actor() {
        let sys = ActorSystem::new();
        assert_eq!(sys.mailbox_size(999), -1);
    }

    #[test]
    fn test_is_alive_unknown() {
        let sys = ActorSystem::new();
        assert!(!sys.is_alive(999));
    }

    #[test]
    fn test_stop_unknown() {
        let mut sys = ActorSystem::new();
        assert!(!sys.stop(999));
    }
}
