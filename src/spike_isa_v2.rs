//! Next-Gen Spike ISA — v731 ERA II Phase 31
//!
//! Dendritic computation, axonal delays, neuromodulatory channels,
//! multi-compartment neurons, eligibility-trace STDP, sub-ms timing.

use std::collections::HashMap;

/// Neuromodulator types affecting synaptic transmission.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Neuromodulator {
    Dopamine,
    Serotonin,
    Acetylcholine,
    Norepinephrine,
}

/// Multi-compartment neuron model.
struct MultiCompartmentNeuron {
    soma_voltage: f64,
    dendrite_voltages: Vec<f64>,
    axon_hillock_voltage: f64,
    threshold: f64,
    resting_potential: f64,
    membrane_tau: f64,
}

impl MultiCompartmentNeuron {
    fn new(n_dendrites: usize, threshold: f64) -> Self {
        Self {
            soma_voltage: -70.0,
            dendrite_voltages: vec![-70.0; n_dendrites],
            axon_hillock_voltage: -70.0,
            threshold,
            resting_potential: -70.0,
            membrane_tau: 20.0,
        }
    }

    /// Integrate dendritic currents into soma.
    fn integrate(&mut self, dt: f64) -> bool {
        // Sum dendritic inputs with cable attenuation
        let mut total_current = 0.0;
        for (i, &v) in self.dendrite_voltages.iter().enumerate() {
            let distance_factor = 1.0 / (1.0 + i as f64 * 0.1);
            total_current += (v - self.resting_potential) * distance_factor;
        }

        // Leaky integration at soma
        let dv = (-( self.soma_voltage - self.resting_potential) + total_current) * (dt / self.membrane_tau);
        self.soma_voltage += dv;

        // Axon hillock threshold check
        self.axon_hillock_voltage = self.soma_voltage * 0.95; // Slight attenuation
        if self.axon_hillock_voltage >= self.threshold {
            self.soma_voltage = self.resting_potential; // Reset
            self.axon_hillock_voltage = self.resting_potential;
            true // Spike!
        } else {
            false
        }
    }
}

/// Dendritic compartment computation using cable equation approximation.
/// V_out = V_in * exp(-L / lambda) where lambda = sqrt(d * Rm / (4 * Ri))
fn dendrite_compute(input_current: f64, cable_length: f64, diameter: f64) -> f64 {
    if diameter <= 0.0 || cable_length < 0.0 {
        return 0.0;
    }
    // Membrane resistance (Rm) and intracellular resistance (Ri) constants
    let rm = 20000.0; // Ohm·cm²
    let ri = 200.0;   // Ohm·cm
    // Length constant (lambda)
    let lambda = (diameter * rm / (4.0 * ri)).sqrt();
    if lambda <= 0.0 {
        return 0.0;
    }
    // Voltage attenuation along dendrite
    let attenuation = (-cable_length / lambda).exp();
    input_current * attenuation
}

/// Axonal propagation delay in microseconds.
fn axon_delay(distance: f64, velocity: f64) -> f64 {
    if velocity <= 0.0 || distance < 0.0 {
        return 0.0;
    }
    // delay = distance / velocity, convert to microseconds
    (distance / velocity) * 1_000_000.0
}

/// Neuromodulatory modulation of synaptic weight.
/// Each neuromodulator has a different effect:
/// - Dopamine: multiplicative gain on weight
/// - Serotonin: inhibitory modulation (dampens)
fn neuromodulate(base_weight: f64, dopamine: f64, serotonin: f64) -> f64 {
    // Dopamine enhances (multiplicative, centered at 1.0)
    let da_factor = 1.0 + dopamine.tanh();
    // Serotonin dampens (subtractive, bounded)
    let ser_factor = 1.0 - (serotonin * 0.3).min(0.9);
    base_weight * da_factor * ser_factor
}

/// Soma integration: sum of currents against threshold with exponential decay.
/// Returns whether a spike occurs.
fn soma_integrate(total_current: i64, threshold: f64, decay: f64) -> bool {
    let v = total_current as f64 * decay;
    v >= threshold
}

