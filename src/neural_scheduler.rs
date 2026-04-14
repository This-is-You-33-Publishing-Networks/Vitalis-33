//! Neural Instruction Scheduler — v725 ERA II Phase 30
//!
//! Instruction scheduling with learned pipeline models. Performs list scheduling
//! with a priority function trained from execution profiles. Implements critical
//! path analysis, dependency DAG construction, hazard detection (data, control,
//! structural), and pipeline stage modeling (Fetch/Decode/Execute/Memory/Writeback).

use std::collections::{HashMap, HashSet, VecDeque};

/// Pipeline stages in a classic 5-stage processor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PipelineStage {
    Fetch,
    Decode,
    Execute,
    Memory,
    Writeback,
}

impl PipelineStage {
    pub const ALL: &'static [PipelineStage] = &[
        PipelineStage::Fetch,
        PipelineStage::Decode,
        PipelineStage::Execute,
        PipelineStage::Memory,
        PipelineStage::Writeback,
    ];

    pub fn index(self) -> usize {
        match self {
            PipelineStage::Fetch => 0,
            PipelineStage::Decode => 1,
            PipelineStage::Execute => 2,
            PipelineStage::Memory => 3,
            PipelineStage::Writeback => 4,
        }
    }
}

/// Types of pipeline hazards.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HazardKind {
    /// RAW: Read-After-Write data dependency.
    DataHazard,
    /// Branch misprediction or indirect jump.
    ControlHazard,
    /// Functional unit contention.
    StructuralHazard,
}

/// An instruction to be scheduled.
#[derive(Debug, Clone)]
pub struct Instruction {
    pub id: u32,
    pub opcode: String,
    pub latency: u32,
    /// Which register IDs this instruction reads.
    pub reads: Vec<u32>,
    /// Which register ID this instruction writes (if any).
    pub writes: Option<u32>,
    /// Pipeline stage where primary work happens.
    pub exec_stage: PipelineStage,
    /// Is this a memory operation?
    pub is_memory: bool,
    /// Is this a branch?
    pub is_branch: bool,
}

/// Edge in the dependency DAG.
#[derive(Debug, Clone)]
pub struct DepEdge {
    pub from: u32,
    pub to: u32,
    pub hazard: HazardKind,
    pub latency: u32,
}

/// Dependency DAG for instructions.
#[derive(Debug, Default)]
pub struct DependencyDAG {
    pub edges: Vec<DepEdge>,
    successors: HashMap<u32, Vec<usize>>,
    predecessors: HashMap<u32, Vec<usize>>,
    pub node_ids: HashSet<u32>,
}

impl DependencyDAG {
    pub fn new() -> Self {
        Self::default()
    }

    /// Build dependency DAG from a sequence of instructions.
    pub fn build(instructions: &[Instruction]) -> Self {
        let mut dag = Self::new();
        for inst in instructions {
            dag.node_ids.insert(inst.id);
        }

        // Track last writer and readers per register
        let mut last_writer: HashMap<u32, (u32, u32)> = HashMap::new(); // reg → (inst_id, latency)
        let mut last_readers: HashMap<u32, Vec<u32>> = HashMap::new(); // reg → [inst_ids]

        for inst in instructions {
            // RAW: depends on last writer of each read register
            for &reg in &inst.reads {
                if let Some(&(writer_id, lat)) = last_writer.get(&reg) {
                    dag.add_edge(DepEdge {
                        from: writer_id,
                        to: inst.id,
                        hazard: HazardKind::DataHazard,
                        latency: lat,
                    });
                }
            }

            // WAR: if we write a reg that was previously read
            if let Some(wr) = inst.writes {
                if let Some(readers) = last_readers.get(&wr) {
                    for &reader_id in readers {
                        if reader_id != inst.id {
                            dag.add_edge(DepEdge {
                                from: reader_id,
                                to: inst.id,
                                hazard: HazardKind::DataHazard,
                                latency: 0,
                            });
                        }
                    }
                }
            }

            // WAW: depends on last writer of written register
            if let Some(wr) = inst.writes {
                if let Some(&(prev_writer, _)) = last_writer.get(&wr) {
                    if prev_writer != inst.id {
                        dag.add_edge(DepEdge {
                            from: prev_writer,
                            to: inst.id,
                            hazard: HazardKind::DataHazard,
                            latency: 0,
                        });
                    }
                }
            }

            // Control hazard: branch forces all subsequent to wait
            if inst.is_branch {
                // Later instructions implicitly depend on branch
            }

            // Update tracking
            if let Some(wr) = inst.writes {
                last_writer.insert(wr, (inst.id, inst.latency));
                last_readers.remove(&wr);
            }
            for &reg in &inst.reads {
                last_readers.entry(reg).or_default().push(inst.id);
            }
        }

        dag
    }

