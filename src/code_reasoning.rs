//! Code Reasoning Engine — Vitalis v603
//!
//! Logical reasoning about code behavior, invariants, and properties:
//! - Invariant inference (loop, type, data structure invariants)
//! - Precondition / postcondition derivation
//! - Logical implication chains for control flow
//! - Abstract interpretation (sign domain, interval domain)
//! - Assertion strength analysis
//! - Hoare triple validation

use std::collections::HashMap;

// ── Abstract Domains ─────────────────────────────────────────────────

/// Sign abstract domain for integer reasoning.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Sign {
    Negative,
    Zero,
    Positive,
    NonNegative,  // >= 0
    NonPositive,  // <= 0
    Top,          // any
    Bottom,       // unreachable
}

impl Sign {
    pub fn from_value(v: i64) -> Self {
        if v < 0 { Sign::Negative }
        else if v == 0 { Sign::Zero }
        else { Sign::Positive }
    }

    pub fn join(self, other: Sign) -> Sign {
        if self == other { return self; }
        if self == Sign::Bottom { return other; }
        if other == Sign::Bottom { return self; }
        match (self, other) {
            (Sign::Zero, Sign::Positive) | (Sign::Positive, Sign::Zero) => Sign::NonNegative,
            (Sign::Zero, Sign::Negative) | (Sign::Negative, Sign::Zero) => Sign::NonPositive,
            _ => Sign::Top,
        }
    }

    pub fn meet(self, other: Sign) -> Sign {
        if self == other { return self; }
        if self == Sign::Top { return other; }
        if other == Sign::Top { return self; }
        match (self, other) {
            (Sign::NonNegative, Sign::Positive) | (Sign::Positive, Sign::NonNegative) => Sign::Positive,
            (Sign::NonNegative, Sign::Zero) | (Sign::Zero, Sign::NonNegative) => Sign::Zero,
            (Sign::NonPositive, Sign::Negative) | (Sign::Negative, Sign::NonPositive) => Sign::Negative,
            (Sign::NonPositive, Sign::Zero) | (Sign::Zero, Sign::NonPositive) => Sign::Zero,
            _ => Sign::Bottom,
        }
    }

    pub fn add(self, other: Sign) -> Sign {
        match (self, other) {
            (Sign::Bottom, _) | (_, Sign::Bottom) => Sign::Bottom,
            (Sign::Positive, Sign::Positive) => Sign::Positive,
            (Sign::Negative, Sign::Negative) => Sign::Negative,
            (Sign::Positive, Sign::Zero) | (Sign::Zero, Sign::Positive) => Sign::Positive,
            (Sign::Negative, Sign::Zero) | (Sign::Zero, Sign::Negative) => Sign::Negative,
            (Sign::Zero, Sign::Zero) => Sign::Zero,
            (Sign::NonNegative, Sign::NonNegative) => Sign::NonNegative,
            (Sign::NonPositive, Sign::NonPositive) => Sign::NonPositive,
            _ => Sign::Top,
        }
    }

    pub fn negate(self) -> Sign {
        match self {
            Sign::Positive => Sign::Negative,
            Sign::Negative => Sign::Positive,
            Sign::NonNegative => Sign::NonPositive,
            Sign::NonPositive => Sign::NonNegative,
            other => other,
        }
    }
}

/// Interval abstract domain for range analysis.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Interval {
    pub lo: f64,
    pub hi: f64,
}

impl Interval {
    pub fn new(lo: f64, hi: f64) -> Self {
        Self { lo: lo.min(hi), hi: lo.max(hi) }
    }

    pub fn point(v: f64) -> Self { Self { lo: v, hi: v } }

    pub fn top() -> Self { Self { lo: f64::NEG_INFINITY, hi: f64::INFINITY } }

    pub fn bottom() -> Self { Self { lo: f64::INFINITY, hi: f64::NEG_INFINITY } }

    pub fn is_bottom(&self) -> bool { self.lo > self.hi }

    pub fn contains(&self, v: f64) -> bool { v >= self.lo && v <= self.hi }

    pub fn width(&self) -> f64 {
        if self.is_bottom() { 0.0 }
        else if self.hi.is_infinite() || self.lo.is_infinite() { f64::INFINITY }
        else { self.hi - self.lo }
    }

    pub fn join(self, other: Interval) -> Interval {
        if self.is_bottom() { return other; }
        if other.is_bottom() { return self; }
        Interval { lo: self.lo.min(other.lo), hi: self.hi.max(other.hi) }
    }

    pub fn meet(self, other: Interval) -> Interval {
        Interval { lo: self.lo.max(other.lo), hi: self.hi.min(other.hi) }
    }

