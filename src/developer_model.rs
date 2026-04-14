//! Model of Developer Behavior — Vitalis v820
//!
//! Models developer behavior patterns to predict next actions and
//! estimate expertise level from observed coding actions.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<DeveloperModel>> = LazyLock::new(|| Mutex::new(DeveloperModel::new()));

pub struct DeveloperModel {
    actions: Vec<String>,
    expertise: f64,
}

impl DeveloperModel {
    pub fn new() -> Self {
        Self { actions: Vec::new(), expertise: 0.5 }
    }

    pub fn observe_action(&mut self, action: &str) {
        self.actions.push(action.to_string());
        if action.contains("refactor") || action.contains("test") {
            self.expertise = (self.expertise + 0.05).min(1.0);
        } else if action.contains("undo") {
            self.expertise = (self.expertise - 0.01).max(0.0);
        }
    }

    pub fn predict_next(&self) -> String {
        if let Some(last) = self.actions.last() {
            if last.contains("write") { return "test".to_string(); }
            if last.contains("test") { return "refactor".to_string(); }
        }
        "write".to_string()
    }

    pub fn expertise_level(&self) -> f64 {
        self.expertise
    }

    pub fn reset(&mut self) {
        self.actions.clear();
        self.expertise = 0.5;
    }
}

impl Default for DeveloperModel {
    fn default() -> Self { Self::new() }
}

#[unsafe(no_mangle)]
pub extern "C" fn dm_observe(action_id: i64) -> i64 {
    let action = match action_id % 4 {
        0 => "write",
        1 => "test",
        2 => "refactor",
        _ => "undo",
    };
    STATE.lock().unwrap().observe_action(action);
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn dm_predict() -> i64 {
    STATE.lock().unwrap().predict_next().len() as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn dm_expertise() -> f64 {
    STATE.lock().unwrap().expertise_level()
}

#[unsafe(no_mangle)]
pub extern "C" fn dm_reset() -> i64 {
    STATE.lock().unwrap().reset();
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_observe_increases_expertise_on_refactor() {
        let mut dm = DeveloperModel::new();
        let before = dm.expertise_level();
        dm.observe_action("refactor code");
        assert!(dm.expertise_level() > before);
    }

    #[test]
    fn test_predict_after_write() {
        let mut dm = DeveloperModel::new();
        dm.observe_action("write function");
        assert_eq!(dm.predict_next(), "test");
    }

    #[test]
    fn test_predict_after_test() {
        let mut dm = DeveloperModel::new();
        dm.observe_action("test module");
        assert_eq!(dm.predict_next(), "refactor");
    }

    #[test]
    fn test_expertise_bounded() {
        let mut dm = DeveloperModel::new();
        for _ in 0..100 {
            dm.observe_action("refactor");
        }
        assert!(dm.expertise_level() <= 1.0);
    }

    #[test]
    fn test_reset() {
        let mut dm = DeveloperModel::new();
        dm.observe_action("write");
        dm.reset();
        assert_eq!(dm.predict_next(), "write");
        assert_eq!(dm.expertise_level(), 0.5);
    }

    #[test]
    fn test_ffi_dm_observe_predict() {
        dm_observe(0);
        let len = dm_predict();
        assert!(len > 0);
    }
}
