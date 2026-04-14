//! v410 — SNN codegen: compile spike network descriptions to Loihi 3 instruction sequences.
//!
//! Translates high-level neuromorphic descriptions into low-level instruction
//! sequences suitable for Intel Loihi 3 neuromorphic hardware (or our emulator
//! in `loihi_sim.rs`). The compilation pipeline:
//!
//! ```text
//! SpikeNetwork → SnnIR (intermediate) → LoihiProgram (instruction list)
//! ```
//!
//! Instruction types model Loihi 3's major functional units:
//! - Soma: neuron dynamics (integrate, threshold, fire)
//! - Synapse: weight lookup and accumulation
//! - Axon: spike routing and fan-out
//! - Dendrite: dendritic compartment processing

use std::fmt;

// ─── SNN Instruction Set ────────────────────────────────────────────────

/// A Loihi 3 instruction.
#[derive(Debug, Clone, PartialEq)]
pub enum SnnInst {
    /// Configure a soma (neuron core).
    SomaConfig {
        core_id: u32,
        neuron_id: u32,
        threshold: f64,
        decay: f64,
        reset: f64,
    },
    /// Integrate input current into soma.
    SomaIntegrate {
        core_id: u32,
        neuron_id: u32,
        current: f64,
    },
    /// Check threshold and fire.
    SomaThreshold {
        core_id: u32,
        neuron_id: u32,
    },
    /// Configure a synapse.
    SynapseConfig {
        source_core: u32,
        source_neuron: u32,
        target_core: u32,
        target_neuron: u32,
        weight: f64,
        delay: u32,
    },
    /// Accumulate synaptic current.
    SynapseAccumulate {
        target_core: u32,
        target_neuron: u32,
        weight: f64,
    },
    /// Route a spike via axon.
    AxonRoute {
        source_core: u32,
        source_neuron: u32,
        target_core: u32,
    },
    /// Fan-out a spike to multiple targets.
    AxonMulticast {
        source_core: u32,
        source_neuron: u32,
        target_cores: Vec<u32>,
    },
    /// Dendrite compartment integration step.
    DendriteProcess {
        core_id: u32,
        neuron_id: u32,
        compartment: u32,
    },
    /// Enable STDP learning on a core.
    LearnSTDP {
        core_id: u32,
        learning_rate: f64,
    },
    /// Timestep barrier — synchronize all cores.
    Barrier,
    /// No-op (used for alignment).
    Nop,
}

impl fmt::Display for SnnInst {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SnnInst::SomaConfig { core_id, neuron_id, threshold, .. } =>
                write!(f, "SOMA.CFG core={} neuron={} thresh={:.2}", core_id, neuron_id, threshold),
            SnnInst::SomaIntegrate { core_id, neuron_id, current } =>
                write!(f, "SOMA.INT core={} neuron={} I={:.2}", core_id, neuron_id, current),
            SnnInst::SomaThreshold { core_id, neuron_id } =>
                write!(f, "SOMA.THR core={} neuron={}", core_id, neuron_id),
            SnnInst::SynapseConfig { source_core, source_neuron, target_core, target_neuron, weight, .. } =>
                write!(f, "SYN.CFG {}:{} -> {}:{} w={:.3}", source_core, source_neuron, target_core, target_neuron, weight),
            SnnInst::SynapseAccumulate { target_core, target_neuron, weight } =>
                write!(f, "SYN.ACC {}:{} w={:.3}", target_core, target_neuron, weight),
            SnnInst::AxonRoute { source_core, source_neuron, target_core } =>
                write!(f, "AXN.RTE {}:{} -> {}", source_core, source_neuron, target_core),
            SnnInst::AxonMulticast { source_core, source_neuron, target_cores } =>
                write!(f, "AXN.MC {}:{} -> {:?}", source_core, source_neuron, target_cores),
            SnnInst::DendriteProcess { core_id, neuron_id, compartment } =>
                write!(f, "DEN.PRC core={} neuron={} comp={}", core_id, neuron_id, compartment),
            SnnInst::LearnSTDP { core_id, learning_rate } =>
                write!(f, "LRN.STDP core={} lr={:.4}", core_id, learning_rate),
            SnnInst::Barrier => write!(f, "BARRIER"),
            SnnInst::Nop => write!(f, "NOP"),
        }
    }
}

// ─── Loihi Program ──────────────────────────────────────────────────────

/// A compiled Loihi program — a sequence of instructions.
#[derive(Debug, Clone)]
pub struct LoihiProgram {
    pub name: String,
    pub instructions: Vec<SnnInst>,
    pub core_count: u32,
    pub neuron_count: u32,
    pub synapse_count: u32,
}

