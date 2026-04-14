//! Symbolic Math — v382
//! Computer algebra system with symbolic expressions, simplification, differentiation, and evaluation.

use std::sync::{LazyLock, Mutex};
use std::collections::HashMap;
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum SymExpr {
    Const(f64),
    Var(String),
    Add(Box<SymExpr>, Box<SymExpr>),
    Sub(Box<SymExpr>, Box<SymExpr>),
    Mul(Box<SymExpr>, Box<SymExpr>),
    Div(Box<SymExpr>, Box<SymExpr>),
    Pow(Box<SymExpr>, Box<SymExpr>),
    Neg(Box<SymExpr>),
    Sin(Box<SymExpr>),
    Cos(Box<SymExpr>),
    Ln(Box<SymExpr>),
    Exp(Box<SymExpr>),
}

impl fmt::Display for SymExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SymExpr::Const(v) => write!(f, "{v}"),
            SymExpr::Var(name) => write!(f, "{name}"),
            SymExpr::Add(a, b) => write!(f, "({a} + {b})"),
            SymExpr::Sub(a, b) => write!(f, "({a} - {b})"),
            SymExpr::Mul(a, b) => write!(f, "({a} * {b})"),
            SymExpr::Div(a, b) => write!(f, "({a} / {b})"),
            SymExpr::Pow(a, b) => write!(f, "({a} ^ {b})"),
            SymExpr::Neg(a) => write!(f, "(-{a})"),
            SymExpr::Sin(a) => write!(f, "sin({a})"),
            SymExpr::Cos(a) => write!(f, "cos({a})"),
            SymExpr::Ln(a) => write!(f, "ln({a})"),
            SymExpr::Exp(a) => write!(f, "exp({a})"),
        }
    }
}

impl SymExpr {
    pub fn constant(v: f64) -> Self { SymExpr::Const(v) }
    pub fn var(name: &str) -> Self { SymExpr::Var(name.to_string()) }

    pub fn add(self, other: Self) -> Self {
        SymExpr::Add(Box::new(self), Box::new(other))
    }
    pub fn sub(self, other: Self) -> Self {
        SymExpr::Sub(Box::new(self), Box::new(other))
    }
    pub fn mul(self, other: Self) -> Self {
        SymExpr::Mul(Box::new(self), Box::new(other))
    }
    pub fn div(self, other: Self) -> Self {
        SymExpr::Div(Box::new(self), Box::new(other))
    }
    pub fn pow(self, exp: Self) -> Self {
        SymExpr::Pow(Box::new(self), Box::new(exp))
    }
    pub fn neg(self) -> Self {
        SymExpr::Neg(Box::new(self))
    }

    /// Evaluate expression with variable bindings.
    pub fn eval(&self, vars: &HashMap<String, f64>) -> Option<f64> {
        match self {
            SymExpr::Const(v) => Some(*v),
            SymExpr::Var(name) => vars.get(name).copied(),
            SymExpr::Add(a, b) => Some(a.eval(vars)? + b.eval(vars)?),
            SymExpr::Sub(a, b) => Some(a.eval(vars)? - b.eval(vars)?),
            SymExpr::Mul(a, b) => Some(a.eval(vars)? * b.eval(vars)?),
            SymExpr::Div(a, b) => {
                let bv = b.eval(vars)?;
                if bv == 0.0 { None } else { Some(a.eval(vars)? / bv) }
            }
            SymExpr::Pow(a, b) => Some(a.eval(vars)?.powf(b.eval(vars)?)),
            SymExpr::Neg(a) => Some(-a.eval(vars)?),
            SymExpr::Sin(a) => Some(a.eval(vars)?.sin()),
            SymExpr::Cos(a) => Some(a.eval(vars)?.cos()),
            SymExpr::Ln(a) => {
                let v = a.eval(vars)?;
                if v > 0.0 { Some(v.ln()) } else { None }
            }
            SymExpr::Exp(a) => Some(a.eval(vars)?.exp()),
        }
    }

