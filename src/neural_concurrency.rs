//! v510 — Neural Concurrency.
//!
//! Concurrency primitives based on neural population dynamics.
//! Mutex → inhibitory neuron (winner-take-all).
//! Channel → synaptic connection (weighted, plastic).
//! Barrier → population synchrony detector.

use std::collections::{HashMap, VecDeque};

/// Neural mutex using winner-take-all inhibition.
///
/// Multiple tasks compete; the one with the highest activation wins
/// exclusive access. Losers are inhibited until the winner releases.
#[derive(Debug)]
pub struct NeuralMutex {
    pub name: String,
    holder: Option<String>,
    pub activation_levels: HashMap<String, f64>,
    wait_queue: Vec<String>,
    pub lock_count: u64,
    pub contention_count: u64,
}

impl NeuralMutex {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            holder: None,
            activation_levels: HashMap::new(),
            wait_queue: Vec::new(),
            lock_count: 0,
            contention_count: 0,
        }
    }

    /// Request the lock with an activation level. Higher activation = higher priority.
    pub fn request(&mut self, requester: &str, activation: f64) -> bool {
        self.activation_levels
            .insert(requester.to_string(), activation);

        if self.holder.is_none() {
            self.holder = Some(requester.to_string());
            self.lock_count += 1;
            true
        } else {
            if !self.wait_queue.contains(&requester.to_string()) {
                self.wait_queue.push(requester.to_string());
            }
            self.contention_count += 1;
            false
        }
    }

    /// Release the lock. Highest-activation waiter gets it next (WTA).
    pub fn release(&mut self, holder: &str) -> Option<String> {
        if self.holder.as_deref() != Some(holder) {
            return None;
        }
        self.holder = None;

        if self.wait_queue.is_empty() {
            return None;
        }

        // Winner-take-all: highest activation waiter wins
        let winner_idx = self
            .wait_queue
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| {
                let act_a = self.activation_levels.get(*a).unwrap_or(&0.0);
                let act_b = self.activation_levels.get(*b).unwrap_or(&0.0);
                act_a.partial_cmp(act_b).unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(i, _)| i);

        if let Some(idx) = winner_idx {
            let winner = self.wait_queue.remove(idx);
            self.holder = Some(winner.clone());
            self.lock_count += 1;
            Some(winner)
        } else {
            None
        }
    }

    /// Check if locked.
    pub fn is_locked(&self) -> bool {
        self.holder.is_some()
    }

    /// Current holder.
    pub fn current_holder(&self) -> Option<&str> {
        self.holder.as_deref()
    }

    /// Number of waiters.
    pub fn waiter_count(&self) -> usize {
        self.wait_queue.len()
    }
}

/// Synaptic channel — weighted, plastic message passing.
///
/// Messages have weights that adapt based on usage patterns.
/// Frequently used channels strengthen, idle ones weaken.
#[derive(Debug)]
pub struct SynapticChannel {
    pub name: String,
    buffer: VecDeque<(i64, f64)>, // (value, weight)
    pub capacity: usize,
    pub weight: f64,
    pub send_count: u64,
    pub recv_count: u64,
    closed: bool,
    plasticity_rate: f64,
}

impl SynapticChannel {
    pub fn new(name: &str, capacity: usize) -> Self {
        Self {
            name: name.to_string(),
            buffer: VecDeque::new(),
            capacity,
            weight: 1.0,
            send_count: 0,
            recv_count: 0,
            closed: false,
            plasticity_rate: 0.01,
        }
    }

