//! v93 Verified optimizer pass equivalence harness.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Op {
    Const(i64),
    Add(usize, usize),
    Mul(usize, usize),
    Copy(usize),
}

#[derive(Debug, Clone)]
pub struct FnIR {
    pub ops: Vec<Op>,
}

pub fn eval(ir: &FnIR) -> Option<i64> {
    let mut vals: Vec<i64> = Vec::new();
    for op in &ir.ops {
        let v = match *op {
            Op::Const(x) => x,
            Op::Add(a, b) => *vals.get(a)? + *vals.get(b)?,
            Op::Mul(a, b) => *vals.get(a)? * *vals.get(b)?,
            Op::Copy(i) => *vals.get(i)?,
        };
        vals.push(v);
    }
    vals.last().copied()
}

pub fn constant_fold(ir: &FnIR) -> FnIR {
    let mut out = ir.clone();
    for i in 0..out.ops.len() {
        match out.ops[i].clone() {
            Op::Add(a, b) => {
                if let (Some(Op::Const(x)), Some(Op::Const(y))) = (out.ops.get(a), out.ops.get(b)) {
                    out.ops[i] = Op::Const(x + y);
                }
            }
            Op::Mul(a, b) => {
                if let (Some(Op::Const(x)), Some(Op::Const(y))) = (out.ops.get(a), out.ops.get(b)) {
                    out.ops[i] = Op::Const(x * y);
                }
            }
            _ => {}
        }
    }
    out
}

pub fn verify_equivalence(before: &FnIR, after: &FnIR) -> bool {
    eval(before) == eval(after)
}

pub fn random_ir(seed: u64, len: usize) -> FnIR {
    fn next(s: &mut u64) -> u64 {
        *s = s.wrapping_mul(6364136223846793005).wrapping_add(1);
        *s
    }
    let mut s = seed;
    let mut ops = vec![Op::Const((next(&mut s) % 13) as i64)];
    for _ in 1..len.max(2) {
        let r = next(&mut s) % 4;
        let max = ops.len();
        let a = (next(&mut s) as usize) % max;
        let b = (next(&mut s) as usize) % max;
        let op = match r {
            0 => Op::Const((next(&mut s) % 17) as i64),
            1 => Op::Add(a, b),
            2 => Op::Mul(a, b),
            _ => Op::Copy(a),
        };
        ops.push(op);
    }
    FnIR { ops }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fold_equivalence_simple() {
        let ir = FnIR { ops: vec![Op::Const(2), Op::Const(3), Op::Add(0, 1)] };
        let opt = constant_fold(&ir);
        assert!(verify_equivalence(&ir, &opt));
    }

    #[test]
    fn test_randomized_equivalence_sweep() {
        for i in 0..128u64 {
            let ir = random_ir(100 + i, 20);
            let opt = constant_fold(&ir);
            assert!(verify_equivalence(&ir, &opt));
        }
    }
}