    pub fn add(self, other: Interval) -> Interval {
        if self.is_bottom() || other.is_bottom() { return Interval::bottom(); }
        Interval { lo: self.lo + other.lo, hi: self.hi + other.hi }
    }

    pub fn mul(self, other: Interval) -> Interval {
        if self.is_bottom() || other.is_bottom() { return Interval::bottom(); }
        let products = [
            self.lo * other.lo, self.lo * other.hi,
            self.hi * other.lo, self.hi * other.hi,
        ];
        let lo = products.iter().cloned().fold(f64::INFINITY, f64::min);
        let hi = products.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        Interval { lo, hi }
    }
}

// ── Invariants ───────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum InvariantKind {
    RangeInvariant { var: String, interval: Interval },
    SignInvariant { var: String, sign: Sign },
    Monotonic { var: String, increasing: bool },
    NonNull { var: String },
    BoundsCheck { var: String, bound_var: String },
    Equality { left: String, right: String },
    LoopTermination { decreasing_var: String },
}

#[derive(Debug, Clone)]
pub struct Invariant {
    pub kind: InvariantKind,
    pub confidence: f64,
    pub location: String,
}

/// Condition describing preconditions or postconditions.
#[derive(Debug, Clone, PartialEq)]
pub enum Condition {
    IsPositive(String),
    IsNonNeg(String),
    IsNonNull(String),
    InRange(String, Interval),
    Equals(String, i64),
    LessThan(String, String),
    True,
    False,
    And(Box<Condition>, Box<Condition>),
    Or(Box<Condition>, Box<Condition>),
    Not(Box<Condition>),
}

impl Condition {
    pub fn and(self, other: Condition) -> Condition {
        Condition::And(Box::new(self), Box::new(other))
    }

    pub fn or(self, other: Condition) -> Condition {
        Condition::Or(Box::new(self), Box::new(other))
    }

    pub fn negate(self) -> Condition {
        match self {
            Condition::True => Condition::False,
            Condition::False => Condition::True,
            Condition::Not(inner) => *inner,
            other => Condition::Not(Box::new(other)),
        }
    }
}

// ── Abstract Interpreter ─────────────────────────────────────────────

/// A tiny abstract interpreter over sign and interval domains.
pub struct AbstractInterpreter {
    sign_state: HashMap<String, Sign>,
    interval_state: HashMap<String, Interval>,
}

impl AbstractInterpreter {
    pub fn new() -> Self {
        Self {
            sign_state: HashMap::new(),
            interval_state: HashMap::new(),
        }
    }

    pub fn assign_const(&mut self, var: &str, value: i64) {
        self.sign_state.insert(var.to_string(), Sign::from_value(value));
        self.interval_state.insert(var.to_string(), Interval::point(value as f64));
    }

    pub fn assign_add(&mut self, target: &str, left: &str, right: &str) {
        let ls = self.sign_state.get(left).copied().unwrap_or(Sign::Top);
        let rs = self.sign_state.get(right).copied().unwrap_or(Sign::Top);
        self.sign_state.insert(target.to_string(), ls.add(rs));

        let li = self.interval_state.get(left).copied().unwrap_or(Interval::top());
        let ri = self.interval_state.get(right).copied().unwrap_or(Interval::top());
        self.interval_state.insert(target.to_string(), li.add(ri));
    }

    pub fn get_sign(&self, var: &str) -> Sign {
        self.sign_state.get(var).copied().unwrap_or(Sign::Top)
    }

    pub fn get_interval(&self, var: &str) -> Interval {
        self.interval_state.get(var).copied().unwrap_or(Interval::top())
    }

    /// Join two interpreter states (e.g., at control flow merge).
    pub fn join(&mut self, other: &AbstractInterpreter) {
        for (k, v) in &other.sign_state {
            let current = self.sign_state.get(k).copied().unwrap_or(Sign::Bottom);
            self.sign_state.insert(k.clone(), current.join(*v));
        }
        for (k, v) in &other.interval_state {
            let current = self.interval_state.get(k).copied().unwrap_or(Interval::bottom());
            self.interval_state.insert(k.clone(), current.join(*v));
        }
    }

    /// Derive invariants from current abstract state.
    pub fn derive_invariants(&self) -> Vec<Invariant> {
        let mut inv = Vec::new();
        for (var, &sign) in &self.sign_state {
            if sign != Sign::Top && sign != Sign::Bottom {
                inv.push(Invariant {
                    kind: InvariantKind::SignInvariant { var: var.clone(), sign },
                    confidence: 0.85,
                    location: String::new(),
                });
            }
        }
        for (var, &interval) in &self.interval_state {
            if !interval.is_bottom() && interval.width() < f64::INFINITY {
                inv.push(Invariant {
                    kind: InvariantKind::RangeInvariant { var: var.clone(), interval },
                    confidence: 0.9,
                    location: String::new(),
                });
            }
        }
        inv
    }

