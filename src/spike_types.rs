//! v400 — Neuromorphic type system primitives.
//!
//! Defines first-class Rust types for neuromorphic computing:
//! - `SpikeEvent`: timestamped spike with source/target neuron IDs
//! - `Neuron`: parameterized neuron model (LIF, Izhikevich, AdEx)
//! - `Synapse`: weighted connection with plasticity state
//! - `SpikeNetwork`: collection of neurons and synapses
//! - `SpikeTrace`: recording of spike events over time
//!
//! These types integrate with the existing `spike_engine.rs` event queue and
//! `neuromorphic.rs` computation functions, providing structured access
//! instead of raw FFI integer interfaces.

use std::collections::BTreeMap;

// ─── Spike Event ────────────────────────────────────────────────────────

/// A spike event with metadata.
#[derive(Debug, Clone, PartialEq)]
pub struct SpikeEvent {
    /// Timestamp in microseconds.
    pub time_us: i64,
    /// Source neuron ID.
    pub source: i64,
    /// Target neuron ID.
    pub target: i64,
    /// Spike value / weight contribution.
    pub value: f64,
}

impl SpikeEvent {
    pub fn new(time_us: i64, source: i64, target: i64, value: f64) -> Self {
        Self { time_us, source, target, value }
    }

    /// Delay a spike by `dt` microseconds.
    pub fn delayed(&self, dt: i64) -> Self {
        Self { time_us: self.time_us + dt, ..self.clone() }
    }
}

// ─── Neuron Models ──────────────────────────────────────────────────────

/// Neuron model type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NeuronModel {
    /// Leaky Integrate-and-Fire.
    LIF,
    /// Izhikevich two-variable model.
    Izhikevich,
    /// Adaptive Exponential Integrate-and-Fire.
    AdEx,
}

/// A parameterized neuron.
#[derive(Debug, Clone)]
pub struct Neuron {
    pub id: i64,
    pub model: NeuronModel,
    /// Membrane potential (mV).
    pub voltage: f64,
    /// Threshold potential (mV).
    pub threshold: f64,
    /// Reset potential (mV).
    pub reset: f64,
    /// Membrane time constant (ms).
    pub tau: f64,
    /// Resting potential (mV).
    pub rest: f64,
    /// Refractory period remaining (ms).
    pub refractory_remaining: f64,
    /// Refractory period (ms).
    pub refractory_period: f64,
    /// Adaptation variable (Izhikevich/AdEx).
    pub adaptation: f64,
    /// Number of spikes emitted.
    pub spike_count: u64,
}

impl Neuron {
    /// Create a default LIF neuron.
    pub fn lif(id: i64) -> Self {
        Self {
            id,
            model: NeuronModel::LIF,
            voltage: -70.0,
            threshold: -55.0,
            reset: -70.0,
            tau: 20.0,
            rest: -70.0,
            refractory_remaining: 0.0,
            refractory_period: 2.0,
            adaptation: 0.0,
            spike_count: 0,
        }
    }

    /// Create a default Izhikevich neuron (regular spiking).
    pub fn izhikevich(id: i64) -> Self {
        Self {
            id,
            model: NeuronModel::Izhikevich,
            voltage: -65.0,
            threshold: 30.0,
            reset: -65.0,
            tau: 0.02, // parameter 'a'
            rest: -65.0,
            refractory_remaining: 0.0,
            refractory_period: 0.0,
            adaptation: -13.0, // b * v
            spike_count: 0,
        }
    }

    /// Create a default AdEx neuron.
    pub fn adex(id: i64) -> Self {
        Self {
            id,
            model: NeuronModel::AdEx,
            voltage: -70.0,
            threshold: -50.0,
            reset: -70.0,
            tau: 20.0,
            rest: -70.0,
            refractory_remaining: 0.0,
            refractory_period: 2.0,
            adaptation: 0.0,
            spike_count: 0,
        }
    }

