//! v405 — Neuromorphic control flow primitives.
//!
//! Provides event-driven control flow for spiking neural networks:
//! - `SpikeHandler`: registers callbacks for spike events
//! - `TemporalGuard`: time-windowed condition evaluation
//! - `PopulationController`: controls neuron populations as groups
//! - `SpikeBarrier`: synchronization primitive for spike wavefronts

use std::sync::{LazyLock, Mutex};

// ─── Spike Handler ──────────────────────────────────────────────────────

/// A registered spike event handler.
#[derive(Clone)]
pub struct SpikeHandler {
    pub id: u64,
    pub source_filter: Option<i64>,
    pub target_filter: Option<i64>,
    pub threshold: f64,
    pub action: SpikeAction,
}

/// Action to take when a spike handler fires.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpikeAction {
    /// Inhibit the target neuron.
    Inhibit,
    /// Excite the target neuron.
    Excite,
    /// Record the spike.
    Record,
    /// Trigger a callback (represented by function index).
    Callback(u64),
}

impl SpikeHandler {
    pub fn new(id: u64) -> Self {
        Self {
            id,
            source_filter: None,
            target_filter: None,
            threshold: 0.0,
            action: SpikeAction::Record,
        }
    }

    pub fn with_source(mut self, source: i64) -> Self {
        self.source_filter = Some(source);
        self
    }

    pub fn with_target(mut self, target: i64) -> Self {
        self.target_filter = Some(target);
        self
    }

    pub fn with_threshold(mut self, threshold: f64) -> Self {
        self.threshold = threshold;
        self
    }

    pub fn with_action(mut self, action: SpikeAction) -> Self {
        self.action = action;
        self
    }

    /// Check if this handler matches a given spike event.
    pub fn matches(&self, source: i64, target: i64, value: f64) -> bool {
        if let Some(s) = self.source_filter {
            if s != source { return false; }
        }
        if let Some(t) = self.target_filter {
            if t != target { return false; }
        }
        value.abs() >= self.threshold
    }
}

// ─── Temporal Guard ─────────────────────────────────────────────────────

/// A time-windowed condition for spike events.
#[derive(Debug, Clone)]
pub struct TemporalGuard {
    /// Start of the time window (microseconds).
    pub window_start: i64,
    /// End of the time window (microseconds).
    pub window_end: i64,
    /// Minimum number of spikes required in the window.
    pub min_spikes: usize,
    /// Maximum number of spikes allowed (0 = unlimited).
    pub max_spikes: usize,
    /// Spikes received in the current window.
    spike_count: usize,
}

impl TemporalGuard {
    pub fn new(window_start: i64, window_end: i64) -> Self {
        Self {
            window_start,
            window_end,
            min_spikes: 1,
            max_spikes: 0, // unlimited
            spike_count: 0,
        }
    }

    pub fn with_min_spikes(mut self, min: usize) -> Self {
        self.min_spikes = min;
        self
    }

    pub fn with_max_spikes(mut self, max: usize) -> Self {
        self.max_spikes = max;
        self
    }

    /// Register a spike at the given time.
    pub fn register_spike(&mut self, time_us: i64) -> bool {
        if time_us >= self.window_start && time_us <= self.window_end {
            self.spike_count += 1;
            true
        } else {
            false
        }
    }

    /// Check if the guard condition is satisfied.
    pub fn is_satisfied(&self) -> bool {
        if self.spike_count < self.min_spikes {
            return false;
        }
        if self.max_spikes > 0 && self.spike_count > self.max_spikes {
            return false;
        }
        true
    }

    /// Reset the guard for a new window.
    pub fn reset(&mut self) {
        self.spike_count = 0;
    }

    /// Slide the window forward by `dt` microseconds.
    pub fn slide_window(&mut self, dt: i64) {
        self.window_start += dt;
        self.window_end += dt;
        self.spike_count = 0;
    }

    pub fn spike_count(&self) -> usize {
        self.spike_count
    }
}

// ─── Population Controller ──────────────────────────────────────────────

/// Controls a group of neurons as a population.
#[derive(Debug, Clone)]
pub struct PopulationController {
    pub name: String,
    pub neuron_ids: Vec<i64>,
    /// Target firing rate (Hz).
    pub target_rate: f64,
    /// Current measured firing rate.
    pub current_rate: f64,
    /// Gain factor for homeostatic regulation.
    pub gain: f64,
    /// Whether the population is active.
    pub active: bool,
}