/// STDP with eligibility trace.
/// Eligibility decays exponentially, gated by neuromodulatory signal.
fn stdp_eligibility(pre_time: f64, post_time: f64, trace_decay: f64) -> f64 {
    let dt = post_time - pre_time;
    if dt.abs() < 1e-9 {
        return 0.0;
    }
    // Standard STDP window
    let a_plus = 1.0;
    let a_minus = -0.5;
    let tau_plus = 20.0;
    let tau_minus = 20.0;

    let stdp = if dt > 0.0 {
        // Pre before post: potentiation
        a_plus * (-dt / tau_plus).exp()
    } else {
        // Post before pre: depression
        a_minus * (dt / tau_minus).exp()
    };

    // Eligibility trace decays the STDP update
    let eligibility = (-dt.abs() * trace_decay).exp();
    stdp * eligibility
}

/// Quantize spike time to discrete time steps.
fn timing_resolution(spike_time: f64, dt: f64) -> i64 {
    if dt <= 0.0 {
        return 0;
    }
    (spike_time / dt).round() as i64
}

/// Count total compartments in a multi-compartment neuron.
fn compartment_count(soma: i64, dendrites: i64, axon_segments: i64) -> i64 {
    soma + dendrites + axon_segments
}

/// Ion channel conductance: g = g_max * sigmoid(V - V_rev)
fn channel_conductance(voltage: f64, reversal: f64, max_g: f64) -> f64 {
    if max_g <= 0.0 {
        return 0.0;
    }
    let x = voltage - reversal;
    let sigmoid = 1.0 / (1.0 + (-x / 10.0).exp());
    max_g * sigmoid
}

/// Compute dendritic tree total input with distance-dependent attenuation.
fn dendritic_tree_input(inputs: &[(f64, f64)]) -> f64 {
    // inputs: (current, distance_from_soma)
    let mut total = 0.0;
    for &(current, dist) in inputs {
        let attn = (-dist / 100.0).exp();
        total += current * attn;
    }
    total
}

/// Axonal branching: split signal across branches with diameter-dependent sharing.
fn axon_branch_split(signal: f64, branch_diameters: &[f64]) -> Vec<f64> {
    let total_area: f64 = branch_diameters.iter().map(|d| d * d * std::f64::consts::PI / 4.0).sum();
    if total_area <= 0.0 {
        return vec![0.0; branch_diameters.len()];
    }
    branch_diameters
        .iter()
        .map(|d| {
            let area = d * d * std::f64::consts::PI / 4.0;
            signal * (area / total_area)
        })
        .collect()
}

// ── FFI functions ──────────────────────────────────────────

