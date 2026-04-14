// Vitalis — Error Handling
//
// Demonstrates try/catch/throw for error recovery.

fn safe_divide(a: i64, b: i64) -> i64 {
    try {
        if b == 0 {
            throw "division by zero";
        }
        a / b
    } catch e {
        -1
    }
}

fn main() -> i64 {
    let ok = safe_divide(100, 5);     // 20
    let err = safe_divide(100, 0);    // -1 (caught)
    ok + err                           // 19
}