    /// Send a value through the channel, weighted by channel strength.
    pub fn send(&mut self, value: i64) -> Result<(), &'static str> {
        if self.closed {
            return Err("channel closed");
        }
        if self.buffer.len() >= self.capacity {
            return Err("channel full");
        }
        self.buffer.push_back((value, self.weight));
        self.send_count += 1;
        // LTP on active use
        self.weight = (self.weight + self.plasticity_rate).min(5.0);
        Ok(())
    }

    /// Receive a weighted value.
    pub fn recv(&mut self) -> Option<(i64, f64)> {
        if let Some(item) = self.buffer.pop_front() {
            self.recv_count += 1;
            self.weight = (self.weight + self.plasticity_rate).min(5.0);
            Some(item)
        } else {
            None
        }
    }

    /// Receive just the value (ignoring weight).
    pub fn recv_value(&mut self) -> Option<i64> {
        self.recv().map(|(v, _)| v)
    }

    /// Apply depression (LTD) for idle channels.
    pub fn depress(&mut self, amount: f64) {
        self.weight = (self.weight - amount).max(0.01);
    }

    /// Close the channel.
    pub fn close(&mut self) {
        self.closed = true;
    }

    pub fn is_closed(&self) -> bool {
        self.closed
    }

    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    pub fn is_full(&self) -> bool {
        self.buffer.len() >= self.capacity
    }
}

/// Population synchrony barrier.
///
/// Detects when a population of tasks reaches synchronous activity.
/// Modeled on neural population synchrony detection.
#[derive(Debug)]
pub struct PopulationBarrier {
    pub name: String,
    expected: usize,
    arrived: Vec<String>,
    pub sync_events: u64,
    threshold: f64,
}

impl PopulationBarrier {
    pub fn new(name: &str, expected: usize) -> Self {
        Self {
            name: name.to_string(),
            expected,
            arrived: Vec::new(),
            sync_events: 0,
            threshold: 1.0, // require all members by default
        }
    }

    /// Set the synchrony threshold (fraction of expected that must arrive).
    pub fn with_threshold(mut self, threshold: f64) -> Self {
        self.threshold = threshold.clamp(0.0, 1.0);
        self
    }

    /// Signal arrival at the barrier.
    pub fn arrive(&mut self, task: &str) -> bool {
        if !self.arrived.contains(&task.to_string()) {
            self.arrived.push(task.to_string());
        }
        self.is_synchronized()
    }

    /// Check if the population has reached synchrony.
    pub fn is_synchronized(&self) -> bool {
        let required = (self.expected as f64 * self.threshold).ceil() as usize;
        if self.arrived.len() >= required {
            return true;
        }
        false
    }

    /// Reset the barrier for the next synchrony event.
    pub fn reset(&mut self) {
        if self.is_synchronized() {
            self.sync_events += 1;
        }
        self.arrived.clear();
    }

    /// Number that have arrived.
    pub fn arrived_count(&self) -> usize {
        self.arrived.len()
    }

    /// Number still pending.
    pub fn pending_count(&self) -> usize {
        self.expected.saturating_sub(self.arrived.len())
    }

    /// Synchrony ratio (arrived / expected).
    pub fn synchrony_ratio(&self) -> f64 {
        if self.expected == 0 {
            return 1.0;
        }
        self.arrived.len() as f64 / self.expected as f64
    }
}

/// Neural read-write lock using excitatory/inhibitory balance.
///
/// Readers are excitatory (can coexist), write is inhibitory (exclusive).
#[derive(Debug)]
pub struct NeuralRwLock {
    pub name: String,
    readers: Vec<String>,
    writer: Option<String>,
    pub read_count: u64,
    pub write_count: u64,
}

impl NeuralRwLock {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            readers: Vec::new(),
            writer: None,
            read_count: 0,
            write_count: 0,
        }
    }

    /// Acquire a read lock (excitatory — multiple allowed).
    pub fn read_lock(&mut self, reader: &str) -> bool {
        if self.writer.is_some() {
            return false;
        }
        if !self.readers.contains(&reader.to_string()) {
            self.readers.push(reader.to_string());
        }
        self.read_count += 1;
        true
    }

    /// Release a read lock.
    pub fn read_unlock(&mut self, reader: &str) -> bool {
        if let Some(pos) = self.readers.iter().position(|r| r == reader) {
            self.readers.remove(pos);
            true
        } else {
            false
        }
    }

    /// Acquire a write lock (inhibitory — exclusive).
    pub fn write_lock(&mut self, writer: &str) -> bool {
        if self.writer.is_some() || !self.readers.is_empty() {
            return false;
        }
        self.writer = Some(writer.to_string());
        self.write_count += 1;
        true
    }

    /// Release the write lock.
    pub fn write_unlock(&mut self, writer: &str) -> bool {
        if self.writer.as_deref() == Some(writer) {
            self.writer = None;
            true
        } else {
            false
        }
    }

    pub fn reader_count(&self) -> usize {
        self.readers.len()
    }

    pub fn is_write_locked(&self) -> bool {
        self.writer.is_some()
    }

    pub fn is_read_locked(&self) -> bool {
        !self.readers.is_empty()
    }
}

