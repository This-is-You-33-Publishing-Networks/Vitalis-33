//! Timeline Surgery — Vitalis v1087
//!
//! Precise surgical edits to program execution timelines
//! to fix bugs retroactively without full recompilation.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<TsState>> = LazyLock::new(|| Mutex::new(TsState::default()));

#[derive(Default)]
struct TsState {
    edits: Vec<(u64, String, String)>,
    timeline_length: u64,
    successful_surgeries: u64,
}

pub struct TimelineSurgery;

impl TimelineSurgery {
    pub fn mark_event(timestamp: u64, event: &str) {
        let mut s = STATE.lock().unwrap();
        s.edits.push((timestamp, event.to_string(), String::new()));
        if timestamp > s.timeline_length { s.timeline_length = timestamp; }
    }

    pub fn surgical_edit(timestamp: u64, original: &str, replacement: &str) -> bool {
        let mut s = STATE.lock().unwrap();
        if let Some(e) = s.edits.iter_mut().find(|(t, ev, _)| *t == timestamp && ev == original) {
            e.2 = replacement.to_string();
            s.successful_surgeries += 1;
            true
        } else { false }
    }

    pub fn verify_timeline() -> bool {
        let s = STATE.lock().unwrap();
        let mut timestamps: Vec<u64> = s.edits.iter().map(|(t, _, _)| *t).collect();
        timestamps.sort();
        timestamps.windows(2).all(|w| w[0] <= w[1])
    }

    pub fn surgery_count() -> u64 { STATE.lock().unwrap().successful_surgeries }
    pub fn timeline_length() -> u64 { STATE.lock().unwrap().timeline_length }
    pub fn event_count() -> usize { STATE.lock().unwrap().edits.len() }
    pub fn reset() { *STATE.lock().unwrap() = TsState::default(); }
}

#[unsafe(no_mangle)]
pub extern "C" fn ts_mark(timestamp: i64) -> i64 { TimelineSurgery::mark_event(timestamp as u64, "event"); timestamp }
#[unsafe(no_mangle)]
pub extern "C" fn ts_edit(timestamp: i64) -> i64 { if TimelineSurgery::surgical_edit(timestamp as u64, "event", "fixed") { 1 } else { 0 } }
#[unsafe(no_mangle)]
pub extern "C" fn ts_surgeries() -> i64 { TimelineSurgery::surgery_count() as i64 }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_mark() { TimelineSurgery::reset(); TimelineSurgery::mark_event(1, "start"); assert_eq!(TimelineSurgery::event_count(), 1); }
    #[test] fn test_edit() { TimelineSurgery::reset(); TimelineSurgery::mark_event(5, "bug"); assert!(TimelineSurgery::surgical_edit(5, "bug", "fix")); }
    #[test] fn test_edit_miss() { TimelineSurgery::reset(); assert!(!TimelineSurgery::surgical_edit(99, "nope", "fix")); }
    #[test] fn test_verify() { TimelineSurgery::reset(); TimelineSurgery::mark_event(1, "a"); TimelineSurgery::mark_event(2, "b"); assert!(TimelineSurgery::verify_timeline()); }
    #[test] fn test_surgery_count() { TimelineSurgery::reset(); TimelineSurgery::mark_event(1, "bug"); TimelineSurgery::surgical_edit(1, "bug", "fix"); assert_eq!(TimelineSurgery::surgery_count(), 1); }
    #[test] fn test_timeline_length() { TimelineSurgery::reset(); TimelineSurgery::mark_event(10, "x"); assert_eq!(TimelineSurgery::timeline_length(), 10); }
    #[test] fn test_ffi() { TimelineSurgery::reset(); assert_eq!(ts_surgeries(), 0); }
}
