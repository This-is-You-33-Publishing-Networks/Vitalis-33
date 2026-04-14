//! v96 Quantum backend prototype transpilation and gate scheduling.

use crate::quantum_ir_research::{QuantumGate, QuantumProgram};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackendCircuit {
    pub lines: Vec<String>,
}

pub fn optimize_gates(gates: &[QuantumGate]) -> Vec<QuantumGate> {
    // Remove adjacent duplicate X on same qubit (X*X = I in this toy model).
    let mut out = Vec::new();
    for g in gates {
        match (out.last(), g) {
            (Some(QuantumGate::X(a)), QuantumGate::X(b)) if a == b => {
                out.pop();
            }
            _ => out.push(g.clone()),
        }
    }
    out
}

pub fn transpile(program: &QuantumProgram) -> BackendCircuit {
    let optimized = optimize_gates(&program.gates);
    let mut lines = vec![format!("qubits {}", program.qubits)];
    for g in optimized {
        let s = match g {
            QuantumGate::H(q) => format!("h q{}", q),
            QuantumGate::X(q) => format!("x q{}", q),
            QuantumGate::CNot(c, t) => format!("cx q{} q{}", c, t),
        };
        lines.push(s);
    }
    BackendCircuit { lines }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::quantum_ir_research::build_program;

    #[test]
    fn test_optimize_double_x() {
        let g = vec![QuantumGate::X(0), QuantumGate::X(0), QuantumGate::H(1)];
        let out = optimize_gates(&g);
        assert_eq!(out, vec![QuantumGate::H(1)]);
    }

    #[test]
    fn test_transpile_output() {
        let mut p = build_program(2, true);
        p.gates = vec![QuantumGate::X(0), QuantumGate::CNot(0, 1)];
        let bc = transpile(&p);
        assert!(bc.lines[0].contains("qubits 2"));
        assert!(bc.lines.iter().any(|l| l == "cx q0 q1"));
    }
}
