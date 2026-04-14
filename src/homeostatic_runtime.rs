//! v520 — Homeostatic Runtime.
//!
//! Self-regulating runtime that maintains stability under varying loads.
//! Homeostatic plasticity: scale spike rates to maintain target firing rates.
//! Load shedding via lateral inhibition.
//! Energy-efficient sleep states for idle cores.



/// Target firing rate for homeostatic regulation.
#[derive(Debug, Clone, Copy)]
pub struct FiringRateTarget {
    pub target_hz: f64,
    pub tolerance: f64,
}

impl FiringRateTarget {
    pub fn new(target_hz: f64, tolerance: f64) -> Self {
        Self {
            target_hz,
            tolerance: tolerance.clamp(0.01, 0.5),
        }
    }

    /// Check if an observed rate is within homeostatic bounds.
    pub fn in_bounds(&self, observed_hz: f64) -> bool {
        let lower = self.target_hz * (1.0 - self.tolerance);
        let upper = self.target_hz * (1.0 + self.tolerance);
        observed_hz >= lower && observed_hz <= upper
    }

    /// Compute correction factor to bring rate towards target.
    pub fn correction(&self, observed_hz: f64) -> f64 {
        if observed_hz < 0.001 {
            return 2.0; // large boost for nearly-silent
        }
        self.target_hz / observed_hz
    }
}

/// Energy state of a runtime core.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoreState {
    Active,
    Drowsy,
    Sleep,
    DeepSleep,
}

impl CoreState {
    pub fn power_factor(&self) -> f64 {
        match self {
            CoreState::Active => 1.0,
            CoreState::Drowsy => 0.5,
            CoreState::Sleep => 0.1,
            CoreState::DeepSleep => 0.01,
        }
    }
}

/// A runtime core with homeostatic regulation.
#[derive(Debug, Clone)]
pub struct HomeostaticCore {
    pub id: usize,
    pub state: CoreState,
    pub firing_rate: f64,
    pub target: FiringRateTarget,
    pub gain: f64,
    pub idle_ticks: u64,
    pub total_spikes: u64,
    pub total_ticks: u64,
    pub load: f64,
}

impl HomeostaticCore {
    pub fn new(id: usize, target: FiringRateTarget) -> Self {
        Self {
            id,
            state: CoreState::Active,
            firing_rate: 0.0,
            target,
            gain: 1.0,
            idle_ticks: 0,
            total_spikes: 0,
            total_ticks: 0,
            load: 0.0,
        }
    }

    /// Record spike activity for this tick.
    pub fn record_activity(&mut self, spikes: u64) {
        self.total_spikes += spikes;
        self.total_ticks += 1;

        if spikes == 0 {
            self.idle_ticks += 1;
        } else {
            self.idle_ticks = 0;
        }

        // Update firing rate (exponential moving average)
        let alpha = 0.1;
        self.firing_rate = self.firing_rate * (1.0 - alpha) + spikes as f64 * alpha;
        self.load = self.firing_rate / self.target.target_hz.max(0.001);
    }

    /// Apply homeostatic correction to the gain.
    pub fn regulate(&mut self) {
        if !self.target.in_bounds(self.firing_rate) {
            let correction = self.target.correction(self.firing_rate);
            // Smooth adjustment (don't jump to correction instantly)
            self.gain = self.gain * 0.8 + correction * 0.2;
            self.gain = self.gain.clamp(0.1, 10.0);
        }
    }

    /// Manage power state based on idle time.
    pub fn manage_power(&mut self) {
        self.state = if self.idle_ticks > 100 {
            CoreState::DeepSleep
        } else if self.idle_ticks > 50 {
            CoreState::Sleep
        } else if self.idle_ticks > 10 {
            CoreState::Drowsy
        } else {
            CoreState::Active
        };
    }

    /// Wake up from sleep state.
    pub fn wake(&mut self) {
        self.state = CoreState::Active;
        self.idle_ticks = 0;
    }

    /// Is this core overloaded?
    pub fn is_overloaded(&self) -> bool {
        self.load > 1.5
    }

    /// Is this core idle?
    pub fn is_idle(&self) -> bool {
        self.idle_ticks > 5
    }

    /// Average firing rate over lifetime.
    pub fn avg_firing_rate(&self) -> f64 {
        if self.total_ticks == 0 {
            return 0.0;
        }
        self.total_spikes as f64 / self.total_ticks as f64
    }

    /// Energy consumption estimate (relative).
    pub fn energy(&self) -> f64 {
        self.state.power_factor() * self.load.max(0.01)
    }
}

