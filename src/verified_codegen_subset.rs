//! v94 Verified codegen subset with interpreter cross-check.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Inst {
    LoadConst(i64),
    Add,
    Mul,
    Return,
}

pub fn emit_stack_machine(expr: &str) -> Vec<Inst> {
    // Tiny subset parser: supports "a+b" and "a*b" with integer literals.
    if let Some((a, b)) = expr.split_once('+') {
        return vec![
            Inst::LoadConst(a.trim().parse().unwrap_or(0)),
            Inst::LoadConst(b.trim().parse().unwrap_or(0)),
            Inst::Add,
            Inst::Return,
        ];
    }
    if let Some((a, b)) = expr.split_once('*') {
        return vec![
            Inst::LoadConst(a.trim().parse().unwrap_or(0)),
            Inst::LoadConst(b.trim().parse().unwrap_or(0)),
            Inst::Mul,
            Inst::Return,
        ];
    }
    vec![Inst::LoadConst(expr.trim().parse().unwrap_or(0)), Inst::Return]
}

pub fn run_stack_machine(code: &[Inst]) -> Option<i64> {
    let mut stack: Vec<i64> = Vec::new();
    for inst in code {
        match inst {
            Inst::LoadConst(v) => stack.push(*v),
            Inst::Add => {
                let b = stack.pop()?;
                let a = stack.pop()?;
                stack.push(a + b);
            }
            Inst::Mul => {
                let b = stack.pop()?;
                let a = stack.pop()?;
                stack.push(a * b);
            }
            Inst::Return => return stack.pop(),
        }
    }
    None
}

pub fn interpret_expr(expr: &str) -> i64 {
    if let Some((a, b)) = expr.split_once('+') {
        return a.trim().parse::<i64>().unwrap_or(0) + b.trim().parse::<i64>().unwrap_or(0);
    }
    if let Some((a, b)) = expr.split_once('*') {
        return a.trim().parse::<i64>().unwrap_or(0) * b.trim().parse::<i64>().unwrap_or(0);
    }
    expr.trim().parse::<i64>().unwrap_or(0)
}

pub fn verify_codegen_equivalence(expr: &str) -> bool {
    run_stack_machine(&emit_stack_machine(expr)) == Some(interpret_expr(expr))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_codegen_subset_add() {
        assert!(verify_codegen_equivalence("7 + 5"));
    }

    #[test]
    fn test_codegen_subset_mul() {
        assert!(verify_codegen_equivalence("6 * 9"));
    }

    #[test]
    fn test_codegen_subset_literal() {
        assert!(verify_codegen_equivalence("42"));
    }
}
