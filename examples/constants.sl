// Vitalis — Constants and Type Aliases
//
// Demonstrates const declarations and type aliases.

const MAX: i64 = 100;
const SCALE: i64 = 3;

type Score = i64;

fn compute(x: Score) -> Score {
    if x > MAX {
        MAX * SCALE
    } else {
        x * SCALE
    }
}

fn main() -> i64 {
    compute(50)   // 50 * 3 = 150
}
