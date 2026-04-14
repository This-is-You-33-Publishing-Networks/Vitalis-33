//! Temporal Reasoning — Vitalis v608
//!
//! Reasons about time-dependent program behavior:
//! - Event ordering and happens-before relations
//! - Temporal logic model checking (LTL-like properties)
//! - Sequence pattern detection (init-before-use, open-close pairs)
//! - Temporal constraint satisfaction
//! - History-aware state tracking

use std::collections::{HashMap, HashSet, VecDeque};

// ── Events & Ordering ────────────────────────────────────────────────

/// A timestamped program event.
#[derive(Debug, Clone, PartialEq)]
pub struct Event {
    pub id: u32,
    pub kind: EventKind,
    pub timestamp: u64,
    pub location: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum EventKind {
    Init(String),       // Variable/resource initialization
    Use(String),        // Variable/resource use
    Mutate(String),     // Mutation
    Release(String),    // Deallocation/close
    Acquire(String),    // Lock acquire
    Signal(String),     // Signal / notify
    Wait(String),       // Wait / block
    Call(String),       // Function call
    Return(String),     // Function return
}

impl EventKind {
    pub fn resource_name(&self) -> &str {
        match self {
            EventKind::Init(s) | EventKind::Use(s) | EventKind::Mutate(s) |
            EventKind::Release(s) | EventKind::Acquire(s) | EventKind::Signal(s) |
            EventKind::Wait(s) | EventKind::Call(s) | EventKind::Return(s) => s,
        }
    }
}

/// Happens-before relation tracker.
pub struct HappensBefore {
    edges: HashMap<u32, HashSet<u32>>, // id → set of ids that happen after
    event_count: u32,
}

impl HappensBefore {
    pub fn new() -> Self {
        Self { edges: HashMap::new(), event_count: 0 }
    }

    pub fn add_order(&mut self, before: u32, after: u32) {
        self.edges.entry(before).or_default().insert(after);
        self.event_count = self.event_count.max(before + 1).max(after + 1);
    }

    /// Check if `a` happens before `b` (transitive).
    pub fn happens_before(&self, a: u32, b: u32) -> bool {
        if a == b { return false; }
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        queue.push_back(a);
        visited.insert(a);
        while let Some(current) = queue.pop_front() {
            if let Some(successors) = self.edges.get(&current) {
                for &succ in successors {
                    if succ == b { return true; }
                    if visited.insert(succ) {
                        queue.push_back(succ);
                    }
                }
            }
        }
        false
    }

    /// Check if two events are concurrent (neither happens before the other).
    pub fn are_concurrent(&self, a: u32, b: u32) -> bool {
        !self.happens_before(a, b) && !self.happens_before(b, a)
    }

    pub fn successors(&self, id: u32) -> Vec<u32> {
        self.edges.get(&id).map(|s| s.iter().copied().collect()).unwrap_or_default()
    }

