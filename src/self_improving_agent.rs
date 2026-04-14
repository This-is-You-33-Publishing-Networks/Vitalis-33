//! v490 — Self-Improving Agent.
//!
//! Agent that improves its own decision-making. Track which actions led to
//! successful outcomes. Evolve action selection policy. Meta-learning from
//! past sessions.

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

/// An action the agent can take.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AgentAction {
    OptimizeCode,
    GenerateTests,
    WriteDocumentation,
    ReviewCode,
    FixBug,
    Refactor,
    AddFeature,
    ProfilePerformance,
}

impl AgentAction {
    pub fn name(&self) -> &'static str {
        match self {
            AgentAction::OptimizeCode => "optimize",
            AgentAction::GenerateTests => "test_gen",
            AgentAction::WriteDocumentation => "doc",
            AgentAction::ReviewCode => "review",
            AgentAction::FixBug => "fix_bug",
            AgentAction::Refactor => "refactor",
            AgentAction::AddFeature => "add_feature",
            AgentAction::ProfilePerformance => "profile",
        }
    }

    pub fn all() -> &'static [AgentAction] {
        &[
            AgentAction::OptimizeCode, AgentAction::GenerateTests,
            AgentAction::WriteDocumentation, AgentAction::ReviewCode,
            AgentAction::FixBug, AgentAction::Refactor,
            AgentAction::AddFeature, AgentAction::ProfilePerformance,
        ]
    }
}

/// Outcome of an agent action.
#[derive(Debug, Clone)]
pub struct ActionOutcome {
    pub action: AgentAction,
    pub success: bool,
    pub reward: f64,
    pub context: String,
}

/// Q-table entry for action-value learning.
#[derive(Debug, Clone)]
pub struct QEntry {
    pub value: f64,
    pub visits: u64,
}

impl QEntry {
    pub fn new() -> Self {
        Self { value: 0.0, visits: 0 }
    }

    /// Update Q-value with exponential moving average.
    pub fn update(&mut self, reward: f64, alpha: f64) {
        self.visits += 1;
        self.value += alpha * (reward - self.value);
    }
}

/// Experience replay buffer entry.
#[derive(Debug, Clone)]
pub struct Experience {
    pub context_hash: u64,
    pub action: AgentAction,
    pub reward: f64,
}

/// Self-improving agent.
pub struct SelfImprovingAgent {
    /// Q-table: context_hash → action → Q-value.
    pub q_table: HashMap<u64, HashMap<AgentAction, QEntry>>,
    /// Experience replay buffer.
    pub replay_buffer: Vec<Experience>,
    pub replay_capacity: usize,
    /// Total actions taken.
    pub total_actions: u64,
    /// Exploration rate (epsilon).
    pub epsilon: f64,
    /// Learning rate.
    pub alpha: f64,
    /// Performance tracking.
    pub cumulative_reward: f64,
    pub best_episode_reward: f64,
}

impl SelfImprovingAgent {
    pub fn new() -> Self {
        Self {
            q_table: HashMap::new(),
            replay_buffer: Vec::new(),
            replay_capacity: 1000,
            total_actions: 0,
            epsilon: 0.3,
            alpha: 0.1,
            cumulative_reward: 0.0,
            best_episode_reward: f64::NEG_INFINITY,
        }
    }

    /// Select an action using epsilon-greedy policy.
    pub fn select_action(&self, context_hash: u64, step_seed: u64) -> AgentAction {
        // Deterministic pseudo-random for epsilon check
        let rand_val = ((step_seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407)) % 1000) as f64 / 1000.0;

