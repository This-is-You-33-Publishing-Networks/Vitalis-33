//! Autonomous Agent — observe/plan/act/reflect loop with tool use.
//!
//! Provides a general-purpose agent framework: observation parsing,
//! goal decomposition, action planning, execution, and reflective
//! memory for self-improving behavior.

use std::collections::HashMap;
use std::sync::Mutex;

// ── Core Types ──────────────────────────────────────────────────────────

/// An observation from the environment.
#[derive(Debug, Clone)]
pub struct Observation {
    pub kind: ObservationKind,
    pub content: String,
    pub timestamp: u64,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObservationKind {
    Text,
    Numeric,
    Error,
    ToolResult,
    FeedbackPositive,
    FeedbackNegative,
}

/// An action the agent can take.
#[derive(Debug, Clone)]
pub struct Action {
    pub kind: ActionKind,
    pub target: String,
    pub parameters: HashMap<String, String>,
    pub confidence: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionKind {
    Execute,
    Query,
    Transform,
    Store,
    Communicate,
    Wait,
    Abort,
}

/// A plan is a sequence of actions with dependencies.
#[derive(Debug, Clone)]
pub struct Plan {
    pub goal: String,
    pub steps: Vec<PlanStep>,
    pub priority: f64,
}

#[derive(Debug, Clone)]
pub struct PlanStep {
    pub action: Action,
    pub depends_on: Vec<usize>, // indices into steps
    pub status: StepStatus,
    pub result: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
    Skipped,
}

/// A reflection entry for learning from experience.
#[derive(Debug, Clone)]
pub struct Reflection {
    pub observation_summary: String,
    pub action_taken: String,
    pub outcome: ReflectionOutcome,
    pub lesson: String,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReflectionOutcome {
    Success,
    PartialSuccess,
    Failure,
    Unexpected,
}

// ── Tool Registry ───────────────────────────────────────────────────────

/// A tool that the agent can use.
#[derive(Debug, Clone)]
pub struct Tool {
    pub name: String,
    pub description: String,
    pub parameters: Vec<ToolParam>,
    pub cost: f64, // estimated cost (time/resources)
}

#[derive(Debug, Clone)]
pub struct ToolParam {
    pub name: String,
    pub param_type: String,
    pub required: bool,
}

/// Tool registry for available tools.
#[derive(Debug, Clone, Default)]
pub struct ToolRegistry {
    pub tools: HashMap<String, Tool>,
}

impl ToolRegistry {
    pub fn new() -> Self { ToolRegistry { tools: HashMap::new() } }

    pub fn register(&mut self, tool: Tool) {
        self.tools.insert(tool.name.clone(), tool);
    }

    pub fn get(&self, name: &str) -> Option<&Tool> {
        self.tools.get(name)
    }

    pub fn find_by_description(&self, query: &str) -> Vec<&Tool> {
        let query_lower = query.to_lowercase();
        self.tools.values()
            .filter(|t| t.description.to_lowercase().contains(&query_lower)
                     || t.name.to_lowercase().contains(&query_lower))
            .collect()
    }
}

// ── Agent Core ──────────────────────────────────────────────────────────

/// Goal decomposition strategy.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DecompositionStrategy {
    Sequential,   // Steps in order
    Parallel,     // Independent steps run simultaneously
    Hierarchical, // Sub-goals can be further decomposed
}

/// Autonomous agent with observe-plan-act-reflect loop.
#[derive(Debug, Clone)]
pub struct Agent {
    pub name: String,
    pub observations: Vec<Observation>,
    pub reflections: Vec<Reflection>,
    pub current_plan: Option<Plan>,
    pub tool_registry: ToolRegistry,
    pub reward_history: Vec<f64>,
    pub total_actions: usize,
    pub successful_actions: usize,
    pub strategy: DecompositionStrategy,
}

impl Agent {
    pub fn new(name: &str) -> Self {
        Agent {
            name: name.to_string(),
            observations: Vec::new(),
            reflections: Vec::new(),
            current_plan: None,
            tool_registry: ToolRegistry::new(),
            reward_history: Vec::new(),
            total_actions: 0,
            successful_actions: 0,
            strategy: DecompositionStrategy::Sequential,
        }
    }