    pub fn edge_count(&self) -> usize {
        self.edges.values().map(|s| s.len()).sum()
    }
}

impl Default for HappensBefore {
    fn default() -> Self { Self::new() }
}

// ── Temporal Properties ──────────────────────────────────────────────

/// A simple temporal property to check.
#[derive(Debug, Clone)]
pub enum TemporalProperty {
    /// `a` must happen before `b`
    Precedes { before: EventKind, after: EventKind },
    /// `a` and `b` must be paired (e.g., open/close)
    Paired { open: EventKind, close: EventKind },
    /// `a` must not happen after `b`
    NeverAfter { event: EventKind, barrier: EventKind },
    /// `a` must eventually happen
    Eventually { event: EventKind },
    /// `a` must always be true at every point
    Always { event: EventKind },
}

#[derive(Debug, Clone, PartialEq)]
pub enum PropertyResult {
    Satisfied,
    Violated(String),
    Unknown,
}

/// Check a temporal property against an event trace.
pub fn check_property(events: &[Event], property: &TemporalProperty) -> PropertyResult {
    match property {
        TemporalProperty::Precedes { before, after } => {
            let before_idx = events.iter().position(|e| e.kind == *before);
            let after_idx = events.iter().position(|e| e.kind == *after);
            match (before_idx, after_idx) {
                (Some(b), Some(a)) if b < a => PropertyResult::Satisfied,
                (Some(b), Some(a)) if b >= a => PropertyResult::Violated(
                    format!("{:?} at {} not before {:?} at {}", before, b, after, a)
                ),
                (None, Some(_)) => PropertyResult::Violated(
                    format!("{:?} never occurs before {:?}", before, after)
                ),
                _ => PropertyResult::Unknown,
            }
        }

        TemporalProperty::Paired { open, close } => {
            let open_count = events.iter().filter(|e| e.kind == *open).count();
            let close_count = events.iter().filter(|e| e.kind == *close).count();
            if open_count == close_count {
                // Verify ordering: each open has a matching close after it
                let mut depth = 0i64;
                for e in events {
                    if e.kind == *open { depth += 1; }
                    if e.kind == *close { depth -= 1; }
                    if depth < 0 {
                        return PropertyResult::Violated("close without matching open".into());
                    }
                }
                PropertyResult::Satisfied
            } else {
                PropertyResult::Violated(format!("open={} close={}", open_count, close_count))
            }
        }

        TemporalProperty::NeverAfter { event, barrier } => {
            let barrier_idx = events.iter().position(|e| e.kind == *barrier);
            if let Some(bi) = barrier_idx {
                let after_barrier = events[bi+1..].iter().any(|e| e.kind == *event);
                if after_barrier {
                    PropertyResult::Violated(format!("{:?} occurs after {:?}", event, barrier))
                } else {
                    PropertyResult::Satisfied
                }
            } else {
                PropertyResult::Satisfied // barrier never occurs, property vacuously true
            }
        }

        TemporalProperty::Eventually { event } => {
            if events.iter().any(|e| e.kind == *event) {
                PropertyResult::Satisfied
            } else {
                PropertyResult::Violated(format!("{:?} never occurs", event))
            }
        }

        TemporalProperty::Always { event } => {
            // Simplified: check the event exists (for traces, "always" is approximate)
            if events.iter().any(|e| e.kind == *event) {
                PropertyResult::Satisfied
            } else {
                PropertyResult::Violated(format!("{:?} not found in trace", event))
            }
        }
    }
}

// ── Sequence Pattern Detection ───────────────────────────────────────

/// Detect init-before-use violations.
pub fn detect_use_before_init(events: &[Event]) -> Vec<String> {
    let mut initialized: HashSet<String> = HashSet::new();
    let mut violations = Vec::new();

    for e in events {
        match &e.kind {
            EventKind::Init(name) => { initialized.insert(name.clone()); }
            EventKind::Use(name) | EventKind::Mutate(name) => {
                if !initialized.contains(name) {
                    violations.push(format!("use-before-init: {} at {}", name, e.location));
                }
            }
            _ => {}
        }
    }
    violations
}

/// Detect resource leak (acquire without release).
pub fn detect_resource_leaks(events: &[Event]) -> Vec<String> {
    let mut acquired: HashMap<String, u32> = HashMap::new();
    let mut leaks = Vec::new();

    for e in events {
        match &e.kind {
            EventKind::Acquire(name) | EventKind::Init(name) => {
                acquired.insert(name.clone(), e.id);
            }
            EventKind::Release(name) => {
                acquired.remove(name);
            }
            _ => {}
        }
    }

    for (name, _id) in acquired {
        leaks.push(format!("resource leak: {} not released", name));
    }
    leaks
}

// ── FFI Exports ──────────────────────────────────────────────────────

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_temporal_happens_before(
    edges_from: *const i64, edges_to: *const i64, edge_count: i64,
    query_a: i64, query_b: i64,
) -> i64 {
    if edges_from.is_null() || edges_to.is_null() || edge_count <= 0 { return 0; }
    let from = unsafe { std::slice::from_raw_parts(edges_from, edge_count as usize) };
    let to = unsafe { std::slice::from_raw_parts(edges_to, edge_count as usize) };

    let mut hb = HappensBefore::new();
    for i in 0..edge_count as usize {
        hb.add_order(from[i] as u32, to[i] as u32);
    }
    if hb.happens_before(query_a as u32, query_b as u32) { 1 } else { 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_temporal_are_concurrent(
    edges_from: *const i64, edges_to: *const i64, edge_count: i64,
    query_a: i64, query_b: i64,
) -> i64 {
    if edges_from.is_null() || edges_to.is_null() || edge_count <= 0 { return 1; }
    let from = unsafe { std::slice::from_raw_parts(edges_from, edge_count as usize) };
    let to = unsafe { std::slice::from_raw_parts(edges_to, edge_count as usize) };

    let mut hb = HappensBefore::new();
    for i in 0..edge_count as usize {
        hb.add_order(from[i] as u32, to[i] as u32);
    }
    if hb.are_concurrent(query_a as u32, query_b as u32) { 1 } else { 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_temporal_detect_leaks(
    init_ids: *const i64, init_count: i64,
    release_ids: *const i64, release_count: i64,
) -> i64 {
    if init_ids.is_null() || init_count < 0 { return 0; }
    let inits = unsafe { std::slice::from_raw_parts(init_ids, init_count as usize) };
    let releases = if release_ids.is_null() || release_count <= 0 {
        &[]
    } else {
        unsafe { std::slice::from_raw_parts(release_ids, release_count as usize) }
    };

    let release_set: HashSet<i64> = releases.iter().copied().collect();
    inits.iter().filter(|id| !release_set.contains(id)).count() as i64
}

// ═══════════════════════════════════════════════════════════════════════
// Tests
// ═══════════════════════════════════════════════════════════════════════
#[cfg(test)]
mod tests {
    use super::*;

    fn mk_event(id: u32, kind: EventKind) -> Event {
        Event { id, kind, timestamp: id as u64, location: format!("loc{}", id) }
    }

    #[test]
    fn test_happens_before_simple() {
        let mut hb = HappensBefore::new();
        hb.add_order(0, 1);
        hb.add_order(1, 2);
        assert!(hb.happens_before(0, 2)); // transitive
        assert!(!hb.happens_before(2, 0));
    }

    #[test]
    fn test_happens_before_no_relation() {
        let mut hb = HappensBefore::new();
        hb.add_order(0, 1);
        hb.add_order(2, 3);
        assert!(!hb.happens_before(0, 3));
    }

    #[test]
    fn test_concurrent() {
        let mut hb = HappensBefore::new();
        hb.add_order(0, 2);
        hb.add_order(1, 3);
        assert!(hb.are_concurrent(0, 1));
        assert!(!hb.are_concurrent(0, 2));
    }

    #[test]
    fn test_precedes_satisfied() {
        let events = vec![
            mk_event(0, EventKind::Init("x".into())),
            mk_event(1, EventKind::Use("x".into())),
        ];
        let prop = TemporalProperty::Precedes {
            before: EventKind::Init("x".into()),
            after: EventKind::Use("x".into()),
        };
        assert_eq!(check_property(&events, &prop), PropertyResult::Satisfied);
    }

    #[test]
    fn test_precedes_violated() {
        let events = vec![
            mk_event(0, EventKind::Use("x".into())),
            mk_event(1, EventKind::Init("x".into())),
        ];
        let prop = TemporalProperty::Precedes {
            before: EventKind::Init("x".into()),
            after: EventKind::Use("x".into()),
        };
        match check_property(&events, &prop) {
            PropertyResult::Violated(_) => {},
            other => panic!("expected Violated, got {:?}", other),
        }
    }

    #[test]
    fn test_paired_balanced() {
        let events = vec![
            mk_event(0, EventKind::Acquire("lock".into())),
            mk_event(1, EventKind::Release("lock".into())),
        ];
        let prop = TemporalProperty::Paired {
            open: EventKind::Acquire("lock".into()),
            close: EventKind::Release("lock".into()),
        };
        assert_eq!(check_property(&events, &prop), PropertyResult::Satisfied);
    }

    #[test]
    fn test_paired_unbalanced() {
        let events = vec![
            mk_event(0, EventKind::Acquire("lock".into())),
        ];
        let prop = TemporalProperty::Paired {
            open: EventKind::Acquire("lock".into()),
            close: EventKind::Release("lock".into()),
        };
        match check_property(&events, &prop) {
            PropertyResult::Violated(_) => {},
            other => panic!("expected Violated, got {:?}", other),
        }
    }

    #[test]
    fn test_never_after_satisfied() {
        let events = vec![
            mk_event(0, EventKind::Use("x".into())),
            mk_event(1, EventKind::Release("x".into())),
        ];
        let prop = TemporalProperty::NeverAfter {
            event: EventKind::Use("x".into()),
            barrier: EventKind::Release("x".into()),
        };
        assert_eq!(check_property(&events, &prop), PropertyResult::Satisfied);
    }

    #[test]
    fn test_eventually_satisfied() {
        let events = vec![
            mk_event(0, EventKind::Init("x".into())),
            mk_event(1, EventKind::Release("x".into())),
        ];
        let prop = TemporalProperty::Eventually { event: EventKind::Release("x".into()) };
        assert_eq!(check_property(&events, &prop), PropertyResult::Satisfied);
    }

    #[test]
    fn test_eventually_violated() {
        let events = vec![mk_event(0, EventKind::Init("x".into()))];
        let prop = TemporalProperty::Eventually { event: EventKind::Release("x".into()) };
        match check_property(&events, &prop) {
            PropertyResult::Violated(_) => {},
            other => panic!("expected Violated, got {:?}", other),
        }
    }

    #[test]
    fn test_use_before_init() {
        let events = vec![
            mk_event(0, EventKind::Use("x".into())),
            mk_event(1, EventKind::Init("x".into())),
            mk_event(2, EventKind::Use("x".into())),
        ];
        let violations = detect_use_before_init(&events);
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn test_no_use_before_init() {
        let events = vec![
            mk_event(0, EventKind::Init("x".into())),
            mk_event(1, EventKind::Use("x".into())),
        ];
        assert!(detect_use_before_init(&events).is_empty());
    }

    #[test]
    fn test_resource_leaks() {
        let events = vec![
            mk_event(0, EventKind::Acquire("file".into())),
            mk_event(1, EventKind::Acquire("socket".into())),
            mk_event(2, EventKind::Release("file".into())),
        ];
        let leaks = detect_resource_leaks(&events);
        assert_eq!(leaks.len(), 1);
        assert!(leaks[0].contains("socket"));
    }

    #[test]
    fn test_no_resource_leaks() {
        let events = vec![
            mk_event(0, EventKind::Acquire("file".into())),
            mk_event(1, EventKind::Release("file".into())),
        ];
        assert!(detect_resource_leaks(&events).is_empty());
    }

    #[test]
    fn test_ffi_happens_before() {
        let from = [0i64, 1];
        let to = [1i64, 2];
        assert_eq!(vitalis_temporal_happens_before(from.as_ptr(), to.as_ptr(), 2, 0, 2), 1);
        assert_eq!(vitalis_temporal_happens_before(from.as_ptr(), to.as_ptr(), 2, 2, 0), 0);
    }

    #[test]
    fn test_ffi_concurrent() {
        let from = [0i64];
        let to = [2i64];
        assert_eq!(vitalis_temporal_are_concurrent(from.as_ptr(), to.as_ptr(), 1, 0, 1), 1);
    }

    #[test]
    fn test_ffi_detect_leaks() {
        let inits = [1i64, 2, 3];
        let releases = [1i64, 3];
        assert_eq!(vitalis_temporal_detect_leaks(inits.as_ptr(), 3, releases.as_ptr(), 2), 1);
    }
}