    /// Symbolic differentiation with respect to a variable.
    pub fn diff(&self, var: &str) -> SymExpr {
        match self {
            SymExpr::Const(_) => SymExpr::Const(0.0),
            SymExpr::Var(name) => {
                if name == var { SymExpr::Const(1.0) } else { SymExpr::Const(0.0) }
            }
            SymExpr::Add(a, b) => a.diff(var).add(b.diff(var)),
            SymExpr::Sub(a, b) => a.diff(var).sub(b.diff(var)),
            SymExpr::Mul(a, b) => {
                // Product rule: d(a*b) = a'*b + a*b'
                a.diff(var).mul(*b.clone()).add((*a.clone()).mul(b.diff(var)))
            }
            SymExpr::Div(a, b) => {
                // Quotient rule: d(a/b) = (a'*b - a*b') / b^2
                let num = a.diff(var).mul(*b.clone()).sub((*a.clone()).mul(b.diff(var)));
                let den = (*b.clone()).pow(SymExpr::Const(2.0));
                num.div(den)
            }
            SymExpr::Pow(base, exp) => {
                if let SymExpr::Const(n) = **exp {
                    // Power rule: d(x^n) = n * x^(n-1) * x'
                    SymExpr::Const(n)
                        .mul((*base.clone()).pow(SymExpr::Const(n - 1.0)))
                        .mul(base.diff(var))
                } else {
                    // General: d(a^b) = a^b * (b' * ln(a) + b * a'/a)
                    let term = self.clone();
                    let part1 = exp.diff(var).mul(SymExpr::Ln(base.clone()));
                    let part2 = (*exp.clone()).mul(base.diff(var).div(*base.clone()));
                    term.mul(part1.add(part2))
                }
            }
            SymExpr::Neg(a) => a.diff(var).neg(),
            SymExpr::Sin(a) => {
                // d(sin(a)) = cos(a) * a'
                SymExpr::Cos(a.clone()).mul(a.diff(var))
            }
            SymExpr::Cos(a) => {
                // d(cos(a)) = -sin(a) * a'
                SymExpr::Sin(a.clone()).neg().mul(a.diff(var))
            }
            SymExpr::Ln(a) => {
                // d(ln(a)) = a'/a
                a.diff(var).div(*a.clone())
            }
            SymExpr::Exp(a) => {
                // d(exp(a)) = exp(a) * a'
                SymExpr::Exp(a.clone()).mul(a.diff(var))
            }
        }
    }

    /// Simplify expression algebraically.
    pub fn simplify(&self) -> SymExpr {
        match self {
            SymExpr::Add(a, b) => {
                let a = a.simplify();
                let b = b.simplify();
                match (&a, &b) {
                    (SymExpr::Const(0.0), _) => b,
                    (_, SymExpr::Const(0.0)) => a,
                    (SymExpr::Const(x), SymExpr::Const(y)) => SymExpr::Const(x + y),
                    _ => a.add(b),
                }
            }
            SymExpr::Sub(a, b) => {
                let a = a.simplify();
                let b = b.simplify();
                match (&a, &b) {
                    (_, SymExpr::Const(0.0)) => a,
                    (SymExpr::Const(x), SymExpr::Const(y)) => SymExpr::Const(x - y),
                    _ if a == b => SymExpr::Const(0.0),
                    _ => a.sub(b),
                }
            }
            SymExpr::Mul(a, b) => {
                let a = a.simplify();
                let b = b.simplify();
                match (&a, &b) {
                    (SymExpr::Const(0.0), _) | (_, SymExpr::Const(0.0)) => SymExpr::Const(0.0),
                    (SymExpr::Const(c), _) if *c == 1.0 => b,
                    (_, SymExpr::Const(c)) if *c == 1.0 => a,
                    (SymExpr::Const(x), SymExpr::Const(y)) => SymExpr::Const(x * y),
                    _ => a.mul(b),
                }
            }
            SymExpr::Div(a, b) => {
                let a = a.simplify();
                let b = b.simplify();
                match (&a, &b) {
                    (SymExpr::Const(0.0), _) => SymExpr::Const(0.0),
                    (_, SymExpr::Const(c)) if *c == 1.0 => a,
                    (SymExpr::Const(x), SymExpr::Const(y)) if *y != 0.0 => SymExpr::Const(x / y),
                    _ if a == b => SymExpr::Const(1.0),
                    _ => a.div(b),
                }
            }
            SymExpr::Pow(a, b) => {
                let a = a.simplify();
                let b = b.simplify();
                match (&a, &b) {
                    (_, SymExpr::Const(0.0)) => SymExpr::Const(1.0),
                    (_, SymExpr::Const(c)) if *c == 1.0 => a,
                    (SymExpr::Const(x), SymExpr::Const(y)) => SymExpr::Const(x.powf(*y)),
                    _ => a.pow(b),
                }
            }
            SymExpr::Neg(a) => {
                let a = a.simplify();
                match &a {
                    SymExpr::Const(v) => SymExpr::Const(-v),
                    SymExpr::Neg(inner) => *inner.clone(),
                    _ => a.neg(),
                }
            }
            other => other.clone(),
        }
    }