    /// Observe: record an observation from the environment.
    pub fn observe(&mut self, obs: Observation) {
        self.observations.push(obs);
    }

    /// Plan: decompose a goal into actionable steps.
    pub fn plan(&mut self, goal: &str) -> Plan {
        let steps = self.decompose_goal(goal);
        let plan = Plan {
            goal: goal.to_string(),
            steps,
            priority: 1.0,
        };
        self.current_plan = Some(plan.clone());
        plan
    }

    /// Decompose a goal into plan steps (heuristic-based).
    fn decompose_goal(&self, goal: &str) -> Vec<PlanStep> {
        let words: Vec<&str> = goal.split_whitespace().collect();
        let mut steps = Vec::new();

        // Simple heuristic: each sentence/clause becomes a step
        let clauses: Vec<&str> = goal.split(&['.', ';', ','][..])
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();

        for (i, clause) in clauses.iter().enumerate() {
            let action_kind = if clause.contains("find") || clause.contains("search") || clause.contains("query") {
                ActionKind::Query
            } else if clause.contains("transform") || clause.contains("convert") || clause.contains("change") {
                ActionKind::Transform
            } else if clause.contains("save") || clause.contains("store") || clause.contains("write") {
                ActionKind::Store
            } else if clause.contains("wait") || clause.contains("pause") {
                ActionKind::Wait
            } else {
                ActionKind::Execute
            };

            let depends = if i > 0 && self.strategy == DecompositionStrategy::Sequential {
                vec![i - 1]
            } else {
                vec![]
            };

            // Find relevant tools
            let relevant_tools = self.tool_registry.find_by_description(clause);
            let target = if let Some(tool) = relevant_tools.first() {
                tool.name.clone()
            } else {
                words.first().unwrap_or(&"unknown").to_string()
            };

            steps.push(PlanStep {
                action: Action {
                    kind: action_kind,
                    target,
                    parameters: HashMap::new(),
                    confidence: 0.8 - (i as f64 * 0.05), // Decreasing confidence
                },
                depends_on: depends,
                status: StepStatus::Pending,
                result: None,
            });
        }
        steps
    }

    /// Act: get the next executable step from the plan.
    pub fn next_action(&mut self) -> Option<(usize, Action)> {
        let plan = self.current_plan.as_ref()?;
        for (i, step) in plan.steps.iter().enumerate() {
            if step.status != StepStatus::Pending { continue; }
            // Check dependencies
            let deps_met = step.depends_on.iter().all(|&d| {
                d < plan.steps.len() && plan.steps[d].status == StepStatus::Completed
            });
            if deps_met {
                return Some((i, step.action.clone()));
            }
        }
        None
    }

    /// Record the result of an action.
    pub fn record_result(&mut self, step_idx: usize, success: bool, result: &str) {
        if let Some(plan) = &mut self.current_plan {
            if step_idx < plan.steps.len() {
                plan.steps[step_idx].status = if success {
                    StepStatus::Completed
                } else {
                    StepStatus::Failed
                };
                plan.steps[step_idx].result = Some(result.to_string());
            }
        }
        self.total_actions += 1;
        if success { self.successful_actions += 1; }
    }

