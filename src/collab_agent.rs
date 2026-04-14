//! v485 — Collaborative Agent.
//!
//! Multi-agent system where specialized agents (optimizer, tester, documenter,
//! reviewer) collaborate via shared memory and conflict resolution.

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

/// Agent role specialization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AgentRole {
    Optimizer,
    Tester,
    Documenter,
    Reviewer,
    Debugger,
    SecurityAuditor,
}

impl AgentRole {
    pub fn name(&self) -> &'static str {
        match self {
            AgentRole::Optimizer => "optimizer",
            AgentRole::Tester => "tester",
            AgentRole::Documenter => "documenter",
            AgentRole::Reviewer => "reviewer",
            AgentRole::Debugger => "debugger",
            AgentRole::SecurityAuditor => "security_auditor",
        }
    }

    pub fn all() -> &'static [AgentRole] {
        &[
            AgentRole::Optimizer, AgentRole::Tester, AgentRole::Documenter,
            AgentRole::Reviewer, AgentRole::Debugger, AgentRole::SecurityAuditor,
        ]
    }
}

/// A message between agents.
#[derive(Debug, Clone)]
pub struct AgentMessage {
    pub from: AgentRole,
    pub to: Option<AgentRole>, // None = broadcast
    pub kind: MessageKind,
    pub payload: String,
    pub priority: u32,
}

/// Message types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageKind {
    /// Request work.
    TaskRequest,
    /// Report results.
    TaskResult,
    /// Report a conflict.
    Conflict,
    /// Propose a resolution.
    Resolution,
    /// Acknowledge.
    Ack,
}

/// An agent in the collaborative system.
#[derive(Debug, Clone)]
pub struct Agent {
    pub role: AgentRole,
    pub tasks_completed: u64,
    pub tasks_failed: u64,
    pub reputation: f64,
    pub active: bool,
}

impl Agent {
    pub fn new(role: AgentRole) -> Self {
        Self {
            role,
            tasks_completed: 0,
            tasks_failed: 0,
            reputation: 0.5,
            active: true,
        }
    }

    /// Record a task completion and update reputation.
    pub fn complete_task(&mut self, success: bool) {
        if success {
            self.tasks_completed += 1;
            self.reputation = (self.reputation + 0.05).min(1.0);
        } else {
            self.tasks_failed += 1;
            self.reputation = (self.reputation - 0.1).max(0.0);
        }
    }

    /// Success rate.
    pub fn success_rate(&self) -> f64 {
        let total = self.tasks_completed + self.tasks_failed;
        if total == 0 { return 0.0; }
        self.tasks_completed as f64 / total as f64
    }
}

/// Conflict between agent actions.
#[derive(Debug, Clone)]
pub struct Conflict {
    pub agent_a: AgentRole,
    pub agent_b: AgentRole,
    pub description: String,
    pub resolved: bool,
    pub winner: Option<AgentRole>,
}

/// Collaborative agent coordinator.
pub struct AgentCoordinator {
    pub agents: HashMap<AgentRole, Agent>,
    pub message_queue: Vec<AgentMessage>,
    pub conflicts: Vec<Conflict>,
    pub total_messages: u64,
}

impl AgentCoordinator {
    pub fn new() -> Self {
        let mut agents = HashMap::new();
        for role in AgentRole::all() {
            agents.insert(*role, Agent::new(*role));
        }
        Self {
            agents,
            message_queue: Vec::new(),
            conflicts: Vec::new(),
            total_messages: 0,
        }
    }

    /// Send a message.
    pub fn send(&mut self, msg: AgentMessage) {
        self.total_messages += 1;
        self.message_queue.push(msg);
    }

    /// Broadcast a message to all agents.
    pub fn broadcast(&mut self, from: AgentRole, kind: MessageKind, payload: &str) {
        self.send(AgentMessage {
            from,
            to: None,
            kind,
            payload: payload.to_string(),
            priority: 1,
        });
    }

    /// Report a conflict.
    pub fn report_conflict(&mut self, a: AgentRole, b: AgentRole, desc: &str) {
        self.conflicts.push(Conflict {
            agent_a: a,
            agent_b: b,
            description: desc.to_string(),
            resolved: false,
            winner: None,
        });
    }

    /// Resolve a conflict by reputation-based winner selection.
    pub fn resolve_conflicts(&mut self) {
        for conflict in &mut self.conflicts {
            if conflict.resolved { continue; }
            let rep_a = self.agents.get(&conflict.agent_a)
                .map(|a| a.reputation).unwrap_or(0.0);
            let rep_b = self.agents.get(&conflict.agent_b)
                .map(|a| a.reputation).unwrap_or(0.0);
            conflict.winner = Some(if rep_a >= rep_b {
                conflict.agent_a
            } else {
                conflict.agent_b
            });
            conflict.resolved = true;
        }
    }

    /// Complete a task for an agent.
    pub fn complete_task(&mut self, role: AgentRole, success: bool) {
        if let Some(agent) = self.agents.get_mut(&role) {
            agent.complete_task(success);
        }
    }

    /// Get agent by role.
    pub fn get_agent(&self, role: AgentRole) -> Option<&Agent> {
        self.agents.get(&role)
    }

    /// Pending messages for a role.
    pub fn pending_for(&self, role: AgentRole) -> Vec<&AgentMessage> {
        self.message_queue.iter()
            .filter(|m| m.to == Some(role) || m.to.is_none())
            .collect()
    }

