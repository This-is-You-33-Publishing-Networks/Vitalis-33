//! v95 Quantum IR research track with feature-gated simulator primitives.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QuantumGate {
    H(usize),
    X(usize),
    CNot(usize, usize),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuantumProgram {
    pub qubits: usize,
    pub gates: Vec<QuantumGate>,
    pub enabled: bool,
}

pub fn build_program(qubits: usize, enabled: bool) -> QuantumProgram {
    QuantumProgram { qubits, gates: Vec::new(), enabled }
}

pub fn add_gate(program: &mut QuantumProgram, gate: QuantumGate) -> bool {
    if !program.enabled {
        return false;
    }
    program.gates.push(gate);
    true
}

pub fn simulate_deterministic(program: &QuantumProgram) -> Vec<u8> {
    // Lightweight deterministic test vector model.
    let mut state = vec![0u8; program.qubits.max(1)];
    if !program.enabled {
        return state;
    }
    for gate in &program.gates {
        match *gate {
            QuantumGate::H(q) | QuantumGate::X(q) => {
                if let Some(v) = state.get_mut(q) {
                    *v ^= 1;
                }
            }
            QuantumGate::CNot(c, t) => {
                let control = state.get(c).copied().unwrap_or(0);
                if control == 1 {
                    if let Some(v) = state.get_mut(t) {
                        *v ^= 1;
                    }
                }
            }
        }
    }
    state
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feature_gate_disabled() {
        let mut p = build_program(2, false);
        assert!(!add_gate(&mut p, QuantumGate::H(0)));
        assert_eq!(simulate_deterministic(&p), vec![0, 0]);
    }

    #[test]
    fn test_simulator_vectors() {
        let mut p = build_program(2, true);
        assert!(add_gate(&mut p, QuantumGate::X(0)));
        assert!(add_gate(&mut p, QuantumGate::CNot(0, 1)));
        assert_eq!(simulate_deterministic(&p), vec![1, 1]);
    }
}