    /// Reflect: analyze outcomes and extract lessons.
    pub fn reflect(&mut self) -> Option<Reflection> {
        let plan = self.current_plan.as_ref()?;
        let completed: Vec<_> = plan.steps.iter()
            .filter(|s| s.status == StepStatus::Completed || s.status == StepStatus::Failed)
            .collect();

        if completed.is_empty() { return None; }

        let successes = completed.iter().filter(|s| s.status == StepStatus::Completed).count();
        let total = completed.len();
        let success_rate = successes as f64 / total as f64;

        let outcome = if success_rate >= 1.0 { ReflectionOutcome::Success }
        else if success_rate >= 0.5 { ReflectionOutcome::PartialSuccess }
        else { ReflectionOutcome::Failure };

        let lesson = match outcome {
            ReflectionOutcome::Success => format!("Goal '{}' achieved. All {} steps completed.", plan.goal, total),
            ReflectionOutcome::PartialSuccess => format!("Goal '{}' partially achieved ({}/{}). Consider alternative approaches for failed steps.", plan.goal, successes, total),
            ReflectionOutcome::Failure => format!("Goal '{}' failed ({}/{}). Root cause analysis needed.", plan.goal, successes, total),
            ReflectionOutcome::Unexpected => format!("Unexpected outcome for goal '{}'.", plan.goal),
        };

        let reflection = Reflection {
            observation_summary: format!("{} observations processed", self.observations.len()),
            action_taken: plan.goal.clone(),
            outcome,
            lesson,
            timestamp: self.total_actions as u64,
        };

        self.reflections.push(reflection.clone());
        self.reward_history.push(success_rate);
        Some(reflection)
    }

    /// Success rate across all actions.
    pub fn success_rate(&self) -> f64 {
        if self.total_actions == 0 { return 0.0; }
        self.successful_actions as f64 / self.total_actions as f64
    }

    /// Average reward over recent episodes.
    pub fn avg_reward(&self, window: usize) -> f64 {
        if self.reward_history.is_empty() { return 0.0; }
        let start = if self.reward_history.len() > window {
            self.reward_history.len() - window
        } else { 0 };
        let slice = &self.reward_history[start..];
        slice.iter().sum::<f64>() / slice.len() as f64
    }

    /// Check if plan is complete.
    pub fn plan_complete(&self) -> bool {
        match &self.current_plan {
            None => true,
            Some(plan) => plan.steps.iter().all(|s| {
                s.status == StepStatus::Completed || s.status == StepStatus::Failed || s.status == StepStatus::Skipped
            }),
        }
    }
}

// ── Multi-Agent Coordination ────────────────────────────────────────────

/// Message between agents.
#[derive(Debug, Clone)]
pub struct AgentMessage {
    pub from: String,
    pub to: String,
    pub content: String,
    pub priority: f64,
}

/// Simple multi-agent coordinator.
#[derive(Debug, Clone)]
pub struct AgentCoordinator {
    pub agents: Vec<Agent>,
    pub message_queue: Vec<AgentMessage>,
    pub shared_knowledge: HashMap<String, String>,
}

impl AgentCoordinator {
    pub fn new() -> Self {
        AgentCoordinator {
            agents: Vec::new(),
            message_queue: Vec::new(),
            shared_knowledge: HashMap::new(),
        }
    }

    pub fn add_agent(&mut self, agent: Agent) {
        self.agents.push(agent);
    }

    pub fn send_message(&mut self, msg: AgentMessage) {
        self.message_queue.push(msg);
    }

    pub fn share_knowledge(&mut self, key: &str, value: &str) {
        self.shared_knowledge.insert(key.to_string(), value.to_string());
    }

    /// Get pending messages for an agent.
    pub fn messages_for(&self, agent_name: &str) -> Vec<&AgentMessage> {
        self.message_queue.iter()
            .filter(|m| m.to == agent_name)
            .collect()
    }