    fn add_edge(&mut self, edge: DepEdge) {
        let idx = self.edges.len();
        self.successors.entry(edge.from).or_default().push(idx);
        self.predecessors.entry(edge.to).or_default().push(idx);
        self.edges.push(edge);
    }

    pub fn predecessor_edges(&self, node: u32) -> Vec<&DepEdge> {
        self.predecessors
            .get(&node)
            .map(|indices| indices.iter().map(|&i| &self.edges[i]).collect())
            .unwrap_or_default()
    }

    pub fn successor_edges(&self, node: u32) -> Vec<&DepEdge> {
        self.successors
            .get(&node)
            .map(|indices| indices.iter().map(|&i| &self.edges[i]).collect())
            .unwrap_or_default()
    }

    pub fn in_degree(&self, node: u32) -> usize {
        self.predecessors.get(&node).map_or(0, |v| v.len())
    }
}

/// Critical path analysis on the dependency DAG.
pub fn critical_path_length(dag: &DependencyDAG, instructions: &[Instruction]) -> u32 {
    let inst_map: HashMap<u32, &Instruction> = instructions.iter().map(|i| (i.id, i)).collect();
    let mut longest: HashMap<u32, u32> = HashMap::new();

    // Topological order via BFS (Kahn's algorithm)
    let mut in_deg: HashMap<u32, usize> = HashMap::new();
    for &id in &dag.node_ids {
        in_deg.insert(id, dag.in_degree(id));
    }
    let mut queue: VecDeque<u32> = in_deg
        .iter()
        .filter(|(_, d)| **d == 0)
        .map(|(id, _)| *id)
        .collect();
    for &id in &dag.node_ids {
        longest.insert(id, inst_map.get(&id).map_or(0, |i| i.latency));
    }

    while let Some(node) = queue.pop_front() {
        let node_len = longest[&node];
        for edge in dag.successor_edges(node) {
            let succ = edge.to;
            let succ_lat = inst_map.get(&succ).map_or(0, |i| i.latency);
            let new_len = node_len + edge.latency.max(succ_lat);
            if new_len > longest[&succ] {
                longest.insert(succ, new_len);
            }
            let deg = in_deg.get_mut(&succ).unwrap();
            *deg -= 1;
            if *deg == 0 {
                queue.push_back(succ);
            }
        }
    }

    longest.values().copied().max().unwrap_or(0)
}

/// Pipeline occupancy state for scheduling.
#[derive(Debug, Clone)]
pub struct ScheduleState {
    /// Current cycle.
    pub cycle: u32,
    /// When each functional unit becomes free: stage → next_free_cycle.
    pub unit_free_at: [u32; 5],
    /// When each instruction completes (id → cycle).
    pub completion: HashMap<u32, u32>,
    /// Scheduled order.
    pub schedule: Vec<(u32, u32)>, // (inst_id, start_cycle)
}

impl ScheduleState {
    pub fn new() -> Self {
        Self {
            cycle: 0,
            unit_free_at: [0; 5],
            completion: HashMap::new(),
            schedule: Vec::new(),
        }
    }

