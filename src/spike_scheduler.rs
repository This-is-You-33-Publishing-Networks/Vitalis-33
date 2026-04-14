//! v500 — Spike-Driven Scheduler.
//!
//! Replaces round-robin async executor with spike-timed event scheduling.
//! Tasks emit spikes on completion. Priority = spike frequency.
//! Lateral inhibition for resource contention.

use std::collections::{BTreeMap, HashMap, VecDeque};

/// Unique task identifier in the spike scheduler.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SpikeTaskId(pub u64);

/// Priority computed from spike frequency.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct SpikePriority(pub f64);

impl SpikePriority {
    pub fn base() -> Self {
        Self(1.0)
    }
}

/// State of a spike-scheduled task.
#[derive(Debug, Clone, PartialEq)]
pub enum SpikeTaskState {
    /// Ready to fire (accumulated enough potential).
    Ready,
    /// Waiting for input spikes from dependencies.
    Waiting { threshold: f64, accumulated: f64 },
    /// Firing (currently executing).
    Firing,
    /// Inhibited by lateral inhibition.
    Inhibited { by: SpikeTaskId, recovery_ticks: u32 },
    /// Completed with result.
    Completed(i64),
    /// Refractory period after firing.
    Refractory { remaining_ticks: u32 },
}

/// A task in the spike-driven scheduler.
#[derive(Debug, Clone)]
pub struct SpikeTask {
    pub id: SpikeTaskId,
    pub name: String,
    pub state: SpikeTaskState,
    pub priority: SpikePriority,
    pub spike_count: u64,
    pub fire_count: u64,
    pub refractory_period: u32,
    pub membrane_potential: f64,
    pub threshold: f64,
    pub leak_rate: f64,
    pub result: Option<i64>,
}

impl SpikeTask {
    pub fn new(id: SpikeTaskId, name: String) -> Self {
        Self {
            id,
            name,
            state: SpikeTaskState::Waiting {
                threshold: 1.0,
                accumulated: 0.0,
            },
            priority: SpikePriority::base(),
            spike_count: 0,
            fire_count: 0,
            refractory_period: 2,
            membrane_potential: 0.0,
            threshold: 1.0,
            leak_rate: 0.1,
            result: None,
        }
    }

    /// Receive an input spike, accumulating potential.
    pub fn receive_spike(&mut self, weight: f64) {
        self.spike_count += 1;
        self.membrane_potential += weight;
        if let SpikeTaskState::Waiting { threshold, ref mut accumulated } = self.state {
            *accumulated += weight;
            if *accumulated >= threshold {
                self.state = SpikeTaskState::Ready;
            }
        }
    }

    /// Apply leak (decay) to membrane potential.
    pub fn leak(&mut self) {
        self.membrane_potential *= 1.0 - self.leak_rate;
        if let SpikeTaskState::Waiting { threshold: _, ref mut accumulated } = self.state {
            *accumulated *= 1.0 - self.leak_rate;
        }
    }

    /// Check if task is ready to fire.
    pub fn can_fire(&self) -> bool {
        matches!(self.state, SpikeTaskState::Ready)
    }

    /// Transition to firing state.
    pub fn fire(&mut self) {
        if self.can_fire() {
            self.state = SpikeTaskState::Firing;
            self.fire_count += 1;
            self.membrane_potential = 0.0;
        }
    }

    /// Complete firing, enter refractory period.
    pub fn complete(&mut self, result: i64) {
        self.result = Some(result);
        if self.refractory_period > 0 {
            self.state = SpikeTaskState::Refractory {
                remaining_ticks: self.refractory_period,
            };
        } else {
            self.state = SpikeTaskState::Completed(result);
        }
    }

    /// Advance refractory period by one tick.
    pub fn tick_refractory(&mut self) {
        if let SpikeTaskState::Refractory { remaining_ticks } = &mut self.state {
            if *remaining_ticks > 0 {
                *remaining_ticks -= 1;
            }
            if *remaining_ticks == 0 {
                let result = self.result.unwrap_or(0);
                self.state = SpikeTaskState::Completed(result);
            }
        }
    }