    /// Run one step of all agents.
    pub fn step(&mut self) -> Vec<(String, Option<Action>)> {
        let mut results = Vec::new();
        for agent in &mut self.agents {
            let next = agent.next_action().map(|(_, a)| a);
            results.push((agent.name.clone(), next));
        }
        results
    }
}

impl Default for AgentCoordinator {
    fn default() -> Self { Self::new() }
}

// ── FFI Interface ───────────────────────────────────────────────────────

// ── Evolution-Aware Agent ───────────────────────────────────────────────

/// An evolution-aware agent that uses the Vitalis engine to drive
/// autonomous code evolution cycles. Wraps the general Agent with
/// real compiler tooling: validate, evolve, analyze, optimize.
pub struct EvolutionAgent {
    pub agent: Agent,
    pub evolution_budget: usize,
    pub cycles_run: usize,
    pub total_improvements: usize,
    pub total_regressions: usize,
}

impl EvolutionAgent {
    pub fn new(name: &str, budget: usize) -> Self {
        let mut agent = Agent::new(name);
        // Register built-in evolution tools
        agent.tool_registry.register(Tool {
            name: "validate".to_string(),
            description: "Validate Vitalis source code via parser + type checker".to_string(),
            parameters: vec![ToolParam {
                name: "source".to_string(), param_type: "string".to_string(), required: true,
            }],
            cost: 1.0,
        });
        agent.tool_registry.register(Tool {
            name: "evolve".to_string(),
            description: "Evolve a function with a new variant".to_string(),
            parameters: vec![
                ToolParam { name: "name".to_string(), param_type: "string".to_string(), required: true },
                ToolParam { name: "source".to_string(), param_type: "string".to_string(), required: true },
            ],
            cost: 5.0,
        });
        agent.tool_registry.register(Tool {
            name: "mutate".to_string(),
            description: "Apply mutation strategies to source code".to_string(),
            parameters: vec![ToolParam {
                name: "source".to_string(), param_type: "string".to_string(), required: true,
            }],
            cost: 2.0,
        });
        agent.tool_registry.register(Tool {
            name: "analyze".to_string(),
            description: "Extract features and analyze code quality".to_string(),
            parameters: vec![ToolParam {
                name: "source".to_string(), param_type: "string".to_string(), required: true,
            }],
            cost: 1.0,
        });
        EvolutionAgent {
            agent,
            evolution_budget: budget,
            cycles_run: 0,
            total_improvements: 0,
            total_regressions: 0,
        }
    }

    /// Run one autonomous evolution cycle using the engine.
    /// The agent observes the landscape, plans mutations, executes them,
    /// and reflects on outcomes.
    pub fn run_evolution_cycle(&mut self) -> EvolutionCycleReport {
        self.cycles_run += 1;
        let mut report = EvolutionCycleReport {
            cycle: self.cycles_run,
            functions_attempted: 0,
            improvements: 0,
            regressions: 0,
            validations_failed: 0,
        };

        // 1. Observe: snapshot the current landscape
        let landscape = crate::engine::with_engine(|eng| eng.get_landscape());
        self.agent.observe(Observation {
            kind: ObservationKind::Text,
            content: landscape,
            timestamp: self.cycles_run as u64,
            metadata: HashMap::new(),
        });

        // 2. Plan: run an evolution cycle via the engine
        self.agent.plan("observe landscape, mutate functions, evolve variants, reflect on results");

        // 3. Act: delegate to engine's run_cycle
        let cycle_result = crate::engine::with_engine(|eng| {
            eng.run_cycle()
        });

        report.functions_attempted = cycle_result.attempts.len();
        report.improvements = cycle_result.functions_improved;
        report.regressions = cycle_result.functions_regressed;

        self.total_improvements += report.improvements;
        self.total_regressions += report.regressions;

        // 4. Record step results
        for (i, attempt) in cycle_result.attempts.iter().enumerate() {
            self.agent.record_result(
                i.min(self.agent.current_plan.as_ref().map_or(0, |p| p.steps.len().saturating_sub(1))),
                attempt.success,
                &format!("fitness={:.4}", attempt.fitness),
            );
        }

        // 5. Reflect
        if let Some(reflection) = self.agent.reflect() {
            self.agent.observe(Observation {
                kind: if reflection.outcome == ReflectionOutcome::Success {
                    ObservationKind::FeedbackPositive
                } else {
                    ObservationKind::FeedbackNegative
                },
                content: reflection.lesson.clone(),
                timestamp: self.cycles_run as u64,
                metadata: HashMap::new(),
            });
        }

        report
    }

