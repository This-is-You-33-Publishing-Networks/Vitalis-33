//! v78 differentiable control-flow checks and CFG-aware trace utility.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlFlowOp {
    Add,
    Mul,
    Branch,
    Loop,
    Io,
    Random,
}

#[derive(Debug, Clone)]
pub struct FlowTrace {
    pub ops: Vec<ControlFlowOp>,
}

impl FlowTrace {
    pub fn new(ops: Vec<ControlFlowOp>) -> Self {
        Self { ops }
    }

    pub fn is_differentiable_region(&self) -> bool {
        self.ops
            .iter()
            .all(|op| !matches!(op, ControlFlowOp::Io | ControlFlowOp::Random))
    }

    pub fn branch_loop_score(&self) -> usize {
        self.ops
            .iter()
            .filter(|op| matches!(op, ControlFlowOp::Branch | ControlFlowOp::Loop))
            .count()
    }
}

pub fn straight_through_branch(cond: bool, then_v: f64, else_v: f64) -> f64 {
    if cond { then_v } else { else_v }
}

pub fn differentiable_scan(init: f64, steps: usize, mut f: impl FnMut(f64, usize) -> f64) -> f64 {
    let mut state = init;
    for i in 0..steps {
        state = f(state, i);
    }
    state
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flags_nondiff_operations() {
        let trace = FlowTrace::new(vec![ControlFlowOp::Add, ControlFlowOp::Branch, ControlFlowOp::Io]);
        assert!(!trace.is_differentiable_region());
        assert_eq!(trace.branch_loop_score(), 1);
    }

    #[test]
    fn scan_accumulates_deterministically() {
        let out = differentiable_scan(1.0, 3, |v, i| v + (i as f64));
        assert_eq!(out, 4.0);
        assert_eq!(straight_through_branch(true, 2.0, 9.0), 2.0);
    }
}