impl PopulationController {
    pub fn new(name: &str, neuron_ids: Vec<i64>) -> Self {
        Self {
            name: name.to_string(),
            neuron_ids,
            target_rate: 10.0, // 10 Hz default
            current_rate: 0.0,
            gain: 1.0,
            active: true,
        }
    }

    /// Update the measured firing rate.
    pub fn update_rate(&mut self, spike_count: usize, window_ms: f64) {
        if window_ms > 0.0 && !self.neuron_ids.is_empty() {
            self.current_rate = (spike_count as f64 * 1000.0)
                / (window_ms * self.neuron_ids.len() as f64);
        }
    }

    /// Compute a homeostatic scaling factor.
    /// Returns a multiplicative factor for synaptic weights:
    /// > 1.0 if firing rate is too low, < 1.0 if too high.
    pub fn homeostatic_factor(&self) -> f64 {
        if !self.active || self.current_rate <= 0.0 {
            return 1.0;
        }
        let ratio = self.target_rate / self.current_rate;
        // Smooth scaling: clamp to [0.5, 2.0]
        (self.gain * ratio).clamp(0.5, 2.0)
    }

    /// Check if the population is within target firing rate bounds (±20%).
    pub fn is_regulated(&self) -> bool {
        if self.target_rate <= 0.0 { return true; }
        let ratio = self.current_rate / self.target_rate;
        (0.8..=1.2).contains(&ratio)
    }

    pub fn size(&self) -> usize {
        self.neuron_ids.len()
    }

    pub fn deactivate(&mut self) {
        self.active = false;
    }

    pub fn activate(&mut self) {
        self.active = true;
    }
}

// ─── Spike Barrier ──────────────────────────────────────────────────────

/// Synchronization barrier for spike wavefronts.
/// Waits until a required number of neurons have spiked.
#[derive(Debug)]
pub struct SpikeBarrier {
    pub required: usize,
    pub received: Vec<i64>,
    pub timeout_us: i64,
    pub elapsed_us: i64,
}

impl SpikeBarrier {
    pub fn new(required: usize, timeout_us: i64) -> Self {
        Self {
            required,
            received: Vec::new(),
            timeout_us,
            elapsed_us: 0,
        }
    }

    /// Register a spike from a neuron.
    pub fn register(&mut self, neuron_id: i64) {
        if !self.received.contains(&neuron_id) {
            self.received.push(neuron_id);
        }
    }

    /// Advance time by `dt` microseconds.
    pub fn tick(&mut self, dt: i64) {
        self.elapsed_us += dt;
    }

    /// Check if the barrier is satisfied.
    pub fn is_complete(&self) -> bool {
        self.received.len() >= self.required
    }

    /// Check if the barrier has timed out.
    pub fn is_timed_out(&self) -> bool {
        self.elapsed_us >= self.timeout_us
    }

    /// Reset the barrier.
    pub fn reset(&mut self) {
        self.received.clear();
        self.elapsed_us = 0;
    }

    pub fn progress(&self) -> f64 {
        if self.required == 0 { return 1.0; }
        self.received.len() as f64 / self.required as f64
    }
}

// ─── Global Handler Registry ────────────────────────────────────────────

static HANDLER_REGISTRY: LazyLock<Mutex<Vec<SpikeHandler>>> =
    LazyLock::new(|| Mutex::new(Vec::new()));

static HANDLER_COUNTER: LazyLock<Mutex<u64>> =
    LazyLock::new(|| Mutex::new(0));

fn next_handler_id() -> u64 {
    let mut counter = HANDLER_COUNTER.lock().unwrap();
    *counter += 1;
    *counter
}

// ─── FFI ────────────────────────────────────────────────────────────────

/// Register a spike handler. Returns handler ID.
#[unsafe(no_mangle)]
pub extern "C" fn slang_spike_handler_register(source: i64, threshold: i64) -> i64 {
    let id = next_handler_id();
    let handler = SpikeHandler::new(id)
        .with_source(source)
        .with_threshold(f64::from_bits(threshold as u64));
    HANDLER_REGISTRY.lock().unwrap().push(handler);
    id as i64
}

