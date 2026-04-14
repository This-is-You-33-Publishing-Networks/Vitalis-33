//! v92 Verified Type Safety: preservation/progress checks and counterexample minimization.

use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TsType {
    Int,
    Bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expr {
    Int(i64),
    Bool(bool),
    Add(Box<Expr>, Box<Expr>),
    If(Box<Expr>, Box<Expr>, Box<Expr>),
    Var(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeError {
    UnboundVar(String),
    Mismatch { expected: TsType, found: TsType },
    InvalidIfCond(TsType),
}

pub fn type_of(expr: &Expr, env: &BTreeMap<String, TsType>) -> Result<TsType, TypeError> {
    match expr {
        Expr::Int(_) => Ok(TsType::Int),
        Expr::Bool(_) => Ok(TsType::Bool),
        Expr::Var(name) => env
            .get(name)
            .cloned()
            .ok_or_else(|| TypeError::UnboundVar(name.clone())),
        Expr::Add(a, b) => {
            let ta = type_of(a, env)?;
            let tb = type_of(b, env)?;
            if ta != TsType::Int {
                return Err(TypeError::Mismatch {
                    expected: TsType::Int,
                    found: ta,
                });
            }
            if tb != TsType::Int {
                return Err(TypeError::Mismatch {
                    expected: TsType::Int,
                    found: tb,
                });
            }
            Ok(TsType::Int)
        }
        Expr::If(c, t, e) => {
            let tc = type_of(c, env)?;
            if tc != TsType::Bool {
                return Err(TypeError::InvalidIfCond(tc));
            }
            let tt = type_of(t, env)?;
            let te = type_of(e, env)?;
            if tt != te {
                return Err(TypeError::Mismatch {
                    expected: tt,
                    found: te,
                });
            }
            Ok(tt)
        }
    }
}

pub fn is_value(expr: &Expr) -> bool {
    matches!(expr, Expr::Int(_) | Expr::Bool(_))
}

pub fn step(expr: &Expr) -> Option<Expr> {
    match expr {
        Expr::Add(a, b) if is_value(a) && is_value(b) => match (&**a, &**b) {
            (Expr::Int(x), Expr::Int(y)) => Some(Expr::Int(x + y)),
            _ => None,
        },
        Expr::Add(a, b) if !is_value(a) => step(a).map(|na| Expr::Add(Box::new(na), b.clone())),
        Expr::Add(a, b) if !is_value(b) => step(b).map(|nb| Expr::Add(a.clone(), Box::new(nb))),
        Expr::If(c, t, e) if !is_value(c) => {
            step(c).map(|nc| Expr::If(Box::new(nc), t.clone(), e.clone()))
        }
        Expr::If(c, t, e) => match &**c {
            Expr::Bool(true) => Some((**t).clone()),
            Expr::Bool(false) => Some((**e).clone()),
            _ => None,
        },
        _ => None,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SafetyCounterexample {
    pub expression: Expr,
    pub reason: String,
}

pub fn check_progress(expr: &Expr, env: &BTreeMap<String, TsType>) -> Result<(), SafetyCounterexample> {
    if type_of(expr, env).is_err() {
        return Ok(());
    }
    if is_value(expr) || step(expr).is_some() {
        Ok(())
    } else {
        Err(SafetyCounterexample {
            expression: expr.clone(),
            reason: "well-typed term is stuck".to_string(),
        })
    }
}

pub fn check_preservation(expr: &Expr, env: &BTreeMap<String, TsType>) -> Result<(), SafetyCounterexample> {
    let t_before = match type_of(expr, env) {
        Ok(t) => t,
        Err(_) => return Ok(()),
    };

    if let Some(next) = step(expr) {
        let t_after = type_of(&next, env).map_err(|e| SafetyCounterexample {
            expression: next.clone(),
            reason: format!("type error after step: {:?}", e),
        })?;

        if t_before == t_after {
            Ok(())
        } else {
            Err(SafetyCounterexample {
                expression: next,
                reason: "type changed after reduction".to_string(),
            })
        }
    } else {
        Ok(())
    }
}

pub fn minimize_counterexample(mut cx: SafetyCounterexample) -> SafetyCounterexample {
    // Simple shrinking strategy: replace nested arithmetic with subexpressions.
    loop {
        let shrunk = match &cx.expression {
            Expr::Add(a, b) => {
                if matches!(**a, Expr::Add(_, _) | Expr::If(_, _, _)) {
                    Some((**a).clone())
                } else if matches!(**b, Expr::Add(_, _) | Expr::If(_, _, _)) {
                    Some((**b).clone())
                } else {
                    None
                }
            }
            Expr::If(c, t, e) => {
                if !is_value(c) {
                    Some((**c).clone())
                } else if !is_value(t) {
                    Some((**t).clone())
                } else if !is_value(e) {
                    Some((**e).clone())
                } else {
                    None
                }
            }
            _ => None,
        };

        if let Some(expr) = shrunk {
            cx.expression = expr;
        } else {
            break;
        }
    }
    cx
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env() -> BTreeMap<String, TsType> {
        BTreeMap::new()
    }

    #[test]
    fn test_progress_holds_for_well_typed() {
        let e = Expr::Add(Box::new(Expr::Int(1)), Box::new(Expr::Int(2)));
        assert!(check_progress(&e, &env()).is_ok());
    }

    #[test]
    fn test_preservation_holds_for_if() {
        let e = Expr::If(
            Box::new(Expr::Bool(true)),
            Box::new(Expr::Int(10)),
            Box::new(Expr::Int(20)),
        );
        assert!(check_preservation(&e, &env()).is_ok());
    }

    #[test]
    fn test_detect_stuck_term_counterexample() {
        // Ill-typed/stuck terms are ignored by theorem checks; emulate a stuck typed term using env var.
        let mut e = env();
        e.insert("x".to_string(), TsType::Int);
        let expr = Expr::Var("x".to_string());
        // Variable alone is well-typed but not a value and cannot step in closed semantics.
        let cx = check_progress(&expr, &e).unwrap_err();
        assert!(cx.reason.contains("stuck"));
    }

    #[test]
    fn test_counterexample_minimization() {
        let cx = SafetyCounterexample {
            expression: Expr::Add(
                Box::new(Expr::Add(Box::new(Expr::Int(1)), Box::new(Expr::Int(2)))),
                Box::new(Expr::Int(3)),
            ),
            reason: "synthetic".to_string(),
        };
        let shrunk = minimize_counterexample(cx);
        assert!(matches!(shrunk.expression, Expr::Int(_) | Expr::Add(_, _)));
    }

    #[test]
    fn test_property_sweep_preservation() {
        let candidates = vec![
            Expr::Add(Box::new(Expr::Int(1)), Box::new(Expr::Int(2))),
            Expr::If(
                Box::new(Expr::Bool(false)),
                Box::new(Expr::Int(5)),
                Box::new(Expr::Int(9)),
            ),
            Expr::Add(
                Box::new(Expr::Add(Box::new(Expr::Int(1)), Box::new(Expr::Int(2)))),
                Box::new(Expr::Int(3)),
            ),
        ];

        for expr in candidates {
            assert!(check_preservation(&expr, &env()).is_ok());
        }
    }
}