        if rand_val < self.epsilon {
            // Explore: pick based on seed
            let actions = AgentAction::all();
            actions[(step_seed as usize) % actions.len()]
        } else {
            // Exploit: pick best Q-value
            self.best_action(context_hash)
        }
    }

    /// Get the action with highest Q-value for a context.
    pub fn best_action(&self, context_hash: u64) -> AgentAction {
        if let Some(entries) = self.q_table.get(&context_hash) {
            entries.iter()
                .max_by(|a, b| a.1.value.partial_cmp(&b.1.value).unwrap_or(std::cmp::Ordering::Equal))
                .map(|(action, _)| *action)
                .unwrap_or(AgentAction::OptimizeCode)
        } else {
            AgentAction::OptimizeCode // Default action
        }
    }

    /// Record an outcome and update Q-table.
    pub fn record_outcome(&mut self, context_hash: u64, outcome: ActionOutcome) {
        self.total_actions += 1;
        self.cumulative_reward += outcome.reward;

        // Update Q-table
        let entry = self.q_table
            .entry(context_hash)
            .or_default()
            .entry(outcome.action)
            .or_insert_with(QEntry::new);
        entry.update(outcome.reward, self.alpha);

        // Add to replay buffer
        if self.replay_buffer.len() >= self.replay_capacity {
            self.replay_buffer.remove(0);
        }
        self.replay_buffer.push(Experience {
            context_hash,
            action: outcome.action,
            reward: outcome.reward,
        });

        // Decay epsilon
        self.epsilon = (self.epsilon * 0.999).max(0.05);
    }

    /// Replay a batch of experiences to reinforce learning.
    pub fn replay_batch(&mut self, batch_size: usize) {
        let n = self.replay_buffer.len().min(batch_size);
        // Replay from the most recent experiences
        let start = self.replay_buffer.len().saturating_sub(n);
        let experiences: Vec<Experience> = self.replay_buffer[start..].to_vec();
        for exp in experiences {
            let entry = self.q_table
                .entry(exp.context_hash)
                .or_default()
                .entry(exp.action)
                .or_insert_with(QEntry::new);
            entry.update(exp.reward, self.alpha * 0.5); // Lower learning rate for replay
        }
    }

    /// Average reward per action.
    pub fn avg_reward(&self) -> f64 {
        if self.total_actions == 0 { return 0.0; }
        self.cumulative_reward / self.total_actions as f64
    }

    /// Known contexts count.
    pub fn known_contexts(&self) -> usize {
        self.q_table.len()
    }

    /// Replay buffer utilization.
    pub fn buffer_utilization(&self) -> f64 {
        self.replay_buffer.len() as f64 / self.replay_capacity as f64
    }
}

// ─── FFI ──────────────────────────────────────────────────────────────

static GLOBAL_SIA: LazyLock<Mutex<SelfImprovingAgent>> =
    LazyLock::new(|| Mutex::new(SelfImprovingAgent::new()));