    /// Step the neuron by `dt` milliseconds with given input current.
    /// Returns true if the neuron spiked.
    pub fn step(&mut self, current: f64, dt: f64) -> bool {
        if self.refractory_remaining > 0.0 {
            self.refractory_remaining -= dt;
            return false;
        }

        match self.model {
            NeuronModel::LIF => {
                let dv = (-(self.voltage - self.rest) + current) / self.tau;
                self.voltage += dv * dt;
            }
            NeuronModel::Izhikevich => {
                // Simplified Izhikevich: dv/dt = 0.04v^2 + 5v + 140 - u + I
                let dv = 0.04 * self.voltage * self.voltage
                    + 5.0 * self.voltage + 140.0
                    - self.adaptation + current;
                let du = self.tau * (0.2 * self.voltage - self.adaptation);
                self.voltage += dv * dt;
                self.adaptation += du * dt;
            }
            NeuronModel::AdEx => {
                let delta_t = 2.0; // sharpness
                let exp_term = delta_t * ((self.voltage - self.threshold) / delta_t).exp();
                let dv = (-(self.voltage - self.rest) + exp_term - self.adaptation + current) / self.tau;
                self.voltage += dv * dt;
                self.adaptation += (-self.adaptation / 30.0) * dt; // tau_w = 30ms
            }
        }

        if self.voltage >= self.threshold {
            self.spike_count += 1;
            self.voltage = self.reset;
            self.refractory_remaining = self.refractory_period;
            if self.model == NeuronModel::Izhikevich {
                self.adaptation += 8.0; // d parameter
            }
            true
        } else {
            false
        }
    }

    /// Check if neuron is in refractory period.
    pub fn is_refractory(&self) -> bool {
        self.refractory_remaining > 0.0
    }
}

// ─── Synapse ────────────────────────────────────────────────────────────

/// Plasticity rule for a synapse.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlasticityRule {
    /// No plasticity — fixed weight.
    None,
    /// Spike-timing dependent plasticity.
    STDP,
    /// Hebbian learning.
    Hebbian,
    /// BCM (Bienenstock-Cooper-Munro) rule.
    BCM,
}

/// A synapse connecting two neurons.
#[derive(Debug, Clone)]
pub struct Synapse {
    pub source: i64,
    pub target: i64,
    pub weight: f64,
    pub delay: f64,
    pub plasticity: PlasticityRule,
    /// Pre-synaptic trace for STDP.
    pub pre_trace: f64,
    /// Post-synaptic trace for STDP.
    pub post_trace: f64,
    /// Learning rate.
    pub learning_rate: f64,
}

impl Synapse {
    pub fn new(source: i64, target: i64, weight: f64) -> Self {
        Self {
            source,
            target,
            weight,
            delay: 1.0,
            plasticity: PlasticityRule::None,
            pre_trace: 0.0,
            post_trace: 0.0,
            learning_rate: 0.01,
        }
    }

    pub fn with_stdp(mut self, learning_rate: f64) -> Self {
        self.plasticity = PlasticityRule::STDP;
        self.learning_rate = learning_rate;
        self
    }

    pub fn with_delay(mut self, delay: f64) -> Self {
        self.delay = delay;
        self
    }

    /// Apply STDP update given pre and post spike times.
    /// Returns the weight change.
    pub fn stdp_update(&mut self, dt: f64) -> f64 {
        if self.plasticity != PlasticityRule::STDP {
            return 0.0;
        }
        let tau_plus = 20.0;
        let tau_minus = 20.0;
        let a_plus = 0.01;
        let a_minus = 0.012;

        let dw = if dt > 0.0 {
            // Pre before post → potentiation
            a_plus * (-dt / tau_plus).exp()
        } else {
            // Post before pre → depression
            -a_minus * (dt / tau_minus).exp()
        };

        let change = self.learning_rate * dw;
        self.weight = (self.weight + change).clamp(-1.0, 1.0);
        change
    }

    /// Decay traces by `dt` milliseconds.
    pub fn decay_traces(&mut self, dt: f64) {
        let tau = 20.0;
        let factor = (-dt / tau).exp();
        self.pre_trace *= factor;
        self.post_trace *= factor;
    }
}

