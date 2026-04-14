//! Scheduler — v372
//! Task scheduling with cron expressions, priority queues, and timed execution.

use std::collections::BinaryHeap;
use std::cmp::Ordering;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskState {
    Pending,
    Running,
    Completed,
    Cancelled,
}

#[derive(Debug, Clone)]
pub struct ScheduledTask {
    pub id: i64,
    pub priority: i64,
    pub execute_at: i64,
    pub state: TaskState,
    pub payload: i64,
}

impl PartialEq for ScheduledTask {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
impl Eq for ScheduledTask {}

impl PartialOrd for ScheduledTask {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ScheduledTask {
    fn cmp(&self, other: &Self) -> Ordering {
        self.priority.cmp(&other.priority)
            .then_with(|| other.execute_at.cmp(&self.execute_at))
    }
}

/// Cron expression with 5 fields: minute, hour, day, month, weekday.
/// Supports: *, specific value, ranges (not full cron but demonstrates concept).
#[derive(Debug, Clone)]
pub struct CronExpr {
    pub minute: CronField,
    pub hour: CronField,
    pub day: CronField,
    pub month: CronField,
    pub weekday: CronField,
}

#[derive(Debug, Clone)]
pub enum CronField {
    Any,
    Value(i64),
    Range(i64, i64),
    Step(i64),
}

impl CronExpr {
    pub fn parse(expr: &str) -> Option<Self> {
        let parts: Vec<&str> = expr.split_whitespace().collect();
        if parts.len() != 5 {
            return None;
        }
        Some(CronExpr {
            minute: Self::parse_field(parts[0])?,
            hour: Self::parse_field(parts[1])?,
            day: Self::parse_field(parts[2])?,
            month: Self::parse_field(parts[3])?,
            weekday: Self::parse_field(parts[4])?,
        })
    }

    fn parse_field(field: &str) -> Option<CronField> {
        if field == "*" {
            return Some(CronField::Any);
        }
        if let Some(step) = field.strip_prefix("*/") {
            return step.parse().ok().map(CronField::Step);
        }
        if field.contains('-') {
            let parts: Vec<&str> = field.split('-').collect();
            if parts.len() == 2 {
                let start = parts[0].parse().ok()?;
                let end = parts[1].parse().ok()?;
                return Some(CronField::Range(start, end));
            }
        }
        field.parse().ok().map(CronField::Value)
    }

    pub fn matches(&self, minute: i64, hour: i64, day: i64, month: i64, weekday: i64) -> bool {
        Self::field_matches(&self.minute, minute)
            && Self::field_matches(&self.hour, hour)
            && Self::field_matches(&self.day, day)
            && Self::field_matches(&self.month, month)
            && Self::field_matches(&self.weekday, weekday)
    }

    fn field_matches(field: &CronField, value: i64) -> bool {
        match field {
            CronField::Any => true,
            CronField::Value(v) => value == *v,
            CronField::Range(lo, hi) => value >= *lo && value <= *hi,
            CronField::Step(s) => *s > 0 && value % *s == 0,
        }
    }

    /// Calculate next execution as minutes from `current_minute`.
    pub fn next_from(&self, current_minute: i64) -> i64 {
        match &self.minute {
            CronField::Value(v) => {
                if *v > current_minute { *v } else { *v + 60 }
            }
            CronField::Step(s) => {
                if *s <= 0 { return current_minute + 1; }
                let next = ((current_minute / *s) + 1) * *s;
                next
            }
            CronField::Range(lo, _hi) => {
                if *lo > current_minute { *lo } else { *lo + 60 }
            }
            CronField::Any => current_minute + 1,
        }
    }
}

#[derive(Debug)]
pub struct Scheduler {
    pub queue: BinaryHeap<ScheduledTask>,
    pub completed: Vec<ScheduledTask>,
    pub next_id: i64,
    pub current_time: i64,
}

impl Scheduler {
    pub fn new() -> Self {
        Self {
            queue: BinaryHeap::new(),
            completed: Vec::new(),
            next_id: 1,
            current_time: 0,
        }
    }

