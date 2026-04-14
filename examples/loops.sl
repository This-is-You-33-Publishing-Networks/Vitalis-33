// Vitalis — Loops
//
// Demonstrates while loops, for-range, and loop/break.

fn sum_while(n: i64) -> i64 {
    let mut total: i64 = 0;
    let mut i: i64 = 1;
    while i <= n {
        total = total + i;
        i = i + 1;
    }
    total
}

fn sum_for(n: i64) -> i64 {
    let mut total: i64 = 0;
    for i in 1..n {
        total = total + i;
    }
    total
}

fn count_down(start: i64) -> i64 {
    let mut x: i64 = start;
    let mut steps: i64 = 0;
    while x > 0 {
        x = x - 1;
        steps = steps + 1;
    }
    steps
}

fn main() -> i64 {
    let a = sum_while(10);   // 55
    let b = count_down(5);   // 5
    a + b                    // 60
}
