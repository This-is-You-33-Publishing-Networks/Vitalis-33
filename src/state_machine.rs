//! State Machine — v371
//! Finite state machine builder with transitions, guards, hierarchical states.

use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Transition {
    pub from: i64,
    pub to: i64,
    pub event: i64,
    pub guard: Option<fn(i64) -> bool>,
}

#[derive(Debug)]
pub struct StateMachine {
    pub states: Vec<i64>,
    pub transitions: Vec<Transition>,
    pub current: i64,
    pub initial: i64,
    pub history: Vec<i64>,
    pub state_names: HashMap<i64, String>,
    pub on_enter: HashMap<i64, Vec<i64>>,
    pub on_exit: HashMap<i64, Vec<i64>>,
}

impl StateMachine {
    pub fn new(initial: i64) -> Self {
        Self {
            states: vec![initial],
            transitions: Vec::new(),
            current: initial,
            initial,
            history: vec![initial],
            state_names: HashMap::new(),
            on_enter: HashMap::new(),
            on_exit: HashMap::new(),
        }
    }

    pub fn add_state(&mut self, state: i64) -> bool {
        if self.states.contains(&state) {
            return false;
        }
        self.states.push(state);
        true
    }

    pub fn add_named_state(&mut self, state: i64, name: &str) -> bool {
        if !self.add_state(state) {
            return false;
        }
        self.state_names.insert(state, name.to_string());
        true
    }

    pub fn add_transition(&mut self, from: i64, event: i64, to: i64) -> bool {
        if !self.states.contains(&from) || !self.states.contains(&to) {
            return false;
        }
        self.transitions.push(Transition {
            from, to, event, guard: None,
        });
        true
    }

    pub fn trigger(&mut self, event: i64) -> bool {
        let current = self.current;
        for t in &self.transitions {
            if t.from == current && t.event == event {
                if let Some(guard) = t.guard {
                    if !guard(current) {
                        continue;
                    }
                }
                self.current = t.to;
                self.history.push(t.to);
                return true;
            }
        }
        false
    }

    pub fn can_trigger(&self, event: i64) -> bool {
        let current = self.current;
        self.transitions.iter().any(|t| t.from == current && t.event == event)
    }

    pub fn current_state(&self) -> i64 {
        self.current
    }

    pub fn reset(&mut self) {
        self.current = self.initial;
        self.history.clear();
        self.history.push(self.initial);
    }

    pub fn state_count(&self) -> i64 {
        self.states.len() as i64
    }

    pub fn transition_count(&self) -> i64 {
        self.transitions.len() as i64
    }

    pub fn history_len(&self) -> i64 {
        self.history.len() as i64
    }

    pub fn available_events(&self) -> Vec<i64> {
        let current = self.current;
        self.transitions.iter()
            .filter(|t| t.from == current)
            .map(|t| t.event)
            .collect()
    }

    pub fn is_terminal(&self) -> bool {
        let current = self.current;
        !self.transitions.iter().any(|t| t.from == current)
    }
}

use std::sync::Mutex;
use std::sync::LazyLock;
static FSM: LazyLock<Mutex<StateMachine>> = LazyLock::new(|| Mutex::new(StateMachine::new(0)));