impl LoihiProgram {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            instructions: Vec::new(),
            core_count: 0,
            neuron_count: 0,
            synapse_count: 0,
        }
    }

    pub fn emit(&mut self, inst: SnnInst) {
        self.instructions.push(inst);
    }

    pub fn instruction_count(&self) -> usize {
        self.instructions.len()
    }

    /// Disassemble the program to text.
    pub fn disassemble(&self) -> String {
        let mut output = format!("; Loihi program: {}\n", self.name);
        output.push_str(&format!("; cores={} neurons={} synapses={}\n",
            self.core_count, self.neuron_count, self.synapse_count));
        for (i, inst) in self.instructions.iter().enumerate() {
            output.push_str(&format!("{:04}: {}\n", i, inst));
        }
        output
    }

    /// Optimize: remove redundant NOPs and merge barriers.
    pub fn optimize(&mut self) {
        // Remove trailing NOPs
        while self.instructions.last() == Some(&SnnInst::Nop) {
            self.instructions.pop();
        }
        // Merge consecutive barriers
        self.instructions.dedup_by(|a, b| {
            matches!((&*a, &*b), (SnnInst::Barrier, SnnInst::Barrier))
        });
    }

    /// Estimate energy cost (simplified model based on instruction count).
    pub fn estimated_energy(&self) -> f64 {
        let mut energy = 0.0;
        for inst in &self.instructions {
            energy += match inst {
                SnnInst::SomaConfig { .. } => 1.0,
                SnnInst::SomaIntegrate { .. } => 0.5,
                SnnInst::SomaThreshold { .. } => 0.3,
                SnnInst::SynapseConfig { .. } => 1.0,
                SnnInst::SynapseAccumulate { .. } => 0.2,
                SnnInst::AxonRoute { .. } => 0.4,
                SnnInst::AxonMulticast { target_cores, .. } => 0.4 + 0.1 * target_cores.len() as f64,
                SnnInst::DendriteProcess { .. } => 0.3,
                SnnInst::LearnSTDP { .. } => 2.0,
                SnnInst::Barrier => 0.1,
                SnnInst::Nop => 0.0,
            };
        }
        energy
    }
}

// ─── SNN Compiler ───────────────────────────────────────────────────────

/// Compile a spike network description to a Loihi program.
pub struct SnnCompiler {
    neurons_per_core: u32,
}

impl SnnCompiler {
    pub fn new() -> Self {
        Self {
            neurons_per_core: 128,
        }
    }

    pub fn with_neurons_per_core(mut self, n: u32) -> Self {
        self.neurons_per_core = n;
        self
    }

    /// Compile a network definition to a LoihiProgram.
    /// `neurons`: list of (id, threshold, decay, reset)
    /// `synapses`: list of (source_id, target_id, weight, delay)
    pub fn compile(
        &self,
        name: &str,
        neurons: &[(u32, f64, f64, f64)],
        synapses: &[(u32, u32, f64, u32)],
    ) -> LoihiProgram {
        let mut program = LoihiProgram::new(name);

        // Phase 1: Configure neurons (assign to cores)
        for &(id, threshold, decay, reset) in neurons {
            let core_id = id / self.neurons_per_core;
            let neuron_id = id % self.neurons_per_core;
            program.emit(SnnInst::SomaConfig {
                core_id,
                neuron_id,
                threshold,
                decay,
                reset,
            });
            program.neuron_count += 1;
            if core_id >= program.core_count {
                program.core_count = core_id + 1;
            }
        }

        // Phase 2: Configure synapses
        for &(src, tgt, weight, delay) in synapses {
            let source_core = src / self.neurons_per_core;
            let source_neuron = src % self.neurons_per_core;
            let target_core = tgt / self.neurons_per_core;
            let target_neuron = tgt % self.neurons_per_core;

            program.emit(SnnInst::SynapseConfig {
                source_core,
                source_neuron,
                target_core,
                target_neuron,
                weight,
                delay,
            });

            // Add axon routing if cross-core
            if source_core != target_core {
                program.emit(SnnInst::AxonRoute {
                    source_core,
                    source_neuron,
                    target_core,
                });
            }

            program.synapse_count += 1;
        }

        // Phase 3: Add barrier for timestep sync
        program.emit(SnnInst::Barrier);

        // Phase 4: Emit simulation loop instructions
        // (For each neuron, integrate then check threshold)
        for &(id, ..) in neurons {
            let core_id = id / self.neurons_per_core;
            let neuron_id = id % self.neurons_per_core;
            program.emit(SnnInst::SomaIntegrate {
                core_id,
                neuron_id,
                current: 0.0, // will be filled at runtime from synapse accumulation
            });
            program.emit(SnnInst::SomaThreshold {
                core_id,
                neuron_id,
            });
        }

        program
    }

