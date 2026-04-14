// Vitalis — Math and Algorithms
//
// Demonstrates common algorithms using stdlib math functions.

fn gcd_impl(a: i64, b: i64) -> i64 {
    if b == 0 {
        a
    } else {
        gcd_impl(b, a % b)
    }
}

fn power(base: i64, exp: i64) -> i64 {
    if exp == 0 {
        1
    } else {
        base * power(base, exp - 1)
    }
}

fn fib(n: i64) -> i64 {
    if n <= 1 {
        n
    } else {
        fib(n - 1) + fib(n - 2)
    }
}

fn main() -> i64 {
    let g = gcd_impl(48, 18);  // 6
    let p = power(2, 10);      // 1024
    let f = fib(10);           // 55
    g + p + f                   // 1085
}
