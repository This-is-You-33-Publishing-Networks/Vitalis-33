fn test_i64() -> i64 {
    let mut flag = 0
    let mut k = 0
    while k < 5 {
        if k == 3 { flag = 1 }
        k = k + 1
    }
    assert_eq(flag, 1)
    println_str("i64 assignment ok")
    0
}

fn test_bool() -> i64 {
    let mut flag = false
    let mut k = 0
    while k < 5 {
        if k == 3 { flag = true }
        k = k + 1
    }
    assert_true(flag)
    println_str("bool assignment ok")
    0
}

fn main() -> i64 {
    test_i64()
    test_bool()
    0
}