/// Neural semaphore using population firing rate.
///
/// Permits are modeled as population-available capacity.
#[derive(Debug)]
pub struct NeuralSemaphore {
    pub name: String,
    max_permits: usize,
    active: Vec<String>,
    pub acquire_count: u64,
}

impl NeuralSemaphore {
    pub fn new(name: &str, max_permits: usize) -> Self {
        Self {
            name: name.to_string(),
            max_permits,
            active: Vec::new(),
            acquire_count: 0,
        }
    }

    /// Try to acquire a permit.
    pub fn acquire(&mut self, task: &str) -> bool {
        if self.active.len() < self.max_permits {
            self.active.push(task.to_string());
            self.acquire_count += 1;
            true
        } else {
            false
        }
    }

    /// Release a permit.
    pub fn release(&mut self, task: &str) -> bool {
        if let Some(pos) = self.active.iter().position(|t| t == task) {
            self.active.remove(pos);
            true
        } else {
            false
        }
    }

    pub fn available(&self) -> usize {
        self.max_permits - self.active.len()
    }

    pub fn active_count(&self) -> usize {
        self.active.len()
    }
}

// ─── FFI ───────────────────────────────────────────────

use std::sync::{LazyLock, Mutex};

static NEURAL_MUTEXES: LazyLock<Mutex<Vec<NeuralMutex>>> =
    LazyLock::new(|| Mutex::new(Vec::new()));