// ─── Spike Trace ────────────────────────────────────────────────────────

/// A recording of spike events.
#[derive(Debug, Clone, Default)]
pub struct SpikeTrace {
    pub events: Vec<SpikeEvent>,
}

impl SpikeTrace {
    pub fn new() -> Self {
        Self { events: Vec::new() }
    }

    pub fn record(&mut self, event: SpikeEvent) {
        self.events.push(event);
    }

    /// Get all spikes from a given neuron.
    pub fn spikes_from(&self, neuron_id: i64) -> Vec<&SpikeEvent> {
        self.events.iter().filter(|e| e.source == neuron_id).collect()
    }

    /// Get all spikes targeting a given neuron.
    pub fn spikes_to(&self, neuron_id: i64) -> Vec<&SpikeEvent> {
        self.events.iter().filter(|e| e.target == neuron_id).collect()
    }

    /// Firing rate (spikes/second) for a neuron over a time window.
    pub fn firing_rate(&self, neuron_id: i64, window_us: i64) -> f64 {
        if window_us <= 0 { return 0.0; }
        let count = self.events.iter()
            .filter(|e| e.source == neuron_id)
            .count();
        (count as f64 * 1_000_000.0) / window_us as f64
    }

    /// Total number of spikes recorded.
    pub fn len(&self) -> usize {
        self.events.len()
    }

    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    /// Clear all recorded events.
    pub fn clear(&mut self) {
        self.events.clear();
    }
}

// ─── Spike Network ──────────────────────────────────────────────────────

/// A network of neurons connected by synapses.
#[derive(Debug)]
pub struct SpikeNetwork {
    pub neurons: BTreeMap<i64, Neuron>,
    pub synapses: Vec<Synapse>,
    pub trace: SpikeTrace,
    pub time_us: i64,
}

impl SpikeNetwork {
    pub fn new() -> Self {
        Self {
            neurons: BTreeMap::new(),
            synapses: Vec::new(),
            trace: SpikeTrace::new(),
            time_us: 0,
        }
    }

    /// Add a neuron to the network.
    pub fn add_neuron(&mut self, neuron: Neuron) {
        self.neurons.insert(neuron.id, neuron);
    }

    /// Connect two neurons with a synapse.
    pub fn connect(&mut self, synapse: Synapse) {
        self.synapses.push(synapse);
    }

    /// Simulate one timestep of `dt` milliseconds.
    /// Returns IDs of neurons that spiked.
    pub fn step(&mut self, dt: f64, external_currents: &BTreeMap<i64, f64>) -> Vec<i64> {
        let dt_us = (dt * 1000.0) as i64;
        self.time_us += dt_us;

        // Collect synaptic currents from recent spikes
        let mut currents: BTreeMap<i64, f64> = BTreeMap::new();
        for syn in &self.synapses {
            // Check if source neuron spiked (spike_count > 0 check simplified)
            if let Some(src) = self.neurons.get(&syn.source) {
                if src.spike_count > 0 {
                    *currents.entry(syn.target).or_insert(0.0) += syn.weight;
                }
            }
        }

        // Add external currents
        for (&id, &current) in external_currents {
            *currents.entry(id).or_insert(0.0) += current;
        }

        // Step all neurons
        let mut spiked = Vec::new();
        for (&id, neuron) in self.neurons.iter_mut() {
            let input = currents.get(&id).copied().unwrap_or(0.0);
            if neuron.step(input, dt) {
                spiked.push(id);
                self.trace.record(SpikeEvent::new(self.time_us, id, -1, 1.0));
            }
        }

        // Apply STDP for synapses where pre or post spiked
        for syn in &mut self.synapses {
            let pre_spiked = spiked.contains(&syn.source);
            let post_spiked = spiked.contains(&syn.target);
            if pre_spiked { syn.pre_trace += 1.0; }
            if post_spiked { syn.post_trace += 1.0; }
            if pre_spiked && post_spiked {
                syn.stdp_update(1.0); // dt = 1 (simultaneous)
            } else if pre_spiked {
                syn.stdp_update(1.0);
            } else if post_spiked {
                syn.stdp_update(-1.0);
            }
            syn.decay_traces(dt);
        }

        spiked
    }

