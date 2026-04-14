// Vitalis — Modules
//
// Demonstrates organizing code into modules with path-based calls.

module math {
    fn square(x: i64) -> i64 {
        x * x
    }

    fn cube(x: i64) -> i64 {
        x * x * x
    }
}

fn main() -> i64 {
    let a = math::square(5);    // 25
    let b = math::cube(3);      // 27
    a + b                        // 52
}