    /// Total active agents.
    pub fn active_agents(&self) -> usize {
        self.agents.values().filter(|a| a.active).count()
    }

    /// Unresolved conflicts count.
    pub fn unresolved_conflicts(&self) -> usize {
        self.conflicts.iter().filter(|c| !c.resolved).count()
    }
}

// ─── FFI ──────────────────────────────────────────────────────────────

static GLOBAL_COORDINATOR: LazyLock<Mutex<AgentCoordinator>> =
    LazyLock::new(|| Mutex::new(AgentCoordinator::new()));

#[unsafe(no_mangle)]
pub extern "C" fn slang_collab_send(from_id: i64, to_id: i64) -> i64 {
    let mut coord = GLOBAL_COORDINATOR.lock().unwrap();
    let roles = AgentRole::all();
    let from = roles[(from_id as usize) % roles.len()];
    let to = if to_id < 0 { None } else { Some(roles[(to_id as usize) % roles.len()]) };
    coord.send(AgentMessage {
        from,
        to,
        kind: MessageKind::TaskRequest,
        payload: "task".to_string(),
        priority: 1,
    });
    coord.total_messages as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_collab_active() -> i64 {
    let coord = GLOBAL_COORDINATOR.lock().unwrap();
    coord.active_agents() as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_collab_conflicts() -> i64 {
    let coord = GLOBAL_COORDINATOR.lock().unwrap();
    coord.unresolved_conflicts() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_role_name() {
        assert_eq!(AgentRole::Optimizer.name(), "optimizer");
    }

    #[test]
    fn test_agent_role_all() {
        assert_eq!(AgentRole::all().len(), 6);
    }

    #[test]
    fn test_agent_new() {
        let a = Agent::new(AgentRole::Tester);
        assert_eq!(a.role, AgentRole::Tester);
        assert!((a.reputation - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_agent_complete_success() {
        let mut a = Agent::new(AgentRole::Optimizer);
        a.complete_task(true);
        assert_eq!(a.tasks_completed, 1);
        assert!(a.reputation > 0.5);
    }

    #[test]
    fn test_agent_complete_failure() {
        let mut a = Agent::new(AgentRole::Optimizer);
        a.complete_task(false);
        assert_eq!(a.tasks_failed, 1);
        assert!(a.reputation < 0.5);
    }

    #[test]
    fn test_agent_success_rate() {
        let mut a = Agent::new(AgentRole::Tester);
        a.complete_task(true);
        a.complete_task(true);
        a.complete_task(false);
        assert!((a.success_rate() - 2.0 / 3.0).abs() < 0.01);
    }

    #[test]
    fn test_coordinator_new() {
        let c = AgentCoordinator::new();
        assert_eq!(c.active_agents(), 6);
    }

    #[test]
    fn test_coordinator_send() {
        let mut c = AgentCoordinator::new();
        c.send(AgentMessage {
            from: AgentRole::Optimizer,
            to: Some(AgentRole::Tester),
            kind: MessageKind::TaskRequest,
            payload: "optimize".to_string(),
            priority: 1,
        });
        assert_eq!(c.total_messages, 1);
    }

    #[test]
    fn test_coordinator_broadcast() {
        let mut c = AgentCoordinator::new();
        c.broadcast(AgentRole::Reviewer, MessageKind::TaskResult, "all clear");
        assert_eq!(c.message_queue.len(), 1);
        assert!(c.message_queue[0].to.is_none());
    }

    #[test]
    fn test_pending_messages() {
        let mut c = AgentCoordinator::new();
        c.send(AgentMessage {
            from: AgentRole::Optimizer,
            to: Some(AgentRole::Tester),
            kind: MessageKind::TaskRequest,
            payload: "test this".to_string(),
            priority: 1,
        });
        let pending = c.pending_for(AgentRole::Tester);
        assert_eq!(pending.len(), 1);
    }

    #[test]
    fn test_conflict_report() {
        let mut c = AgentCoordinator::new();
        c.report_conflict(AgentRole::Optimizer, AgentRole::SecurityAuditor, "unsafe optimization");
        assert_eq!(c.unresolved_conflicts(), 1);
    }

    #[test]
    fn test_conflict_resolution() {
        let mut c = AgentCoordinator::new();
        c.complete_task(AgentRole::Optimizer, true);
        c.complete_task(AgentRole::Optimizer, true);
        c.report_conflict(AgentRole::Optimizer, AgentRole::SecurityAuditor, "speed vs safety");
        c.resolve_conflicts();
        assert_eq!(c.unresolved_conflicts(), 0);
        assert_eq!(c.conflicts[0].winner, Some(AgentRole::Optimizer));
    }

    #[test]
    fn test_get_agent() {
        let c = AgentCoordinator::new();
        assert!(c.get_agent(AgentRole::Debugger).is_some());
    }

    #[test]
    fn test_ffi_send() {
        let n = slang_collab_send(0, 1);
        assert!(n >= 1);
    }

    #[test]
    fn test_ffi_active() {
        let n = slang_collab_active();
        assert_eq!(n, 6);
    }

    #[test]
    fn test_ffi_conflicts() {
        let n = slang_collab_conflicts();
        assert!(n >= 0);
    }
}
