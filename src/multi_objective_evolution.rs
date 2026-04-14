//! v450 — Multi-Objective Evolution.
//!
//! Pareto-optimal evolution across: compile time, runtime speed, binary size,
//! memory usage, energy consumption. Builds on `evolution_advanced.rs` NSGA-II.
//! Auto-selects trade-off based on target profile (embedded vs server vs WASM).

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

/// Objectives for multi-objective optimization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Objective {
    CompileTime,
    RuntimeSpeed,
    BinarySize,
    MemoryUsage,
    EnergyConsumption,
}

impl Objective {
    pub fn all() -> &'static [Objective] {
        &[
            Objective::CompileTime,
            Objective::RuntimeSpeed,
            Objective::BinarySize,
            Objective::MemoryUsage,
            Objective::EnergyConsumption,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            Objective::CompileTime => "compile_time",
            Objective::RuntimeSpeed => "runtime_speed",
            Objective::BinarySize => "binary_size",
            Objective::MemoryUsage => "memory_usage",
            Objective::EnergyConsumption => "energy",
        }
    }
}

/// Target deployment profile that determines objective weights.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetProfile {
    /// Server: optimize runtime speed and memory.
    Server,
    /// Embedded: optimize binary size and energy.
    Embedded,
    /// WASM: optimize binary size and compile time.
    Wasm,
    /// Development: optimize compile time only.
    Development,
    /// Balanced: equal weights.
    Balanced,
}

impl TargetProfile {
    pub fn name(&self) -> &'static str {
        match self {
            TargetProfile::Server => "server",
            TargetProfile::Embedded => "embedded",
            TargetProfile::Wasm => "wasm",
            TargetProfile::Development => "dev",
            TargetProfile::Balanced => "balanced",
        }
    }

    /// Objective weights for this profile (sum to 1.0).
    pub fn weights(&self) -> HashMap<Objective, f64> {
        let mut w = HashMap::new();
        match self {
            TargetProfile::Server => {
                w.insert(Objective::CompileTime, 0.1);
                w.insert(Objective::RuntimeSpeed, 0.4);
                w.insert(Objective::BinarySize, 0.1);
                w.insert(Objective::MemoryUsage, 0.3);
                w.insert(Objective::EnergyConsumption, 0.1);
            }
            TargetProfile::Embedded => {
                w.insert(Objective::CompileTime, 0.05);
                w.insert(Objective::RuntimeSpeed, 0.15);
                w.insert(Objective::BinarySize, 0.35);
                w.insert(Objective::MemoryUsage, 0.15);
                w.insert(Objective::EnergyConsumption, 0.30);
            }
            TargetProfile::Wasm => {
                w.insert(Objective::CompileTime, 0.25);
                w.insert(Objective::RuntimeSpeed, 0.2);
                w.insert(Objective::BinarySize, 0.35);
                w.insert(Objective::MemoryUsage, 0.15);
                w.insert(Objective::EnergyConsumption, 0.05);
            }
            TargetProfile::Development => {
                w.insert(Objective::CompileTime, 0.6);
                w.insert(Objective::RuntimeSpeed, 0.15);
                w.insert(Objective::BinarySize, 0.05);
                w.insert(Objective::MemoryUsage, 0.15);
                w.insert(Objective::EnergyConsumption, 0.05);
            }
            TargetProfile::Balanced => {
                w.insert(Objective::CompileTime, 0.2);
                w.insert(Objective::RuntimeSpeed, 0.2);
                w.insert(Objective::BinarySize, 0.2);
                w.insert(Objective::MemoryUsage, 0.2);
                w.insert(Objective::EnergyConsumption, 0.2);
            }
        }
        w
    }
}

/// A solution with multiple objective values.
#[derive(Debug, Clone)]
pub struct Solution {
    pub id: u64,
    pub name: String,
    pub objectives: HashMap<Objective, f64>,
    /// Pareto rank (0 = front, 1 = next front, etc.)
    pub pareto_rank: usize,
    /// Crowding distance for diversity preservation.
    pub crowding_distance: f64,
    pub generation: u64,
}