    pub fn add_task(&mut self, priority: i64, execute_at: i64, payload: i64) -> i64 {
        let id = self.next_id;
        self.next_id += 1;
        self.queue.push(ScheduledTask {
            id,
            priority,
            execute_at,
            state: TaskState::Pending,
            payload,
        });
        id
    }

    pub fn run_pending(&mut self, current_time: i64) -> i64 {
        self.current_time = current_time;
        let mut executed = 0;
        let mut remaining = Vec::new();
        while let Some(mut task) = self.queue.pop() {
            if task.execute_at <= current_time && task.state == TaskState::Pending {
                task.state = TaskState::Completed;
                executed += 1;
                self.completed.push(task);
            } else {
                remaining.push(task);
            }
        }
        for t in remaining {
            self.queue.push(t);
        }
        executed
    }

    pub fn cancel(&mut self, task_id: i64) -> bool {
        let mut found = false;
        let tasks: Vec<_> = self.queue.drain().collect();
        for mut t in tasks {
            if t.id == task_id {
                t.state = TaskState::Cancelled;
                found = true;
                self.completed.push(t);
            } else {
                self.queue.push(t);
            }
        }
        found
    }

    pub fn pending_count(&self) -> i64 {
        self.queue.len() as i64
    }

    pub fn clear(&mut self) {
        self.queue.clear();
        self.completed.clear();
    }