#[unsafe(no_mangle)]
pub extern "C" fn slang_spike2_dendrite_compute(input_current: f64, cable_length: f64, diameter: f64) -> i64 {
    (dendrite_compute(input_current, cable_length, diameter) * 1000.0) as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_spike2_axon_delay(distance: f64, velocity: f64) -> i64 {
    axon_delay(distance, velocity) as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_spike2_neuromod(base_weight: f64, dopamine: f64, serotonin: f64) -> i64 {
    (neuromodulate(base_weight, dopamine, serotonin) * 1000.0) as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_spike2_soma_integrate(currents: i64, threshold: f64, decay: f64) -> i64 {
    if soma_integrate(currents, threshold, decay) { 1 } else { 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_spike2_stdp_eligibility(pre_time: f64, post_time: f64, trace_decay: f64) -> i64 {
    (stdp_eligibility(pre_time, post_time, trace_decay) * 1000.0) as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_spike2_timing_resolution(spike_time: f64, dt: f64) -> i64 {
    timing_resolution(spike_time, dt)
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_spike2_compartment_count(soma: i64, dendrites: i64, axon: i64) -> i64 {
    compartment_count(soma, dendrites, axon)
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_spike2_channel_conductance(voltage: f64, reversal: f64, max_g: f64) -> i64 {
    (channel_conductance(voltage, reversal, max_g) * 1000.0) as i64
}

// ── Tests ──────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dendrite_compute_no_attenuation() {
        let v = dendrite_compute(10.0, 0.0, 1.0);
        assert!((v - 10.0).abs() < 0.01);
    }

    #[test]
    fn test_dendrite_attenuates_with_length() {
        let short = dendrite_compute(10.0, 10.0, 1.0);
        let long = dendrite_compute(10.0, 100.0, 1.0);
        assert!(short > long);
    }

    #[test]
    fn test_dendrite_wider_less_attenuation() {
        let thin = dendrite_compute(10.0, 50.0, 0.5);
        let thick = dendrite_compute(10.0, 50.0, 2.0);
        assert!(thick > thin);
    }

    #[test]
    fn test_dendrite_zero_diameter() {
        assert_eq!(dendrite_compute(10.0, 50.0, 0.0), 0.0);
    }

    #[test]
    fn test_axon_delay_basic() {
        let d = axon_delay(1.0, 100.0); // 1m at 100 m/s = 10ms = 10000us
        assert!((d - 10000.0).abs() < 1.0);
    }

    #[test]
    fn test_axon_delay_zero_velocity() {
        assert_eq!(axon_delay(1.0, 0.0), 0.0);
    }

    #[test]
    fn test_neuromod_baseline() {
        let w = neuromodulate(1.0, 0.0, 0.0);
        assert!((w - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_neuromod_dopamine_enhances() {
        let w = neuromodulate(1.0, 1.0, 0.0);
        assert!(w > 1.0);
    }

    #[test]
    fn test_neuromod_serotonin_dampens() {
        let w = neuromodulate(1.0, 0.0, 1.0);
        assert!(w < 1.0);
    }

    #[test]
    fn test_soma_spike() {
        assert!(soma_integrate(100, 50.0, 1.0));
    }

    #[test]
    fn test_soma_no_spike() {
        assert!(!soma_integrate(10, 50.0, 1.0));
    }

    #[test]
    fn test_stdp_potentiation() {
        let e = stdp_eligibility(10.0, 15.0, 0.01);
        assert!(e > 0.0); // Pre before post = potentiation
    }

    #[test]
    fn test_stdp_depression() {
        let e = stdp_eligibility(15.0, 10.0, 0.01);
        assert!(e < 0.0); // Post before pre = depression
    }

    #[test]
    fn test_stdp_simultaneous() {
        let e = stdp_eligibility(10.0, 10.0, 0.01);
        assert_eq!(e, 0.0);
    }

    #[test]
    fn test_timing_resolution() {
        assert_eq!(timing_resolution(10.5, 1.0), 11);
        assert_eq!(timing_resolution(10.0, 0.5), 20);
    }

    #[test]
    fn test_compartment_count() {
        assert_eq!(compartment_count(1, 4, 3), 8);
    }

    #[test]
    fn test_channel_conductance_at_reversal() {
        let g = channel_conductance(0.0, 0.0, 1.0);
        assert!((g - 0.5).abs() < 0.01); // Sigmoid at 0 = 0.5
    }

    #[test]
    fn test_channel_conductance_above_reversal() {
        let g = channel_conductance(50.0, 0.0, 1.0);
        assert!(g > 0.5);
    }

    #[test]
    fn test_dendritic_tree_input() {
        let inputs = vec![(10.0, 0.0), (10.0, 100.0)];
        let total = dendritic_tree_input(&inputs);
        assert!(total > 10.0 && total < 20.0); // Near + attenuated far
    }

    #[test]
    fn test_axon_branch_split() {
        let splits = axon_branch_split(100.0, &[1.0, 1.0]);
        assert!((splits[0] - 50.0).abs() < 0.01);
    }

    #[test]
    fn test_multi_compartment_neuron() {
        let mut neuron = MultiCompartmentNeuron::new(3, -55.0);
        neuron.dendrite_voltages[0] = -20.0; // Strong input
        let spiked = neuron.integrate(1.0);
        // May or may not spike depending on integration
        assert!(spiked || neuron.soma_voltage > -70.0);
    }

    #[test]
    fn test_ffi_compartment_count() {
        assert_eq!(slang_spike2_compartment_count(1, 8, 2), 11);
    }

    #[test]
    fn test_ffi_timing() {
        assert_eq!(slang_spike2_timing_resolution(5.0, 0.5), 10);
    }

    #[test]
    fn test_ffi_conductance() {
        let g = slang_spike2_channel_conductance(50.0, 0.0, 1.0);
        assert!(g > 500); // > 0.5 * 1000
    }
}