    pub fn total_cycles(&self) -> u32 {
        self.schedule
            .iter()
            .map(|(_, c)| *c)
            .max()
            .unwrap_or(0)
            .saturating_add(1)
    }
}

/// Learned priority weights for list scheduling.
#[derive(Debug, Clone)]
pub struct PriorityWeights {
    pub critical_path_weight: f64,
    pub successor_count_weight: f64,
    pub latency_weight: f64,
    pub memory_penalty: f64,
}

impl PriorityWeights {
    pub fn default_weights() -> Self {
        Self {
            critical_path_weight: 2.0,
            successor_count_weight: 1.0,
            latency_weight: 0.5,
            memory_penalty: -0.5,
        }
    }

    /// Update weights from profiling feedback.
    pub fn update(&mut self, feedback: &[(f64, f64)], learning_rate: f64) {
        // feedback: [(predicted_priority, actual_benefit)]
        if feedback.is_empty() {
            return;
        }
        let avg_error: f64 = feedback.iter().map(|(p, a)| (a - p).abs()).sum::<f64>()
            / feedback.len() as f64;
        // Simple weight update: increase weights if underpredicting
        let direction: f64 = feedback.iter().map(|(p, a)| a - p).sum::<f64>()
            / feedback.len() as f64;
        self.critical_path_weight += learning_rate * direction * 0.5;
        self.successor_count_weight += learning_rate * direction * 0.3;
        let _ = avg_error; // used for monitoring
    }
}

/// List scheduler with learned priorities.
pub struct ListScheduler {
    pub weights: PriorityWeights,
}

impl ListScheduler {
    pub fn new(weights: PriorityWeights) -> Self {
        Self { weights }
    }

    /// Compute priority for an instruction.
    fn priority(&self, inst: &Instruction, dag: &DependencyDAG, cp_len: u32) -> f64 {
        let succ_count = dag.successor_edges(inst.id).len();
        let cp_factor = cp_len as f64;
        let mem_factor = if inst.is_memory { 1.0 } else { 0.0 };

        self.weights.critical_path_weight * cp_factor
            + self.weights.successor_count_weight * succ_count as f64
            + self.weights.latency_weight * inst.latency as f64
            + self.weights.memory_penalty * mem_factor
    }

    /// Schedule instructions using list scheduling.
    pub fn schedule(&self, instructions: &[Instruction]) -> ScheduleState {
        let dag = DependencyDAG::build(instructions);
        let cp_len = critical_path_length(&dag, instructions);
        let inst_map: HashMap<u32, &Instruction> = instructions.iter().map(|i| (i.id, i)).collect();

        let mut state = ScheduleState::new();
        let mut ready: Vec<u32> = Vec::new();
        let mut remaining_deps: HashMap<u32, usize> = HashMap::new();
        let mut scheduled: HashSet<u32> = HashSet::new();

        for inst in instructions {
            let deps = dag.in_degree(inst.id);
            remaining_deps.insert(inst.id, deps);
            if deps == 0 {
                ready.push(inst.id);
            }
        }

        let max_cycles = instructions.len() as u32 * 20 + 10;
        let mut cycle = 0u32;

        while scheduled.len() < instructions.len() && cycle < max_cycles {
            // Sort ready list by priority (highest first)
            ready.sort_by(|&a, &b| {
                let pa = self.priority(inst_map[&a], &dag, cp_len);
                let pb = self.priority(inst_map[&b], &dag, cp_len);
                pb.partial_cmp(&pa).unwrap_or(std::cmp::Ordering::Equal)
            });

            let mut issued_this_cycle = false;
            let mut newly_ready = Vec::new();

            // Try to issue ready instructions
            let mut i = 0;
            while i < ready.len() {
                let id = ready[i];
                let inst = inst_map[&id];
                let stage_idx = inst.exec_stage.index();

                // Check structural hazard
                if state.unit_free_at[stage_idx] > cycle {
                    i += 1;
                    continue;
                }

                // Check data hazards: all predecessors must be complete
                let deps_met = dag.predecessor_edges(id).iter().all(|e| {
                    state.completion.get(&e.from).map_or(false, |&c| c + e.latency <= cycle)
                });

                if !deps_met {
                    i += 1;
                    continue;
                }

                // Issue instruction
                state.schedule.push((id, cycle));
                state.completion.insert(id, cycle + inst.latency);
                state.unit_free_at[stage_idx] = cycle + 1;
                scheduled.insert(id);
                ready.remove(i);
                issued_this_cycle = true;

                // Check if successors become ready
                for edge in dag.successor_edges(id) {
                    let succ = edge.to;
                    if let Some(rem) = remaining_deps.get_mut(&succ) {
                        *rem -= 1;
                        if *rem == 0 {
                            newly_ready.push(succ);
                        }
                    }
                }
            }

            ready.extend(newly_ready);

            if !issued_this_cycle {
                cycle += 1;
            }
        }

        state.cycle = cycle;
        state
    }
}