impl Solution {
    pub fn new(id: u64, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            objectives: HashMap::new(),
            pareto_rank: usize::MAX,
            crowding_distance: 0.0,
            generation: 0,
        }
    }

    /// Set an objective value.
    pub fn set_objective(&mut self, obj: Objective, value: f64) {
        self.objectives.insert(obj, value);
    }

    /// Get an objective value.
    pub fn get_objective(&self, obj: Objective) -> f64 {
        self.objectives.get(&obj).copied().unwrap_or(f64::INFINITY)
    }

    /// Weighted scalarized fitness for a given profile.
    pub fn weighted_fitness(&self, profile: TargetProfile) -> f64 {
        let weights = profile.weights();
        let mut sum = 0.0;
        for (obj, weight) in &weights {
            // Lower is better for all objectives → negate for fitness
            let val = self.get_objective(*obj);
            if val.is_finite() {
                sum += weight * (1.0 / (1.0 + val));
            }
        }
        sum
    }

    /// Check if this solution dominates another (all objectives ≤, at least one <).
    pub fn dominates(&self, other: &Solution) -> bool {
        let mut dominated_one = false;
        for obj in Objective::all() {
            let a = self.get_objective(*obj);
            let b = other.get_objective(*obj);
            if a > b {
                return false; // worse on at least one
            }
            if a < b {
                dominated_one = true;
            }
        }
        dominated_one
    }
}

/// Multi-objective evolution engine.
pub struct MultiObjectiveEvolver {
    pub solutions: Vec<Solution>,
    pub profile: TargetProfile,
    pub generation: u64,
    solution_counter: u64,
    /// Max population size.
    pub population_cap: usize,
}

impl MultiObjectiveEvolver {
    pub fn new(profile: TargetProfile) -> Self {
        Self {
            solutions: Vec::new(),
            profile,
            generation: 0,
            solution_counter: 0,
            population_cap: 100,
        }
    }

    /// Add a solution.
    pub fn add_solution(&mut self, mut solution: Solution) -> u64 {
        self.solution_counter += 1;
        solution.id = self.solution_counter;
        solution.generation = self.generation;
        let id = solution.id;
        self.solutions.push(solution);
        id
    }

    /// Perform non-dominated sorting (NSGA-II).
    pub fn non_dominated_sort(&mut self) {
        let n = self.solutions.len();
        if n == 0 { return; }

        // Compute domination
        let mut domination_count = vec![0usize; n];
        let mut dominated_by: Vec<Vec<usize>> = vec![Vec::new(); n];

        for i in 0..n {
            for j in 0..n {
                if i == j { continue; }
                if self.solutions[i].dominates(&self.solutions[j]) {
                    dominated_by[i].push(j);
                } else if self.solutions[j].dominates(&self.solutions[i]) {
                    domination_count[i] += 1;
                }
            }
        }

        // Assign ranks by fronts
        let mut current_front: Vec<usize> = (0..n)
            .filter(|&i| domination_count[i] == 0)
            .collect();
        let mut rank = 0;

        while !current_front.is_empty() {
            for &i in &current_front {
                self.solutions[i].pareto_rank = rank;
            }
            let mut next_front = Vec::new();
            for &i in &current_front {
                for &j in &dominated_by[i] {
                    domination_count[j] = domination_count[j].saturating_sub(1);
                    if domination_count[j] == 0 {
                        next_front.push(j);
                    }
                }
            }
            next_front.sort_unstable();
            next_front.dedup();
            current_front = next_front;
            rank += 1;
        }
    }

    /// Compute crowding distances for diversity.
    pub fn compute_crowding_distance(&mut self) {
        let n = self.solutions.len();
        if n < 3 { return; }

        for s in &mut self.solutions {
            s.crowding_distance = 0.0;
        }

        for obj in Objective::all() {
            let mut indices: Vec<usize> = (0..n).collect();
            indices.sort_by(|&a, &b| {
                self.solutions[a].get_objective(*obj)
                    .partial_cmp(&self.solutions[b].get_objective(*obj))
                    .unwrap_or(std::cmp::Ordering::Equal)
            });

            // Boundary solutions get infinite distance
            self.solutions[indices[0]].crowding_distance = f64::INFINITY;
            self.solutions[indices[n - 1]].crowding_distance = f64::INFINITY;

            let range = self.solutions[indices[n - 1]].get_objective(*obj)
                - self.solutions[indices[0]].get_objective(*obj);
            if range < 1e-15 { continue; }

            for i in 1..n - 1 {
                let diff = self.solutions[indices[i + 1]].get_objective(*obj)
                    - self.solutions[indices[i - 1]].get_objective(*obj);
                self.solutions[indices[i]].crowding_distance += diff / range;
            }
        }
    }