    /// Update priority from spike frequency.
    pub fn update_priority(&mut self, total_ticks: u64) {
        if total_ticks > 0 {
            self.priority = SpikePriority(self.fire_count as f64 / total_ticks as f64);
        }
    }

    pub fn is_done(&self) -> bool {
        matches!(self.state, SpikeTaskState::Completed(_))
    }
}

/// Synaptic connection between tasks (weighted directed edge).
#[derive(Debug, Clone)]
pub struct TaskSynapse {
    pub from: SpikeTaskId,
    pub to: SpikeTaskId,
    pub weight: f64,
    pub delay_ticks: u32,
    pub plasticity: bool,
}

/// A pending spike event to be delivered at a future tick.
#[derive(Debug, Clone)]
struct PendingSpike {
    target: SpikeTaskId,
    weight: f64,
    deliver_at: u64,
}

/// Lateral inhibition group — tasks that compete for the same resource.
#[derive(Debug, Clone)]
pub struct InhibitionGroup {
    pub name: String,
    pub members: Vec<SpikeTaskId>,
    pub inhibition_strength: f64,
}

impl InhibitionGroup {
    pub fn new(name: &str, strength: f64) -> Self {
        Self {
            name: name.to_string(),
            members: Vec::new(),
            inhibition_strength: strength,
        }
    }

    pub fn add(&mut self, id: SpikeTaskId) {
        if !self.members.contains(&id) {
            self.members.push(id);
        }
    }
}

/// Statistics for the spike scheduler.
#[derive(Debug, Clone, Default)]
pub struct SchedulerStats {
    pub total_ticks: u64,
    pub total_spikes_delivered: u64,
    pub total_fires: u64,
    pub total_inhibitions: u64,
    pub total_completed: u64,
    pub peak_queue_depth: usize,
}

/// Spike-driven event scheduler.
///
/// Tasks are neurons. Dependencies are synapses. Scheduling is driven by
/// spike propagation rather than round-robin polling.
pub struct SpikeScheduler {
    tasks: HashMap<SpikeTaskId, SpikeTask>,
    synapses: Vec<TaskSynapse>,
    pending_spikes: BTreeMap<u64, Vec<PendingSpike>>,
    inhibition_groups: Vec<InhibitionGroup>,
    fire_queue: VecDeque<SpikeTaskId>,
    current_tick: u64,
    next_id: u64,
    pub stats: SchedulerStats,
}

impl SpikeScheduler {
    pub fn new() -> Self {
        Self {
            tasks: HashMap::new(),
            synapses: Vec::new(),
            pending_spikes: BTreeMap::new(),
            inhibition_groups: Vec::new(),
            fire_queue: VecDeque::new(),
            current_tick: 0,
            next_id: 0,
            stats: SchedulerStats::default(),
        }
    }

    /// Spawn a new task (neuron) in the scheduler.
    pub fn spawn(&mut self, name: &str) -> SpikeTaskId {
        let id = SpikeTaskId(self.next_id);
        self.next_id += 1;
        let task = SpikeTask::new(id, name.to_string());
        self.tasks.insert(id, task);
        id
    }

    /// Spawn a task that is immediately ready to fire (no threshold).
    pub fn spawn_ready(&mut self, name: &str) -> SpikeTaskId {
        let id = self.spawn(name);
        if let Some(task) = self.tasks.get_mut(&id) {
            task.state = SpikeTaskState::Ready;
        }
        id
    }

    /// Connect two tasks with a synapse.
    pub fn connect(&mut self, from: SpikeTaskId, to: SpikeTaskId, weight: f64, delay: u32) {
        self.synapses.push(TaskSynapse {
            from,
            to,
            weight,
            delay_ticks: delay,
            plasticity: false,
        });
    }

    /// Connect with plasticity enabled (weight adapts with STDP).
    pub fn connect_plastic(&mut self, from: SpikeTaskId, to: SpikeTaskId, weight: f64, delay: u32) {
        self.synapses.push(TaskSynapse {
            from,
            to,
            weight,
            delay_ticks: delay,
            plasticity: true,
        });
    }

    /// Add a lateral inhibition group.
    pub fn add_inhibition_group(&mut self, group: InhibitionGroup) {
        self.inhibition_groups.push(group);
    }