    /// Compile with STDP learning enabled on all cores.
    pub fn compile_with_learning(
        &self,
        name: &str,
        neurons: &[(u32, f64, f64, f64)],
        synapses: &[(u32, u32, f64, u32)],
        learning_rate: f64,
    ) -> LoihiProgram {
        let mut program = self.compile(name, neurons, synapses);

        // Add learning instructions for each core
        for core in 0..program.core_count {
            program.emit(SnnInst::LearnSTDP {
                core_id: core,
                learning_rate,
            });
        }

        program
    }
}

impl Default for SnnCompiler {
    fn default() -> Self { Self::new() }
}

// ─── FFI ────────────────────────────────────────────────────────────────

/// Compile a simple network with N neurons (fully connected, equal weights).
/// Returns instruction count.
#[unsafe(no_mangle)]
pub extern "C" fn slang_snn_compile(neuron_count: i64, weight: i64) -> i64 {
    let n = neuron_count.clamp(1, 1000) as u32;
    let w = f64::from_bits(weight as u64);

    let neurons: Vec<(u32, f64, f64, f64)> = (0..n)
        .map(|i| (i, -55.0, 0.95, -70.0))
        .collect();

    let mut synapses = Vec::new();
    for i in 0..n {
        for j in 0..n {
            if i != j {
                synapses.push((i, j, w, 1));
            }
        }
    }

    let compiler = SnnCompiler::new();
    let program = compiler.compile("ffi_network", &neurons, &synapses);
    program.instruction_count() as i64
}

/// Compile and return estimated energy.
#[unsafe(no_mangle)]
pub extern "C" fn slang_snn_compile_energy(neuron_count: i64) -> i64 {
    let n = neuron_count.clamp(1, 100) as u32;
    let neurons: Vec<(u32, f64, f64, f64)> = (0..n)
        .map(|i| (i, -55.0, 0.95, -70.0))
        .collect();
    let compiler = SnnCompiler::new();
    let program = compiler.compile("energy_test", &neurons, &[]);
    f64::to_bits(program.estimated_energy()) as i64
}