/// Load shedding strategy when the system is overloaded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SheddingStrategy {
    /// Drop lowest-priority tasks.
    DropLowPriority,
    /// Reduce firing rates across all cores.
    ThrottleAll,
    /// Migrate tasks from hot cores to cool cores.
    Migrate,
}

/// Statistics for the homeostatic runtime.
#[derive(Debug, Clone, Default)]
pub struct HomeostaticStats {
    pub total_ticks: u64,
    pub regulation_events: u64,
    pub shed_events: u64,
    pub wake_events: u64,
    pub sleep_events: u64,
    pub migration_events: u64,
    pub total_energy: f64,
}

/// Homeostatic Runtime — self-regulating multi-core scheduler.
pub struct HomeostaticRuntime {
    cores: Vec<HomeostaticCore>,
    pub shedding_strategy: SheddingStrategy,
    pub overload_threshold: f64,
    pub stats: HomeostaticStats,
}

impl HomeostaticRuntime {
    pub fn new(num_cores: usize, target_hz: f64) -> Self {
        let target = FiringRateTarget::new(target_hz, 0.2);
        let cores = (0..num_cores)
            .map(|id| HomeostaticCore::new(id, target))
            .collect();
        Self {
            cores,
            shedding_strategy: SheddingStrategy::ThrottleAll,
            overload_threshold: 1.5,
            stats: HomeostaticStats::default(),
        }
    }

    /// Record activity on a specific core.
    pub fn record_activity(&mut self, core_id: usize, spikes: u64) {
        if let Some(core) = self.cores.get_mut(core_id) {
            core.record_activity(spikes);
        }
    }

    /// Run one tick of homeostatic regulation across all cores.
    pub fn regulate_tick(&mut self) {
        self.stats.total_ticks += 1;

        for core in &mut self.cores {
            core.regulate();
            core.manage_power();
            self.stats.total_energy += core.energy();

            if core.state != CoreState::Active {
                self.stats.sleep_events += 1;
            }
        }

        self.stats.regulation_events += 1;

        // Check for system-wide overload
        let avg_load = self.average_load();
        if avg_load > self.overload_threshold {
            self.shed_load();
        }
    }

    /// Shed load when system is overloaded.
    fn shed_load(&mut self) {
        self.stats.shed_events += 1;
        match self.shedding_strategy {
            SheddingStrategy::ThrottleAll => {
                for core in &mut self.cores {
                    core.gain *= 0.8;
                    core.gain = core.gain.max(0.1);
                }
            }
            SheddingStrategy::DropLowPriority => {
                // In drop mode, reduce load on the most loaded core
                if let Some(core) = self
                    .cores
                    .iter_mut()
                    .max_by(|a, b| a.load.partial_cmp(&b.load).unwrap_or(std::cmp::Ordering::Equal))
                {
                    core.gain *= 0.5;
                    core.gain = core.gain.max(0.1);
                }
            }
            SheddingStrategy::Migrate => {
                self.stats.migration_events += 1;
                // Balance load from hot to cold cores
                let loads: Vec<(usize, f64)> =
                    self.cores.iter().map(|c| (c.id, c.load)).collect();
                if loads.len() >= 2 {
                    let max_id = loads
                        .iter()
                        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
                        .map(|x| x.0);
                    let min_id = loads
                        .iter()
                        .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
                        .map(|x| x.0);

                    if let (Some(hot), Some(cold)) = (max_id, min_id) {
                        if hot != cold {
                            let transfer = self.cores[hot].load * 0.3;
                            self.cores[hot].load -= transfer;
                            self.cores[cold].load += transfer;
                            self.cores[cold].wake();
                            self.stats.wake_events += 1;
                        }
                    }
                }
            }
        }
    }

    /// Average load across all cores.
    pub fn average_load(&self) -> f64 {
        if self.cores.is_empty() {
            return 0.0;
        }
        let total: f64 = self.cores.iter().map(|c| c.load).sum();
        total / self.cores.len() as f64
    }

    /// Number of active cores.
    pub fn active_cores(&self) -> usize {
        self.cores.iter().filter(|c| c.state == CoreState::Active).count()
    }

    /// Number of sleeping cores.
    pub fn sleeping_cores(&self) -> usize {
        self.cores
            .iter()
            .filter(|c| matches!(c.state, CoreState::Sleep | CoreState::DeepSleep))
            .count()
    }

    /// Total energy consumption.
    pub fn total_energy(&self) -> f64 {
        self.cores.iter().map(|c| c.energy()).sum()
    }