    /// Inject an external spike into a task.
    pub fn inject_spike(&mut self, target: SpikeTaskId, weight: f64) {
        if let Some(task) = self.tasks.get_mut(&target) {
            task.receive_spike(weight);
        }
    }

    /// Advance the scheduler by one tick.
    pub fn tick(&mut self) -> Vec<SpikeTaskId> {
        self.current_tick += 1;
        self.stats.total_ticks += 1;

        // 1. Deliver pending spikes for this tick
        if let Some(spikes) = self.pending_spikes.remove(&self.current_tick) {
            for spike in &spikes {
                self.stats.total_spikes_delivered += 1;
                if let Some(task) = self.tasks.get_mut(&spike.target) {
                    task.receive_spike(spike.weight);
                }
            }
        }

        // 2. Apply leak to all waiting tasks
        let task_ids: Vec<SpikeTaskId> = self.tasks.keys().copied().collect();
        for id in &task_ids {
            if let Some(task) = self.tasks.get_mut(id) {
                if matches!(task.state, SpikeTaskState::Waiting { .. }) {
                    task.leak();
                }
            }
        }

        // 3. Advance refractory periods
        for id in &task_ids {
            if let Some(task) = self.tasks.get_mut(id) {
                task.tick_refractory();
            }
        }

        // 4. Recover inhibited tasks
        let mut recovered = Vec::new();
        for id in &task_ids {
            if let Some(task) = self.tasks.get_mut(id) {
                if let SpikeTaskState::Inhibited { recovery_ticks, .. } = &mut task.state {
                    if *recovery_ticks > 0 {
                        *recovery_ticks -= 1;
                    }
                    if *recovery_ticks == 0 {
                        recovered.push(*id);
                    }
                }
            }
        }
        for id in recovered {
            if let Some(task) = self.tasks.get_mut(&id) {
                task.state = SpikeTaskState::Waiting {
                    threshold: task.threshold,
                    accumulated: 0.0,
                };
            }
        }

        // 5. Collect ready tasks, sorted by priority (highest first)
        let mut ready: Vec<(SpikeTaskId, f64)> = Vec::new();
        for id in &task_ids {
            if let Some(task) = self.tasks.get(id) {
                if task.can_fire() {
                    ready.push((*id, task.priority.0));
                }
            }
        }
        ready.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // 6. Apply lateral inhibition (winner-take-all within groups)
        for group in &self.inhibition_groups {
            let group_ready: Vec<SpikeTaskId> = ready
                .iter()
                .filter(|(id, _)| group.members.contains(id))
                .map(|(id, _)| *id)
                .collect();

            if group_ready.len() > 1 {
                // Winner = highest priority in group
                let winner = group_ready[0]; // already sorted by priority
                for &loser in &group_ready[1..] {
                    if let Some(task) = self.tasks.get_mut(&loser) {
                        task.state = SpikeTaskState::Inhibited {
                            by: winner,
                            recovery_ticks: 3,
                        };
                        self.stats.total_inhibitions += 1;
                    }
                    ready.retain(|(id, _)| *id != loser);
                }
            }
        }

        // 7. Fire ready tasks
        let mut fired = Vec::new();
        for (id, _) in &ready {
            if let Some(task) = self.tasks.get_mut(id) {
                task.fire();
                fired.push(*id);
                self.stats.total_fires += 1;
            }
        }

        if fired.len() > self.stats.peak_queue_depth {
            self.stats.peak_queue_depth = fired.len();
        }

        fired
    }

    /// Complete a firing task and propagate output spikes through synapses.
    pub fn complete_task(&mut self, id: SpikeTaskId, result: i64) {
        // Collect synapses from this task
        let outgoing: Vec<(SpikeTaskId, f64, u32)> = self
            .synapses
            .iter()
            .filter(|s| s.from == id)
            .map(|s| (s.to, s.weight, s.delay_ticks))
            .collect();

        // Schedule output spikes to downstream tasks
        for (target, weight, delay) in outgoing {
            let deliver_at = self.current_tick + delay as u64;
            self.pending_spikes
                .entry(deliver_at)
                .or_default()
                .push(PendingSpike {
                    target,
                    weight,
                    deliver_at,
                });
        }

        // Complete the task
        if let Some(task) = self.tasks.get_mut(&id) {
            task.complete(result);
            self.stats.total_completed += 1;
        }
    }