/// Get number of instruction types supported.
#[unsafe(no_mangle)]
pub extern "C" fn slang_snn_inst_types() -> i64 {
    11 // SomaConfig, SomaIntegrate, SomaThreshold, SynapseConfig, SynapseAccumulate,
       // AxonRoute, AxonMulticast, DendriteProcess, LearnSTDP, Barrier, Nop
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── SnnInst ─────────────────────────────────────────────────────

    #[test]
    fn test_inst_display_soma() {
        let inst = SnnInst::SomaConfig {
            core_id: 0, neuron_id: 5, threshold: -55.0, decay: 0.95, reset: -70.0,
        };
        let s = format!("{}", inst);
        assert!(s.contains("SOMA.CFG"));
        assert!(s.contains("neuron=5"));
    }

    #[test]
    fn test_inst_display_synapse() {
        let inst = SnnInst::SynapseConfig {
            source_core: 0, source_neuron: 1, target_core: 0, target_neuron: 2,
            weight: 0.5, delay: 1,
        };
        let s = format!("{}", inst);
        assert!(s.contains("SYN.CFG"));
    }

    #[test]
    fn test_inst_display_barrier() {
        assert_eq!(format!("{}", SnnInst::Barrier), "BARRIER");
    }

    // ── LoihiProgram ────────────────────────────────────────────────

    #[test]
    fn test_program_new() {
        let p = LoihiProgram::new("test");
        assert_eq!(p.name, "test");
        assert_eq!(p.instruction_count(), 0);
    }

    #[test]
    fn test_program_emit() {
        let mut p = LoihiProgram::new("test");
        p.emit(SnnInst::Barrier);
        p.emit(SnnInst::Nop);
        assert_eq!(p.instruction_count(), 2);
    }

    #[test]
    fn test_program_disassemble() {
        let mut p = LoihiProgram::new("test");
        p.emit(SnnInst::Barrier);
        let asm = p.disassemble();
        assert!(asm.contains("Loihi program: test"));
        assert!(asm.contains("BARRIER"));
    }

    #[test]
    fn test_program_optimize_nops() {
        let mut p = LoihiProgram::new("test");
        p.emit(SnnInst::Barrier);
        p.emit(SnnInst::Nop);
        p.emit(SnnInst::Nop);
        p.optimize();
        assert_eq!(p.instruction_count(), 1); // NOPs removed
    }

    #[test]
    fn test_program_optimize_barriers() {
        let mut p = LoihiProgram::new("test");
        p.emit(SnnInst::Barrier);
        p.emit(SnnInst::Barrier);
        p.emit(SnnInst::Barrier);
        p.emit(SnnInst::SomaThreshold { core_id: 0, neuron_id: 0 });
        p.optimize();
        // Consecutive barriers should be merged
        assert!(p.instruction_count() <= 2);
    }

    #[test]
    fn test_program_energy() {
        let mut p = LoihiProgram::new("test");
        p.emit(SnnInst::SomaConfig {
            core_id: 0, neuron_id: 0, threshold: -55.0, decay: 0.95, reset: -70.0,
        });
        p.emit(SnnInst::SomaIntegrate { core_id: 0, neuron_id: 0, current: 1.0 });
        let e = p.estimated_energy();
        assert!(e > 0.0);
    }

    // ── SnnCompiler ─────────────────────────────────────────────────

    #[test]
    fn test_compile_single_neuron() {
        let compiler = SnnCompiler::new();
        let neurons = vec![(0, -55.0, 0.95, -70.0)];
        let program = compiler.compile("single", &neurons, &[]);
        assert!(program.instruction_count() > 0);
        assert_eq!(program.neuron_count, 1);
        assert_eq!(program.synapse_count, 0);
    }

    #[test]
    fn test_compile_two_neurons_connected() {
        let compiler = SnnCompiler::new();
        let neurons = vec![
            (0, -55.0, 0.95, -70.0),
            (1, -55.0, 0.95, -70.0),
        ];
        let synapses = vec![(0, 1, 0.5, 1)];
        let program = compiler.compile("pair", &neurons, &synapses);
        assert_eq!(program.neuron_count, 2);
        assert_eq!(program.synapse_count, 1);
        // Should have: 2 SomaConfig + 1 SynapseConfig + 1 Barrier + 2*(Integrate+Threshold)
        assert!(program.instruction_count() >= 7);
    }

    #[test]
    fn test_compile_cross_core() {
        let compiler = SnnCompiler::new().with_neurons_per_core(1);
        let neurons = vec![
            (0, -55.0, 0.95, -70.0),
            (1, -55.0, 0.95, -70.0),
        ];
        let synapses = vec![(0, 1, 0.5, 1)];
        let program = compiler.compile("cross_core", &neurons, &synapses);
        // Should have AxonRoute for cross-core connection
        let has_route = program.instructions.iter().any(|i| matches!(i, SnnInst::AxonRoute { .. }));
        assert!(has_route, "Cross-core connection should generate AxonRoute");
    }

    #[test]
    fn test_compile_no_route_same_core() {
        let compiler = SnnCompiler::new().with_neurons_per_core(128);
        let neurons = vec![
            (0, -55.0, 0.95, -70.0),
            (1, -55.0, 0.95, -70.0),
        ];
        let synapses = vec![(0, 1, 0.5, 1)];
        let program = compiler.compile("same_core", &neurons, &synapses);
        // Should NOT have AxonRoute for same-core connection
        let has_route = program.instructions.iter().any(|i| matches!(i, SnnInst::AxonRoute { .. }));
        assert!(!has_route, "Same-core connection should not generate AxonRoute");
    }

    #[test]
    fn test_compile_with_learning() {
        let compiler = SnnCompiler::new();
        let neurons = vec![(0, -55.0, 0.95, -70.0)];
        let program = compiler.compile_with_learning("learn", &neurons, &[], 0.01);
        let has_learn = program.instructions.iter().any(|i| matches!(i, SnnInst::LearnSTDP { .. }));
        assert!(has_learn, "Learning program should have STDP instructions");
    }

    #[test]
    fn test_compile_core_assignment() {
        let compiler = SnnCompiler::new().with_neurons_per_core(2);
        let neurons: Vec<(u32, f64, f64, f64)> = (0..6)
            .map(|i| (i, -55.0, 0.95, -70.0))
            .collect();
        let program = compiler.compile("multi_core", &neurons, &[]);
        assert_eq!(program.core_count, 3); // 6 neurons / 2 per core
        assert_eq!(program.neuron_count, 6);
    }

    #[test]
    fn test_compile_energy_scales() {
        let compiler = SnnCompiler::new();
        let n1: Vec<(u32, f64, f64, f64)> = vec![(0, -55.0, 0.95, -70.0)];
        let n10: Vec<(u32, f64, f64, f64)> = (0..10).map(|i| (i, -55.0, 0.95, -70.0)).collect();

        let p1 = compiler.compile("small", &n1, &[]);
        let p10 = compiler.compile("larger", &n10, &[]);
        assert!(p10.estimated_energy() > p1.estimated_energy());
    }

    // ── FFI ─────────────────────────────────────────────────────────

    #[test]
    fn test_ffi_compile() {
        let count = slang_snn_compile(3, f64::to_bits(0.5) as i64);
        assert!(count > 0);
    }

    #[test]
    fn test_ffi_compile_energy() {
        let bits = slang_snn_compile_energy(5);
        let energy = f64::from_bits(bits as u64);
        assert!(energy > 0.0);
    }

    #[test]
    fn test_ffi_inst_types() {
        assert_eq!(slang_snn_inst_types(), 11);
    }
}