    /// Substitute a variable with an expression.
    pub fn substitute(&self, var: &str, replacement: &SymExpr) -> SymExpr {
        match self {
            SymExpr::Var(name) if name == var => replacement.clone(),
            SymExpr::Var(_) | SymExpr::Const(_) => self.clone(),
            SymExpr::Add(a, b) => a.substitute(var, replacement).add(b.substitute(var, replacement)),
            SymExpr::Sub(a, b) => a.substitute(var, replacement).sub(b.substitute(var, replacement)),
            SymExpr::Mul(a, b) => a.substitute(var, replacement).mul(b.substitute(var, replacement)),
            SymExpr::Div(a, b) => a.substitute(var, replacement).div(b.substitute(var, replacement)),
            SymExpr::Pow(a, b) => a.substitute(var, replacement).pow(b.substitute(var, replacement)),
            SymExpr::Neg(a) => a.substitute(var, replacement).neg(),
            SymExpr::Sin(a) => SymExpr::Sin(Box::new(a.substitute(var, replacement))),
            SymExpr::Cos(a) => SymExpr::Cos(Box::new(a.substitute(var, replacement))),
            SymExpr::Ln(a) => SymExpr::Ln(Box::new(a.substitute(var, replacement))),
            SymExpr::Exp(a) => SymExpr::Exp(Box::new(a.substitute(var, replacement))),
        }
    }

    /// Polynomial degree (counts max power of variable).
    pub fn degree(&self, var: &str) -> i64 {
        match self {
            SymExpr::Const(_) => 0,
            SymExpr::Var(name) if name == var => 1,
            SymExpr::Var(_) => 0,
            SymExpr::Add(a, b) | SymExpr::Sub(a, b) => a.degree(var).max(b.degree(var)),
            SymExpr::Mul(a, b) => a.degree(var) + b.degree(var),
            SymExpr::Pow(base, exp) => {
                if let SymExpr::Const(n) = **exp {
                    base.degree(var) * n as i64
                } else {
                    -1 // Non-polynomial
                }
            }
            SymExpr::Neg(a) => a.degree(var),
            _ => -1,
        }
    }

    /// Expand products: (a+b)*c → a*c + b*c.
    pub fn expand(&self) -> SymExpr {
        match self {
            SymExpr::Mul(a, b) => {
                let a = a.expand();
                let b = b.expand();
                match &a {
                    SymExpr::Add(l, r) => {
                        l.clone().mul(b.clone()).expand().add(r.clone().mul(b).expand())
                    }
                    _ => match &b {
                        SymExpr::Add(l, r) => {
                            a.clone().mul(*l.clone()).expand().add(a.mul(*r.clone()).expand())
                        }
                        _ => a.mul(b),
                    }
                }
            }
            SymExpr::Add(a, b) => a.expand().add(b.expand()),
            SymExpr::Sub(a, b) => a.expand().sub(b.expand()),
            SymExpr::Neg(a) => a.expand().neg(),
            other => other.clone(),
        }
    }
}

static SYM_STORE: LazyLock<Mutex<HashMap<i64, SymExpr>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
static SYM_NEXT_ID: LazyLock<Mutex<i64>> = LazyLock::new(|| Mutex::new(1));