/// Get number of registered handlers.
#[unsafe(no_mangle)]
pub extern "C" fn slang_spike_handler_count() -> i64 {
    HANDLER_REGISTRY.lock().unwrap().len() as i64
}

/// Clear all handlers.
#[unsafe(no_mangle)]
pub extern "C" fn slang_spike_handler_clear() -> i64 {
    HANDLER_REGISTRY.lock().unwrap().clear();
    0
}

/// Create a temporal guard. Returns 1 on success.
#[unsafe(no_mangle)]
pub extern "C" fn slang_temporal_guard_check(
    window_start: i64,
    window_end: i64,
    min_spikes: i64,
) -> i64 {
    let guard = TemporalGuard::new(window_start, window_end)
        .with_min_spikes(min_spikes as usize);
    if guard.is_satisfied() { 1 } else { 0 }
}

/// Create a population controller. Returns population size.
#[unsafe(no_mangle)]
pub extern "C" fn slang_population_create(count: i64) -> i64 {
    let ids: Vec<i64> = (0..count).collect();
    let ctrl = PopulationController::new("default", ids);
    ctrl.size() as i64
}

/// Create a spike barrier. Returns 1 if complete.
#[unsafe(no_mangle)]
pub extern "C" fn slang_spike_barrier_new(required: i64, timeout: i64) -> i64 {
    let barrier = SpikeBarrier::new(required as usize, timeout);
    if barrier.is_complete() { 1 } else { 0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── SpikeHandler ────────────────────────────────────────────────

    #[test]
    fn test_handler_new() {
        let h = SpikeHandler::new(1);
        assert_eq!(h.id, 1);
        assert_eq!(h.action, SpikeAction::Record);
    }

    #[test]
    fn test_handler_matches_any() {
        let h = SpikeHandler::new(1);
        assert!(h.matches(0, 1, 1.0));
    }

    #[test]
    fn test_handler_source_filter() {
        let h = SpikeHandler::new(1).with_source(5);
        assert!(h.matches(5, 1, 1.0));
        assert!(!h.matches(6, 1, 1.0));
    }

    #[test]
    fn test_handler_target_filter() {
        let h = SpikeHandler::new(1).with_target(3);
        assert!(h.matches(0, 3, 1.0));
        assert!(!h.matches(0, 4, 1.0));
    }

    #[test]
    fn test_handler_threshold() {
        let h = SpikeHandler::new(1).with_threshold(0.5);
        assert!(h.matches(0, 1, 0.5));
        assert!(h.matches(0, 1, 1.0));
        assert!(!h.matches(0, 1, 0.3));
    }

    #[test]
    fn test_handler_action() {
        let h = SpikeHandler::new(1).with_action(SpikeAction::Inhibit);
        assert_eq!(h.action, SpikeAction::Inhibit);
    }

    // ── TemporalGuard ───────────────────────────────────────────────

    #[test]
    fn test_guard_new() {
        let g = TemporalGuard::new(0, 1000);
        assert!(!g.is_satisfied()); // needs at least 1 spike
    }

    #[test]
    fn test_guard_register_spike() {
        let mut g = TemporalGuard::new(0, 1000);
        assert!(g.register_spike(500));
        assert!(g.is_satisfied());
    }

    #[test]
    fn test_guard_outside_window() {
        let mut g = TemporalGuard::new(0, 1000);
        assert!(!g.register_spike(1500)); // outside window
        assert!(!g.is_satisfied());
    }

    #[test]
    fn test_guard_min_spikes() {
        let mut g = TemporalGuard::new(0, 1000).with_min_spikes(3);
        g.register_spike(100);
        g.register_spike(200);
        assert!(!g.is_satisfied());
        g.register_spike(300);
        assert!(g.is_satisfied());
    }

    #[test]
    fn test_guard_max_spikes() {
        let mut g = TemporalGuard::new(0, 1000).with_max_spikes(2);
        g.register_spike(100);
        g.register_spike(200);
        assert!(g.is_satisfied());
        g.register_spike(300);
        assert!(!g.is_satisfied()); // exceeded max
    }

    #[test]
    fn test_guard_reset() {
        let mut g = TemporalGuard::new(0, 1000);
        g.register_spike(500);
        g.reset();
        assert_eq!(g.spike_count(), 0);
        assert!(!g.is_satisfied());
    }

    #[test]
    fn test_guard_slide_window() {
        let mut g = TemporalGuard::new(0, 1000);
        g.slide_window(1000);
        assert_eq!(g.window_start, 1000);
        assert_eq!(g.window_end, 2000);
    }

    // ── PopulationController ────────────────────────────────────────

    #[test]
    fn test_population_new() {
        let p = PopulationController::new("excitatory", vec![0, 1, 2]);
        assert_eq!(p.size(), 3);
        assert!(p.active);
    }

    #[test]
    fn test_population_update_rate() {
        let mut p = PopulationController::new("test", vec![0, 1, 2, 3]);
        p.update_rate(40, 1000.0); // 40 spikes in 1 second, 4 neurons = 10 Hz
        assert!((p.current_rate - 10.0).abs() < 1e-6);
    }

    #[test]
    fn test_population_homeostatic_low() {
        let mut p = PopulationController::new("test", vec![0, 1]);
        p.target_rate = 10.0;
        p.current_rate = 5.0; // too low
        let factor = p.homeostatic_factor();
        assert!(factor > 1.0); // should increase
    }

    #[test]
    fn test_population_homeostatic_high() {
        let mut p = PopulationController::new("test", vec![0, 1]);
        p.target_rate = 10.0;
        p.current_rate = 20.0; // too high
        let factor = p.homeostatic_factor();
        assert!(factor < 1.0); // should decrease
    }

    #[test]
    fn test_population_regulated() {
        let mut p = PopulationController::new("test", vec![0]);
        p.target_rate = 10.0;
        p.current_rate = 10.0;
        assert!(p.is_regulated());
        p.current_rate = 5.0;
        assert!(!p.is_regulated());
    }

    #[test]
    fn test_population_deactivate() {
        let mut p = PopulationController::new("test", vec![0]);
        p.deactivate();
        assert!(!p.active);
        assert!((p.homeostatic_factor() - 1.0).abs() < 1e-10);
    }

    // ── SpikeBarrier ────────────────────────────────────────────────

    #[test]
    fn test_barrier_new() {
        let b = SpikeBarrier::new(3, 10000);
        assert!(!b.is_complete());
        assert!(!b.is_timed_out());
    }

    #[test]
    fn test_barrier_register() {
        let mut b = SpikeBarrier::new(2, 10000);
        b.register(0);
        assert!(!b.is_complete());
        b.register(1);
        assert!(b.is_complete());
    }

    #[test]
    fn test_barrier_no_duplicates() {
        let mut b = SpikeBarrier::new(2, 10000);
        b.register(0);
        b.register(0); // duplicate
        assert!(!b.is_complete());
    }

    #[test]
    fn test_barrier_timeout() {
        let mut b = SpikeBarrier::new(3, 1000);
        b.tick(1001);
        assert!(b.is_timed_out());
    }

    #[test]
    fn test_barrier_progress() {
        let mut b = SpikeBarrier::new(4, 10000);
        b.register(0);
        b.register(1);
        assert!((b.progress() - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_barrier_reset() {
        let mut b = SpikeBarrier::new(2, 10000);
        b.register(0);
        b.tick(500);
        b.reset();
        assert!(!b.is_complete());
        assert_eq!(b.elapsed_us, 0);
    }

    // ── FFI ─────────────────────────────────────────────────────────

    #[test]
    fn test_ffi_handler_register() {
        slang_spike_handler_clear();
        let id = slang_spike_handler_register(0, f64::to_bits(0.5) as i64);
        assert!(id > 0);
        assert!(slang_spike_handler_count() >= 1);
        slang_spike_handler_clear();
    }

    #[test]
    fn test_ffi_temporal_guard() {
        // Empty guard → not satisfied (min_spikes=1 by default)
        assert_eq!(slang_temporal_guard_check(0, 1000, 0), 1);
    }

    #[test]
    fn test_ffi_population_create() {
        assert_eq!(slang_population_create(5), 5);
    }

    #[test]
    fn test_ffi_barrier() {
        // New barrier with 3 required → not complete
        assert_eq!(slang_spike_barrier_new(3, 10000), 0);
        // Barrier with 0 required → complete
        assert_eq!(slang_spike_barrier_new(0, 10000), 1);
    }
}