    /// Get core info.
    pub fn get_core(&self, id: usize) -> Option<&HomeostaticCore> {
        self.cores.get(id)
    }

    /// Number of cores.
    pub fn core_count(&self) -> usize {
        self.cores.len()
    }

    /// Number of overloaded cores.
    pub fn overloaded_cores(&self) -> usize {
        self.cores.iter().filter(|c| c.is_overloaded()).count()
    }

    /// System health score (0.0 = bad, 1.0 = perfect).
    pub fn health_score(&self) -> f64 {
        if self.cores.is_empty() {
            return 1.0;
        }
        let in_bounds = self
            .cores
            .iter()
            .filter(|c| c.target.in_bounds(c.firing_rate))
            .count();
        in_bounds as f64 / self.cores.len() as f64
    }

    /// Wake all sleeping cores.
    pub fn wake_all(&mut self) {
        for core in &mut self.cores {
            if core.state != CoreState::Active {
                core.wake();
                self.stats.wake_events += 1;
            }
        }
    }
}

// ─── FFI ───────────────────────────────────────────────

use std::sync::{LazyLock, Mutex};

static HOMEO_RT: LazyLock<Mutex<HomeostaticRuntime>> =
    LazyLock::new(|| Mutex::new(HomeostaticRuntime::new(4, 10.0)));

