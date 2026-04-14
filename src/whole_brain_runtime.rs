//! Full-Brain Computation Model — Vitalis v760
//!
//! Implements a whole-brain-inspired runtime that processes sensory input,
//! generates motor output, and tracks synaptic plasticity updates.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<WholeBrainRuntime>> = LazyLock::new(|| Mutex::new(WholeBrainRuntime::new()));

pub struct WholeBrainRuntime {
    motor: Vec<f64>,
    plasticity: usize,
}

impl WholeBrainRuntime {
    pub fn new() -> Self {
        Self { motor: Vec::new(), plasticity: 0 }
    }

    pub fn process_sensory(&mut self, input: &[f64]) -> Vec<f64> {
        self.plasticity += input.len();
        let output: Vec<f64> = input.iter().map(|x| x * 0.9 + 0.1).collect();
        self.motor = output.clone();
        output
    }

    pub fn motor_output(&self) -> Vec<f64> {
        self.motor.clone()
    }

    pub fn plasticity_updates(&self) -> usize {
        self.plasticity
    }

    pub fn reset(&mut self) {
        self.motor.clear();
        self.plasticity = 0;
    }
}

impl Default for WholeBrainRuntime {
    fn default() -> Self { Self::new() }
}

#[unsafe(no_mangle)]
pub extern "C" fn wbr_process(input_len: i64) -> i64 {
    let input: Vec<f64> = (0..input_len.max(0)).map(|i| i as f64 * 0.1).collect();
    let mut s = STATE.lock().unwrap();
    s.process_sensory(&input).len() as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn wbr_motor_size() -> i64 {
    STATE.lock().unwrap().motor_output().len() as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn wbr_plasticity_count() -> i64 {
    STATE.lock().unwrap().plasticity_updates() as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn wbr_reset() -> i64 {
    STATE.lock().unwrap().reset();
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_sensory_output_size() {
        let mut wbr = WholeBrainRuntime::new();
        let out = wbr.process_sensory(&[1.0, 2.0, 3.0]);
        assert_eq!(out.len(), 3);
    }

    #[test]
    fn test_motor_output_matches_last_process() {
        let mut wbr = WholeBrainRuntime::new();
        wbr.process_sensory(&[1.0, 0.5]);
        assert_eq!(wbr.motor_output().len(), 2);
    }

    #[test]
    fn test_plasticity_accumulates() {
        let mut wbr = WholeBrainRuntime::new();
        wbr.process_sensory(&[1.0, 2.0]);
        wbr.process_sensory(&[3.0]);
        assert_eq!(wbr.plasticity_updates(), 3);
    }

    #[test]
    fn test_reset() {
        let mut wbr = WholeBrainRuntime::new();
        wbr.process_sensory(&[1.0]);
        wbr.reset();
        assert_eq!(wbr.motor_output().len(), 0);
        assert_eq!(wbr.plasticity_updates(), 0);
    }

    #[test]
    fn test_output_values_transformed() {
        let mut wbr = WholeBrainRuntime::new();
        let out = wbr.process_sensory(&[0.0]);
        assert!((out[0] - 0.1).abs() < 1e-9);
    }

    #[test]
    fn test_ffi_wbr_process() {
        wbr_reset();
        let sz = wbr_process(5);
        assert_eq!(sz, 5);
    }
}