fn sym_store(expr: SymExpr) -> i64 {
    let mut next = SYM_NEXT_ID.lock().unwrap();
    let id = *next;
    *next += 1;
    SYM_STORE.lock().unwrap().insert(id, expr);
    id
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_sym_parse(kind: i64, val: f64) -> i64 {
    let expr = match kind {
        0 => SymExpr::Const(val),
        1 => SymExpr::Var("x".into()),
        2 => SymExpr::Var("y".into()),
        _ => SymExpr::Const(0.0),
    };
    sym_store(expr)
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_sym_simplify(id: i64) -> i64 {
    let store = SYM_STORE.lock().unwrap();
    if let Some(expr) = store.get(&id) {
        let simplified = expr.simplify();
        drop(store);
        sym_store(simplified)
    } else {
        -1
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_sym_diff(id: i64) -> i64 {
    let store = SYM_STORE.lock().unwrap();
    if let Some(expr) = store.get(&id) {
        let d = expr.diff("x");
        drop(store);
        sym_store(d)
    } else {
        -1
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_sym_eval(id: i64, x_val: f64) -> f64 {
    let store = SYM_STORE.lock().unwrap();
    if let Some(expr) = store.get(&id) {
        let mut vars = HashMap::new();
        vars.insert("x".into(), x_val);
        expr.eval(&vars).unwrap_or(f64::NAN)
    } else {
        f64::NAN
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_sym_add(a: i64, b: i64) -> i64 {
    let store = SYM_STORE.lock().unwrap();
    if let (Some(ea), Some(eb)) = (store.get(&a), store.get(&b)) {
        let result = ea.clone().add(eb.clone());
        drop(store);
        sym_store(result)
    } else {
        -1
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_sym_mul(a: i64, b: i64) -> i64 {
    let store = SYM_STORE.lock().unwrap();
    if let (Some(ea), Some(eb)) = (store.get(&a), store.get(&b)) {
        let result = ea.clone().mul(eb.clone());
        drop(store);
        sym_store(result)
    } else {
        -1
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_sym_degree(id: i64) -> i64 {
    let store = SYM_STORE.lock().unwrap();
    if let Some(expr) = store.get(&id) {
        expr.degree("x")
    } else {
        -1
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_sym_substitute(id: i64, val: f64) -> i64 {
    let store = SYM_STORE.lock().unwrap();
    if let Some(expr) = store.get(&id) {
        let result = expr.substitute("x", &SymExpr::Const(val));
        drop(store);
        sym_store(result)
    } else {
        -1
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_sym_expand(id: i64) -> i64 {
    let store = SYM_STORE.lock().unwrap();
    if let Some(expr) = store.get(&id) {
        let expanded = expr.expand();
        drop(store);
        sym_store(expanded)
    } else {
        -1
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_sym_factor(id: i64) -> i64 {
    // Factor by simplification as basic factoring
    let store = SYM_STORE.lock().unwrap();
    if let Some(expr) = store.get(&id) {
        let factored = expr.simplify();
        drop(store);
        sym_store(factored)
    } else {
        -1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_const_eval() {
        let e = SymExpr::Const(42.0);
        assert_eq!(e.eval(&HashMap::new()), Some(42.0));
    }

    #[test]
    fn test_var_eval() {
        let e = SymExpr::Var("x".into());
        let mut vars = HashMap::new();
        vars.insert("x".into(), 5.0);
        assert_eq!(e.eval(&vars), Some(5.0));
    }

    #[test]
    fn test_var_missing() {
        let e = SymExpr::Var("x".into());
        assert_eq!(e.eval(&HashMap::new()), None);
    }

    #[test]
    fn test_add_eval() {
        let e = SymExpr::Const(3.0).add(SymExpr::Const(4.0));
        assert_eq!(e.eval(&HashMap::new()), Some(7.0));
    }

    #[test]
    fn test_sub_eval() {
        let e = SymExpr::Const(10.0).sub(SymExpr::Const(3.0));
        assert_eq!(e.eval(&HashMap::new()), Some(7.0));
    }

    #[test]
    fn test_mul_eval() {
        let e = SymExpr::Const(3.0).mul(SymExpr::Const(5.0));
        assert_eq!(e.eval(&HashMap::new()), Some(15.0));
    }

    #[test]
    fn test_div_eval() {
        let e = SymExpr::Const(10.0).div(SymExpr::Const(2.0));
        assert_eq!(e.eval(&HashMap::new()), Some(5.0));
    }

    #[test]
    fn test_div_by_zero() {
        let e = SymExpr::Const(1.0).div(SymExpr::Const(0.0));
        assert_eq!(e.eval(&HashMap::new()), None);
    }

    #[test]
    fn test_pow_eval() {
        let e = SymExpr::Const(2.0).pow(SymExpr::Const(3.0));
        assert_eq!(e.eval(&HashMap::new()), Some(8.0));
    }

    #[test]
    fn test_diff_const() {
        let e = SymExpr::Const(5.0);
        let d = e.diff("x").simplify();
        assert_eq!(d, SymExpr::Const(0.0));
    }

    #[test]
    fn test_diff_var() {
        let e = SymExpr::Var("x".into());
        let d = e.diff("x");
        assert_eq!(d, SymExpr::Const(1.0));
    }

    #[test]
    fn test_diff_other_var() {
        let e = SymExpr::Var("y".into());
        let d = e.diff("x");
        assert_eq!(d, SymExpr::Const(0.0));
    }

    #[test]
    fn test_diff_sum() {
        // d/dx (x + 5) = 1
        let e = SymExpr::Var("x".into()).add(SymExpr::Const(5.0));
        let d = e.diff("x").simplify();
        let mut vars = HashMap::new();
        vars.insert("x".into(), 0.0);
        assert_eq!(d.eval(&vars), Some(1.0));
    }

    #[test]
    fn test_diff_power() {
        // d/dx (x^3) = 3x^2
        let e = SymExpr::Var("x".into()).pow(SymExpr::Const(3.0));
        let d = e.diff("x").simplify();
        let mut vars = HashMap::new();
        vars.insert("x".into(), 2.0);
        // 3 * 2^2 = 12
        assert_eq!(d.eval(&vars), Some(12.0));
    }

    #[test]
    fn test_simplify_add_zero() {
        let e = SymExpr::Const(0.0).add(SymExpr::Var("x".into()));
        let s = e.simplify();
        assert_eq!(s, SymExpr::Var("x".into()));
    }

    #[test]
    fn test_simplify_mul_zero() {
        let e = SymExpr::Const(0.0).mul(SymExpr::Var("x".into()));
        let s = e.simplify();
        assert_eq!(s, SymExpr::Const(0.0));
    }

    #[test]
    fn test_simplify_mul_one() {
        let e = SymExpr::Const(1.0).mul(SymExpr::Var("x".into()));
        let s = e.simplify();
        assert_eq!(s, SymExpr::Var("x".into()));
    }

    #[test]
    fn test_simplify_sub_self() {
        let x = SymExpr::Var("x".into());
        let e = x.clone().sub(x);
        let s = e.simplify();
        assert_eq!(s, SymExpr::Const(0.0));
    }

    #[test]
    fn test_substitute() {
        let e = SymExpr::Var("x".into()).add(SymExpr::Const(1.0));
        let s = e.substitute("x", &SymExpr::Const(5.0));
        assert_eq!(s.eval(&HashMap::new()), Some(6.0));
    }

    #[test]
    fn test_degree_linear() {
        let e = SymExpr::Var("x".into());
        assert_eq!(e.degree("x"), 1);
    }

    #[test]
    fn test_degree_quadratic() {
        let e = SymExpr::Var("x".into()).pow(SymExpr::Const(2.0));
        assert_eq!(e.degree("x"), 2);
    }

    #[test]
    fn test_degree_constant() {
        assert_eq!(SymExpr::Const(5.0).degree("x"), 0);
    }

    #[test]
    fn test_expand() {
        // (x + 1) * (x + 2) → x*x + x*2 + 1*x + 1*2
        let e = SymExpr::Var("x".into()).add(SymExpr::Const(1.0))
            .mul(SymExpr::Var("x".into()).add(SymExpr::Const(2.0)));
        let expanded = e.expand();
        let mut vars = HashMap::new();
        vars.insert("x".into(), 3.0);
        // (3+1)*(3+2) = 20
        assert_eq!(expanded.eval(&vars), Some(20.0));
    }

    #[test]
    fn test_neg() {
        let e = SymExpr::Const(5.0).neg();
        assert_eq!(e.simplify(), SymExpr::Const(-5.0));
    }

    #[test]
    fn test_double_neg() {
        let e = SymExpr::Var("x".into()).neg().neg();
        let s = e.simplify();
        assert_eq!(s, SymExpr::Var("x".into()));
    }

    #[test]
    fn test_display() {
        let e = SymExpr::Var("x".into()).add(SymExpr::Const(1.0));
        let s = format!("{}", e);
        assert!(s.contains("x"));
        assert!(s.contains("1"));
    }
}