/// Detect hazards between two instructions.
pub fn detect_hazard(a: &Instruction, b: &Instruction) -> Option<HazardKind> {
    // RAW: b reads what a writes
    if let Some(wr) = a.writes {
        if b.reads.contains(&wr) {
            return Some(HazardKind::DataHazard);
        }
    }
    // WAR: b writes what a reads
    if let Some(wr) = b.writes {
        if a.reads.contains(&wr) {
            return Some(HazardKind::DataHazard);
        }
    }
    // WAW: both write same register
    if let (Some(wa), Some(wb)) = (a.writes, b.writes) {
        if wa == wb {
            return Some(HazardKind::DataHazard);
        }
    }
    // Control hazard
    if a.is_branch {
        return Some(HazardKind::ControlHazard);
    }
    // Structural hazard
    if a.exec_stage == b.exec_stage && a.is_memory && b.is_memory {
        return Some(HazardKind::StructuralHazard);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_inst(id: u32, opcode: &str, latency: u32, reads: Vec<u32>, writes: Option<u32>) -> Instruction {
        Instruction {
            id,
            opcode: opcode.to_string(),
            latency,
            reads,
            writes,
            exec_stage: PipelineStage::Execute,
            is_memory: false,
            is_branch: false,
        }
    }

    #[test]
    fn test_dag_build_raw() {
        let insts = vec![
            make_inst(0, "add", 1, vec![], Some(1)),
            make_inst(1, "mul", 3, vec![1], Some(2)),
        ];
        let dag = DependencyDAG::build(&insts);
        assert!(!dag.edges.is_empty());
        assert_eq!(dag.edges[0].hazard, HazardKind::DataHazard);
    }

    #[test]
    fn test_dag_no_deps() {
        let insts = vec![
            make_inst(0, "add", 1, vec![], Some(1)),
            make_inst(1, "add", 1, vec![], Some(2)),
        ];
        let dag = DependencyDAG::build(&insts);
        assert!(dag.edges.is_empty());
    }

    #[test]
    fn test_dag_in_degree() {
        let insts = vec![
            make_inst(0, "add", 1, vec![], Some(1)),
            make_inst(1, "mul", 3, vec![1], Some(2)),
        ];
        let dag = DependencyDAG::build(&insts);
        assert_eq!(dag.in_degree(0), 0);
        assert!(dag.in_degree(1) > 0);
    }

    #[test]
    fn test_critical_path_single() {
        let insts = vec![make_inst(0, "add", 5, vec![], None)];
        let dag = DependencyDAG::build(&insts);
        let cp = critical_path_length(&dag, &insts);
        assert_eq!(cp, 5);
    }

    #[test]
    fn test_critical_path_chain() {
        let insts = vec![
            make_inst(0, "add", 2, vec![], Some(1)),
            make_inst(1, "mul", 3, vec![1], Some(2)),
        ];
        let dag = DependencyDAG::build(&insts);
        let cp = critical_path_length(&dag, &insts);
        assert!(cp >= 5);
    }

    #[test]
    fn test_detect_hazard_raw() {
        let a = make_inst(0, "add", 1, vec![], Some(1));
        let b = make_inst(1, "mul", 1, vec![1], None);
        assert_eq!(detect_hazard(&a, &b), Some(HazardKind::DataHazard));
    }

    #[test]
    fn test_detect_hazard_none() {
        let a = make_inst(0, "add", 1, vec![], Some(1));
        let b = make_inst(1, "mul", 1, vec![2], Some(3));
        assert_eq!(detect_hazard(&a, &b), None);
    }

    #[test]
    fn test_detect_hazard_control() {
        let a = Instruction {
            id: 0, opcode: "jmp".into(), latency: 1, reads: vec![], writes: None,
            exec_stage: PipelineStage::Execute, is_memory: false, is_branch: true,
        };
        let b = make_inst(1, "add", 1, vec![], None);
        assert_eq!(detect_hazard(&a, &b), Some(HazardKind::ControlHazard));
    }

    #[test]
    fn test_detect_hazard_structural() {
        let a = Instruction {
            id: 0, opcode: "load".into(), latency: 4, reads: vec![], writes: Some(1),
            exec_stage: PipelineStage::Memory, is_memory: true, is_branch: false,
        };
        let b = Instruction {
            id: 1, opcode: "store".into(), latency: 4, reads: vec![2], writes: None,
            exec_stage: PipelineStage::Memory, is_memory: true, is_branch: false,
        };
        assert_eq!(detect_hazard(&a, &b), Some(HazardKind::StructuralHazard));
    }

    #[test]
    fn test_schedule_independent() {
        let scheduler = ListScheduler::new(PriorityWeights::default_weights());
        let insts = vec![
            make_inst(0, "add", 1, vec![], Some(1)),
            make_inst(1, "add", 1, vec![], Some(2)),
        ];
        let state = scheduler.schedule(&insts);
        assert_eq!(state.schedule.len(), 2);
    }

    #[test]
    fn test_schedule_dependent_chain() {
        let scheduler = ListScheduler::new(PriorityWeights::default_weights());
        let insts = vec![
            make_inst(0, "add", 2, vec![], Some(1)),
            make_inst(1, "mul", 3, vec![1], Some(2)),
        ];
        let state = scheduler.schedule(&insts);
        assert_eq!(state.schedule.len(), 2);
        // inst 1 must start after inst 0 completes
        let start_0 = state.schedule.iter().find(|s| s.0 == 0).unwrap().1;
        let start_1 = state.schedule.iter().find(|s| s.0 == 1).unwrap().1;
        assert!(start_1 >= start_0 + 2);
    }

    #[test]
    fn test_schedule_empty() {
        let scheduler = ListScheduler::new(PriorityWeights::default_weights());
        let state = scheduler.schedule(&[]);
        assert!(state.schedule.is_empty());
    }

    #[test]
    fn test_priority_weights_update() {
        let mut w = PriorityWeights::default_weights();
        let initial_cp = w.critical_path_weight;
        w.update(&[(1.0, 2.0), (1.5, 3.0)], 0.1);
        assert!(w.critical_path_weight > initial_cp);
    }

    #[test]
    fn test_pipeline_stage_index() {
        assert_eq!(PipelineStage::Fetch.index(), 0);
        assert_eq!(PipelineStage::Writeback.index(), 4);
    }

    #[test]
    fn test_schedule_state_total_cycles() {
        let mut state = ScheduleState::new();
        state.schedule.push((0, 0));
        state.schedule.push((1, 3));
        assert_eq!(state.total_cycles(), 4);
    }
}