#[unsafe(no_mangle)]
pub extern "C" fn slang_neural_mutex_create() -> i64 {
    let mut mutexes = NEURAL_MUTEXES.lock().unwrap();
    let id = mutexes.len();
    mutexes.push(NeuralMutex::new(&format!("mutex_{id}")));
    id as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_neural_barrier_sync(expected: i64, arrived: i64) -> i64 {
    if arrived >= expected { 1 } else { 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_neural_channel_weight() -> i64 {
    1000 // default weight 1.0 × 1000
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_neural_mutex_basic() {
        let mut m = NeuralMutex::new("m1");
        assert!(m.request("task_a", 1.0));
        assert!(m.is_locked());
        assert_eq!(m.current_holder(), Some("task_a"));
    }

    #[test]
    fn test_neural_mutex_contention() {
        let mut m = NeuralMutex::new("m1");
        assert!(m.request("a", 1.0));
        assert!(!m.request("b", 2.0)); // b waits
        assert_eq!(m.waiter_count(), 1);
        assert_eq!(m.contention_count, 1);
    }

    #[test]
    fn test_neural_mutex_wta_release() {
        let mut m = NeuralMutex::new("m1");
        m.request("a", 1.0);
        m.request("b", 3.0); // higher activation
        m.request("c", 2.0);
        let winner = m.release("a").unwrap();
        assert_eq!(winner, "b"); // highest activation wins
    }

    #[test]
    fn test_neural_mutex_release_empty() {
        let mut m = NeuralMutex::new("m1");
        m.request("a", 1.0);
        let next = m.release("a");
        assert!(next.is_none());
        assert!(!m.is_locked());
    }

    #[test]
    fn test_synaptic_channel_send_recv() {
        let mut ch = SynapticChannel::new("ch1", 10);
        assert!(ch.send(42).is_ok());
        let (val, weight) = ch.recv().unwrap();
        assert_eq!(val, 42);
        assert!(weight > 0.0);
    }

    #[test]
    fn test_synaptic_channel_full() {
        let mut ch = SynapticChannel::new("ch1", 2);
        assert!(ch.send(1).is_ok());
        assert!(ch.send(2).is_ok());
        assert!(ch.send(3).is_err());
    }

    #[test]
    fn test_synaptic_channel_closed() {
        let mut ch = SynapticChannel::new("ch1", 10);
        ch.close();
        assert!(ch.send(1).is_err());
        assert!(ch.is_closed());
    }

    #[test]
    fn test_synaptic_channel_plasticity() {
        let mut ch = SynapticChannel::new("ch1", 10);
        let initial = ch.weight;
        for i in 0..100 {
            let _ = ch.send(i);
        }
        assert!(ch.weight > initial);
    }

    #[test]
    fn test_synaptic_channel_depression() {
        let mut ch = SynapticChannel::new("ch1", 10);
        ch.depress(0.5);
        assert!(ch.weight < 1.0);
    }

    #[test]
    fn test_population_barrier_basic() {
        let mut b = PopulationBarrier::new("b1", 3);
        assert!(!b.arrive("a"));
        assert!(!b.arrive("b"));
        assert!(b.arrive("c"));
        assert!(b.is_synchronized());
    }

    #[test]
    fn test_population_barrier_threshold() {
        let mut b = PopulationBarrier::new("b1", 4).with_threshold(0.5);
        assert!(!b.arrive("a"));
        assert!(b.arrive("b")); // 2/4 = 50% meets threshold
    }

    #[test]
    fn test_population_barrier_reset() {
        let mut b = PopulationBarrier::new("b1", 2);
        b.arrive("a");
        b.arrive("b");
        b.reset();
        assert_eq!(b.sync_events, 1);
        assert_eq!(b.arrived_count(), 0);
    }

    #[test]
    fn test_population_barrier_ratio() {
        let mut b = PopulationBarrier::new("b1", 4);
        b.arrive("a");
        b.arrive("b");
        assert!((b.synchrony_ratio() - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_neural_rwlock_readers() {
        let mut rw = NeuralRwLock::new("rw1");
        assert!(rw.read_lock("r1"));
        assert!(rw.read_lock("r2"));
        assert_eq!(rw.reader_count(), 2);
    }

    #[test]
    fn test_neural_rwlock_exclusive_write() {
        let mut rw = NeuralRwLock::new("rw1");
        assert!(rw.read_lock("r1"));
        assert!(!rw.write_lock("w1")); // can't write while readers active
    }

    #[test]
    fn test_neural_rwlock_write_lock() {
        let mut rw = NeuralRwLock::new("rw1");
        assert!(rw.write_lock("w1"));
        assert!(!rw.read_lock("r1")); // can't read while writing
        assert!(rw.write_unlock("w1"));
        assert!(rw.read_lock("r1"));
    }

    #[test]
    fn test_neural_semaphore() {
        let mut sem = NeuralSemaphore::new("s1", 2);
        assert!(sem.acquire("t1"));
        assert!(sem.acquire("t2"));
        assert!(!sem.acquire("t3")); // no permits
        assert_eq!(sem.available(), 0);
    }

    #[test]
    fn test_neural_semaphore_release() {
        let mut sem = NeuralSemaphore::new("s1", 1);
        sem.acquire("t1");
        assert!(sem.release("t1"));
        assert_eq!(sem.available(), 1);
        assert!(sem.acquire("t2"));
    }

    #[test]
    fn test_ffi_mutex_create() {
        let id = slang_neural_mutex_create();
        assert!(id >= 0);
    }

    #[test]
    fn test_ffi_barrier_sync() {
        assert_eq!(slang_neural_barrier_sync(3, 3), 1);
        assert_eq!(slang_neural_barrier_sync(3, 2), 0);
    }

    #[test]
    fn test_channel_recv_value() {
        let mut ch = SynapticChannel::new("ch1", 10);
        ch.send(99).unwrap();
        assert_eq!(ch.recv_value(), Some(99));
    }

    #[test]
    fn test_barrier_dedup_arrival() {
        let mut b = PopulationBarrier::new("b1", 3);
        b.arrive("a");
        b.arrive("a"); // duplicate
        assert_eq!(b.arrived_count(), 1);
    }

    #[test]
    fn test_rwlock_stats() {
        let mut rw = NeuralRwLock::new("rw");
        rw.read_lock("r1");
        rw.read_lock("r2");
        assert_eq!(rw.read_count, 2);
        rw.read_unlock("r1");
        rw.read_unlock("r2");
        rw.write_lock("w1");
        assert_eq!(rw.write_count, 1);
    }
}