#[unsafe(no_mangle)]
pub extern "C" fn slang_homeo_record(core_id: i64, spikes: i64) -> i64 {
    let mut rt = HOMEO_RT.lock().unwrap();
    rt.record_activity(core_id as usize, spikes as u64);
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_homeo_regulate() -> i64 {
    let mut rt = HOMEO_RT.lock().unwrap();
    rt.regulate_tick();
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_homeo_active_cores() -> i64 {
    let rt = HOMEO_RT.lock().unwrap();
    rt.active_cores() as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_homeo_health() -> i64 {
    let rt = HOMEO_RT.lock().unwrap();
    (rt.health_score() * 1000.0) as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_firing_rate_target_in_bounds() {
        let t = FiringRateTarget::new(10.0, 0.2);
        assert!(t.in_bounds(10.0));
        assert!(t.in_bounds(9.0));
        assert!(t.in_bounds(11.0));
        assert!(!t.in_bounds(5.0));
        assert!(!t.in_bounds(15.0));
    }

    #[test]
    fn test_firing_rate_correction() {
        let t = FiringRateTarget::new(10.0, 0.2);
        let corr = t.correction(5.0);
        assert!((corr - 2.0).abs() < 0.01);
    }

    #[test]
    fn test_core_state_power() {
        assert_eq!(CoreState::Active.power_factor(), 1.0);
        assert!(CoreState::Sleep.power_factor() < CoreState::Drowsy.power_factor());
        assert!(CoreState::DeepSleep.power_factor() < CoreState::Sleep.power_factor());
    }

    #[test]
    fn test_core_record_activity() {
        let target = FiringRateTarget::new(10.0, 0.2);
        let mut core = HomeostaticCore::new(0, target);
        core.record_activity(5);
        assert!(core.firing_rate > 0.0);
        assert_eq!(core.total_spikes, 5);
        assert_eq!(core.total_ticks, 1);
    }

    #[test]
    fn test_core_idle_tracking() {
        let target = FiringRateTarget::new(10.0, 0.2);
        let mut core = HomeostaticCore::new(0, target);
        for _ in 0..15 {
            core.record_activity(0);
        }
        assert!(core.idle_ticks >= 15);
        core.manage_power();
        assert_eq!(core.state, CoreState::Drowsy);
    }

    #[test]
    fn test_core_deep_sleep() {
        let target = FiringRateTarget::new(10.0, 0.2);
        let mut core = HomeostaticCore::new(0, target);
        for _ in 0..110 {
            core.record_activity(0);
        }
        core.manage_power();
        assert_eq!(core.state, CoreState::DeepSleep);
    }

    #[test]
    fn test_core_wake() {
        let target = FiringRateTarget::new(10.0, 0.2);
        let mut core = HomeostaticCore::new(0, target);
        core.state = CoreState::Sleep;
        core.idle_ticks = 100;
        core.wake();
        assert_eq!(core.state, CoreState::Active);
        assert_eq!(core.idle_ticks, 0);
    }

    #[test]
    fn test_core_regulate_gain() {
        let target = FiringRateTarget::new(10.0, 0.2);
        let mut core = HomeostaticCore::new(0, target);
        core.firing_rate = 2.0; // way below target
        core.regulate();
        assert!(core.gain > 1.0); // gain increased to compensate
    }

    #[test]
    fn test_core_overload() {
        let target = FiringRateTarget::new(10.0, 0.2);
        let mut core = HomeostaticCore::new(0, target);
        core.load = 2.0;
        assert!(core.is_overloaded());
    }

    #[test]
    fn test_runtime_creation() {
        let rt = HomeostaticRuntime::new(4, 10.0);
        assert_eq!(rt.core_count(), 4);
        assert_eq!(rt.active_cores(), 4);
    }

    #[test]
    fn test_runtime_record_activity() {
        let mut rt = HomeostaticRuntime::new(2, 10.0);
        rt.record_activity(0, 5);
        rt.record_activity(1, 8);
        assert!(rt.get_core(0).unwrap().total_spikes == 5);
    }

    #[test]
    fn test_runtime_regulate() {
        let mut rt = HomeostaticRuntime::new(2, 10.0);
        rt.regulate_tick();
        assert_eq!(rt.stats.regulation_events, 1);
    }

    #[test]
    fn test_runtime_average_load() {
        let mut rt = HomeostaticRuntime::new(2, 10.0);
        rt.cores[0].load = 1.0;
        rt.cores[1].load = 2.0;
        assert!((rt.average_load() - 1.5).abs() < 0.01);
    }

    #[test]
    fn test_runtime_health() {
        let rt = HomeostaticRuntime::new(2, 10.0);
        // Initially all at 0 firing rate, not in bounds
        assert!(rt.health_score() <= 1.0);
    }

    #[test]
    fn test_runtime_shed_throttle() {
        let mut rt = HomeostaticRuntime::new(2, 10.0);
        rt.shedding_strategy = SheddingStrategy::ThrottleAll;
        rt.cores[0].load = 3.0;
        rt.cores[1].load = 3.0;
        rt.overload_threshold = 1.0;
        rt.regulate_tick();
        assert!(rt.stats.shed_events > 0);
    }

    #[test]
    fn test_runtime_shed_migrate() {
        let mut rt = HomeostaticRuntime::new(2, 10.0);
        rt.shedding_strategy = SheddingStrategy::Migrate;
        rt.cores[0].load = 3.0;
        rt.cores[1].load = 0.5;
        rt.overload_threshold = 1.0;
        rt.regulate_tick();
        assert!(rt.stats.migration_events > 0);
    }

    #[test]
    fn test_runtime_wake_all() {
        let mut rt = HomeostaticRuntime::new(3, 10.0);
        rt.cores[0].state = CoreState::Sleep;
        rt.cores[1].state = CoreState::DeepSleep;
        rt.wake_all();
        assert_eq!(rt.active_cores(), 3);
    }

    #[test]
    fn test_runtime_sleeping_cores() {
        let mut rt = HomeostaticRuntime::new(3, 10.0);
        rt.cores[0].state = CoreState::Sleep;
        rt.cores[1].state = CoreState::DeepSleep;
        assert_eq!(rt.sleeping_cores(), 2);
    }

    #[test]
    fn test_avg_firing_rate() {
        let target = FiringRateTarget::new(10.0, 0.2);
        let mut core = HomeostaticCore::new(0, target);
        core.record_activity(10);
        core.record_activity(20);
        assert!((core.avg_firing_rate() - 15.0).abs() < 0.01);
    }

    #[test]
    fn test_energy_consumption() {
        let target = FiringRateTarget::new(10.0, 0.2);
        let mut core = HomeostaticCore::new(0, target);
        core.load = 1.0;
        core.state = CoreState::Active;
        let active_energy = core.energy();
        core.state = CoreState::Sleep;
        let sleep_energy = core.energy();
        assert!(active_energy > sleep_energy);
    }

    #[test]
    fn test_ffi_homeo_health() {
        let h = slang_homeo_health();
        assert!(h >= 0 && h <= 1000);
    }

    #[test]
    fn test_ffi_homeo_active() {
        let active = slang_homeo_active_cores();
        assert!(active >= 0);
    }

    #[test]
    fn test_overloaded_cores_count() {
        let mut rt = HomeostaticRuntime::new(3, 10.0);
        rt.cores[0].load = 2.0;
        rt.cores[1].load = 2.0;
        assert_eq!(rt.overloaded_cores(), 2);
    }

    #[test]
    fn test_gain_clamping() {
        let target = FiringRateTarget::new(10.0, 0.2);
        let mut core = HomeostaticCore::new(0, target);
        core.firing_rate = 0.001;
        for _ in 0..100 {
            core.regulate();
        }
        assert!(core.gain <= 10.0);
    }
}
