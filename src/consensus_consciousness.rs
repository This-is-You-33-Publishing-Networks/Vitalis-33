//! Consensus Consciousness — Vitalis v1005
//!
//! Byzantine-fault-tolerant agreement among sentient compiler nodes.
//! Reaches consensus on optimization strategies across a distributed hive.

use std::sync::{LazyLock, Mutex};

static STATE: LazyLock<Mutex<ConscState>> = LazyLock::new(|| Mutex::new(ConscState::default()));

#[derive(Default)]
struct ConscState {
    proposals: Vec<(String, f64)>,
    votes: Vec<(String, bool)>,
    rounds: u64,
    consensus_reached: bool,
}

pub struct ConsensusConsciousness;

impl ConsensusConsciousness {
    pub fn propose(strategy: &str, confidence: f64) -> usize {
        let mut s = STATE.lock().unwrap();
        s.proposals.push((strategy.to_string(), confidence));
        s.proposals.len()
    }

    pub fn vote(strategy: &str, approve: bool) -> usize {
        let mut s = STATE.lock().unwrap();
        s.votes.push((strategy.to_string(), approve));
        s.votes.iter().filter(|(_, a)| *a).count()
    }

    pub fn finalize_round() -> bool {
        let mut s = STATE.lock().unwrap();
        s.rounds += 1;
        let total = s.votes.len().max(1);
        let approvals = s.votes.iter().filter(|(_, a)| *a).count();
        s.consensus_reached = approvals * 3 > total * 2;
        s.consensus_reached
    }

    pub fn rounds() -> u64 { STATE.lock().unwrap().rounds }
    pub fn has_consensus() -> bool { STATE.lock().unwrap().consensus_reached }
    pub fn proposal_count() -> usize { STATE.lock().unwrap().proposals.len() }
    pub fn reset() { *STATE.lock().unwrap() = ConscState::default(); }
}

#[unsafe(no_mangle)]
pub extern "C" fn consc_propose(conf: f64) -> i64 { ConsensusConsciousness::propose("ffi_strat", conf) as i64 }
#[unsafe(no_mangle)]
pub extern "C" fn consc_vote(approve: i64) -> i64 { ConsensusConsciousness::vote("ffi_strat", approve != 0) as i64 }
#[unsafe(no_mangle)]
pub extern "C" fn consc_finalize() -> i64 { if ConsensusConsciousness::finalize_round() { 1 } else { 0 } }
#[unsafe(no_mangle)]
pub extern "C" fn consc_rounds() -> i64 { ConsensusConsciousness::rounds() as i64 }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_propose() { ConsensusConsciousness::reset(); assert_eq!(ConsensusConsciousness::propose("opt", 0.9), 1); }
    #[test] fn test_vote() { ConsensusConsciousness::reset(); let n = ConsensusConsciousness::vote("opt", true); assert_eq!(n, 1); }
    #[test] fn test_consensus() { ConsensusConsciousness::reset(); for _ in 0..3 { ConsensusConsciousness::vote("x", true); } assert!(ConsensusConsciousness::finalize_round()); }
    #[test] fn test_no_consensus() { ConsensusConsciousness::reset(); ConsensusConsciousness::vote("x", true); ConsensusConsciousness::vote("x", false); ConsensusConsciousness::vote("x", false); assert!(!ConsensusConsciousness::finalize_round()); }
    #[test] fn test_rounds() { ConsensusConsciousness::reset(); ConsensusConsciousness::finalize_round(); assert_eq!(ConsensusConsciousness::rounds(), 1); }
    #[test] fn test_proposal_count() { ConsensusConsciousness::reset(); ConsensusConsciousness::propose("a", 0.5); ConsensusConsciousness::propose("b", 0.6); assert_eq!(ConsensusConsciousness::proposal_count(), 2); }
    #[test] fn test_ffi() { ConsensusConsciousness::reset(); assert_eq!(consc_rounds(), 0); }
}