    /// Get the Pareto front (rank 0 solutions).
    pub fn pareto_front(&self) -> Vec<&Solution> {
        self.solutions.iter().filter(|s| s.pareto_rank == 0).collect()
    }

    /// Get the best solution for the current profile (scalarized).
    pub fn best_for_profile(&self) -> Option<&Solution> {
        self.solutions.iter()
            .max_by(|a, b| {
                a.weighted_fitness(self.profile)
                    .partial_cmp(&b.weighted_fitness(self.profile))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    }

    /// Evolve: cull weak solutions, keep Pareto front + diverse.
    pub fn selection_step(&mut self) {
        self.generation += 1;
        self.non_dominated_sort();
        self.compute_crowding_distance();

        // Sort by rank (ascending), then crowding distance (descending)
        self.solutions.sort_by(|a, b| {
            a.pareto_rank.cmp(&b.pareto_rank)
                .then_with(|| b.crowding_distance.partial_cmp(&a.crowding_distance)
                    .unwrap_or(std::cmp::Ordering::Equal))
        });

        // Keep top solutions
        if self.solutions.len() > self.population_cap {
            self.solutions.truncate(self.population_cap);
        }
    }

    /// Population size.
    pub fn population_size(&self) -> usize {
        self.solutions.len()
    }

    /// Pareto front size.
    pub fn pareto_front_size(&self) -> usize {
        self.pareto_front().len()
    }
}

// ─── FFI ──────────────────────────────────────────────────────────────

static GLOBAL_MOE: LazyLock<Mutex<MultiObjectiveEvolver>> =
    LazyLock::new(|| Mutex::new(MultiObjectiveEvolver::new(TargetProfile::Balanced)));

#[unsafe(no_mangle)]
pub extern "C" fn slang_moe_add_solution(compile: i64, speed: i64, size: i64) -> i64 {
    let mut moe = GLOBAL_MOE.lock().unwrap();
    let mut s = Solution::new(0, "auto");
    s.set_objective(Objective::CompileTime, f64::from_bits(compile as u64));
    s.set_objective(Objective::RuntimeSpeed, f64::from_bits(speed as u64));
    s.set_objective(Objective::BinarySize, f64::from_bits(size as u64));
    moe.add_solution(s) as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_moe_sort() -> i64 {
    let mut moe = GLOBAL_MOE.lock().unwrap();
    moe.non_dominated_sort();
    moe.pareto_front_size() as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_moe_select() -> i64 {
    let mut moe = GLOBAL_MOE.lock().unwrap();
    moe.selection_step();
    moe.population_size() as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_moe_population_size() -> i64 {
    let moe = GLOBAL_MOE.lock().unwrap();
    moe.population_size() as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_moe_generation() -> i64 {
    let moe = GLOBAL_MOE.lock().unwrap();
    moe.generation as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_objective_all() {
        assert_eq!(Objective::all().len(), 5);
    }

    #[test]
    fn test_objective_name() {
        assert_eq!(Objective::CompileTime.name(), "compile_time");
        assert_eq!(Objective::EnergyConsumption.name(), "energy");
    }

    #[test]
    fn test_profile_name() {
        assert_eq!(TargetProfile::Server.name(), "server");
        assert_eq!(TargetProfile::Embedded.name(), "embedded");
    }

    #[test]
    fn test_profile_weights_sum() {
        for profile in &[TargetProfile::Server, TargetProfile::Embedded, TargetProfile::Wasm,
                         TargetProfile::Development, TargetProfile::Balanced] {
            let sum: f64 = profile.weights().values().sum();
            assert!((sum - 1.0).abs() < 1e-10, "Profile {} weights don't sum to 1.0", profile.name());
        }
    }

    #[test]
    fn test_solution_new() {
        let s = Solution::new(1, "test");
        assert_eq!(s.id, 1);
        assert_eq!(s.name, "test");
    }

    #[test]
    fn test_solution_objectives() {
        let mut s = Solution::new(1, "test");
        s.set_objective(Objective::CompileTime, 10.0);
        assert!((s.get_objective(Objective::CompileTime) - 10.0).abs() < 1e-10);
    }

    #[test]
    fn test_solution_dominates() {
        let mut a = Solution::new(1, "a");
        a.set_objective(Objective::CompileTime, 1.0);
        a.set_objective(Objective::RuntimeSpeed, 1.0);
        a.set_objective(Objective::BinarySize, 1.0);
        a.set_objective(Objective::MemoryUsage, 1.0);
        a.set_objective(Objective::EnergyConsumption, 1.0);

        let mut b = Solution::new(2, "b");
        b.set_objective(Objective::CompileTime, 2.0);
        b.set_objective(Objective::RuntimeSpeed, 2.0);
        b.set_objective(Objective::BinarySize, 2.0);
        b.set_objective(Objective::MemoryUsage, 2.0);
        b.set_objective(Objective::EnergyConsumption, 2.0);

        assert!(a.dominates(&b));
        assert!(!b.dominates(&a));
    }

    #[test]
    fn test_solution_no_domination() {
        let mut a = Solution::new(1, "a");
        a.set_objective(Objective::CompileTime, 1.0);
        a.set_objective(Objective::RuntimeSpeed, 3.0);

        let mut b = Solution::new(2, "b");
        b.set_objective(Objective::CompileTime, 3.0);
        b.set_objective(Objective::RuntimeSpeed, 1.0);

        assert!(!a.dominates(&b));
        assert!(!b.dominates(&a));
    }

    #[test]
    fn test_weighted_fitness() {
        let mut s = Solution::new(1, "test");
        s.set_objective(Objective::CompileTime, 0.5);
        let fitness = s.weighted_fitness(TargetProfile::Development);
        assert!(fitness > 0.0);
    }

    #[test]
    fn test_evolver_new() {
        let e = MultiObjectiveEvolver::new(TargetProfile::Balanced);
        assert_eq!(e.population_size(), 0);
    }

    #[test]
    fn test_evolver_add() {
        let mut e = MultiObjectiveEvolver::new(TargetProfile::Server);
        let id = e.add_solution(Solution::new(0, "s1"));
        assert!(id >= 1);
        assert_eq!(e.population_size(), 1);
    }

    #[test]
    fn test_non_dominated_sort() {
        let mut e = MultiObjectiveEvolver::new(TargetProfile::Balanced);
        let mut s1 = Solution::new(0, "front");
        s1.set_objective(Objective::CompileTime, 1.0);
        s1.set_objective(Objective::RuntimeSpeed, 1.0);
        let mut s2 = Solution::new(0, "dominated");
        s2.set_objective(Objective::CompileTime, 5.0);
        s2.set_objective(Objective::RuntimeSpeed, 5.0);
        e.add_solution(s1);
        e.add_solution(s2);
        e.non_dominated_sort();
        assert_eq!(e.pareto_front_size(), 1);
        assert_eq!(e.pareto_front()[0].name, "front");
    }

    #[test]
    fn test_selection_step() {
        let mut e = MultiObjectiveEvolver::new(TargetProfile::Balanced);
        e.population_cap = 5;
        for i in 0..10 {
            let mut s = Solution::new(0, format!("s{}", i));
            s.set_objective(Objective::CompileTime, i as f64);
            s.set_objective(Objective::RuntimeSpeed, (10 - i) as f64);
            e.add_solution(s);
        }
        e.selection_step();
        assert!(e.population_size() <= 5);
    }

    #[test]
    fn test_best_for_profile() {
        let mut e = MultiObjectiveEvolver::new(TargetProfile::Development);
        let mut fast_compile = Solution::new(0, "fast_compile");
        fast_compile.set_objective(Objective::CompileTime, 0.1);
        let mut slow_compile = Solution::new(0, "slow_compile");
        slow_compile.set_objective(Objective::CompileTime, 10.0);
        e.add_solution(fast_compile);
        e.add_solution(slow_compile);
        let best = e.best_for_profile().unwrap();
        assert_eq!(best.name, "fast_compile");
    }

    #[test]
    fn test_ffi_add() {
        let id = slang_moe_add_solution(
            f64::to_bits(1.0) as i64,
            f64::to_bits(2.0) as i64,
            f64::to_bits(3.0) as i64,
        );
        assert!(id >= 1);
    }

    #[test]
    fn test_ffi_population() {
        let size = slang_moe_population_size();
        assert!(size >= 0);
    }

    #[test]
    fn test_ffi_generation() {
        let generation = slang_moe_generation();
        assert!(generation >= 0);
    }
}