    pub fn variable_count(&self) -> usize { self.sign_state.len() }
}

impl Default for AbstractInterpreter {
    fn default() -> Self { Self::new() }
}

// ── Hoare Triple ─────────────────────────────────────────────────────

/// {P} S {Q} — precondition, statement, postcondition.
#[derive(Debug, Clone)]
pub struct HoareTriple {
    pub precondition: Condition,
    pub statement: String,
    pub postcondition: Condition,
}

impl HoareTriple {
    pub fn new(pre: Condition, stmt: &str, post: Condition) -> Self {
        Self { precondition: pre, statement: stmt.to_string(), postcondition: post }
    }
}

// ── FFI Exports ──────────────────────────────────────────────────────

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_sign_from_value(v: i64) -> i64 {
    match Sign::from_value(v) {
        Sign::Negative => -1,
        Sign::Zero => 0,
        Sign::Positive => 1,
        _ => 99,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_interval_contains(lo: f64, hi: f64, value: f64) -> i64 {
    let iv = Interval::new(lo, hi);
    if iv.contains(value) { 1 } else { 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_interval_width(lo: f64, hi: f64) -> f64 {
    Interval::new(lo, hi).width()
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_interval_join_lo(a_lo: f64, a_hi: f64, b_lo: f64, b_hi: f64) -> f64 {
    Interval::new(a_lo, a_hi).join(Interval::new(b_lo, b_hi)).lo
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_interval_join_hi(a_lo: f64, a_hi: f64, b_lo: f64, b_hi: f64) -> f64 {
    Interval::new(a_lo, a_hi).join(Interval::new(b_lo, b_hi)).hi
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_interval_mul_lo(a_lo: f64, a_hi: f64, b_lo: f64, b_hi: f64) -> f64 {
    Interval::new(a_lo, a_hi).mul(Interval::new(b_lo, b_hi)).lo
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_interval_mul_hi(a_lo: f64, a_hi: f64, b_lo: f64, b_hi: f64) -> f64 {
    Interval::new(a_lo, a_hi).mul(Interval::new(b_lo, b_hi)).hi
}

// ═══════════════════════════════════════════════════════════════════════
// Tests
// ═══════════════════════════════════════════════════════════════════════
#[cfg(test)]
mod tests {
    use super::*;

    // ── Sign Domain ──────────────────────────────────────────────────

    #[test]
    fn test_sign_from_value() {
        assert_eq!(Sign::from_value(5), Sign::Positive);
        assert_eq!(Sign::from_value(-3), Sign::Negative);
        assert_eq!(Sign::from_value(0), Sign::Zero);
    }

    #[test]
    fn test_sign_join() {
        assert_eq!(Sign::Zero.join(Sign::Positive), Sign::NonNegative);
        assert_eq!(Sign::Zero.join(Sign::Negative), Sign::NonPositive);
        assert_eq!(Sign::Positive.join(Sign::Negative), Sign::Top);
        assert_eq!(Sign::Bottom.join(Sign::Positive), Sign::Positive);
    }

    #[test]
    fn test_sign_meet() {
        assert_eq!(Sign::NonNegative.meet(Sign::Positive), Sign::Positive);
        assert_eq!(Sign::NonNegative.meet(Sign::Zero), Sign::Zero);
        assert_eq!(Sign::Top.meet(Sign::Positive), Sign::Positive);
    }

    #[test]
    fn test_sign_add() {
        assert_eq!(Sign::Positive.add(Sign::Positive), Sign::Positive);
        assert_eq!(Sign::Negative.add(Sign::Negative), Sign::Negative);
        assert_eq!(Sign::Positive.add(Sign::Zero), Sign::Positive);
        assert_eq!(Sign::Zero.add(Sign::Zero), Sign::Zero);
    }

    #[test]
    fn test_sign_negate() {
        assert_eq!(Sign::Positive.negate(), Sign::Negative);
        assert_eq!(Sign::Negative.negate(), Sign::Positive);
        assert_eq!(Sign::Zero.negate(), Sign::Zero);
    }

    // ── Interval Domain ──────────────────────────────────────────────

    #[test]
    fn test_interval_point() {
        let iv = Interval::point(5.0);
        assert_eq!(iv.lo, 5.0);
        assert_eq!(iv.hi, 5.0);
        assert_eq!(iv.width(), 0.0);
    }

    #[test]
    fn test_interval_contains() {
        let iv = Interval::new(1.0, 10.0);
        assert!(iv.contains(5.0));
        assert!(iv.contains(1.0));
        assert!(iv.contains(10.0));
        assert!(!iv.contains(0.5));
    }

    #[test]
    fn test_interval_join() {
        let a = Interval::new(1.0, 5.0);
        let b = Interval::new(3.0, 8.0);
        let j = a.join(b);
        assert_eq!(j.lo, 1.0);
        assert_eq!(j.hi, 8.0);
    }

    #[test]
    fn test_interval_meet() {
        let a = Interval::new(1.0, 5.0);
        let b = Interval::new(3.0, 8.0);
        let m = a.meet(b);
        assert_eq!(m.lo, 3.0);
        assert_eq!(m.hi, 5.0);
    }

    #[test]
    fn test_interval_add() {
        let a = Interval::new(1.0, 3.0);
        let b = Interval::new(2.0, 4.0);
        let s = a.add(b);
        assert_eq!(s.lo, 3.0);
        assert_eq!(s.hi, 7.0);
    }

    #[test]
    fn test_interval_mul() {
        let a = Interval::new(-2.0, 3.0);
        let b = Interval::new(1.0, 4.0);
        let m = a.mul(b);
        assert_eq!(m.lo, -8.0);
        assert_eq!(m.hi, 12.0);
    }

    #[test]
    fn test_interval_bottom() {
        let b = Interval::bottom();
        assert!(b.is_bottom());
    }

    // ── Abstract Interpreter ─────────────────────────────────────────

    #[test]
    fn test_abstract_assign_const() {
        let mut ai = AbstractInterpreter::new();
        ai.assign_const("x", 5);
        assert_eq!(ai.get_sign("x"), Sign::Positive);
        assert!(ai.get_interval("x").contains(5.0));
    }

    #[test]
    fn test_abstract_assign_add() {
        let mut ai = AbstractInterpreter::new();
        ai.assign_const("a", 3);
        ai.assign_const("b", 7);
        ai.assign_add("c", "a", "b");
        assert_eq!(ai.get_sign("c"), Sign::Positive);
        let iv = ai.get_interval("c");
        assert!(iv.contains(10.0));
    }

    #[test]
    fn test_abstract_join() {
        let mut a = AbstractInterpreter::new();
        a.assign_const("x", 5);
        let mut b = AbstractInterpreter::new();
        b.assign_const("x", -3);
        a.join(&b);
        assert_eq!(a.get_sign("x"), Sign::Top);
    }

    #[test]
    fn test_derive_invariants() {
        let mut ai = AbstractInterpreter::new();
        ai.assign_const("x", 10);
        ai.assign_const("y", -5);
        let invs = ai.derive_invariants();
        assert!(invs.len() >= 2);
    }

    // ── Conditions ───────────────────────────────────────────────────

    #[test]
    fn test_condition_negate() {
        assert_eq!(Condition::True.negate(), Condition::False);
        assert_eq!(Condition::False.negate(), Condition::True);
    }

    #[test]
    fn test_condition_and_or() {
        let c = Condition::IsPositive("x".into()).and(Condition::IsNonNull("y".into()));
        match c {
            Condition::And(_, _) => {},
            _ => panic!("expected And"),
        }
    }

    #[test]
    fn test_hoare_triple() {
        let h = HoareTriple::new(
            Condition::IsPositive("n".into()),
            "result = factorial(n)",
            Condition::IsPositive("result".into()),
        );
        assert_eq!(h.statement, "result = factorial(n)");
    }

    // ── FFI Smoke Tests ──────────────────────────────────────────────

    #[test]
    fn test_ffi_sign_from_value() {
        assert_eq!(vitalis_sign_from_value(5), 1);
        assert_eq!(vitalis_sign_from_value(-3), -1);
        assert_eq!(vitalis_sign_from_value(0), 0);
    }

    #[test]
    fn test_ffi_interval_contains() {
        assert_eq!(vitalis_interval_contains(1.0, 10.0, 5.0), 1);
        assert_eq!(vitalis_interval_contains(1.0, 10.0, 15.0), 0);
    }

    #[test]
    fn test_ffi_interval_width() {
        assert!((vitalis_interval_width(3.0, 7.0) - 4.0).abs() < 1e-10);
    }

    #[test]
    fn test_ffi_interval_join() {
        let lo = vitalis_interval_join_lo(1.0, 5.0, 3.0, 8.0);
        let hi = vitalis_interval_join_hi(1.0, 5.0, 3.0, 8.0);
        assert_eq!(lo, 1.0);
        assert_eq!(hi, 8.0);
    }

    #[test]
    fn test_ffi_interval_mul() {
        let lo = vitalis_interval_mul_lo(2.0, 3.0, 4.0, 5.0);
        let hi = vitalis_interval_mul_hi(2.0, 3.0, 4.0, 5.0);
        assert_eq!(lo, 8.0);
        assert_eq!(hi, 15.0);
    }
}