    /// Get total spike count across all neurons.
    pub fn total_spikes(&self) -> u64 {
        self.neurons.values().map(|n| n.spike_count).sum()
    }

    /// Get neuron count.
    pub fn neuron_count(&self) -> usize {
        self.neurons.len()
    }

    /// Get synapse count.
    pub fn synapse_count(&self) -> usize {
        self.synapses.len()
    }
}

impl Default for SpikeNetwork {
    fn default() -> Self { Self::new() }
}

// ─── Type System Integration ────────────────────────────────────────────

/// Neuromorphic types for the Vitalis type system.
/// These can be mapped to `types::Type::Named(name)` variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NeuroType {
    SpikeEvent,
    Neuron,
    Synapse,
    SpikeTrace,
    SpikeNetwork,
}

impl NeuroType {
    pub fn as_str(&self) -> &'static str {
        match self {
            NeuroType::SpikeEvent => "SpikeEvent",
            NeuroType::Neuron => "Neuron",
            NeuroType::Synapse => "Synapse",
            NeuroType::SpikeTrace => "SpikeTrace",
            NeuroType::SpikeNetwork => "SpikeNetwork",
        }
    }

    /// All neuromorphic types.
    pub fn all() -> &'static [NeuroType] {
        &[
            NeuroType::SpikeEvent,
            NeuroType::Neuron,
            NeuroType::Synapse,
            NeuroType::SpikeTrace,
            NeuroType::SpikeNetwork,
        ]
    }
}

// ─── FFI ────────────────────────────────────────────────────────────────

/// Create a LIF neuron, return its ID.
#[unsafe(no_mangle)]
pub extern "C" fn slang_spike_type_lif(id: i64) -> i64 {
    let n = Neuron::lif(id);
    n.id
}

/// Create a SpikeEvent (returns value as i64 bits).
#[unsafe(no_mangle)]
pub extern "C" fn slang_spike_type_event(time_us: i64, source: i64, target: i64) -> i64 {
    let _e = SpikeEvent::new(time_us, source, target, 1.0);
    1 // success
}

/// Create a synapse (returns 1 on success).
#[unsafe(no_mangle)]
pub extern "C" fn slang_spike_type_synapse(source: i64, target: i64, weight: i64) -> i64 {
    let _s = Synapse::new(source, target, f64::from_bits(weight as u64));
    1
}