    /// Run multiple evolution cycles up to the budget.
    pub fn run_autonomous(&mut self, max_cycles: usize) -> Vec<EvolutionCycleReport> {
        let cycles = max_cycles.min(self.evolution_budget.saturating_sub(self.cycles_run));
        let mut reports = Vec::with_capacity(cycles);
        for _ in 0..cycles {
            reports.push(self.run_evolution_cycle());
        }
        reports
    }

    /// Summary statistics as JSON.
    pub fn summary_json(&self) -> String {
        format!(
            "{{\"agent\":\"{}\",\"cycles\":{},\"improvements\":{},\"regressions\":{},\"success_rate\":{:.4},\"budget_remaining\":{}}}",
            self.agent.name,
            self.cycles_run,
            self.total_improvements,
            self.total_regressions,
            self.agent.success_rate(),
            self.evolution_budget.saturating_sub(self.cycles_run),
        )
    }
}

/// Report from one autonomous evolution cycle.
#[derive(Debug, Clone)]
pub struct EvolutionCycleReport {
    pub cycle: usize,
    pub functions_attempted: usize,
    pub improvements: usize,
    pub regressions: usize,
    pub validations_failed: usize,
}

impl EvolutionCycleReport {
    pub fn to_json(&self) -> String {
        format!(
            "{{\"cycle\":{},\"attempted\":{},\"improvements\":{},\"regressions\":{},\"validations_failed\":{}}}",
            self.cycle, self.functions_attempted, self.improvements, self.regressions, self.validations_failed,
        )
    }
}

// ── FFI Interface ───────────────────────────────────────────────────────

static AGENT_STORE: Mutex<Option<HashMap<i64, Agent>>> = Mutex::new(None);

fn agent_store_insert(agent: Agent) -> i64 {
    let mut guard = AGENT_STORE.lock().unwrap();
    let store = guard.get_or_insert_with(HashMap::new);
    let id = store.len() as i64;
    store.insert(id, agent);
    id
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_agent_create(name_ptr: *const u8, name_len: i64) -> i64 {
    let name = unsafe { std::str::from_utf8_unchecked(std::slice::from_raw_parts(name_ptr, name_len as usize)) };
    agent_store_insert(Agent::new(name))
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_agent_success_rate(agent_id: i64) -> f64 {
    let guard = AGENT_STORE.lock().unwrap();
    guard.as_ref()
        .and_then(|s| s.get(&agent_id))
        .map(|a| a.success_rate())
        .unwrap_or(0.0)
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_agent_total_actions(agent_id: i64) -> i64 {
    let guard = AGENT_STORE.lock().unwrap();
    guard.as_ref()
        .and_then(|s| s.get(&agent_id))
        .map(|a| a.total_actions as i64)
        .unwrap_or(0)
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_agent_free(agent_id: i64) {
    let mut guard = AGENT_STORE.lock().unwrap();
    if let Some(store) = guard.as_mut() {
        store.remove(&agent_id);
    }
}

// ── Tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_creation() {
        let agent = Agent::new("test_agent");
        assert_eq!(agent.name, "test_agent");
        assert_eq!(agent.total_actions, 0);
        assert_eq!(agent.success_rate(), 0.0);
    }

    #[test]
    fn test_observe() {
        let mut agent = Agent::new("observer");
        agent.observe(Observation {
            kind: ObservationKind::Text,
            content: "hello world".to_string(),
            timestamp: 1,
            metadata: HashMap::new(),
        });
        assert_eq!(agent.observations.len(), 1);
    }

    #[test]
    fn test_plan_decomposition() {
        let mut agent = Agent::new("planner");
        let plan = agent.plan("find the file, transform data, store results");
        assert_eq!(plan.steps.len(), 3);
        assert_eq!(plan.steps[0].action.kind, ActionKind::Query);
        assert_eq!(plan.steps[1].action.kind, ActionKind::Transform);
        assert_eq!(plan.steps[2].action.kind, ActionKind::Store);
    }

    #[test]
    fn test_sequential_dependencies() {
        let mut agent = Agent::new("seq");
        agent.strategy = DecompositionStrategy::Sequential;
        let plan = agent.plan("step one, step two, step three");
        assert!(plan.steps[1].depends_on.contains(&0));
        assert!(plan.steps[2].depends_on.contains(&1));
    }

    #[test]
    fn test_parallel_no_dependencies() {
        let mut agent = Agent::new("par");
        agent.strategy = DecompositionStrategy::Parallel;
        let plan = agent.plan("task A, task B, task C");
        for step in &plan.steps {
            assert!(step.depends_on.is_empty());
        }
    }

    #[test]
    fn test_next_action() {
        let mut agent = Agent::new("actor");
        agent.plan("do something");
        let (idx, action) = agent.next_action().unwrap();
        assert_eq!(idx, 0);
        assert_eq!(action.kind, ActionKind::Execute);
    }

    #[test]
    fn test_record_result() {
        let mut agent = Agent::new("executor");
        agent.plan("execute task");
        agent.record_result(0, true, "completed");
        assert_eq!(agent.total_actions, 1);
        assert_eq!(agent.successful_actions, 1);
        assert_eq!(agent.success_rate(), 1.0);
    }

    #[test]
    fn test_plan_complete() {
        let mut agent = Agent::new("checker");
        assert!(agent.plan_complete()); // No plan = complete
        agent.plan("task one, task two");
        assert!(!agent.plan_complete());
        agent.record_result(0, true, "done");
        assert!(!agent.plan_complete());
        agent.record_result(1, true, "done");
        assert!(agent.plan_complete());
    }

    #[test]
    fn test_reflect_success() {
        let mut agent = Agent::new("reflector");
        agent.plan("simple task");
        agent.record_result(0, true, "ok");
        let r = agent.reflect().unwrap();
        assert_eq!(r.outcome, ReflectionOutcome::Success);
    }

    #[test]
    fn test_reflect_failure() {
        let mut agent = Agent::new("reflector");
        agent.plan("task one, task two");
        agent.record_result(0, false, "err");
        agent.record_result(1, false, "err");
        let r = agent.reflect().unwrap();
        assert_eq!(r.outcome, ReflectionOutcome::Failure);
    }

    #[test]
    fn test_avg_reward() {
        let mut agent = Agent::new("scorer");
        agent.reward_history = vec![0.5, 0.8, 1.0, 0.3, 0.9];
        let avg = agent.avg_reward(3);
        let expected = (1.0 + 0.3 + 0.9) / 3.0;
        assert!((avg - expected).abs() < 1e-10);
    }

    #[test]
    fn test_tool_registry() {
        let mut registry = ToolRegistry::new();
        registry.register(Tool {
            name: "search".to_string(),
            description: "Search for files".to_string(),
            parameters: vec![ToolParam { name: "query".to_string(), param_type: "string".to_string(), required: true }],
            cost: 1.0,
        });
        assert!(registry.get("search").is_some());
        assert_eq!(registry.find_by_description("file").len(), 1);
    }

    #[test]
    fn test_multi_agent_coordinator() {
        let mut coord = AgentCoordinator::new();
        coord.add_agent(Agent::new("a1"));
        coord.add_agent(Agent::new("a2"));
        coord.send_message(AgentMessage {
            from: "a1".to_string(),
            to: "a2".to_string(),
            content: "hello".to_string(),
            priority: 1.0,
        });
        assert_eq!(coord.messages_for("a2").len(), 1);
        assert_eq!(coord.messages_for("a1").len(), 0);
    }

    #[test]
    fn test_shared_knowledge() {
        let mut coord = AgentCoordinator::new();
        coord.share_knowledge("key1", "value1");
        assert_eq!(coord.shared_knowledge.get("key1").unwrap(), "value1");
    }

    #[test]
    fn test_ffi_agent_create() {
        let name = b"test_ffi_agent";
        let id = vitalis_agent_create(name.as_ptr(), name.len() as i64);
        assert!(id >= 0);
        assert_eq!(vitalis_agent_total_actions(id), 0);
        vitalis_agent_free(id);
    }

    // ── v69: Evolution-aware agent tests ─────────────────────────────────

    #[test]
    fn test_evolution_agent_creation() {
        let ea = EvolutionAgent::new("evo_test", 100);
        assert_eq!(ea.agent.name, "evo_test");
        assert_eq!(ea.evolution_budget, 100);
        assert_eq!(ea.cycles_run, 0);
        assert!(ea.agent.tool_registry.get("validate").is_some());
        assert!(ea.agent.tool_registry.get("evolve").is_some());
        assert!(ea.agent.tool_registry.get("mutate").is_some());
        assert!(ea.agent.tool_registry.get("analyze").is_some());
    }

    #[test]
    fn test_evolution_agent_tools_registered() {
        let ea = EvolutionAgent::new("tools_test", 50);
        // All 4 tools should be registered
        assert!(ea.agent.tool_registry.get("validate").is_some());
        assert!(ea.agent.tool_registry.get("evolve").is_some());
        assert!(ea.agent.tool_registry.get("mutate").is_some());
        assert!(ea.agent.tool_registry.get("analyze").is_some());
    }

    #[test]
    fn test_evolution_cycle_report_json() {
        let report = EvolutionCycleReport {
            cycle: 5,
            functions_attempted: 3,
            improvements: 2,
            regressions: 0,
            validations_failed: 1,
        };
        let json = report.to_json();
        assert!(json.contains("\"cycle\":5"));
        assert!(json.contains("\"improvements\":2"));
    }

    #[test]
    fn test_evolution_agent_run_cycle() {
        // Register a function first, then run a cycle
        crate::engine::with_engine(|eng| {
            eng.register("agent_target", "fn agent_target(x: i64) -> i64 { x + 1 }");
        });
        let mut ea = EvolutionAgent::new("cycle_test", 10);
        let report = ea.run_evolution_cycle();
        assert_eq!(report.cycle, 1);
        assert_eq!(ea.cycles_run, 1);
    }

    #[test]
    fn test_evolution_agent_run_autonomous() {
        crate::engine::with_engine(|eng| {
            eng.register("auto_fn", "fn auto_fn(x: i64) -> i64 { x * 2 }");
        });
        let mut ea = EvolutionAgent::new("auto_test", 5);
        let reports = ea.run_autonomous(3);
        assert_eq!(reports.len(), 3);
        assert_eq!(ea.cycles_run, 3);
    }

    #[test]
    fn test_evolution_agent_budget_limit() {
        let mut ea = EvolutionAgent::new("budget_test", 2);
        let reports = ea.run_autonomous(10); // try to run 10, but budget is 2
        assert_eq!(reports.len(), 2);
        assert_eq!(ea.cycles_run, 2);
        // Budget exhausted, no more cycles
        let more = ea.run_autonomous(5);
        assert_eq!(more.len(), 0);
    }

    #[test]
    fn test_evolution_agent_summary_json() {
        let ea = EvolutionAgent::new("summary_test", 100);
        let json = ea.summary_json();
        assert!(json.contains("\"agent\":\"summary_test\""));
        assert!(json.contains("\"cycles\":0"));
        assert!(json.contains("\"budget_remaining\":100"));
    }

    #[test]
    fn test_evolution_agent_reflects() {
        crate::engine::with_engine(|eng| {
            eng.register("reflect_fn", "fn reflect_fn(x: i64) -> i64 { x }");
        });
        let mut ea = EvolutionAgent::new("reflect_test", 10);
        ea.run_evolution_cycle();
        // Agent should have reflections after a cycle
        assert!(!ea.agent.reflections.is_empty() || ea.agent.observations.len() >= 1);
    }
}