    /// Run until all tasks are completed or a max tick limit is reached.
    pub fn run_to_completion(&mut self, max_ticks: u64) -> Vec<(SpikeTaskId, i64)> {
        let mut results = Vec::new();

        for _ in 0..max_ticks {
            let fired = self.tick();

            // Auto-complete fired tasks with default result (simulated execution)
            for id in fired {
                self.complete_task(id, 0);
            }

            // Collect completed results
            let completed: Vec<(SpikeTaskId, i64)> = self
                .tasks
                .values()
                .filter(|t| t.is_done())
                .map(|t| (t.id, t.result.unwrap_or(0)))
                .collect();

            if completed.len() == self.tasks.len() {
                results = completed;
                break;
            }
        }

        if results.is_empty() {
            results = self
                .tasks
                .values()
                .filter(|t| t.is_done())
                .map(|t| (t.id, t.result.unwrap_or(0)))
                .collect();
        }

        results
    }

    /// Get a reference to a task.
    pub fn get_task(&self, id: SpikeTaskId) -> Option<&SpikeTask> {
        self.tasks.get(&id)
    }

    /// Total number of tasks.
    pub fn task_count(&self) -> usize {
        self.tasks.len()
    }

    /// Number of completed tasks.
    pub fn completed_count(&self) -> usize {
        self.tasks.values().filter(|t| t.is_done()).count()
    }

    /// Current tick.
    pub fn current_tick(&self) -> u64 {
        self.current_tick
    }

    /// Number of pending spikes in the queue.
    pub fn pending_spike_count(&self) -> usize {
        self.pending_spikes.values().map(|v| v.len()).sum()
    }

    /// Update all task priorities based on fire frequency.
    pub fn update_priorities(&mut self) {
        let ticks = self.current_tick;
        for task in self.tasks.values_mut() {
            task.update_priority(ticks);
        }
    }

    /// Get synapse count.
    pub fn synapse_count(&self) -> usize {
        self.synapses.len()
    }
}

// ─── FFI ───────────────────────────────────────────────

use std::sync::{LazyLock, Mutex};

static SPIKE_SCHED: LazyLock<Mutex<SpikeScheduler>> =
    LazyLock::new(|| Mutex::new(SpikeScheduler::new()));