#[unsafe(no_mangle)]
pub extern "C" fn slang_fsm_create(initial: i64) -> i64 {
    *FSM.lock().unwrap() = StateMachine::new(initial);
    1
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_fsm_add_state(state: i64) -> i64 {
    if FSM.lock().unwrap().add_state(state) { 1 } else { 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_fsm_add_transition(from: i64, event: i64, to: i64) -> i64 {
    if FSM.lock().unwrap().add_transition(from, event, to) { 1 } else { 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_fsm_trigger(event: i64) -> i64 {
    if FSM.lock().unwrap().trigger(event) { 1 } else { 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_fsm_current() -> i64 {
    FSM.lock().unwrap().current_state()
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_fsm_can_trigger(event: i64) -> i64 {
    if FSM.lock().unwrap().can_trigger(event) { 1 } else { 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_fsm_reset() -> i64 {
    FSM.lock().unwrap().reset();
    1
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_fsm_state_count() -> i64 {
    FSM.lock().unwrap().state_count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_fsm() {
        let fsm = StateMachine::new(0);
        assert_eq!(fsm.current_state(), 0);
        assert_eq!(fsm.state_count(), 1);
    }

    #[test]
    fn test_add_state() {
        let mut fsm = StateMachine::new(0);
        assert!(fsm.add_state(1));
        assert!(fsm.add_state(2));
        assert_eq!(fsm.state_count(), 3);
    }

    #[test]
    fn test_add_duplicate_state() {
        let mut fsm = StateMachine::new(0);
        assert!(!fsm.add_state(0));
    }

    #[test]
    fn test_add_transition() {
        let mut fsm = StateMachine::new(0);
        fsm.add_state(1);
        assert!(fsm.add_transition(0, 10, 1));
        assert_eq!(fsm.transition_count(), 1);
    }

    #[test]
    fn test_trigger() {
        let mut fsm = StateMachine::new(0);
        fsm.add_state(1);
        fsm.add_transition(0, 10, 1);
        assert!(fsm.trigger(10));
        assert_eq!(fsm.current_state(), 1);
    }

    #[test]
    fn test_trigger_invalid_event() {
        let mut fsm = StateMachine::new(0);
        fsm.add_state(1);
        fsm.add_transition(0, 10, 1);
        assert!(!fsm.trigger(99));
        assert_eq!(fsm.current_state(), 0);
    }

    #[test]
    fn test_can_trigger() {
        let mut fsm = StateMachine::new(0);
        fsm.add_state(1);
        fsm.add_transition(0, 10, 1);
        assert!(fsm.can_trigger(10));
        assert!(!fsm.can_trigger(99));
    }

    #[test]
    fn test_reset() {
        let mut fsm = StateMachine::new(0);
        fsm.add_state(1);
        fsm.add_transition(0, 10, 1);
        fsm.trigger(10);
        fsm.reset();
        assert_eq!(fsm.current_state(), 0);
    }

    #[test]
    fn test_history() {
        let mut fsm = StateMachine::new(0);
        fsm.add_state(1);
        fsm.add_state(2);
        fsm.add_transition(0, 10, 1);
        fsm.add_transition(1, 20, 2);
        fsm.trigger(10);
        fsm.trigger(20);
        assert_eq!(fsm.history, vec![0, 1, 2]);
    }

    #[test]
    fn test_history_len() {
        let mut fsm = StateMachine::new(0);
        assert_eq!(fsm.history_len(), 1);
        fsm.add_state(1);
        fsm.add_transition(0, 10, 1);
        fsm.trigger(10);
        assert_eq!(fsm.history_len(), 2);
    }

    #[test]
    fn test_multiple_transitions() {
        let mut fsm = StateMachine::new(0);
        fsm.add_state(1);
        fsm.add_state(2);
        fsm.add_transition(0, 1, 1);
        fsm.add_transition(0, 2, 2);
        fsm.trigger(2);
        assert_eq!(fsm.current_state(), 2);
    }

    #[test]
    fn test_transition_to_invalid_state() {
        let mut fsm = StateMachine::new(0);
        assert!(!fsm.add_transition(0, 1, 99));
    }

    #[test]
    fn test_available_events() {
        let mut fsm = StateMachine::new(0);
        fsm.add_state(1);
        fsm.add_state(2);
        fsm.add_transition(0, 10, 1);
        fsm.add_transition(0, 20, 2);
        let events = fsm.available_events();
        assert_eq!(events.len(), 2);
        assert!(events.contains(&10));
        assert!(events.contains(&20));
    }

    #[test]
    fn test_is_terminal() {
        let mut fsm = StateMachine::new(0);
        fsm.add_state(1);
        fsm.add_transition(0, 10, 1);
        assert!(!fsm.is_terminal());
        fsm.trigger(10);
        assert!(fsm.is_terminal());
    }

    #[test]
    fn test_self_transition() {
        let mut fsm = StateMachine::new(0);
        fsm.add_transition(0, 1, 0);
        assert!(fsm.trigger(1));
        assert_eq!(fsm.current_state(), 0);
        assert_eq!(fsm.history_len(), 2);
    }

    #[test]
    fn test_named_state() {
        let mut fsm = StateMachine::new(0);
        fsm.add_named_state(1, "running");
        assert_eq!(fsm.state_names[&1], "running");
    }

    #[test]
    fn test_chain_transitions() {
        let mut fsm = StateMachine::new(0);
        for i in 1..=5 {
            fsm.add_state(i);
            fsm.add_transition(i - 1, i, i);
        }
        for i in 1..=5 {
            assert!(fsm.trigger(i));
        }
        assert_eq!(fsm.current_state(), 5);
    }

    #[test]
    fn test_reset_clears_history() {
        let mut fsm = StateMachine::new(0);
        fsm.add_state(1);
        fsm.add_transition(0, 1, 1);
        fsm.trigger(1);
        fsm.reset();
        assert_eq!(fsm.history_len(), 1);
        assert_eq!(fsm.history[0], 0);
    }

    #[test]
    fn test_no_events_from_terminal() {
        let mut fsm = StateMachine::new(0);
        fsm.add_state(1);
        fsm.add_transition(0, 1, 1);
        fsm.trigger(1);
        assert!(fsm.available_events().is_empty());
    }
}