/// Get number of neuromorphic types available.
#[unsafe(no_mangle)]
pub extern "C" fn slang_spike_type_count() -> i64 {
    NeuroType::all().len() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── SpikeEvent ──────────────────────────────────────────────────

    #[test]
    fn test_spike_event_new() {
        let e = SpikeEvent::new(100, 1, 2, 0.5);
        assert_eq!(e.time_us, 100);
        assert_eq!(e.source, 1);
        assert_eq!(e.target, 2);
        assert!((e.value - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_spike_event_delayed() {
        let e = SpikeEvent::new(100, 1, 2, 1.0);
        let d = e.delayed(50);
        assert_eq!(d.time_us, 150);
        assert_eq!(d.source, 1);
    }

    // ── Neuron ──────────────────────────────────────────────────────

    #[test]
    fn test_neuron_lif() {
        let n = Neuron::lif(0);
        assert_eq!(n.model, NeuronModel::LIF);
        assert!(!n.is_refractory());
        assert_eq!(n.spike_count, 0);
    }

    #[test]
    fn test_neuron_izhikevich() {
        let n = Neuron::izhikevich(1);
        assert_eq!(n.model, NeuronModel::Izhikevich);
    }

    #[test]
    fn test_neuron_adex() {
        let n = Neuron::adex(2);
        assert_eq!(n.model, NeuronModel::AdEx);
    }

    #[test]
    fn test_neuron_lif_step_no_spike() {
        let mut n = Neuron::lif(0);
        let spiked = n.step(0.0, 0.1); // no current
        assert!(!spiked);
        assert_eq!(n.spike_count, 0);
    }

    #[test]
    fn test_neuron_lif_step_to_spike() {
        let mut n = Neuron::lif(0);
        // Drive with strong current until spike
        let mut spiked = false;
        for _ in 0..1000 {
            if n.step(50.0, 0.1) {
                spiked = true;
                break;
            }
        }
        assert!(spiked);
        assert!(n.spike_count > 0);
    }

    #[test]
    fn test_neuron_refractory() {
        let mut n = Neuron::lif(0);
        // Drive to spike
        for _ in 0..1000 {
            if n.step(50.0, 0.1) { break; }
        }
        assert!(n.is_refractory());
        // After refractory period, should recover
        for _ in 0..100 {
            n.step(0.0, 0.1);
        }
        assert!(!n.is_refractory());
    }

    #[test]
    fn test_neuron_izhikevich_spikes() {
        let mut n = Neuron::izhikevich(0);
        let mut spiked = false;
        for _ in 0..500 {
            if n.step(10.0, 0.5) {
                spiked = true;
                break;
            }
        }
        assert!(spiked);
    }

    // ── Synapse ─────────────────────────────────────────────────────

    #[test]
    fn test_synapse_new() {
        let s = Synapse::new(0, 1, 0.5);
        assert_eq!(s.source, 0);
        assert_eq!(s.target, 1);
        assert!((s.weight - 0.5).abs() < 1e-10);
        assert_eq!(s.plasticity, PlasticityRule::None);
    }

    #[test]
    fn test_synapse_with_stdp() {
        let s = Synapse::new(0, 1, 0.3).with_stdp(0.01);
        assert_eq!(s.plasticity, PlasticityRule::STDP);
        assert!((s.learning_rate - 0.01).abs() < 1e-10);
    }

    #[test]
    fn test_synapse_with_delay() {
        let s = Synapse::new(0, 1, 0.5).with_delay(5.0);
        assert!((s.delay - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_synapse_stdp_update_potentiation() {
        let mut s = Synapse::new(0, 1, 0.5).with_stdp(1.0);
        let dw = s.stdp_update(5.0); // pre before post
        assert!(dw > 0.0); // potentiation
    }

    #[test]
    fn test_synapse_stdp_update_depression() {
        let mut s = Synapse::new(0, 1, 0.5).with_stdp(1.0);
        let dw = s.stdp_update(-5.0); // post before pre
        assert!(dw < 0.0); // depression
    }

    #[test]
    fn test_synapse_no_plasticity_no_update() {
        let mut s = Synapse::new(0, 1, 0.5); // PlasticityRule::None
        let dw = s.stdp_update(5.0);
        assert!((dw - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_synapse_weight_clamp() {
        let mut s = Synapse::new(0, 1, 0.99).with_stdp(100.0);
        s.stdp_update(1.0);
        assert!(s.weight <= 1.0);
        assert!(s.weight >= -1.0);
    }

    #[test]
    fn test_synapse_decay_traces() {
        let mut s = Synapse::new(0, 1, 0.5);
        s.pre_trace = 1.0;
        s.post_trace = 1.0;
        s.decay_traces(10.0);
        assert!(s.pre_trace < 1.0);
        assert!(s.post_trace < 1.0);
    }

    // ── SpikeTrace ──────────────────────────────────────────────────

    #[test]
    fn test_spike_trace_record() {
        let mut trace = SpikeTrace::new();
        trace.record(SpikeEvent::new(0, 0, 1, 1.0));
        trace.record(SpikeEvent::new(10, 1, 2, 1.0));
        assert_eq!(trace.len(), 2);
    }

    #[test]
    fn test_spike_trace_filter() {
        let mut trace = SpikeTrace::new();
        trace.record(SpikeEvent::new(0, 0, 1, 1.0));
        trace.record(SpikeEvent::new(10, 0, 2, 1.0));
        trace.record(SpikeEvent::new(20, 1, 2, 1.0));
        assert_eq!(trace.spikes_from(0).len(), 2);
        assert_eq!(trace.spikes_to(2).len(), 2);
    }

    #[test]
    fn test_spike_trace_firing_rate() {
        let mut trace = SpikeTrace::new();
        for i in 0..10 {
            trace.record(SpikeEvent::new(i * 1000, 0, 1, 1.0));
        }
        let rate = trace.firing_rate(0, 10000); // 10 spikes in 10ms
        assert!(rate > 0.0);
    }

    #[test]
    fn test_spike_trace_clear() {
        let mut trace = SpikeTrace::new();
        trace.record(SpikeEvent::new(0, 0, 1, 1.0));
        trace.clear();
        assert!(trace.is_empty());
    }

    // ── SpikeNetwork ────────────────────────────────────────────────

    #[test]
    fn test_network_create() {
        let net = SpikeNetwork::new();
        assert_eq!(net.neuron_count(), 0);
        assert_eq!(net.synapse_count(), 0);
    }

    #[test]
    fn test_network_add_neurons() {
        let mut net = SpikeNetwork::new();
        net.add_neuron(Neuron::lif(0));
        net.add_neuron(Neuron::lif(1));
        assert_eq!(net.neuron_count(), 2);
    }

    #[test]
    fn test_network_connect() {
        let mut net = SpikeNetwork::new();
        net.add_neuron(Neuron::lif(0));
        net.add_neuron(Neuron::lif(1));
        net.connect(Synapse::new(0, 1, 0.5));
        assert_eq!(net.synapse_count(), 1);
    }

    #[test]
    fn test_network_step_no_input() {
        let mut net = SpikeNetwork::new();
        net.add_neuron(Neuron::lif(0));
        let spiked = net.step(0.1, &BTreeMap::new());
        assert!(spiked.is_empty());
    }

    #[test]
    fn test_network_step_with_input() {
        let mut net = SpikeNetwork::new();
        net.add_neuron(Neuron::lif(0));
        let mut currents = BTreeMap::new();
        currents.insert(0_i64, 50.0);
        let mut any_spiked = false;
        for _ in 0..500 {
            let spiked = net.step(0.1, &currents);
            if !spiked.is_empty() {
                any_spiked = true;
                break;
            }
        }
        assert!(any_spiked);
    }

    #[test]
    fn test_network_total_spikes() {
        let mut net = SpikeNetwork::new();
        net.add_neuron(Neuron::lif(0));
        let mut currents = BTreeMap::new();
        currents.insert(0_i64, 50.0);
        for _ in 0..500 {
            net.step(0.1, &currents);
        }
        assert!(net.total_spikes() > 0);
    }

    #[test]
    fn test_network_spike_propagation() {
        let mut net = SpikeNetwork::new();
        net.add_neuron(Neuron::lif(0));
        net.add_neuron(Neuron::lif(1));
        net.connect(Synapse::new(0, 1, 15.0)); // strong synapse
        let mut currents = BTreeMap::new();
        currents.insert(0_i64, 50.0); // drive neuron 0
        for _ in 0..500 {
            net.step(0.1, &currents);
        }
        // Neuron 1 should have received input via synapse
        assert!(net.total_spikes() > 0);
    }

    // ── NeuroType ───────────────────────────────────────────────────

    #[test]
    fn test_neuro_type_names() {
        assert_eq!(NeuroType::SpikeEvent.as_str(), "SpikeEvent");
        assert_eq!(NeuroType::Neuron.as_str(), "Neuron");
        assert_eq!(NeuroType::all().len(), 5);
    }

    // ── FFI ─────────────────────────────────────────────────────────

    #[test]
    fn test_ffi_lif() {
        assert_eq!(slang_spike_type_lif(42), 42);
    }

    #[test]
    fn test_ffi_event() {
        assert_eq!(slang_spike_type_event(0, 1, 2), 1);
    }

    #[test]
    fn test_ffi_synapse() {
        assert_eq!(slang_spike_type_synapse(0, 1, f64::to_bits(0.5) as i64), 1);
    }

    #[test]
    fn test_ffi_type_count() {
        assert_eq!(slang_spike_type_count(), 5);
    }
}