#[unsafe(no_mangle)]
pub extern "C" fn slang_sia_select(context: i64, seed: i64) -> i64 {
    let agent = GLOBAL_SIA.lock().unwrap();
    let action = agent.select_action(context as u64, seed as u64);
    AgentAction::all().iter().position(|a| *a == action).unwrap_or(0) as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_sia_record(context: i64, action_id: i64, reward: i64) -> i64 {
    let mut agent = GLOBAL_SIA.lock().unwrap();
    let actions = AgentAction::all();
    let action = actions[(action_id as usize) % actions.len()];
    agent.record_outcome(context as u64, ActionOutcome {
        action,
        success: reward > 0,
        reward: f64::from_bits(reward as u64),
        context: String::new(),
    });
    agent.total_actions as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_sia_avg_reward() -> i64 {
    let agent = GLOBAL_SIA.lock().unwrap();
    f64::to_bits(agent.avg_reward()) as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_sia_contexts() -> i64 {
    let agent = GLOBAL_SIA.lock().unwrap();
    agent.known_contexts() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_action_all() {
        assert_eq!(AgentAction::all().len(), 8);
    }

    #[test]
    fn test_action_name() {
        assert_eq!(AgentAction::FixBug.name(), "fix_bug");
    }

    #[test]
    fn test_q_entry_new() {
        let q = QEntry::new();
        assert!((q.value - 0.0).abs() < 1e-10);
        assert_eq!(q.visits, 0);
    }

    #[test]
    fn test_q_entry_update() {
        let mut q = QEntry::new();
        q.update(1.0, 0.1);
        assert!(q.value > 0.0);
        assert_eq!(q.visits, 1);
    }

    #[test]
    fn test_agent_new() {
        let a = SelfImprovingAgent::new();
        assert_eq!(a.total_actions, 0);
        assert!((a.epsilon - 0.3).abs() < 1e-10);
    }

    #[test]
    fn test_select_action() {
        let a = SelfImprovingAgent::new();
        let action = a.select_action(0, 42);
        // Should return some valid action
        assert!(AgentAction::all().contains(&action));
    }

    #[test]
    fn test_record_outcome() {
        let mut a = SelfImprovingAgent::new();
        a.record_outcome(0, ActionOutcome {
            action: AgentAction::OptimizeCode,
            success: true,
            reward: 1.0,
            context: String::new(),
        });
        assert_eq!(a.total_actions, 1);
        assert!(a.cumulative_reward > 0.0);
    }

    #[test]
    fn test_best_action() {
        let mut a = SelfImprovingAgent::new();
        // Record good outcome for FixBug
        for _ in 0..10 {
            a.record_outcome(42, ActionOutcome {
                action: AgentAction::FixBug,
                success: true,
                reward: 1.0,
                context: String::new(),
            });
        }
        // Record bad outcome for OptimizeCode
        for _ in 0..10 {
            a.record_outcome(42, ActionOutcome {
                action: AgentAction::OptimizeCode,
                success: false,
                reward: -1.0,
                context: String::new(),
            });
        }
        let best = a.best_action(42);
        assert_eq!(best, AgentAction::FixBug);
    }

    #[test]
    fn test_replay_batch() {
        let mut a = SelfImprovingAgent::new();
        for i in 0..5u64 {
            a.record_outcome(i, ActionOutcome {
                action: AgentAction::GenerateTests,
                success: true,
                reward: 0.5,
                context: String::new(),
            });
        }
        a.replay_batch(3);
        // Should not panic, values should be updated
    }

    #[test]
    fn test_epsilon_decay() {
        let mut a = SelfImprovingAgent::new();
        let initial = a.epsilon;
        a.record_outcome(0, ActionOutcome {
            action: AgentAction::Refactor,
            success: true,
            reward: 0.5,
            context: String::new(),
        });
        assert!(a.epsilon < initial);
    }

    #[test]
    fn test_avg_reward() {
        let mut a = SelfImprovingAgent::new();
        assert!((a.avg_reward() - 0.0).abs() < 1e-10);
        a.record_outcome(0, ActionOutcome {
            action: AgentAction::OptimizeCode,
            success: true,
            reward: 2.0,
            context: String::new(),
        });
        assert!((a.avg_reward() - 2.0).abs() < 1e-10);
    }

    #[test]
    fn test_known_contexts() {
        let mut a = SelfImprovingAgent::new();
        a.record_outcome(1, ActionOutcome {
            action: AgentAction::OptimizeCode,
            success: true,
            reward: 1.0,
            context: String::new(),
        });
        a.record_outcome(2, ActionOutcome {
            action: AgentAction::FixBug,
            success: true,
            reward: 1.0,
            context: String::new(),
        });
        assert_eq!(a.known_contexts(), 2);
    }

    #[test]
    fn test_buffer_utilization() {
        let a = SelfImprovingAgent::new();
        assert!((a.buffer_utilization() - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_ffi_select() {
        let r = slang_sia_select(0, 1);
        assert!(r >= 0 && r < 8);
    }

    #[test]
    fn test_ffi_contexts() {
        let n = slang_sia_contexts();
        assert!(n >= 0);
    }
}