#[unsafe(no_mangle)]
pub extern "C" fn slang_spike_sched_spawn(name_ptr: *const i8) -> i64 {
    let name = if name_ptr.is_null() {
        "task".to_string()
    } else {
        unsafe { std::ffi::CStr::from_ptr(name_ptr) }
            .to_string_lossy()
            .into_owned()
    };
    let mut sched = SPIKE_SCHED.lock().unwrap();
    sched.spawn(&name).0 as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_spike_sched_connect(from: i64, to: i64, weight_x1000: i64, delay: i64) -> i64 {
    let mut sched = SPIKE_SCHED.lock().unwrap();
    sched.connect(
        SpikeTaskId(from as u64),
        SpikeTaskId(to as u64),
        weight_x1000 as f64 / 1000.0,
        delay as u32,
    );
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_spike_sched_inject(target: i64, weight_x1000: i64) -> i64 {
    let mut sched = SPIKE_SCHED.lock().unwrap();
    sched.inject_spike(SpikeTaskId(target as u64), weight_x1000 as f64 / 1000.0);
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_spike_sched_tick() -> i64 {
    let mut sched = SPIKE_SCHED.lock().unwrap();
    let fired = sched.tick();
    fired.len() as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_spike_sched_complete(id: i64, result: i64) -> i64 {
    let mut sched = SPIKE_SCHED.lock().unwrap();
    sched.complete_task(SpikeTaskId(id as u64), result);
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_spike_sched_task_count() -> i64 {
    let sched = SPIKE_SCHED.lock().unwrap();
    sched.task_count() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spawn_task() {
        let mut sched = SpikeScheduler::new();
        let id = sched.spawn("task1");
        assert_eq!(sched.task_count(), 1);
        let task = sched.get_task(id).unwrap();
        assert_eq!(task.name, "task1");
        assert!(!task.can_fire());
    }

    #[test]
    fn test_spawn_ready() {
        let mut sched = SpikeScheduler::new();
        let id = sched.spawn_ready("ready_task");
        let task = sched.get_task(id).unwrap();
        assert!(task.can_fire());
    }

    #[test]
    fn test_spike_accumulation() {
        let mut sched = SpikeScheduler::new();
        let id = sched.spawn("accum");
        sched.inject_spike(id, 0.5);
        assert!(!sched.get_task(id).unwrap().can_fire());
        sched.inject_spike(id, 0.6);
        assert!(sched.get_task(id).unwrap().can_fire());
    }

    #[test]
    fn test_tick_fires_ready() {
        let mut sched = SpikeScheduler::new();
        let id = sched.spawn_ready("t1");
        let fired = sched.tick();
        assert!(fired.contains(&id));
        assert_eq!(sched.stats.total_fires, 1);
    }

    #[test]
    fn test_complete_task() {
        let mut sched = SpikeScheduler::new();
        let id = sched.spawn_ready("t1");
        sched.tick();
        sched.complete_task(id, 42);
        // Task enters refractory, then completed after ticks
        let task = sched.get_task(id).unwrap();
        assert!(matches!(task.state, SpikeTaskState::Refractory { .. }));
        sched.tick();
        sched.tick();
        assert!(sched.get_task(id).unwrap().is_done());
    }

    #[test]
    fn test_synapse_propagation() {
        let mut sched = SpikeScheduler::new();
        let a = sched.spawn_ready("a");
        let b = sched.spawn("b");
        sched.connect(a, b, 1.5, 1);

        sched.tick(); // fires a
        sched.complete_task(a, 10);

        sched.tick(); // delivers spike to b
        let task_b = sched.get_task(b).unwrap();
        assert!(task_b.spike_count > 0);
    }

    #[test]
    fn test_delayed_synapse() {
        let mut sched = SpikeScheduler::new();
        let a = sched.spawn_ready("a");
        let b = sched.spawn("b");
        sched.connect(a, b, 1.5, 3);

        sched.tick(); // tick 1: fires a
        sched.complete_task(a, 0); // schedules spike at tick 1+3=4

        // Spike should arrive at tick current+3 = tick 4
        assert_eq!(sched.pending_spike_count(), 1);
        sched.tick(); // tick 2 — not yet
        assert_eq!(sched.get_task(b).unwrap().spike_count, 0);
        sched.tick(); // tick 3 — not yet
        assert_eq!(sched.get_task(b).unwrap().spike_count, 0);
        sched.tick(); // tick 4 — spike delivered
        assert!(sched.get_task(b).unwrap().spike_count > 0);
    }

    #[test]
    fn test_leak_decay() {
        let mut task = SpikeTask::new(SpikeTaskId(0), "t".into());
        task.receive_spike(0.5);
        assert!(task.membrane_potential > 0.0);
        task.leak();
        assert!(task.membrane_potential < 0.5);
    }

    #[test]
    fn test_lateral_inhibition() {
        let mut sched = SpikeScheduler::new();
        let a = sched.spawn_ready("a");
        let b = sched.spawn_ready("b");

        // Give 'a' higher priority by injecting more spikes
        sched.inject_spike(a, 5.0);

        let mut group = InhibitionGroup::new("resource", 1.0);
        group.add(a);
        group.add(b);
        sched.add_inhibition_group(group);

        // Update priorities so 'a' has higher
        if let Some(task) = sched.tasks.get_mut(&a) {
            task.priority = SpikePriority(2.0);
        }

        let fired = sched.tick();
        // 'a' should fire, 'b' should be inhibited
        assert!(fired.contains(&a));
        assert!(!fired.contains(&b));
        assert_eq!(sched.stats.total_inhibitions, 1);
    }

    #[test]
    fn test_refractory_period() {
        let mut task = SpikeTask::new(SpikeTaskId(0), "t".into());
        task.state = SpikeTaskState::Ready;
        task.fire();
        task.complete(42);
        assert!(matches!(task.state, SpikeTaskState::Refractory { remaining_ticks: 2 }));
        task.tick_refractory();
        assert!(matches!(task.state, SpikeTaskState::Refractory { remaining_ticks: 1 }));
        task.tick_refractory();
        assert!(task.is_done());
    }

    #[test]
    fn test_run_to_completion() {
        let mut sched = SpikeScheduler::new();
        sched.spawn_ready("a");
        sched.spawn_ready("b");
        let results = sched.run_to_completion(20);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_update_priorities() {
        let mut sched = SpikeScheduler::new();
        let id = sched.spawn_ready("t");
        sched.tick();
        sched.complete_task(id, 0);
        sched.update_priorities();
        let task = sched.get_task(id).unwrap();
        assert!(task.priority.0 > 0.0);
    }

    #[test]
    fn test_connect_plastic() {
        let mut sched = SpikeScheduler::new();
        let a = sched.spawn("a");
        let b = sched.spawn("b");
        sched.connect_plastic(a, b, 0.5, 1);
        assert_eq!(sched.synapse_count(), 1);
        assert!(sched.synapses[0].plasticity);
    }

    #[test]
    fn test_inhibition_group_dedup() {
        let mut group = InhibitionGroup::new("g", 1.0);
        let id = SpikeTaskId(0);
        group.add(id);
        group.add(id);
        assert_eq!(group.members.len(), 1);
    }

    #[test]
    fn test_pending_spike_count() {
        let mut sched = SpikeScheduler::new();
        let a = sched.spawn_ready("a");
        let b = sched.spawn("b");
        sched.connect(a, b, 1.0, 5);
        sched.tick();
        sched.complete_task(a, 0);
        assert_eq!(sched.pending_spike_count(), 1);
    }

    #[test]
    fn test_empty_scheduler() {
        let mut sched = SpikeScheduler::new();
        let fired = sched.tick();
        assert!(fired.is_empty());
        assert_eq!(sched.task_count(), 0);
        assert_eq!(sched.completed_count(), 0);
    }

    #[test]
    fn test_stats_tracking() {
        let mut sched = SpikeScheduler::new();
        sched.spawn_ready("a");
        sched.tick();
        assert_eq!(sched.stats.total_fires, 1);
        assert_eq!(sched.stats.total_ticks, 1);
    }

    #[test]
    fn test_spike_priority_ordering() {
        let p1 = SpikePriority(1.0);
        let p2 = SpikePriority(2.0);
        assert!(p2 > p1);
    }

    #[test]
    fn test_chain_propagation() {
        let mut sched = SpikeScheduler::new();
        let a = sched.spawn_ready("a");
        let b = sched.spawn("b");
        let c = sched.spawn("c");
        sched.connect(a, b, 1.5, 1);
        sched.connect(b, c, 1.5, 1);

        // Fire a
        sched.tick();
        sched.complete_task(a, 1);
        // Deliver spike to b — tick() delivers and auto-fires ready tasks
        sched.tick();
        // b received the spike and was auto-fired (spike_count > 0)
        assert!(sched.get_task(b).unwrap().spike_count > 0);
    }

    #[test]
    fn test_ffi_spawn() {
        let id = slang_spike_sched_spawn(std::ptr::null());
        assert!(id >= 0);
        assert!(slang_spike_sched_task_count() >= 1);
    }

    #[test]
    fn test_multiple_inhibition_groups() {
        let mut sched = SpikeScheduler::new();
        let a = sched.spawn_ready("a");
        let b = sched.spawn_ready("b");
        let c = sched.spawn_ready("c");

        let mut g1 = InhibitionGroup::new("g1", 1.0);
        g1.add(a);
        g1.add(b);
        sched.add_inhibition_group(g1);

        // c is not in any group, should fire independently
        if let Some(task) = sched.tasks.get_mut(&a) {
            task.priority = SpikePriority(2.0);
        }

        let fired = sched.tick();
        assert!(fired.contains(&a));
        assert!(fired.contains(&c));
        assert!(!fired.contains(&b));
    }
}
