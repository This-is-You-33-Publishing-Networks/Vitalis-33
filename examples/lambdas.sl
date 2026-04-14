// Vitalis — Lambdas and Closures
//
// Demonstrates anonymous functions and variable capture.

fn apply(f: fn(i64) -> i64, x: i64) -> i64 {
    f(x)
}

fn main() -> i64 {
    // Simple lambda
    let double = |x: i64| -> i64 { x * 2 };
    let a = double(21);  // 42

    // Lambda passed to higher-order function
    let triple = |x: i64| -> i64 { x * 3 };
    let b = apply(triple, 10);  // 30

    a + b  // 72
}
