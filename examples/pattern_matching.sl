// Vitalis — Pattern Matching
//
// Demonstrates the match expression with various patterns.

fn classify(x: i64) -> i64 {
    match x {
        0 => 0,
        1 => 100,
        2 => 200,
        _ => 999,
    }
}

fn sign(x: i64) -> i64 {
    if x > 0 {
        1
    } else {
        if x < 0 {
            -1
        } else {
            0
        }
    }
}

fn main() -> i64 {
    let a = classify(0);
    let b = classify(2);
    let c = classify(99);
    // 0 + 200 + 999 = 1199
    a + b + c
}