    pub fn completed_count(&self) -> i64 {
        self.completed.iter().filter(|t| t.state == TaskState::Completed).count() as i64
    }
}

use std::sync::Mutex;
use std::sync::LazyLock;
static SCHED: LazyLock<Mutex<Scheduler>> = LazyLock::new(|| Mutex::new(Scheduler::new()));

#[unsafe(no_mangle)]
pub extern "C" fn slang_sched_create() -> i64 {
    *SCHED.lock().unwrap() = Scheduler::new();
    1
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_sched_add_task(priority: i64, execute_at: i64, payload: i64) -> i64 {
    SCHED.lock().unwrap().add_task(priority, execute_at, payload)
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_sched_run_pending(current_time: i64) -> i64 {
    SCHED.lock().unwrap().run_pending(current_time)
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_sched_cancel(task_id: i64) -> i64 {
    if SCHED.lock().unwrap().cancel(task_id) { 1 } else { 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_sched_pending_count() -> i64 {
    SCHED.lock().unwrap().pending_count()
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_sched_cron_parse(minute: i64) -> i64 {
    let expr = format!("{} * * * *", minute);
    if CronExpr::parse(&expr).is_some() { 1 } else { 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_sched_cron_next(current_minute: i64, target_minute: i64) -> i64 {
    let expr = format!("{} * * * *", target_minute);
    if let Some(cron) = CronExpr::parse(&expr) {
        cron.next_from(current_minute)
    } else {
        -1
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_sched_clear() -> i64 {
    SCHED.lock().unwrap().clear();
    1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_scheduler() {
        let sched = Scheduler::new();
        assert_eq!(sched.pending_count(), 0);
    }

    #[test]
    fn test_add_task() {
        let mut sched = Scheduler::new();
        let id = sched.add_task(1, 100, 42);
        assert_eq!(id, 1);
        assert_eq!(sched.pending_count(), 1);
    }

    #[test]
    fn test_run_pending() {
        let mut sched = Scheduler::new();
        sched.add_task(1, 50, 1);
        sched.add_task(1, 100, 2);
        let executed = sched.run_pending(75);
        assert_eq!(executed, 1);
        assert_eq!(sched.pending_count(), 1);
    }

    #[test]
    fn test_priority_ordering() {
        let mut sched = Scheduler::new();
        sched.add_task(1, 0, 10);
        sched.add_task(10, 0, 20);
        sched.add_task(5, 0, 30);
        let executed = sched.run_pending(100);
        assert_eq!(executed, 3);
        assert_eq!(sched.completed[0].priority, 10);
    }

    #[test]
    fn test_cancel_task() {
        let mut sched = Scheduler::new();
        let id = sched.add_task(1, 100, 1);
        assert!(sched.cancel(id));
        assert_eq!(sched.pending_count(), 0);
    }

    #[test]
    fn test_cancel_nonexistent() {
        let mut sched = Scheduler::new();
        assert!(!sched.cancel(999));
    }

    #[test]
    fn test_clear() {
        let mut sched = Scheduler::new();
        sched.add_task(1, 0, 1);
        sched.add_task(1, 0, 2);
        sched.clear();
        assert_eq!(sched.pending_count(), 0);
    }

    #[test]
    fn test_completed_count() {
        let mut sched = Scheduler::new();
        sched.add_task(1, 0, 1);
        sched.add_task(1, 0, 2);
        sched.run_pending(100);
        assert_eq!(sched.completed_count(), 2);
    }

    #[test]
    fn test_future_tasks_not_run() {
        let mut sched = Scheduler::new();
        sched.add_task(1, 1000, 1);
        let executed = sched.run_pending(500);
        assert_eq!(executed, 0);
        assert_eq!(sched.pending_count(), 1);
    }

    #[test]
    fn test_cron_parse_valid() {
        let cron = CronExpr::parse("30 * * * *");
        assert!(cron.is_some());
    }

    #[test]
    fn test_cron_parse_invalid() {
        assert!(CronExpr::parse("invalid").is_none());
        assert!(CronExpr::parse("* *").is_none());
    }

    #[test]
    fn test_cron_any() {
        let cron = CronExpr::parse("* * * * *").unwrap();
        assert!(cron.matches(0, 0, 1, 1, 0));
        assert!(cron.matches(59, 23, 31, 12, 6));
    }

    #[test]
    fn test_cron_specific_minute() {
        let cron = CronExpr::parse("30 * * * *").unwrap();
        assert!(cron.matches(30, 12, 1, 1, 0));
        assert!(!cron.matches(15, 12, 1, 1, 0));
    }

    #[test]
    fn test_cron_range() {
        let cron = CronExpr::parse("0-15 * * * *").unwrap();
        assert!(cron.matches(10, 0, 1, 1, 0));
        assert!(!cron.matches(20, 0, 1, 1, 0));
    }

    #[test]
    fn test_cron_step() {
        let cron = CronExpr::parse("*/15 * * * *").unwrap();
        assert!(cron.matches(0, 0, 1, 1, 0));
        assert!(cron.matches(15, 0, 1, 1, 0));
        assert!(cron.matches(30, 0, 1, 1, 0));
        assert!(!cron.matches(7, 0, 1, 1, 0));
    }

    #[test]
    fn test_cron_next() {
        let cron = CronExpr::parse("30 * * * *").unwrap();
        let next = cron.next_from(10);
        assert_eq!(next, 30);
    }

    #[test]
    fn test_cron_next_wrap() {
        let cron = CronExpr::parse("10 * * * *").unwrap();
        let next = cron.next_from(20);
        assert_eq!(next, 70);
    }

    #[test]
    fn test_multiple_tasks_same_time() {
        let mut sched = Scheduler::new();
        for i in 0..5 {
            sched.add_task(1, 50, i);
        }
        let executed = sched.run_pending(50);
        assert_eq!(executed, 5);
    }

    #[test]
    fn test_auto_increment_ids() {
        let mut sched = Scheduler::new();
        let id1 = sched.add_task(1, 0, 0);
        let id2 = sched.add_task(1, 0, 0);
        let id3 = sched.add_task(1, 0, 0);
        assert_eq!(id2, id1 + 1);
        assert_eq!(id3, id2 + 1);
    }

    #[test]
    fn test_cron_hour_match() {
        let cron = CronExpr::parse("0 9 * * *").unwrap();
        assert!(cron.matches(0, 9, 1, 1, 0));
        assert!(!cron.matches(0, 10, 1, 1, 0));
    }

    #[test]
    fn test_cron_weekday() {
        let cron = CronExpr::parse("0 0 * * 1").unwrap();
        assert!(cron.matches(0, 0, 1, 1, 1));
        assert!(!cron.matches(0, 0, 1, 1, 0));
    }
}
